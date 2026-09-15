import { describe, expect, it } from 'vitest';
import type { AppInfo, OpenResult } from '../bindings';
import appInfoJson from '../ipc/fixtures/app_info.json';
import openResultJson from '../ipc/fixtures/open_result.raw.json';
import { EMPTY } from '../lib/items';
import {
  defaultSaveName,
  diffStory,
  initialState,
  reducer,
  resolveDifficulty,
  storyDraftFromView,
  storyDraftsFromView,
  toEditSet,
  viewToEditSet,
} from './store';

// SAFETY: the fixture is rendered from a real OpenResult by `dw4ipc` and is
// pinned by `crates/dw4ipc/tests/ui_fixtures.rs`.
const openResult = openResultJson as unknown as OpenResult;
// SAFETY: `app_info.json` is rendered from `dw4ipc::app_info()` and pinned by
// `crates/dw4ipc/tests/ui_fixtures.rs`.
const appInfo = appInfoJson as unknown as AppInfo;

/** A store with app-info loaded, so the mirror table is available. */
function withAppInfo() {
  return reducer(initialState, { type: 'appInfo', info: appInfo });
}

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

describe('storyDraftsFromView', () => {
  it('reads each difficulty out of its own mirror column', () => {
    const drafts = storyDraftsFromView(openResult.view, appInfo.ui.mirrors);
    // Flag 1 is mirrored by every difficulty; only Normal's column is stored.
    expect(drafts.Normal.flags[1]).toBe(true);
    expect(drafts.Hard.flags[1]).toBe(false);
    expect(drafts.VeryHard.flags[1]).toBe(false);
    // A lobby flag has no mirror, so every difficulty shares the live value.
    expect(drafts.Hard.flags[24]).toBe(true);
    expect(drafts.VeryHard.flags[24]).toBe(true);
  });
});

describe('resolveDifficulty', () => {
  it('follows the detected difficulty for auto', () => {
    const loaded = reducer(withAppInfo(), { type: 'loaded', result: openResult });
    expect(resolveDifficulty('auto', loaded.session)).toBe('Normal');
    expect(resolveDifficulty({ fixed: 'VeryHard' }, loaded.session)).toBe('VeryHard');
    expect(resolveDifficulty('auto', null)).toBe('Normal');
  });
});

describe('diffStory', () => {
  it('emits only the bits that changed, tagged with their difficulty', () => {
    const base = { flags: [false, false], folders: [false] };
    const next = { flags: [true, false], folders: [true] };
    expect(diffStory(next, base, 'VeryHard')).toEqual([
      { kind: 'flag', index: 0, value: true, difficulty: 'VeryHard' },
      { kind: 'folder', index: 0, value: true, difficulty: 'VeryHard' },
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

  it('treats a difficulty change as a view change, not an edit', () => {
    const loaded = reducer(initialState, { type: 'loaded', result: openResult });
    const changed = reducer(loaded, { type: 'difficulty', difficulty: { fixed: 'Hard' } });
    expect(changed.difficulty).toEqual({ fixed: 'Hard' });
    // Switching the selector must not push an undo step or discard a draft.
    expect(changed.history.past).toHaveLength(0);
    expect(changed.stories).toBe(loaded.stories);
  });

  it('seeds one draft per difficulty and keeps them apart', () => {
    const loaded = reducer(withAppInfo(), { type: 'loaded', result: openResult });
    // The card is a Normal save: flag 1 is live and its Normal mirror is set.
    expect(loaded.stories.Normal.flags[1]).toBe(true);
    // The card stores no Hard column, so Hard's own draft reads the bit off...
    expect(loaded.stories.Hard.flags[1]).toBe(false);
    // ...while the shared lobby flag is the same on every difficulty.
    expect(loaded.stories.Hard.flags[24]).toBe(true);
    expect(loaded.stories.VeryHard.flags[24]).toBe(true);
  });

  it('diffs a story edit against the difficulty it was made on', () => {
    const loaded = reducer(withAppInfo(), { type: 'loaded', result: openResult });
    const onHard = reducer(loaded, { type: 'difficulty', difficulty: { fixed: 'Hard' } });
    const edited = reducer(onHard, { type: 'story', kind: 'flag', index: 1, value: true });
    expect(toEditSet(edited).story).toEqual([
      { kind: 'flag', index: 1, value: true, difficulty: 'Hard' },
    ]);
  });

  it('keeps an edit made on one difficulty when the selector moves away and back', () => {
    // Regression: applying a preset and then changing difficulty used to
    // re-seed the visible draft and silently drop the unsaved edit.
    const loaded = reducer(withAppInfo(), { type: 'loaded', result: openResult });
    const onHard = reducer(loaded, { type: 'difficulty', difficulty: { fixed: 'Hard' } });
    const edited = reducer(onHard, { type: 'story', kind: 'flag', index: 1, value: true });
    expect(edited.stories.Hard.flags[1]).toBe(true);

    const onNormal = reducer(edited, { type: 'difficulty', difficulty: { fixed: 'Normal' } });
    const backOnHard = reducer(onNormal, { type: 'difficulty', difficulty: { fixed: 'Hard' } });
    expect(backOnHard.stories.Hard.flags[1]).toBe(true);
    expect(toEditSet(backOnHard).story).toEqual([
      { kind: 'flag', index: 1, value: true, difficulty: 'Hard' },
    ]);
  });
});
