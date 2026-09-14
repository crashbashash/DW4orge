import { describe, expect, it } from 'vitest';
import type { NamedCap, PowerupLimit } from '../bindings';
import { capFor, powerupCap } from './caps';

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
});
