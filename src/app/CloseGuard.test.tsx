import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { BackendProvider } from '../ipc/context';
import { createMockBackend } from '../ipc/mock';
import { CloseGuard } from './CloseGuard';
import { EditorProvider, useEditor } from './EditorProvider';

function Harness() {
  const { state, openSample, setField } = useEditor();
  return (
    <>
      <span data-testid="session">{state.session ? 'open' : 'none'}</span>
      <button type="button" onClick={() => void openSample()}>
        load
      </button>
      <button type="button" onClick={() => setField({ bit: 7 }, 'bit')}>
        edit
      </button>
      <CloseGuard />
    </>
  );
}

/** A backend whose close request the test can fire by hand. */
function setup() {
  let handler: (() => boolean) | undefined;
  const closeWindow = vi.fn(async () => {});
  const backend = createMockBackend({
    onCloseRequested: async (next) => {
      handler = next;
      return () => {
        handler = undefined;
      };
    },
    closeWindow,
  });

  render(
    <BackendProvider backend={backend}>
      <EditorProvider>
        <Harness />
      </EditorProvider>
    </BackendProvider>,
  );

  return { closeWindow, requestClose: () => handler };
}

async function load() {
  await userEvent.click(screen.getByRole('button', { name: 'load' }));
  await waitFor(() => expect(screen.getByTestId('session').textContent).toBe('open'));
}

describe('CloseGuard', () => {
  it('lets the window close when there is nothing unsaved', async () => {
    const { closeWindow, requestClose } = setup();
    await load();

    await waitFor(() => expect(requestClose()).toBeTypeOf('function'));
    expect(requestClose()?.()).toBe(false);
    expect(screen.queryByText(/discard unsaved changes/i)).toBeNull();
    expect(closeWindow).not.toHaveBeenCalled();
  });

  it('warns instead of closing when there is unsaved work, and Cancel keeps it open', async () => {
    const { closeWindow, requestClose } = setup();
    await load();
    await waitFor(() => expect(requestClose()).toBeTypeOf('function'));

    await userEvent.click(screen.getByRole('button', { name: 'edit' }));
    expect(requestClose()?.()).toBe(true);
    expect(await screen.findByText(/discard unsaved changes/i)).toBeTruthy();

    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByText(/discard unsaved changes/i)).toBeNull();
    expect(closeWindow).not.toHaveBeenCalled();
  });

  it('closes the window once the user discards', async () => {
    const { closeWindow, requestClose } = setup();
    await load();
    await waitFor(() => expect(requestClose()).toBeTypeOf('function'));

    await userEvent.click(screen.getByRole('button', { name: 'edit' }));
    requestClose()?.();

    await userEvent.click(await screen.findByRole('button', { name: 'Discard and close' }));
    expect(closeWindow).toHaveBeenCalledTimes(1);
  });
});
