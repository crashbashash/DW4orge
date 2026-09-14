import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { EditorProvider, useEditor } from '../../app/EditorProvider';
import { BackendProvider } from '../../ipc/context';
import { createMockBackend } from '../../ipc/mock';
import { EMPTY } from '../../lib/items';
import { EquipmentSection } from './EquipmentSection';

function Harness() {
  const { state, openSample } = useEditor();
  return (
    <>
      <button type="button" onClick={() => void openSample()}>
        load
      </button>
      <span data-testid="weapon0">{state.draft?.weapons[0] ?? '-'}</span>
      <span data-testid="device-count">
        {state.draft ? state.draft.device.filter((id) => id !== EMPTY).length : '-'}
      </span>
      {state.session ? <EquipmentSection /> : null}
    </>
  );
}

async function setup() {
  render(
    <BackendProvider backend={createMockBackend()}>
      <EditorProvider>
        <Harness />
      </EditorProvider>
    </BackendProvider>,
  );
  await userEvent.click(screen.getByRole('button', { name: 'load' }));
  await screen.findByText('Equipment');
}

describe('EquipmentSection', () => {
  it('offers the eligible device slots and writes the chosen index', async () => {
    await setup();
    expect(screen.getByTestId('weapon0').textContent).toBe('0');

    await userEvent.click(screen.getByRole('button', { name: /weapon 1/i }));
    await userEvent.click(await screen.findByRole('option', { name: '(none)' }));
    expect(screen.getByTestId('weapon0').textContent).toBe(String(EMPTY));
  });

  it('adds a chosen mod chip to the device folder', async () => {
    await setup();
    expect(screen.getByTestId('device-count').textContent).toBe('3');

    await userEvent.click(screen.getByRole('button', { name: /weapon mod 1/i }));
    await userEvent.type(await screen.findByRole('searchbox'), 'Wisdom Chip α');
    await userEvent.click(await screen.findByRole('option', { name: 'Wisdom Chip α' }));

    expect(screen.getByTestId('device-count').textContent).toBe('4');
  });
});
