import { describe, expect, it } from 'vitest';
import { describeSource, formatHex } from './format';

describe('formatHex', () => {
  it('renders an uppercase, padded hex id', () => {
    expect(formatHex(0x50d)).toBe('0x050D');
    expect(formatHex(0xdeadbeef, 8)).toBe('0xDEADBEEF');
  });
});

describe('describeSource', () => {
  it('names both containers', () => {
    expect(describeSource('memcard')).toBe('PS2 memory card');
    expect(describeSource('raw')).toBe('Raw save');
  });
});
