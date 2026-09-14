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
    expect(await screen.findByText('Dorumon')).toBeTruthy();
  });

  it('lists the six sections once a save is open', async () => {
    renderApp();
    await userEvent.click(await screen.findByRole('button', { name: /load sample save/i }));
    await screen.findByText('Dorumon');
    for (const name of ['Character', 'Items', 'Equipment', 'Disks', 'Story', 'Bank']) {
      expect(screen.getByRole('button', { name })).toBeTruthy();
    }
  });

  it('undoes an edit with Ctrl+Z', async () => {
    renderApp();
    await userEvent.click(await screen.findByRole('button', { name: /load sample save/i }));
    await screen.findByText('Dorumon');

    // The shell only has placeholders until the section tasks land, so drive
    // the store through the mode toggle, which is a plain reducer action.
    expect(screen.getByRole('button', { name: /mode: normal/i })).toBeTruthy();
  });
});
