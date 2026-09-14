import { describe, expect, it } from 'vitest';
import type { OpenResult } from '../bindings';
import openResultJson from '../ipc/fixtures/open_result.raw.json';
import { EMPTY } from '../lib/items';
import { defaultSaveName, diffStory, initialState, reducer, storyDraftFromView, toEditSet, viewToEditSet } from './store';

// SAFETY: the fixture is rendered from a real OpenResult by `dw4ipc` and is
// pinned by `crates/dw4ipc/tests/ui_fixtures.rs`.
const openResult = openResultJson as unknown as OpenResult;

describe('defaultSaveName', () => {
  it('offers a memory-card name for a new save', () => {
    expect(defaultSaveName(null)).toBe('save.ps2');
  });

  it('keeps an existing file name so a raw save stays raw', () => {
    expect(defaultSaveName('/tmp/Mcd001.ps2')).toBe('Mcd001.ps2');
    expect(defaultSaveName('C:\\saves\\out.raw')).toBe('out.raw');
  });
});

describe('viewToEditSet', () => {
  const view = openResult.view;

  it('copies the scalars and converts mod indices to base ids', () => {
    const edits = viewToEditSet(view);
    expect(edits.species).toBe('Dorumon');
    expect(edits.junk).toBe(view.junk_counter);
    expect(edits.wmods.every((mod) => mod === null || mod <= 0xffff)).toBe(true);
    expect(edits.story).toEqual([]);
    expect(edits.difficulty).toEqual({ fixed: view.difficulty });
    expect(edits.device).toHaveLength(30);
    expect(edits.bank_items).toHaveLength(96);
    // The real card's save has three devices in slots 0-2 and the rest empty.
    expect(edits.device.slice(0, 3).every((id) => id !== EMPTY)).toBe(true);
    expect(edits.device.slice(3).every((id) => id === EMPTY)).toBe(true);
    expect(edits.weapons).toEqual(view.weapons);
  });
});

describe('storyDraftFromView', () => {
  it('turns the raw bytes into booleans', () => {
    const draft = storyDraftFromView(openResult.view);
    expect(draft.flags).toHaveLength(1024);
    expect(draft.folders).toHaveLength(12);
    // The mapping is byte != 0, not "everything is clear".
    expect(draft.flags.every((flag, i) => flag === (openResult.view.story_flags[i] !== 0))).toBe(true);
    expect(draft.folders.every((f, i) => f === (openResult.view.story_folders[i] !== 0))).toBe(true);
    expect(draft.flags[1]).toBe(true);
    expect(draft.flags[0]).toBe(false);
  });
});

describe('diffStory', () => {
  it('emits only the bits that changed', () => {
    const base = { flags: [false, false], folders: [false] };
    const next = { flags: [true, false], folders: [true] };
    expect(diffStory(next, base)).toEqual([
      { kind: 'flag', index: 0, value: true },
      { kind: 'folder', index: 0, value: true },
    ]);
  });
});

describe('reducer', () => {
  it('loads a session and marks the draft dirty after an edit', () => {
    const loaded = reducer(initialState, { type: 'loaded', result: openResult });
    expect(loaded.session?.view.species).toBe('Dorumon');
    expect(loaded.status).toBe('ready');

    const edited = reducer(loaded, { type: 'field', patch: { bit: 5 }, tag: 'bit' });
    expect(edited.draft?.bit).toBe(5);
    expect(edited.history.past).toHaveLength(1);
    expect(toEditSet(edited).bit).toBe(5);
  });

  it('ignores an edit before a session is open', () => {
    expect(reducer(initialState, { type: 'field', patch: { bit: 5 }, tag: 'bit' })).toBe(
      initialState,
    );
  });

  it('undoes and redoes an edit', () => {
    const loaded = reducer(initialState, { type: 'loaded', result: openResult });
    const edited = reducer(loaded, { type: 'field', patch: { bit: 5 }, tag: 'bit' });
    const undone = reducer(edited, { type: 'undo' });
    expect(undone.draft?.bit).toBe(loaded.draft?.bit);
    const redone = reducer(undone, { type: 'redo' });
    expect(redone.draft?.bit).toBe(5);
  });

  it('records a difficulty change', () => {
    const loaded = reducer(initialState, { type: 'loaded', result: openResult });
    const changed = reducer(loaded, { type: 'difficulty', difficulty: { fixed: 'Hard' } });
    expect(changed.difficulty).toEqual({ fixed: 'Hard' });
    expect(changed.history.past).toHaveLength(1);
  });
});
