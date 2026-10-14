import { describe, expect, it } from 'vitest';
import type { Item } from '../bindings';
import {
  EMPTY,
  buildItemId,
  clampBonusToRarity,
  colorForSeed,
  describeItemId,
  gradeLetter,
  itemLabel,
  rarityName,
  socketBaseId,
  splitItemId,
} from './items';

const catalogue: Item[] = [
  { base_id: 0, name: 'Battle Hawk', category: 'weapon', grade: 0, note: null },
  { base_id: 1281, name: 'Omega Blade', category: 'styled', grade: null, note: null },
];

describe('item ids', () => {
  it('packs and splits the upper half', () => {
    expect(splitItemId(0x050d)).toEqual({ baseId: 0x050d, seed: 0, mods: 0 });
    expect(buildItemId(0x1010, 0x123, 0x7)).toBe((0x1010 | (0x1237 << 16)) >>> 0);
    expect(splitItemId(buildItemId(0x1010, 0x123, 0x7))).toEqual({
      baseId: 0x1010,
      seed: 0x123,
      mods: 0x7,
    });
  });

  it('masks the seed to 12 bits and the mods to 4', () => {
    expect(splitItemId(buildItemId(0xffff, 0xffff, 0xff))).toEqual({
      baseId: 0xffff,
      seed: 0xfff,
      mods: 0xf,
    });
  });

  it('reads a socket base id, or null when empty or out of range', () => {
    expect(socketBaseId([0x0000_1234], 0)).toBe(0x1234);
    expect(socketBaseId([EMPTY], 0)).toBeNull();
    expect(socketBaseId([0x050d], EMPTY)).toBeNull();
    expect(socketBaseId([0x050d], 7)).toBeNull();
  });
});

describe('rarity', () => {
  it('maps the band boundaries', () => {
    expect(colorForSeed(0x000)).toBe('white');
    expect(colorForSeed(0x001)).toBe('blue');
    expect(colorForSeed(0x00f)).toBe('blue');
    expect(colorForSeed(0x010)).toBe('green');
    expect(colorForSeed(0x07f)).toBe('green');
    expect(colorForSeed(0x080)).toBe('yellow');
    expect(colorForSeed(0x1ff)).toBe('yellow');
    expect(colorForSeed(0x200)).toBe('orange');
    expect(colorForSeed(0x3ff)).toBe('orange');
    expect(colorForSeed(0x400)).toBe('pink');
    expect(colorForSeed(0x7ff)).toBe('pink');
  });

  it('masks bit 11 off', () => {
    expect(colorForSeed(0x800)).toBe('white');
    expect(colorForSeed(0xfff)).toBe('pink');
  });

  it('clamps a bonus into a band', () => {
    expect(clampBonusToRarity(0x7ff, 'blue')).toBe(0x00f);
    expect(clampBonusToRarity(0, 'pink')).toBe(0x400);
    expect(clampBonusToRarity(0x0a9, 'yellow')).toBe(0x0a9);
  });

  it('names a seed with its colour and bonus', () => {
    expect(rarityName(0x530)).toBe('pink (+1328)');
    expect(rarityName(0)).toBe('white (+0)');
  });
});

describe('display', () => {
  it('letters a grade', () => {
    expect(gradeLetter(0)).toBe('\u03b1');
    expect(gradeLetter(4)).toBe('\u03b5');
    expect(gradeLetter(null)).toBe('');
  });

  it('labels a catalogue item', () => {
    expect(itemLabel(catalogue[0])).toBe('Battle Hawk \u03b1');
    expect(itemLabel(catalogue[1])).toBe('Omega Blade');
  });

  it('describes a stored id', () => {
    expect(describeItemId(EMPTY, catalogue)).toBe('(empty)');
    expect(describeItemId(0x0502, catalogue)).toBe('[invalid] Weapon (styled) 0x00000502');
    expect(describeItemId(buildItemId(1281, 0x530, 2), catalogue)).toBe(
      'Omega Blade (pink (+1328), 2 mods)',
    );
    expect(describeItemId(0, catalogue)).toBe('Battle Hawk \u03b1');
  });
});
