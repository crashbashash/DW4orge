import { describe, expect, it } from 'vitest';
import type { OpenResult } from '../bindings';
import openResultJson from '../ipc/fixtures/open_result.raw.json';
import { dirty, errorCount, errorsByPath } from './selectors';
import { initialState, reducer } from './store';

// SAFETY: rendered from a real OpenResult by `dw4ipc`, pinned by the fixture
// drift test.
const openResult = openResultJson as unknown as OpenResult;

describe('selectors', () => {
  it('is clean on load and dirty after an edit', () => {
    const loaded = reducer(initialState, { type: 'loaded', result: openResult });
    expect(dirty(loaded)).toBe(false);
    expect(dirty(reducer(loaded, { type: 'field', patch: { bit: 1 }, tag: 'bit' }))).toBe(true);
    expect(dirty(reducer(loaded, { type: 'story', kind: 'flag', index: 0, value: true }))).toBe(
      true,
    );
  });

  it('marks a brand-new, never-written save as unsaved', () => {
    // `new_save` returns a pathless in-memory save: nothing has been written
    // yet, even though the draft matches its baseline exactly.
    const created = reducer(initialState, {
      type: 'loaded',
      result: { ...openResult, path: null },
    });
    expect(created.session?.path).toBeNull();
    expect(dirty(created)).toBe(true);

    // Once it is written, the draft-vs-baseline comparison takes over.
    const saved = reducer(created, {
      type: 'saved',
      result: { ...openResult, path: '/mock/new.ps2' },
      message: 'Saved',
    });
    expect(dirty(saved)).toBe(false);
  });

  it('counts errors and groups them by path', () => {
    const state = reducer(initialState, {
      type: 'validation',
      report: {
        errors: [
          { path: 'bit', message: 'a', severity: 'error' },
          { path: 'bit', message: 'b', severity: 'error' },
          { path: 'level', message: 'c', severity: 'error' },
        ],
        warnings: [],
      },
    });
    expect(errorCount(state)).toBe(3);
    expect(errorsByPath(state).get('bit')).toHaveLength(2);
    expect(errorsByPath(state).get('level')).toHaveLength(1);
  });
});
