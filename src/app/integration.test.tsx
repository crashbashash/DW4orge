import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { EditSet } from '../bindings';
import { BackendProvider } from '../ipc/context';
import { createMockBackend } from '../ipc/mock';
import { App } from './App';

/**
 * Every field of the generated `EditSet` binding
 * (`src/bindings/EditSet.ts`). Kept here so a shape change in Rust fails this
 * test rather than silently shipping a payload the frontend does not fill.
 */
const EDIT_SET_KEYS: readonly string[] = [
  'species',
  'name',
  'bit',
  'xdata',
  'junk',
  'level',
  'exp',
  'tech',
  'upcnt',
  'device',
  'weapons',
  'armor',
  'sub',
  'wmods',
  'amods',
  'story',
  'bank_bit',
  'disks',
  'bank_items',
];

async function setup() {
  const backend = createMockBackend();
  const saves: EditSet[] = [];
  vi.spyOn(backend, 'save').mockImplementation(async (edits: EditSet, mode) => {
    saves.push(edits);
    return backend.saveAs('/mock/out.raw', edits, mode);
  });

  render(
    <BackendProvider backend={backend}>
      <App />
    </BackendProvider>,
  );

  await userEvent.click(await screen.findByRole('button', { name: /load sample save/i }));
  await screen.findAllByText('Dorumon');
  return { backend, saves };
}

describe('open → edit → save', () => {
  it('sends a complete EditSet with the edited value', async () => {
    const { saves } = await setup();

    const bit = screen.getByLabelText('BIT');
    await userEvent.clear(bit);
    await userEvent.type(bit, '1234{Enter}');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => expect(saves).toHaveLength(1));
    const payload = saves[0];
    expect(payload).toBeDefined();
    expect(Object.keys(payload ?? {}).sort()).toEqual([...EDIT_SET_KEYS].sort());
    expect(payload).toMatchObject({ bit: 1234, story: [] });
    expect(payload?.device).toHaveLength(30);
    expect(payload?.bank_items).toHaveLength(96);
  });

  it('carries only the story bits that changed', async () => {
    const { saves } = await setup();

    await userEvent.click(screen.getByRole('button', { name: 'Story' }));
    await userEvent.click(await screen.findByLabelText(/apocalymon/i));
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => expect(saves).toHaveLength(1));
    expect(saves[0]?.story).toEqual([
      { kind: 'flag', index: 66, value: true, difficulty: 'Normal' },
    ]);
  });

  it('refreshes the summary from the saved view', async () => {
    await setup();
    const bit = screen.getByLabelText('BIT');
    await userEvent.clear(bit);
    await userEvent.type(bit, '1234{Enter}');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByText('1,234')).toBeTruthy();
  });
});
