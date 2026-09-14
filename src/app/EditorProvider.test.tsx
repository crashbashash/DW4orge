import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { BackendProvider } from '../ipc/context';
import { createMockBackend, type MockOptions } from '../ipc/mock';
import { EditorProvider, useEditor } from './EditorProvider';

function Probe() {
  const api = useEditor();
  return (
    <div>
      <span data-testid="info">{api.state.appInfo ? 'loaded' : 'missing'}</span>
      <span data-testid="species">{api.state.session?.view.species ?? 'none'}</span>
      <span data-testid="bit">{api.state.draft?.bit ?? '-'}</span>
      <span data-testid="errors">{api.state.validation.errors.length}</span>
      <span data-testid="error">{api.state.error ?? ''}</span>
      <button onClick={() => void api.openSample()}>sample</button>
      <button onClick={() => void api.newSave({ species: 'Dorumon', name: 'TST', story: null, difficulty: 'auto' })}>
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
    await userEvent.click(screen.getByRole('button', { name: 'edit' }));
    expect(screen.getByTestId('bit').textContent).toBe('7');
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
