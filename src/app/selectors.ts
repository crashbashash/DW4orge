import type { FieldError } from '../bindings';
import { diffStory, type StoreState } from './store';

/**
 * Whether the draft differs from what was loaded.
 *
 * The `EditSet`s are built by the same function at load, so their key order
 * matches and a JSON comparison is a faithful deep-equal; `story` and
 * `difficulty` are compared separately because they live beside the draft.
 */
export function dirty(state: StoreState): boolean {
  if (!state.draft || !state.story || !state.session) return false;
  if (diffStory(state.story, state.session.baselineStory).length > 0) return true;

  const current = { ...state.draft, story: [], difficulty: state.difficulty };
  const baseline = {
    ...state.session.baseline,
    story: [],
    difficulty: state.session.baseline.difficulty,
  };
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
