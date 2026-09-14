import { describe, expect, it } from 'vitest';
import { EMPTY } from './items';
import { buildItemId } from './items';
import { findOrAddMod, resolveMods } from './mods';
import { emptyEditSet } from './editSet';

describe('findOrAddMod', () => {
  it('reuses a slot already holding the chip', () => {
    const device = [0x050d, buildItemId(0x3040, 0, 0), EMPTY];
    const index = findOrAddMod(device, 0x3040);
    expect(index).toBe(1);
    expect(device[1]).toBe(buildItemId(0x3040, 0, 0));
  });

  it('fills the first empty slot when the chip is absent', () => {
    const device = [0x050d, EMPTY, EMPTY];
    expect(findOrAddMod(device, 0x3040)).toBe(1);
    expect(device[1]).toBe(0x3040);
  });

  it('returns null when every slot is full', () => {
    const device = Array.from({ length: 30 }, (_v, i) => 0x0100 + i);
    expect(findOrAddMod(device, 0x3040)).toBeNull();
  });
});

describe('resolveMods', () => {
  it('adds an absent chip and points the socket at it', () => {
    const draft = emptyEditSet();
    draft.wmods[0] = 0x3040;
    const resolved = resolveMods(draft);
    expect(resolved.device[0]).toBe(0x3040);
    expect(resolved.wmods[0]).toBe(0);
    expect(resolved.failed).toEqual([]);
  });

  it('records a socket that could not be placed', () => {
    const draft = emptyEditSet();
    draft.device = Array.from({ length: 30 }, (_v, i) => 0x0100 + i) as typeof draft.device;
    draft.wmods[3] = 0x3040;
    const resolved = resolveMods(draft);
    expect(resolved.wmods[3]).toBeNull();
    expect(resolved.failed).toEqual(['equip.wmod3']);
  });
});
