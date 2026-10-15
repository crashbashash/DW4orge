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
  speciesStats?: (species: Species, mode: Mode) => SpeciesStats | Promise<SpeciesStats>;
  openPaths?: (string | null)[];
  savePaths?: (string | null)[];
  onCloseRequested?: (handler: () => boolean) => Promise<() => void>;
  closeWindow?: () => Promise<void>;
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

  const files = new Map<string, { result: OpenResult; views: Map<string, SaveView> }>();
  let workingViews = new Map<string, SaveView>();
  const openMock = (path: string, region?: import('../bindings').Region, selection?: string): OpenResult => {
    let stored = files.get(path);
    if (!stored) {
      const result: OpenResult = { ...fixture(), path, source: /\.ps2$/i.test(path) ? 'memcard' : 'raw', region: region ?? (/\.ps2$/i.test(path) ? 'usa' : null) };
      stored = { result, views: new Map([[result.character_selection ?? 'ps2#1', result.view]]) };
      files.set(path, structuredClone(stored));
    }
    const id = selection ?? stored.result.character_boxes?.[0]?.id ?? stored.result.character_selection;
    if (selection && !stored.result.character_boxes?.some(box => box.id === selection)) throw { kind: 'unsupported', message: 'No occupied character box matches the selection' };
    current = { ...structuredClone(stored.result), path, region: region ?? stored.result.region, pending_write: false,
      character_selection: id, view: structuredClone(stored.views.get(id ?? '') ?? stored.result.view) };
    workingViews = structuredClone(stored.views);
    return structuredClone(current);
  };
  const persist = (path: string, edits: EditSet, region?: import('../bindings').Region): OpenResult => {
    const doc = needDoc();
    const view = applyEdits(doc.view, edits, appInfo.ui.mirrors);
    const id = doc.character_selection ?? (doc.source.startsWith('gamecube') ? 'gamecube#1' : 'ps2#1');
    workingViews.set(id, structuredClone(view));
    const choices = doc.character_boxes ?? [{ id, label: `Box 1 · ${view.name}`, box_number: 1 }];
    current = { ...doc, path, pending_write: false, region: region ?? doc.region, view,
      character_selection: id, character_boxes: choices.map(box => box.id === id ? { ...box, label: `Box ${box.box_number} · ${view.name}` } : box) };
    files.set(path, { result: structuredClone(current), views: structuredClone(workingViews) });
    return structuredClone(current);
  };

  return {
    appInfo: async () => structuredClone(appInfo),

    openSave: async (path, region, selection) => openMock(path, region, selection),

    openSample: async () => openMock('/mock/Mcd001.ps2', 'usa'),

    newSave: async (request: NewSaveRequest) => {
      const base = fixture();
      current = {
        region: request.region ?? 'usa',
        path: null,
        pending_write: true,
        empty_boxes: request.format === 'xbox' || request.format === 'xbox_mu' ? undefined : [2,3,4],
        source: request.format === 'memcard' ? 'raw' : request.format ?? 'raw',
        view: { ...base.view, species: request.species, name: request.name },
      };
      workingViews = new Map();
      return structuredClone(current);
    },

    createBox: async (boxNumber, request) => {
      const previous = needDoc();
      const stored = previous.path ? files.get(previous.path) : undefined;
      if (!stored?.result.empty_boxes?.includes(boxNumber)) throw { kind: 'unsupported', message: 'This character box is not empty' };
      const doc = openMock(previous.path!, previous.region ?? undefined);
      const prefix = doc.character_selection?.split('#')[0] ?? (doc.source.startsWith('gamecube') ? 'gamecube' : 'ps2');
      current = { ...doc, pending_write: true, character_selection: `${prefix}#${boxNumber}`,
        empty_boxes: doc.empty_boxes!.filter(number => number !== boxNumber),
        character_boxes: [...doc.character_boxes ?? [], { id: `${prefix}#${boxNumber}`, label: `Box ${boxNumber} · ${request.name}`, box_number: boxNumber }].sort((a,b)=>a.box_number-b.box_number),
        view: { ...fixture().view, species: request.species, name: request.name, bit: 0, xdata: 0, level: 1, exp: 0 } };
      workingViews.set(`${prefix}#${boxNumber}`, structuredClone(current.view));
      return structuredClone(current);
    },

    getView: async () => structuredClone(needDoc().view),

    validateEdits: async (edits, mode) =>
      options.validateEdits?.(edits, mode) ?? { errors: [], warnings: [] },

    save: async (edits) => persist(needDoc().path ?? '/mock/out.ps2', edits),
    saveAs: async (path, edits, _mode, region) => persist(path, edits, region),

    speciesStats: async (species, mode) =>
      options.speciesStats?.(species, mode) ?? {
        level: 1,
        exp: 0,
        tech: [1, 1, 1, 1, 1, 1, 1, 1, 1],
        upcnt: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
      },

    pickOpenPath: async () => openPaths.shift() ?? null,
    pickSavePath: async () => savePaths.shift() ?? null,

    // The browser has no native window to close, so the guard stays inert
    // unless a test supplies its own handler.
    onCloseRequested: options.onCloseRequested ?? (async () => () => {}),
    closeWindow: options.closeWindow ?? (async () => {}),
  };
}

/** Mock-only: fold a draft back into a view so edits are visible in dev. */
function applyEdits(view: SaveView, edits: EditSet, mirrors: AppInfo['ui']['mirrors']): SaveView {
  const storyFlags = [...view.story_flags];
  const storyFolders = [...view.story_folders];
  for (const edit of edits.story) {
    const value = edit.value ? 1 : 0;
    if (edit.kind === 'flag') storyFlags[edit.index] = value;
    else storyFolders[edit.index] = value;
  }
  // Rust copies each edit into its own difficulty's mirror column and every
  // lower one; the mock follows so dev saves look right.
  for (const edit of edits.story) {
    const value = edit.value ? 1 : 0;
    for (const { flag } of mirrorTargets(edit, mirrors)) storyFlags[flag] = value;
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
