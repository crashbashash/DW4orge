import { describe, expect, it } from 'vitest';
import type { StoryPreset } from '../bindings';
import appInfoJson from '../ipc/fixtures/app_info.json';
import { applyPresetToStory, folderMirrorFlag, mirrorPreview, storyForDifficulty, storyGroups } from './story';

const labels = appInfoJson.ui.flag_labels;
const groups = storyGroups(labels);
const allGroupFlags = groups.flatMap((group) => group.flags.map((flag) => flag.flag));

const afterWorld2: StoryPreset = {
  name: 'After World 2',
  flags: [0, 4, 66, 67, 68, 703],
  folders: [0, 1, 2, 3, 4, 5],
};

describe('storyGroups', () => {
  it('slices the flat label list in the Rust order', () => {
    expect(groups.map((group) => group.key)).toEqual([
      'intro',
      'chapters',
      'bosses',
      'quests',
      'lobby',
    ]);
    expect(groups.map((group) => group.flags.length)).toEqual([7, 6, 7, 6, 7]);
    expect(labels).toHaveLength(33);
    // The boundary between intro and chapters is flag 701, per flags.rs.
    expect(groups[0].flags.at(-1)?.flag).toBe(6);
    expect(groups[1].flags[0].flag).toBe(701);
  });

  it('refuses a label list it cannot slice', () => {
    expect(() => storyGroups(labels.slice(0, 5))).toThrow(/33/);
  });
});

describe('folderMirrorFlag', () => {
  it('mirrors folders 0-9 to 518/530/542 plus the index', () => {
    expect(folderMirrorFlag(0, 'Normal')).toBe(518);
    expect(folderMirrorFlag(9, 'Hard')).toBe(539);
    expect(folderMirrorFlag(4, 'VeryHard')).toBe(546);
    expect(folderMirrorFlag(10, 'Normal')).toBeNull();
  });
});

describe('applyPresetToStory', () => {
  it('replaces the governed flags and folders, as the Python editor does', () => {
    const before = {
      flags: new Array(1024).fill(false) as boolean[],
      folders: new Array(12).fill(true) as boolean[],
    };
    before.flags[3] = true;
    before.flags[66] = true;
    before.flags[82] = true;
    before.flags[999] = true;

    const after = applyPresetToStory(afterWorld2, before, allGroupFlags);

    expect(after.flags[0]).toBe(true);
    expect(after.flags[4]).toBe(true);
    expect(after.flags[3]).toBe(false);
    expect(after.flags[66]).toBe(true);
    expect(after.flags[82]).toBe(false);
    // A flag outside every group is not touched.
    expect(after.flags[999]).toBe(true);
    expect(after.folders.slice(6)).toEqual(new Array(6).fill(false));
    expect(after.folders[5]).toBe(true);
  });
});

describe('storyForDifficulty', () => {
  // A live block whose only story bits are a Normal-mirrored flag and folder,
  // plus a shared lobby flag that no difficulty mirrors.
  const live = {
    flags: new Array(1024).fill(false) as boolean[],
    folders: new Array(12).fill(false) as boolean[],
  };
  live.flags[699] = true; // flag 0's Normal mirror
  live.flags[518] = true; // folder 0's Normal mirror
  live.flags[24] = true; // lobby flag, not mirrored

  it('reads a difficulty story out of its own mirror column', () => {
    const normal = storyForDifficulty(live.flags, live.folders, 'Normal', appInfoJson.ui.mirrors);
    expect(normal.flags[0]).toBe(true);
    expect(normal.folders[0]).toBe(true);
    expect(normal.flags[24]).toBe(true);
  });

  it('shows a difficulty with no stored column as empty, not as the live flags', () => {
    const hard = storyForDifficulty(live.flags, live.folders, 'Hard', appInfoJson.ui.mirrors);
    expect(hard.flags[0]).toBe(false);
    expect(hard.folders[0]).toBe(false);
    // Non-mirrored bits are shared, so they survive the difficulty switch.
    expect(hard.flags[24]).toBe(true);
  });

  it('does not mutate the live arrays it is given', () => {
    const flags = [...live.flags];
    storyForDifficulty(flags, live.folders, 'Normal', appInfoJson.ui.mirrors);
    expect(flags).toEqual(live.flags);
  });
});

describe('mirrorPreview', () => {
  it('lists the mirror target for each edited bit at the chosen difficulty and below', () => {
    const preview = mirrorPreview(
      [
        { kind: 'flag', index: 66, value: true },
        { kind: 'folder', index: 0, value: true },
      ],
      appInfoJson.ui.mirrors,
      'Hard',
    );
    expect(preview).toContainEqual({ difficulty: 'Normal', flag: 707, value: true });
    expect(preview).toContainEqual({ difficulty: 'Hard', flag: 73, value: true });
    expect(preview).toContainEqual({ difficulty: 'Normal', flag: 518, value: true });
    expect(preview).toContainEqual({ difficulty: 'Hard', flag: 530, value: true });
    // An edit made on Hard must never reach Very Hard.
    expect(preview.some((row) => row.difficulty === 'VeryHard')).toBe(false);
  });

  it('copies a Very Hard edit into all three columns', () => {
    const preview = mirrorPreview(
      [{ kind: 'flag', index: 66, value: true }],
      appInfoJson.ui.mirrors,
      'VeryHard',
    );
    expect(preview).toEqual([
      { difficulty: 'Normal', flag: 707, value: true },
      { difficulty: 'Hard', flag: 73, value: true },
      { difficulty: 'VeryHard', flag: 77, value: true },
    ]);
  });

  it('keeps a Normal edit on Normal alone', () => {
    const preview = mirrorPreview(
      [{ kind: 'flag', index: 66, value: true }],
      appInfoJson.ui.mirrors,
      'Normal',
    );
    expect(preview).toEqual([{ difficulty: 'Normal', flag: 707, value: true }]);
  });

  it('skips a bit that has no mirror', () => {
    const preview = mirrorPreview(
      [{ kind: 'flag', index: 999, value: true }],
      appInfoJson.ui.mirrors,
      'Normal',
    );
    expect(preview).toEqual([]);
  });
});
