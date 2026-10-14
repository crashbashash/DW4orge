import type { FieldError } from '../bindings';
import { DIFFICULTY_ORDER } from '../lib/story';
import { diffStory, type StoreState } from './store';

/**
 * Whether the draft differs from what was loaded.
 *
 * A session with no path has never been written — the in-memory save
 * `new_save` returns — so it is unsaved work even before the first edit.
 * Saving once sets the path and the comparison below takes over.
 *
 * The `EditSet`s are built by the same function at load, so their key order
 * matches and a JSON comparison is a faithful deep-equal; the story drafts are
 * compared separately because they live beside the draft, one per difficulty.
 */
export function dirty(state: StoreState): boolean {
  if (!state.draft || !state.session) return false;
  if (state.session.path === null) return true;
  for (const difficulty of DIFFICULTY_ORDER) {
    const pending = diffStory(
      state.stories[difficulty],
      state.session.baselines[difficulty],
      difficulty,
    );
    if (pending.length > 0) return true;
  }

  const current = { ...state.draft, story: [] };
  const baseline = { ...state.session.baseline, story: [] };
  return JSON.stringify(current) !== JSON.stringify(baseline);
}

export function errorCount(state: StoreState): number {
  return state.validation.errors.length;
}

/** Validation errors indexed by the store `path` the validator reports. */
export function errorsByPath(state: StoreState): Map<string, FieldError[]> {
  const map = new Map<string, FieldError[]>();
  for (const error of state.validation.errors) {
    const existing = map.get(error.path);
    if (existing) existing.push(error);
    else map.set(error.path, [error]);
  }
  return map;
}
