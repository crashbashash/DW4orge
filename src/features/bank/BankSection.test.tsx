import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { EditorProvider, useEditor } from '../../app/EditorProvider';
import { BackendProvider } from '../../ipc/context';
import { createMockBackend } from '../../ipc/mock';
import { EMPTY } from '../../lib/items';
import { BankSection } from './BankSection';

function Harness() {
  const { state, openSample } = useEditor();
  return (
    <>
      <button type="button" onClick={() => void openSample()}>
        load
      </button>
      <span data-testid="bank0">{state.draft?.bank_items[0] ?? '-'}</span>
      {state.session ? <BankSection /> : null}
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
  await screen.findByText('Bank');
}

describe('BankSection', () => {
  it('pages through 96 slots and writes a picked item', async () => {
    await setup();
    expect(screen.getAllByTestId(/^item-row-/)).toHaveLength(24);
    expect(screen.getByTestId('item-row-0')).toBeTruthy();

    const row = screen.getByTestId('item-row-0');
    await userEvent.click(within(row).getByRole('button', { name: /item/i }));
    await userEvent.type(await screen.findByRole('searchbox'), 'Omega');
    await userEvent.click(await screen.findByRole('option', { name: 'Omega Blade' }));

    expect(screen.getByTestId('bank0').textContent).toBe(String(1281));
    expect(within(screen.getByTestId('item-row-0')).getAllByText(/Omega Blade/).length).toBeGreaterThan(0);
    expect(screen.getByTestId('bank0').textContent).not.toBe(String(EMPTY));
  });

  it('moves to the fourth page', async () => {
    await setup();
    await userEvent.click(screen.getByRole('tab', { name: 'Page 4' }));
    expect(screen.getByTestId('item-row-72')).toBeTruthy();
    expect(screen.queryByTestId('item-row-0')).toBeNull();
  });
});
