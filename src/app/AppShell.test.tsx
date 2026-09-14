import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { BackendProvider } from '../ipc/context';
import { createMockBackend } from '../ipc/mock';
import { App } from './App';

function renderApp() {
  return render(
    <BackendProvider backend={createMockBackend()}>
      <App />
    </BackendProvider>,
  );
}

describe('AppShell', () => {
  it('shows an empty state with a way to load the sample save', async () => {
    renderApp();
    expect(screen.getByText(/open a ps2 memory card/i)).toBeTruthy();
    await userEvent.click(await screen.findByRole('button', { name: /load sample save/i }));
    expect((await screen.findAllByText('Dorumon')).length).toBeGreaterThan(0);
  });

  it('lists the six sections once a save is open', async () => {
    renderApp();
    await userEvent.click(await screen.findByRole('button', { name: /load sample save/i }));
    await screen.findAllByText('Dorumon');
    for (const name of ['Character', 'Items', 'Equipment', 'Disks', 'Story', 'Bank']) {
      expect(screen.getByRole('button', { name })).toBeTruthy();
    }
  });

  it('undoes a character edit with Ctrl+Z', async () => {
    renderApp();
    await userEvent.click(await screen.findByRole('button', { name: /load sample save/i }));
    await screen.findAllByText('Dorumon');

    const bit = screen.getByLabelText('BIT');
    await userEvent.clear(bit);
    await userEvent.type(bit, '1234{Enter}');
    expect((screen.getByLabelText('BIT') as HTMLInputElement).value).toBe('1234');

    await userEvent.keyboard('{Control>}z{/Control}');
    expect((screen.getByLabelText('BIT') as HTMLInputElement).value).toBe('0');
  });

  it('confirms before discarding unsaved changes', async () => {
    renderApp();
    await userEvent.click(await screen.findByRole('button', { name: /load sample save/i }));
    await screen.findAllByText('Dorumon');

    const bit = screen.getByLabelText('BIT');
    await userEvent.clear(bit);
    await userEvent.type(bit, '5{Enter}');

    await userEvent.click(screen.getByRole('button', { name: 'New' }));
    expect(await screen.findByText(/discard unsaved changes/i)).toBeTruthy();

    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByText(/discard unsaved changes/i)).toBeNull();
    expect(screen.getAllByText('Dorumon').length).toBeGreaterThan(0);
  });
});
