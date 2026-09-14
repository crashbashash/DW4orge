import type {
  AppInfo,
  EditSet,
  IpcError,
  Mode,
  NewSaveRequest,
  OpenResult,
  SaveView,
  Species,
  SpeciesStats,
} from '../bindings';
import type { Backend, ValidationReport } from './backend';
import appInfoJson from './fixtures/app_info.json';
import openResultJson from './fixtures/open_result.raw.json';
import { mirrorTargets } from '../lib/story';

export type MockOptions = {
  validateEdits?: (edits: EditSet, mode: Mode) => ValidationReport;
  speciesStats?: (species: Species, mode: Mode) => SpeciesStats;
  openPaths?: (string | null)[];
  savePaths?: (string | null)[];
};

const NO_DOCUMENT: IpcError = { kind: 'no_open_document' };

/**
 * An in-memory backend over the committed fixtures.
 *
 * A development and test convenience, never a source of truth: it applies a
 * draft to a view only so `npm run dev` shows edits landing. Cap, category and
 * mod-socket rules stay in Rust.
 */
export function createMockBackend(options: MockOptions = {}): Backend {
  // SAFETY: `app_info.json` is rendered from `dw4ipc::app_info()`, so the JSON
  // matches `AppInfo` by construction; the drift test pins that.
  const appInfo = appInfoJson as unknown as AppInfo;
  let current: OpenResult | null = null;

  const openPaths = [...(options.openPaths ?? ['/mock/Mcd001.ps2'])];
  const savePaths = [...(options.savePaths ?? ['/mock/out.raw'])];

  // SAFETY: `open_result.raw.json` is rendered from a real `OpenResult` by the
  // same generator and pinned by `tests/ui_fixtures.rs`.
  const fixture = (): OpenResult => structuredClone(openResultJson) as unknown as OpenResult;

  const needDoc = (): OpenResult => {
    if (!current) throw NO_DOCUMENT;
    return current;
  };

  return {
    appInfo: async () => structuredClone(appInfo),

    openSave: async (path) => {
      current = { ...fixture(), path };
      return structuredClone(current);
    },

    openSample: async () => {
      current = { ...fixture(), path: '/mock/Mcd001.ps2' };
      return structuredClone(current);
    },

    newSave: async (request: NewSaveRequest) => {
      const base = fixture();
      current = {
        path: null,
        source: 'raw',
        view: { ...base.view, species: request.species, name: request.name },
      };
      return structuredClone(current);
    },

    getView: async () => structuredClone(needDoc().view),

    validateEdits: async (edits, mode) =>
      options.validateEdits?.(edits, mode) ?? { errors: [], warnings: [] },

    save: async (edits) => {
      const doc = needDoc();
      current = { ...doc, view: applyEdits(doc.view, edits, appInfo.ui.mirrors) };
      return structuredClone(current);
    },

    saveAs: async (path, edits) => {
      const doc = needDoc();
      current = { ...doc, path, view: applyEdits(doc.view, edits, appInfo.ui.mirrors) };
      return structuredClone(current);
    },

    speciesStats: async (species, mode) =>
      options.speciesStats?.(species, mode) ?? {
        level: 1,
        exp: 0,
        tech: [1, 1, 1, 1, 1, 1, 1, 1, 1],
        upcnt: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
      },

    pickOpenPath: async () => openPaths.shift() ?? null,
    pickSavePath: async () => savePaths.shift() ?? null,
  };
}

/** Mock-only: fold a draft back into a view so edits are visible in dev. */
function applyEdits(view: SaveView, edits: EditSet, mirrors: AppInfo['ui']['mirrors']): SaveView {
  const storyFlags = [...view.story_flags];
  const storyFolders = [...view.story_folders];
  const difficulty = edits.difficulty === 'auto' ? view.difficulty : edits.difficulty.fixed;
  for (const edit of edits.story) {
    const value = edit.value ? 1 : 0;
    if (edit.kind === 'flag') storyFlags[edit.index] = value;
    else storyFolders[edit.index] = value;
  }
  // Rust copies each edit into the chosen difficulty's mirror column and every
  // lower one; the mock follows so dev saves look right.
  for (const edit of edits.story) {
    const value = edit.value ? 1 : 0;
    for (const { flag } of mirrorTargets(edit, mirrors, difficulty)) storyFlags[flag] = value;
  }

  return {
    ...view,
    species: edits.species,
    name: edits.name,
    bit: edits.bit,
    xdata: edits.xdata,
    junk_counter: edits.junk,
    level: edits.level,
    exp: edits.exp,
    tech: edits.tech,
    upcnt: edits.upcnt,
    device: edits.device,
    weapons: edits.weapons,
    armor: edits.armor,
    sub: edits.sub,
    // `wmods`/`amods` are device indices in a view and base ids in a draft; the
    // mock leaves them as loaded rather than reimplementing Rust's resolution.
    story_flags: storyFlags,
    story_folders: storyFolders,
    bank_bit: edits.bank_bit,
    disks: edits.disks,
    bank_items: [...edits.bank_items],
  };
}
