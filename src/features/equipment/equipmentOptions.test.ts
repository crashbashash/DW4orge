import { describe, expect, it } from 'vitest';
import type { Item } from '../../bindings';
import { EMPTY } from '../../lib/items';
import { deviceOptions } from './equipmentOptions';

const catalogue: Item[] = [
  { base_id: 0, name: 'Battle Hawk', category: 'weapon', grade: 0, note: null },
  { base_id: 1281, name: 'Omega Blade', category: 'styled', grade: null, note: null },
  { base_id: 4096, name: 'Brave Core', category: 'core', grade: null, note: null },
  { base_id: 8192, name: 'Achilles Board', category: 'board', grade: null, note: null },
];

describe('deviceOptions', () => {
  it('offers (none) plus only the slots matching the wanted categories', () => {
    const device = [0, 4096, 8192, 1281, EMPTY];
    const weapons = deviceOptions(device, catalogue, ['weapon', 'styled']);
    expect(weapons.map((option) => option.value)).toEqual([EMPTY, 0, 3]);
    expect(weapons[1]?.label).toContain('slot 1');

    const armor = deviceOptions(device, catalogue, ['core']);
    expect(armor.map((option) => option.value)).toEqual([EMPTY, 1]);
  });

  it('ignores empty slots and unknown ids', () => {
    expect(deviceOptions([EMPTY, 0x9999], catalogue, ['weapon']).map((o) => o.value)).toEqual([EMPTY]);
  });
});
