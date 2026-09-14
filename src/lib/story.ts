import type { Difficulty, FlagLabel, Mirror, StoryEdit, StoryPreset } from '../bindings';

/** The two story fields the editor can change. */
export type StoryDraft = { flags: boolean[]; folders: boolean[] };

export type StoryGroupKey = 'intro' | 'chapters' | 'bosses' | 'quests' | 'lobby';

export type StoryGroup = { key: StoryGroupKey; label: string; flags: FlagLabel[] };

/**
 * Group sizes in the order `dw4ipc::ui::flag_labels` concatenates them
 * (`INTRO_FLAGS`, `CHAPTER_FLAGS`, `BOSS_FLAGS`, `QUEST_FLAGS`, `LOBBY_FLAGS`).
 */
const GROUPS: readonly { key: StoryGroupKey; label: string; size: number }[] = [
  { key: 'intro', label: 'Intro', size: 7 },
  { key: 'chapters', label: 'Chapters', size: 6 },
  { key: 'bosses', label: 'Bosses', size: 7 },
  { key: 'quests', label: 'Quests', size: 6 },
  { key: 'lobby', label: 'Lobby', size: 7 },
];

const TOTAL_LABELS = GROUPS.reduce((sum, group) => sum + group.size, 0);

/**
 * Slice the flat label list into its groups.
 *
 * Throws if the length is not the Rust total, so a label added in `flags.rs`
 * without updating the frontend fails loudly instead of mis-grouping.
 */
export function storyGroups(labels: readonly FlagLabel[]): StoryGroup[] {
  if (labels.length !== TOTAL_LABELS) {
    throw new Error(
      `expected ${TOTAL_LABELS} flag labels in the Rust group order, got ${labels.length}`,
    );
  }
  let at = 0;
  return GROUPS.map(({ key, label, size }) => {
    const flags = labels.slice(at, at + size);
    at += size;
    return { key, label, flags };
  });
}

/** `flags::FOLDER_MIRROR_BASE`, indexed by `Difficulty::index`. */
const FOLDER_MIRROR_BASE: Record<Difficulty, number> = { Normal: 518, Hard: 530, VeryHard: 542 };

/** `flags::MIRRORED_FOLDERS`. */
const MIRRORED_FOLDERS = 10;

/** The flag that mirrors folder `folder` on `difficulty`, if it has one. */
export function folderMirrorFlag(folder: number, difficulty: Difficulty): number | null {
  if (folder < 0 || folder >= MIRRORED_FOLDERS) return null;
  return FOLDER_MIRROR_BASE[difficulty] + folder;
}

/**
 * Apply a preset to the story draft.
 *
 * The Python `_on_story_preset` writes the preset's full dict — explicit 0s
 * included — so every flag in the five groups and all twelve folders are
 * *replaced*, not OR'd. `governedFlags` is that group union, which the caller
 * has from [`storyGroups`]; flags outside it are left alone.
 */
export function applyPresetToStory(
  preset: StoryPreset,
  story: StoryDraft,
  governedFlags: readonly number[],
): StoryDraft {
  const flags = [...story.flags];
  const folders = [...story.folders];

  const onFlags = new Set(preset.flags);
  for (const flag of governedFlags) {
    if (flag >= 0 && flag < flags.length) flags[flag] = onFlags.has(flag);
  }

  const onFolders = new Set(preset.folders);
  for (let i = 0; i < folders.length; i += 1) folders[i] = onFolders.has(i);

  return { flags, folders };
}

function mirrorFor(row: Mirror, difficulty: Difficulty): number {
  switch (difficulty) {
    case 'Normal':
      return row.normal;
    case 'Hard':
      return row.hard;
    case 'VeryHard':
      return row.very_hard;
  }
}

/**
 * The mirror bits a set of story edits will write at `difficulty`.
 *
 * Display only: Rust performs the real mirroring in `Document::apply`.
 */
export function mirrorPreview(
  edits: readonly StoryEdit[],
  mirrors: readonly Mirror[],
  difficulty: Difficulty,
): { flag: number; value: boolean }[] {
  const out: { flag: number; value: boolean }[] = [];
  for (const edit of edits) {
    if (edit.kind === 'flag') {
      const row = mirrors.find((entry) => entry.active === edit.index);
      if (row) out.push({ flag: mirrorFor(row, difficulty), value: edit.value });
    } else {
      const target = folderMirrorFlag(edit.index, difficulty);
      if (target !== null) out.push({ flag: target, value: edit.value });
    }
  }
  return out;
}
