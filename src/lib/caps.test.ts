import { describe, expect, it } from 'vitest';
import type { NamedCap, PowerupLimit } from '../bindings';
import { capFor, capHint, normalMax, normalPowerupMax, powerupCap } from './caps';

const caps: NamedCap[] = [
  { field: 'bit', cap: { normal_max: 9_999_999, dtype_min: 0, dtype_max: 0xffffffff } },
];

const powerups: PowerupLimit[] = [
  { slot: 0, stat: 'HP max', normal_max: 99_999 },
  { slot: 1, stat: 'MP max', normal_max: 99_999 },
  { slot: 2, stat: 'Strength', normal_max: 9_999 },
];

describe('caps', () => {
  it('finds a cap by its validator path', () => {
    expect(capFor(caps, 'bit')?.cap.normal_max).toBe(9_999_999);
    expect(capFor(caps, 'nope')).toBeUndefined();
  });

  it('reads a power-up slot cap', () => {
    expect(powerupCap(powerups, 0)).toBe(99_999);
    expect(powerupCap(powerups, 2)).toBe(9_999);
  });

  it('returns the Normal cap only in Normal mode', () => {
    expect(normalMax(caps, 'bit', 'normal')).toBe(9_999_999);
    expect(normalMax(caps, 'bit', 'advanced')).toBeUndefined();
    expect(normalMax(caps, 'nope', 'normal')).toBeUndefined();
  });

  it('returns a power-up cap only in Normal mode', () => {
    expect(normalPowerupMax(powerups, 0, 'normal')).toBe(99_999);
    expect(normalPowerupMax(powerups, 0, 'advanced')).toBeUndefined();
  });

  it('formats a hint for a cap, and nothing without one', () => {
    expect(capHint(9_999_999)).toBe('max 9,999,999');
    expect(capHint(undefined)).toBeUndefined();
  });
});
