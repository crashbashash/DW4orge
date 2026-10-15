import { render, screen, within, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import { App } from '../../app/App';
import { BackendProvider } from '../../ipc/context';
import { createMockBackend } from '../../ipc/mock';
import { viewToEditSet } from '../../app/store';

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
  const modal = await screen.findByRole('dialog', { name: 'Create character box' });
  expect(within(modal).queryByRole('button', { name: /save format/i })).toBeNull();
  await userEvent.click(within(modal).getByRole('button', { name: /empty box/i }));
  expect(screen.queryByRole('option', { name: 'Box 1 (empty)' })).toBeNull();
  await userEvent.click(await screen.findByRole('option', { name: 'Box 3 (empty)' }));
  await userEvent.click(within(modal).getByRole('button', { name: /species/i }));
  await userEvent.click(await screen.findByRole('option', { name: 'Veemon' }));
  await userEvent.clear(within(modal).getByLabelText('Player name'));
  await userEvent.type(within(modal).getByLabelText('Player name'), 'NEW3');
  await userEvent.click(within(modal).getByRole('button', { name: 'Create' }));
  await waitFor(() => expect(create).toHaveBeenCalledWith(3, expect.objectContaining({ format: undefined, region: 'usa', name: 'NEW3', species: 'Veemon' })));
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
  const modal = await screen.findByRole('dialog', { name: 'Create character box' });
  await userEvent.click(within(modal).getByRole('button', { name: 'Create' }));
  expect(await within(modal).findByRole('alert')).toHaveProperty('textContent', 'The selected PS2 box is occupied');
  expect(screen.getByRole('dialog', { name: 'Create character box' })).toBeTruthy();
  await userEvent.click(within(modal).getByRole('button', { name: 'Cancel' }));
  expect((screen.getByLabelText('BIT') as HTMLInputElement).value).toBe('77');
});

it.each(['xbox', 'xbox_mu'] as const)('creates %s boxes with the shared dialog and retains pending state', async source => {
  const backend = createMockBackend();
  const sample = await backend.openSample!();
  const prefix = source === 'xbox' ? 'SaveData.dat' : '42410002/SECOND/SaveData.dat';
  backend.openSample = async () => ({ ...sample, source, region: null, empty_boxes: [2,3],
    character_selection: `${prefix}#4`, character_boxes: [{id: `${prefix}#4`, label: 'Box 4 · ORIGINAL', box_number: 4}] });
  const create = vi.spyOn(backend, 'createBox').mockImplementation(async (number, request) => ({
    ...sample, source, region: null, pending_write: true, empty_boxes: [3],
    character_selection: `${prefix}#${number}`, character_boxes: [{ id: `${prefix}#${number}`, label: `Box ${number} · ${request.name}`, box_number: number }],
    view: { ...sample.view, name: request.name, species: request.species }
  }));
  await openEditor(backend);
  await userEvent.click(screen.getByRole('button', { name: 'Create box' }));
  const modal = await screen.findByRole('dialog', { name: 'Create character box' });
  expect(within(modal).queryByRole('button', { name: /save format|region/i })).toBeNull();
  if (source === 'xbox_mu') expect(within(modal).getByText(/save container selected in Character box/)).toBeTruthy();
  await userEvent.clear(within(modal).getByLabelText('Player name'));
  await userEvent.type(within(modal).getByLabelText('Player name'), 'XBNEW2');
  await userEvent.click(within(modal).getByRole('button', { name: 'Create' }));
  expect(await screen.findByDisplayValue('XBNEW2')).toBeTruthy();
  expect(create).toHaveBeenCalledWith(2, expect.objectContaining({format: undefined, region: null, name: 'XBNEW2'}));
  expect(screen.getByRole('img', {name:'Unsaved changes'})).toBeTruthy();
  await userEvent.click(screen.getByRole('button', {name:'Open'}));
  expect(await screen.findByRole('dialog', {name:/discard unsaved/i})).toBeTruthy();
  await userEvent.click(screen.getByRole('button', {name:'Cancel'}));
  expect(screen.getByDisplayValue('XBNEW2')).toBeTruthy();
});

it('disables creation when every PS2 box is occupied', async () => {
  const backend = createMockBackend();const sample = await backend.openSample!();backend.openSample = async () => ({ ...sample, empty_boxes: [] });
  await openEditor(backend);
  expect((screen.getByRole('button', { name: 'Create box' }) as HTMLButtonElement).disabled).toBe(true);
});


it.each(['gamecube', 'gamecube_card'] as const)('creates and persists GameCube boxes on %s with the shared dialog', async format => {
  const backend = createMockBackend();
  const base = await backend.newSave({ format, region: 'japan', species: 'Dorumon', name: 'ORIGINAL', story: null, difficulty: 'auto' });
  const path = format === 'gamecube' ? '/mock/input.gci' : '/mock/input.raw';
  await backend.saveAs(path, viewToEditSet(base.view), 'normal');
  backend.openSample = () => backend.openSave(path);
  const create = vi.spyOn(backend, 'createBox');
  await openEditor(backend);
  await userEvent.click(screen.getByRole('button', { name: 'Create box' }));
  const modal = await screen.findByRole('dialog', { name: 'Create character box' });
  await userEvent.click(within(modal).getByRole('button', { name: /empty box/i }));
  await userEvent.click(await screen.findByRole('option', { name: 'Box 3 (empty)' }));
  await userEvent.clear(within(modal).getByLabelText('Player name'));
  await userEvent.type(within(modal).getByLabelText('Player name'), 'GCNEW3');
  await userEvent.click(within(modal).getByRole('button', { name: 'Create' }));
  expect(await screen.findByDisplayValue('GCNEW3')).toBeTruthy();
  expect(create).toHaveBeenCalledWith(3, expect.objectContaining({ format: undefined, region: 'japan' }));
  expect(screen.getByRole('img', { name: 'Unsaved changes' })).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: 'Save' }));
  await waitFor(() => expect(screen.getByRole('img', { name: 'No changes' })).toBeTruthy());
  expect((await backend.openSave(path, undefined, 'gamecube#1')).view.name).toBe('ORIGINAL');
  expect((await backend.openSave(path, undefined, 'gamecube#3')).view.name).toBe('GCNEW3');
});
