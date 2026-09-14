import { describe, expect, it } from 'vitest';
import type { StoryPreset } from '../bindings';
import appInfoJson from '../ipc/fixtures/app_info.json';
import { applyPresetToStory, folderMirrorFlag, mirrorPreview, storyGroups } from './story';

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

describe('mirrorPreview', () => {
  it('lists the mirror target for each edited bit at the chosen difficulty', () => {
    const preview = mirrorPreview(
      [
        { kind: 'flag', index: 66, value: true },
        { kind: 'folder', index: 0, value: true },
      ],
      appInfoJson.ui.mirrors,
      'Hard',
    );
    expect(preview).toContainEqual({ flag: 73, value: true });
    expect(preview).toContainEqual({ flag: 530, value: true });
  });

  it('skips a bit that has no mirror', () => {
    const preview = mirrorPreview([{ kind: 'flag', index: 999, value: true }], appInfoJson.ui.mirrors, 'Normal');
    expect(preview).toEqual([]);
  });
});
