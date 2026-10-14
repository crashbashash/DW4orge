import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { BackendProvider } from '../ipc/context';
import { createMockBackend, type MockOptions } from '../ipc/mock';
import { EditorProvider, useEditor } from './EditorProvider';
import { dirty } from './selectors';

function Probe() {
  const api = useEditor();
  return (
    <div>
      <span data-testid="info">{api.state.appInfo ? 'loaded' : 'missing'}</span>
      <span data-testid="species">{api.state.session?.view.species ?? 'none'}</span>
      <span data-testid="bit">{api.state.draft?.bit ?? '-'}</span>
      <span data-testid="errors">{api.state.validation.errors.length}</span>
      <span data-testid="error">{api.state.error ?? ''}</span>
      <span data-testid="dirty">{dirty(api.state) ? 'yes' : 'no'}</span>
      <button onClick={() => void api.open()}>open</button>
      <button onClick={() => void api.openSample()}>sample</button>
      <button onClick={() => void api.newSave({ region: null, species: 'Dorumon', name: 'TST', story: null, difficulty: 'auto' })}>
        new
      </button>
      <button onClick={() => void api.saveAs()}>save as</button>
      <button onClick={() => api.setField({ bit: 7 }, 'bit')}>edit</button>
    </div>
  );
}

function renderProbe(options: MockOptions = {}) {
  const backend = createMockBackend(options);
  render(
    <BackendProvider backend={backend}>
      <EditorProvider>
        <Probe />
      </EditorProvider>
    </BackendProvider>,
  );
  return backend;
}

afterEach(() => {
  vi.useRealTimers();
});

describe('EditorProvider', () => {
  it('loads app_info on mount', async () => {
    renderProbe();
    await waitFor(() => expect(screen.getByTestId('info').textContent).toBe('loaded'));
    expect(screen.getByTestId('species').textContent).toBe('none');
  });

  it('loads the sample save and edits the draft', async () => {
    renderProbe();
    await userEvent.click(screen.getByRole('button', { name: 'sample' }));
    await waitFor(() => expect(screen.getByTestId('species').textContent).toBe('Dorumon'));
    expect(screen.getByTestId('dirty').textContent).toBe('no');
    await userEvent.click(screen.getByRole('button', { name: 'edit' }));
    expect(screen.getByTestId('bit').textContent).toBe('7');
    expect(screen.getByTestId('dirty').textContent).toBe('yes');
  });

  it('marks a new save as unsaved before any edit', async () => {
    renderProbe();
    await userEvent.click(screen.getByRole('button', { name: 'new' }));
    await waitFor(() => expect(screen.getByTestId('species').textContent).toBe('Dorumon'));
    // The file has never been written, so it is unsaved from the start.
    expect(screen.getByTestId('dirty').textContent).toBe('yes');
  });

  it('surfaces a rejected validation as an error', async () => {
    renderProbe({
      validateEdits: () => {
        throw { kind: 'core', variant: 'Io', message: 'disk full' };
      },
    });
    await userEvent.click(screen.getByRole('button', { name: 'sample' }));
    await waitFor(() => expect(screen.getByTestId('error').textContent).toBe('disk full'));
  });

  it('shows validation errors returned as a report', async () => {
    renderProbe({
      validateEdits: () => ({
        errors: [{ path: 'bit', message: 'too big', severity: 'error' }],
        warnings: [],
      }),
    });
    await userEvent.click(screen.getByRole('button', { name: 'sample' }));
    await waitFor(() => expect(screen.getByTestId('errors').textContent).toBe('1'));
  });

  it('offers a .ps2 name for a new save', async () => {
    const backend = renderProbe();
    const pick = vi.spyOn(backend, 'pickSavePath').mockResolvedValue(null);
    await waitFor(() => expect(screen.getByTestId('info').textContent).toBe('loaded'));
    await userEvent.click(screen.getByRole('button', { name: 'new' }));
    await waitFor(() => expect(screen.getByTestId('species').textContent).toBe('Dorumon'));
    await userEvent.click(screen.getByRole('button', { name: 'save as' }));
    expect(pick).toHaveBeenCalledWith('save.ps2');
  });

  it('keeps the open file name on Save As', async () => {
    const backend = renderProbe();
    const pick = vi.spyOn(backend, 'pickSavePath').mockResolvedValue(null);
    await userEvent.click(screen.getByRole('button', { name: 'sample' }));
    await waitFor(() => expect(screen.getByTestId('species').textContent).toBe('Dorumon'));
    await userEvent.click(screen.getByRole('button', { name: 'save as' }));
    expect(pick).toHaveBeenCalledWith('Mcd001.ps2');
  });
});


describe('regional choices', () => {
  it('asks before opening a mixed-region card and retries the selected region', async () => {
    const backend = renderProbe();
    vi.spyOn(backend, 'pickOpenPath').mockResolvedValue('/mixed.ps2');
    const original = backend.openSave;
    const open = vi.spyOn(backend, 'openSave').mockRejectedValueOnce({ kind: 'region_choice', regions: ['usa', 'japan'], message: 'choose a region' }).mockImplementation(original);
    await userEvent.click(screen.getByRole('button', { name: 'open' }));
    await screen.findByRole('dialog', { name: 'Choose save region' });
    await userEvent.click(screen.getByRole('button', { name: 'Japan (Digimon World X)' }));
    await waitFor(() => expect(open).toHaveBeenLastCalledWith('/mixed.ps2', 'japan'));
    expect(screen.getByTestId('error').textContent).toBe('');
  });

  it('canceling region selection retains the current document and writes nothing', async () => {
    const backend = renderProbe();
    await userEvent.click(screen.getByRole('button', { name: 'sample' }));
    vi.spyOn(backend, 'pickOpenPath').mockResolvedValue('/mixed.ps2');
    const open = vi.spyOn(backend, 'openSave').mockRejectedValue({ kind: 'region_choice', regions: ['usa', 'japan'], message: 'choose' });
    const save = vi.spyOn(backend, 'saveAs');
    await userEvent.click(screen.getByRole('button', { name: 'open' }));
    await screen.findByRole('dialog', { name: 'Choose save region' });
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(open).toHaveBeenCalledTimes(1);
    expect(save).not.toHaveBeenCalled();
    expect(screen.getByTestId('species').textContent).toBe('Dorumon');
    expect(screen.getByTestId('error').textContent).toBe('');
  });

  it('asks which region to use when exporting an unknown raw save as a card', async () => {
    const backend = renderProbe();
    vi.spyOn(backend, 'pickOpenPath').mockResolvedValue('/unknown.raw');
    vi.spyOn(backend, 'pickSavePath').mockResolvedValue('/export.ps2');
    const save = vi.spyOn(backend, 'saveAs');
    await userEvent.click(screen.getByRole('button', { name: 'open' }));
    await waitFor(() => expect(screen.getByTestId('species').textContent).toBe('Dorumon'));
    await userEvent.click(screen.getByRole('button', { name: 'save as' }));
    await screen.findByRole('dialog', { name: 'Choose save region' });
    expect(save).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: 'Europe' }));
    await waitFor(() => expect(save).toHaveBeenCalledWith('/export.ps2', expect.any(Object), 'normal', 'europe'));
  });
});
