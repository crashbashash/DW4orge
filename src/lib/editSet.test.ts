import { describe, expect, it } from 'vitest';
import { emptyEditSet } from './editSet';
import { EMPTY } from './items';

describe('emptyEditSet', () => {
  it('fills every fixed-length field to the binding length', () => {
    const edits = emptyEditSet();
    expect(edits.device).toHaveLength(30);
    expect(edits.bank_items).toHaveLength(96);
    expect(edits.tech).toHaveLength(9);
    expect(edits.upcnt).toHaveLength(11);
    expect(edits.disks).toHaveLength(12);
    expect(edits.weapons).toHaveLength(3);
    expect(edits.wmods).toHaveLength(5);
    expect(edits.amods).toHaveLength(5);
  });

  it('matches a freshly built save', () => {
    const edits = emptyEditSet();
    expect(edits.species).toBe('Dorumon');
    expect(edits.name).toBe('TST');
    expect(edits.level).toBe(1);
    expect(edits.tech).toEqual([1, 1, 1, 1, 1, 1, 1, 1, 1]);
    expect(edits.device.every((id) => id === EMPTY)).toBe(true);
    expect(edits.story).toEqual([]);
    expect(edits.difficulty).toBe('auto');
  });
});
