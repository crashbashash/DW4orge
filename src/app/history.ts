import type { Difficulty, DifficultyChoice, EditSet } from '../bindings';
import type { StoryDraft } from '../lib/story';

/** The whole editable state at one point, for undo/redo. */
export type Snapshot = {
  draft: EditSet;
  /**
   * One draft per difficulty, so switching the Story difficulty is a view
   * change and never discards in-flight edits.
   */
  stories: Record<Difficulty, StoryDraft>;
  difficulty: DifficultyChoice;
};

/**
 * Undo/redo stacks plus the tag of the top entry.
 *
 * A tag lets consecutive edits to the same control coalesce into one undo
 * step; a discrete edit passes `null` and always pushes.
 */
export type History = {
  past: Snapshot[];
  future: Snapshot[];
  tag: string | null;
};

/** Record `current` as the state before a new `next` edit. */
export function record(history: History, current: Snapshot, tag: string | null): History {
  const coalesce = tag !== null && tag === history.tag;
  return {
    past: coalesce ? history.past : [...history.past, current],
    future: [],
    tag,
  };
}

/** Step back, returning the restored snapshot, or null when there is none. */
export function undo(
  history: History,
  current: Snapshot,
): { history: History; present: Snapshot } | null {
  const present = history.past.at(-1);
  if (!present) return null;
  return {
    history: { past: history.past.slice(0, -1), future: [current, ...history.future], tag: null },
    present,
  };
}

/** Step forward again, the mirror of [`undo`]. */
export function redo(
  history: History,
  current: Snapshot,
): { history: History; present: Snapshot } | null {
  const present = history.future[0];
  if (!present) return null;
  return {
    history: { past: [...history.past, current], future: history.future.slice(1), tag: null },
    present,
  };
}
