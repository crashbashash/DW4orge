import { describe, expect, it } from 'vitest';
import { formatNumber, parseIntLoose } from './num';

describe('parseIntLoose', () => {
  it('accepts decimal, hex and underscores', () => {
    expect(parseIntLoose('42')).toBe(42);
    expect(parseIntLoose('0x1F')).toBe(31);
    expect(parseIntLoose('1_000')).toBe(1000);
    expect(parseIntLoose('  7 ')).toBe(7);
    expect(parseIntLoose('-3')).toBe(-3);
  });

  it('rejects anything else', () => {
    expect(parseIntLoose('')).toBeNull();
    expect(parseIntLoose('abc')).toBeNull();
    expect(parseIntLoose('12abc')).toBeNull();
    expect(parseIntLoose('0x')).toBeNull();
  });
});

describe('formatNumber', () => {
  it('groups thousands', () => {
    expect(formatNumber(3_956_000)).toBe('3,956,000');
    expect(formatNumber(0)).toBe('0');
  });
});
