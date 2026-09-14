import { describe, expect, it } from 'vitest';
import { describeIpcError, fieldErrors, isIpcError } from './errors';

describe('IpcError helpers', () => {
  it('recognises every variant by kind', () => {
    expect(isIpcError({ kind: 'no_open_document' })).toBe(true);
    expect(isIpcError({ kind: 'validation', fields: [] })).toBe(true);
    expect(isIpcError({ kind: 'core', variant: 'Io', message: 'x' })).toBe(true);
    expect(isIpcError({ kind: 'unsupported', message: 'x' })).toBe(true);
    expect(isIpcError(new Error('nope'))).toBe(false);
    expect(isIpcError({ kind: 'other' })).toBe(false);
    expect(isIpcError(null)).toBe(false);
  });

  it('extracts validation fields only', () => {
    expect(
      fieldErrors({
        kind: 'validation',
        fields: [{ path: 'bit', message: 'x', severity: 'error' }],
      }),
    ).toHaveLength(1);
    expect(fieldErrors({ kind: 'no_open_document' })).toBeNull();
    expect(fieldErrors({ kind: 'core', variant: 'Io', message: 'x' })).toBeNull();
  });

  it('renders a message a human can act on', () => {
    expect(describeIpcError({ kind: 'unsupported', message: 'no free slot' })).toBe('no free slot');
    expect(describeIpcError({ kind: 'no_open_document' })).toMatch(/no save is open/i);
    expect(describeIpcError({ kind: 'core', variant: 'Io', message: 'disk full' })).toMatch(
      /disk full/,
    );
    expect(
      describeIpcError({
        kind: 'validation',
        fields: [{ path: 'bit', message: 'BIT too big', severity: 'error' }],
      }),
    ).toBe('BIT too big');
    expect(describeIpcError(new Error('plain'))).toBe('plain');
  });
});
