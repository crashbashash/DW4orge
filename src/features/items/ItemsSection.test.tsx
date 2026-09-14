import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { EditorProvider, useEditor } from '../../app/EditorProvider';
import { BackendProvider } from '../../ipc/context';
import { createMockBackend } from '../../ipc/mock';
import { ItemsSection } from './ItemsSection';

function Harness() {
  const { state, openSample } = useEditor();
  return (
    <>
      <button type="button" onClick={() => void openSample()}>
        load
      </button>
      {state.session ? <ItemsSection /> : null}
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
  await screen.findByText('Items');
}

describe('ItemsSection', () => {
  it('shows ten rows per page and switches pages', async () => {
    await setup();
    expect(screen.getAllByTestId(/^item-row-/)).toHaveLength(10);
    expect(screen.getByTestId('item-row-0')).toBeTruthy();

    await userEvent.click(screen.getByRole('tab', { name: 'Page 2' }));
    expect(screen.getByTestId('item-row-10')).toBeTruthy();
    expect(screen.queryByTestId('item-row-0')).toBeNull();
  });

  it('writes a picked item into the device folder', async () => {
    await setup();
    const row = screen.getByTestId('item-row-0');
    expect(within(row).getAllByText(/Bash Katana/).length).toBeGreaterThan(0);

    await userEvent.click(within(row).getByRole('button', { name: /item/i }));
    await userEvent.type(await screen.findByRole('searchbox'), 'Omega');
    await userEvent.click(await screen.findByRole('option', { name: 'Omega Blade' }));

    expect(
      within(screen.getByTestId('item-row-0')).getAllByText(/Omega Blade/).length,
    ).toBeGreaterThan(0);
  });
});
