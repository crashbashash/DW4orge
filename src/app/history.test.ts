import { describe, expect, it } from 'vitest';
import type { Snapshot } from './history';
import { record, redo, undo } from './history';

const emptyStory = () => ({ flags: [], folders: [] });

const snap = (bit: number): Snapshot => ({
  // SAFETY: the test only reads `bit`; the rest of the EditSet is irrelevant.
  draft: { bit } as unknown as Snapshot['draft'],
  stories: { Normal: emptyStory(), Hard: emptyStory(), VeryHard: emptyStory() },
  difficulty: 'auto',
});

const empty = { past: [], future: [], tag: null };

describe('history', () => {
  it('coalesces consecutive edits with the same tag', () => {
    let history = record(empty, snap(0), 'bit');
    history = record(history, snap(1), 'bit');
    expect(history.past).toEqual([snap(0)]);
    expect(history.past).toHaveLength(1);
  });

  it('pushes a new entry when the tag changes', () => {
    let history = record(empty, snap(0), 'bit');
    history = record(history, snap(1), 'xdata');
    expect(history.past).toHaveLength(2);
  });

  it('pushes for a discrete edit with a null tag', () => {
    const history = record({ past: [], future: [], tag: 'bit' }, snap(0), null);
    expect(history.past).toHaveLength(1);
    expect(history.tag).toBeNull();
  });

  it('clears the redo stack on a new edit', () => {
    const history = record({ past: [snap(0)], future: [snap(9)], tag: null }, snap(1), null);
    expect(history.future).toEqual([]);
  });

  it('undo and redo round-trip', () => {
    const history = record(empty, snap(0), null);
    const stepped = undo(history, snap(5));
    expect(stepped).not.toBeNull();
    expect(stepped?.present).toEqual(snap(0));
    const forward = redo(stepped!.history, stepped!.present);
    expect(forward?.present).toEqual(snap(5));
  });

  it('returns null when there is nothing to undo or redo', () => {
    expect(undo(empty, snap(0))).toBeNull();
    expect(redo(empty, snap(0))).toBeNull();
  });
});
