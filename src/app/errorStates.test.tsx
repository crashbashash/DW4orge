import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import type { Backend } from '../ipc/backend';
import { BackendProvider } from '../ipc/context';
import { createMockBackend } from '../ipc/mock';
import { App } from './App';

describe('empty and error states', () => {
  it('shows a failure in a dismissible banner', async () => {
    render(
      <BackendProvider
        backend={createMockBackend({
          validateEdits: () => {
            throw { kind: 'core', variant: 'Io', message: 'disk full' };
          },
        })}
      >
        <App />
      </BackendProvider>,
    );

    await userEvent.click(await screen.findByRole('button', { name: /load sample save/i }));

    // The failure arrives with the debounced validation.
    const banner = await screen.findByRole('alert');
    expect(banner.textContent).toContain('disk full');

    await userEvent.click(within(banner).getByRole('button', { name: /dismiss/i }));
    await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
  });

  it('disables the toolbar while a load is in flight', async () => {
    const base = createMockBackend();
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const backend: Backend = {
      ...base,
      openSample: async () => {
        await gate;
        return base.openSave('/mock/late.raw');
      },
    };

    render(
      <BackendProvider backend={backend}>
        <App />
      </BackendProvider>,
    );

    await userEvent.click(await screen.findByRole('button', { name: /load sample save/i }));

    const open = screen.getByRole('button', { name: 'Open' });
    await waitFor(() => expect((open as HTMLButtonElement).disabled).toBe(true));

    release();
    await waitFor(() => expect((open as HTMLButtonElement).disabled).toBe(false));
  });
});
