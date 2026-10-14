import { describe, expect, it } from 'vitest';
import { levelFromExp, levelThreshold } from './level';

describe('level curve', () => {
  it('matches the cubic', () => {
    expect(levelThreshold(1)).toBe(0);
    expect(levelThreshold(2)).toBe(341);
    expect(levelThreshold(999)).toBe(1_133_652_152);
  });

  it('inverts the curve', () => {
    expect(levelFromExp(0)).toBe(1);
    expect(levelFromExp(340)).toBe(1);
    expect(levelFromExp(341)).toBe(2);
  });

  it('terminates at the data-type maximum', () => {
    const level = levelFromExp(0xffffffff);
    expect(level).toBeGreaterThanOrEqual(1500);
    expect(level).toBeLessThanOrEqual(1700);
  });
});
