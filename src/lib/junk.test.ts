import { describe, expect, it } from 'vitest';
import { JUNK_TIERS, junkThreshold, junkTierFromCounter } from './junk';

describe('junk tiers', () => {
  it('has the ten cumulative thresholds', () => {
    expect(JUNK_TIERS).toHaveLength(10);
    expect(JUNK_TIERS[0]).toEqual({ tier: 0, threshold: 0 });
    expect(junkThreshold(1)).toBe(1_000);
    expect(junkThreshold(9)).toBe(3_956_000);
    expect(junkThreshold(10)).toBeUndefined();
  });

  it('reads the highest reached tier', () => {
    expect(junkTierFromCounter(0)).toBe(0);
    expect(junkTierFromCounter(999)).toBe(0);
    expect(junkTierFromCounter(1_000)).toBe(1);
    expect(junkTierFromCounter(5_999)).toBe(1);
    expect(junkTierFromCounter(6_000)).toBe(2);
    expect(junkTierFromCounter(3_955_999)).toBe(8);
    expect(junkTierFromCounter(3_956_000)).toBe(9);
    expect(junkTierFromCounter(0xffffffff)).toBe(9);
  });
});
