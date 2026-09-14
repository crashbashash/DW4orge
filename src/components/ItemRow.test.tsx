import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { Item } from '../bindings';
import { EMPTY, buildItemId } from '../lib/items';
import { ItemRow } from './ItemRow';

const catalogue: Item[] = [
  { base_id: 0, name: 'Battle Hawk', category: 'weapon', grade: 0, note: null },
  { base_id: 1281, name: 'Omega Blade', category: 'styled', grade: null, note: null },
];

describe('ItemRow', () => {
  it('picks a catalogue item and packs its base id', async () => {
    const onChange = vi.fn();
    render(
      <ItemRow slot={0} value={EMPTY} catalogue={catalogue} mode="normal" onChange={onChange} />,
    );
    const input = screen.getByRole('combobox', { name: /item/i });
    await userEvent.click(input);
    await userEvent.type(input, 'Omega');
    await userEvent.click(await screen.findByRole('option', { name: 'Omega Blade' }));
    expect(onChange).toHaveBeenCalledWith(1281);
  });

  it('clamps the +N bonus into the chosen rarity band', async () => {
    const onChange = vi.fn();
    render(
      <ItemRow
        slot={0}
        value={buildItemId(1281, 0x7ff, 0)}
        catalogue={catalogue}
        mode="normal"
        onChange={onChange}
      />,
    );
    await userEvent.selectOptions(screen.getByLabelText('Rarity 1'), 'blue');
    expect(onChange).toHaveBeenCalledWith(buildItemId(1281, 0x00f, 0));
  });

  it('clears a slot to EMPTY', async () => {
    const onChange = vi.fn();
    render(
      <ItemRow
        slot={0}
        value={buildItemId(1281, 0x530, 2)}
        catalogue={catalogue}
        mode="normal"
        onChange={onChange}
      />,
    );
    await userEvent.click(screen.getByRole('button', { name: 'Clear' }));
    expect(onChange).toHaveBeenCalledWith(EMPTY);
  });
});
