import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { EditorProvider } from '../../app/EditorProvider';
import { BackendProvider } from '../../ipc/context';
import { createMockBackend } from '../../ipc/mock';
import { NewSaveDialog } from './NewSaveDialog';

function renderDialog() {
  const backend = createMockBackend();
  const spy = vi.spyOn(backend, 'newSave');
  render(
    <BackendProvider backend={backend}>
      <EditorProvider>
        <NewSaveDialog isOpen onClose={() => {}} />
      </EditorProvider>
    </BackendProvider>,
  );
  return spy;
}

describe('NewSaveDialog', () => {
  it('submits exactly the NewSaveRequest shape', async () => {
    const spy = renderDialog();
    await screen.findByRole('button', { name: /species/i });

    await userEvent.click(screen.getByRole('button', { name: /species/i }));
    await userEvent.click(await screen.findByRole('option', { name: 'Agumon' }));

    const name = screen.getByLabelText('Player name');
    await userEvent.clear(name);
    await userEvent.type(name, 'abc');

    await userEvent.click(screen.getByRole('button', { name: /story preset/i }));
    await userEvent.click(await screen.findByRole('option', { name: 'Fresh (tutorial)' }));
    await userEvent.click(screen.getByRole('button', { name: /difficulty/i }));
    await userEvent.click(await screen.findByRole('option', { name: 'Hard' }));
    await userEvent.click(screen.getByRole('button', { name: 'Create' }));

    await waitFor(() =>
      expect(spy).toHaveBeenCalledWith({
        species: 'Agumon',
        name: 'abc',
        story: 'Fresh (tutorial)',
        difficulty: { fixed: 'Hard' },
      }),
    );
  });
});
