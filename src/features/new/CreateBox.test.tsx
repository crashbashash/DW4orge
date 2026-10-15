import { render, screen, within, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import { App } from '../../app/App';
import { BackendProvider } from '../../ipc/context';
import { createMockBackend } from '../../ipc/mock';

async function openEditor(backend = createMockBackend()) {
  render(<BackendProvider backend={backend}><App /></BackendProvider>);
  await userEvent.click(await screen.findByRole('button', { name: /load sample save/i }));
  await screen.findByRole('button', { name: 'Create box' });
  return backend;
}

it('creates only an empty PS2 box and treats it as unsaved before any field edit', async () => {
  const backend = createMockBackend();
  const create = vi.spyOn(backend, 'createBox');
  await openEditor(backend);
  await userEvent.click(screen.getByRole('button', { name: 'Create box' }));
  const modal = await screen.findByRole('dialog', { name: 'Create PS2 character box' });
  expect(within(modal).queryByRole('button', { name: /save format/i })).toBeNull();
  await userEvent.click(within(modal).getByRole('button', { name: /empty box/i }));
  expect(screen.queryByRole('option', { name: 'Box 1 (empty)' })).toBeNull();
  await userEvent.click(await screen.findByRole('option', { name: 'Box 3 (empty)' }));
  await userEvent.click(within(modal).getByRole('button', { name: /species/i }));
  await userEvent.click(await screen.findByRole('option', { name: 'Veemon' }));
  await userEvent.clear(within(modal).getByLabelText('Player name'));
  await userEvent.type(within(modal).getByLabelText('Player name'), 'NEW3');
  await userEvent.click(within(modal).getByRole('button', { name: 'Create' }));
  await waitFor(() => expect(create).toHaveBeenCalledWith(3, expect.objectContaining({ format: 'memcard', region: 'usa', name: 'NEW3', species: 'Veemon' })));
  expect(await screen.findByDisplayValue('NEW3')).toBeTruthy();
  expect(screen.getByRole('img', { name: 'Unsaved changes' })).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: /character box/i }));
  await userEvent.click(await screen.findByRole('option', { name: /Box 1/ }));
  await userEvent.click(await screen.findByRole('button', { name: 'Cancel' }));
  expect(screen.getByDisplayValue('NEW3')).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: 'Save' }));
  await waitFor(() => expect(screen.getByRole('img', { name: 'No changes' })).toBeTruthy());
});

it('Cancel preserves drafts and failed creation keeps the form and previous character', async () => {
  const backend = createMockBackend();
  const create = vi.spyOn(backend, 'createBox').mockRejectedValue({ kind: 'core', variant: 'NoSave', message: 'The selected PS2 box is occupied' });
  await openEditor(backend);
  await userEvent.clear(screen.getByLabelText('BIT'));
  await userEvent.type(screen.getByLabelText('BIT'), '77{Enter}');
  await userEvent.click(screen.getByRole('button', { name: 'Create box' }));
  await userEvent.click(await screen.findByRole('button', { name: 'Cancel' }));
  expect((screen.getByLabelText('BIT') as HTMLInputElement).value).toBe('77');
  expect(create).not.toHaveBeenCalled();
  await userEvent.click(screen.getByRole('button', { name: 'Create box' }));
  await userEvent.click(await screen.findByRole('button', { name: 'Discard' }));
  const modal = await screen.findByRole('dialog', { name: 'Create PS2 character box' });
  await userEvent.click(within(modal).getByRole('button', { name: 'Create' }));
  expect(await within(modal).findByRole('alert')).toHaveProperty('textContent', 'The selected PS2 box is occupied');
  expect(screen.getByRole('dialog', { name: 'Create PS2 character box' })).toBeTruthy();
  await userEvent.click(within(modal).getByRole('button', { name: 'Cancel' }));
  expect((screen.getByLabelText('BIT') as HTMLInputElement).value).toBe('77');
});

it.each(['gamecube', 'gamecube_card', 'xbox', 'xbox_mu'] as const)('does not offer creation for %s', async source => {
  const backend = createMockBackend();const sample = await backend.openSample!();
  backend.openSample = async () => ({ ...sample, source, empty_boxes: undefined });
  render(<BackendProvider backend={backend}><App /></BackendProvider>);
  await userEvent.click(await screen.findByRole('button', { name: /load sample save/i }));
  await screen.findByLabelText('BIT');
  expect(screen.queryByRole('button', { name: 'Create box' })).toBeNull();
});

it('disables creation when every PS2 box is occupied', async () => {
  const backend = createMockBackend();const sample = await backend.openSample!();backend.openSample = async () => ({ ...sample, empty_boxes: [] });
  await openEditor(backend);
  expect((screen.getByRole('button', { name: 'Create box' }) as HTMLButtonElement).disabled).toBe(true);
});
