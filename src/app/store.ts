import type {
  AppInfo,
  Difficulty,
  DifficultyChoice,
  EditSet,
  Mirror,
  Mode,
  OpenResult,
  SaveView,
  SourceKind,
  StoryEdit,
  StoryKind,
} from '../bindings';
import type { ValidationReport } from '../ipc/backend';
import { socketBaseId } from '../lib/items';
import { storyForDifficulty, type StoryDraft } from '../lib/story';
import type { History, Snapshot } from './history';
import { record, redo, undo } from './history';
import type { Theme } from './theme';

export type SectionId = 'character' | 'items' | 'equipment' | 'disks' | 'story' | 'bank';

/** The open save plus the state a dirty check compares against. */
export type Session = {
  path: string | null;
  source: SourceKind;
  view: SaveView;
  baseline: EditSet;
  /**
   * The story each difficulty is stored with, one entry per difficulty.
   *
   * The Story tab edits one difficulty at a time, so the visible draft is
   * re-seeded from here whenever the difficulty selector changes; the entry
   * also supplies that difficulty's diff baseline.
   */
  baselines: Record<Difficulty, StoryDraft>;
};

/**
 * The filename the Save As dialog should offer.
 *
 * A pathless session is a brand-new save, so it defaults to a `.ps2` memory
 * card — the form PCSX2 mounts. An already-open file keeps its own name, so a
 * raw save stays raw on Save As.
 */
export function defaultSaveName(path: string | null): string {
  if (!path) return 'save.ps2';
  return path.split(/[\\/]/).pop() || 'save.ps2';
}

export type StoreState = {
  status: 'loading' | 'ready' | 'busy';
  appInfo: AppInfo | null;
  session: Session | null;
  draft: EditSet | null;
  story: StoryDraft | null;
  difficulty: DifficultyChoice;
  mode: Mode;
  validation: ValidationReport;
  history: History;
  theme: Theme;
  section: SectionId;
  lastWrite: { path: string | null; message: string } | null;
  error: string | null;
};

export type Action =
  | { type: 'appInfo'; info: AppInfo }
  | { type: 'loaded'; result: OpenResult }
  | { type: 'field'; patch: Partial<EditSet>; tag: string }
  | { type: 'device'; index: number; id: number }
  | { type: 'bankItem'; index: number; id: number }
  | { type: 'story'; kind: StoryKind; index: number; value: boolean }
  | { type: 'storyDraft'; story: StoryDraft }
  | { type: 'difficulty'; difficulty: DifficultyChoice }
  | { type: 'mode'; mode: Mode }
  | { type: 'undo' }
  | { type: 'redo' }
  | { type: 'validation'; report: ValidationReport }
  | { type: 'busy'; busy: boolean }
  | { type: 'saved'; result: OpenResult; message: string }
  | { type: 'failed'; message: string }
  | { type: 'clearError' }
  | { type: 'theme'; theme: Theme }
  | { type: 'section'; section: SectionId };

export const initialState: StoreState = {
  status: 'loading',
  appInfo: null,
  session: null,
  draft: null,
  story: null,
  difficulty: 'auto',
  mode: 'normal',
  validation: { errors: [], warnings: [] },
  history: { past: [], future: [], tag: null },
  theme: 'dark',
  section: 'character',
  lastWrite: null,
  error: null,
};

/** The draft an [`OpenResult`] starts from, mirroring `SaveView::to_edit_set`. */
export function viewToEditSet(view: SaveView): EditSet {
  const [w0, w1, w2, w3, w4] = view.wmods;
  const [a0, a1, a2, a3, a4] = view.amods;

  return {
    species: view.species,
    name: view.name,
    bit: view.bit,
    xdata: view.xdata,
    // `EditSet::junk` carries the stored counter, exactly as `to_edit_set` does.
    junk: view.junk_counter,
    level: view.level,
    exp: view.exp,
    tech: view.tech,
    upcnt: view.upcnt,
    device: view.device,
    weapons: view.weapons,
    armor: view.armor,
    sub: view.sub,
    // Sockets are device indices in a view and chip base ids in a draft.
    wmods: [
      socketBaseId(view.device, w0),
      socketBaseId(view.device, w1),
      socketBaseId(view.device, w2),
      socketBaseId(view.device, w3),
      socketBaseId(view.device, w4),
    ],
    amods: [
      socketBaseId(view.device, a0),
      socketBaseId(view.device, a1),
      socketBaseId(view.device, a2),
      socketBaseId(view.device, a3),
      socketBaseId(view.device, a4),
    ],
    story: [],
    difficulty: { fixed: view.difficulty },
    bank_bit: view.bank_bit,
    disks: view.disks,
    bank_items: [...view.bank_items],
  };
}

/** The live/active story draft a view starts from. */
export function storyDraftFromView(view: SaveView): StoryDraft {
  return {
    flags: view.story_flags.map((byte) => byte !== 0),
    folders: view.story_folders.map((byte) => byte !== 0),
  };
}

/**
 * Each difficulty's stored story, read from the save's mirror columns.
 *
 * The live bytes are the shared base; `storyForDifficulty` overlays the
 * selected difficulty's mirrored bits on top of them.
 */
export function storyDraftsFromView(
  view: SaveView,
  mirrors: readonly Mirror[],
): Record<Difficulty, StoryDraft> {
  const live = storyDraftFromView(view);
  return {
    Normal: storyForDifficulty(live.flags, live.folders, 'Normal', mirrors),
    Hard: storyForDifficulty(live.flags, live.folders, 'Hard', mirrors),
    VeryHard: storyForDifficulty(live.flags, live.folders, 'VeryHard', mirrors),
  };
}

/**
 * The difficulty a selector value resolves to.
 *
 * `auto` follows the save's detected difficulty, which is what the Rust side
 * does too, so the draft shown and the mirrors written always agree.
 */
export function resolveDifficulty(
  choice: DifficultyChoice,
  session: Session | null,
): Difficulty {
  if (choice === 'auto') return session?.view.difficulty ?? 'Normal';
  return choice.fixed;
}

/** The story bits that differ from the baseline, as the wire wants them. */
export function diffStory(current: StoryDraft, baseline: StoryDraft): StoryEdit[] {
  const out: StoryEdit[] = [];
  const flags = Math.min(current.flags.length, baseline.flags.length);
  for (let i = 0; i < flags; i += 1) {
    if (current.flags[i] !== baseline.flags[i]) {
      out.push({ kind: 'flag', index: i, value: current.flags[i] ?? false });
    }
  }
  const folders = Math.min(current.folders.length, baseline.folders.length);
  for (let i = 0; i < folders; i += 1) {
    if (current.folders[i] !== baseline.folders[i]) {
      out.push({ kind: 'folder', index: i, value: current.folders[i] ?? false });
    }
  }
  return out;
}

/**
 * The wire `EditSet` for the given parts.
 *
 * Separated from [`toEditSet`] so a React effect can depend on exactly these
 * values instead of the whole store object (which would loop on validation).
 */
export function buildEditSet(
  draft: EditSet,
  story: StoryDraft,
  baselineStory: StoryDraft,
  difficulty: DifficultyChoice,
): EditSet {
  return { ...draft, story: diffStory(story, baselineStory), difficulty };
}

/**
 * The wire `EditSet` for the current store.
 *
 * Throws when there is no draft; every caller guards on `state.draft` first.
 */
export function toEditSet(state: StoreState): EditSet {
  if (!state.draft || !state.story || !state.session) {
    throw new Error('no draft to serialise');
  }
  const difficulty = resolveDifficulty(state.difficulty, state.session);
  return buildEditSet(
    state.draft,
    state.story,
    state.session.baselines[difficulty],
    state.difficulty,
  );
}

function snapshotOf(state: StoreState): Snapshot | null {
  if (!state.draft || !state.story) return null;
  return { draft: state.draft, story: state.story, difficulty: state.difficulty };
}

/** Apply a pure change to the draft/story and record it in history. */
function edit(
  state: StoreState,
  mutate: (draft: EditSet, story: StoryDraft) => { draft: EditSet; story: StoryDraft },
  tag: string | null,
): StoreState {
  const current = snapshotOf(state);
  if (!current || !state.draft || !state.story) return state;
  const next = mutate(state.draft, state.story);
  return {
    ...state,
    draft: next.draft,
    story: next.story,
    history: record(state.history, current, tag),
  };
}

function sessionFrom(result: OpenResult, mirrors: readonly Mirror[]): Session {
  return {
    path: result.path,
    source: result.source,
    view: result.view,
    baseline: viewToEditSet(result.view),
    baselines: storyDraftsFromView(result.view, mirrors),
  };
}

function withSlot(slots: boolean[], index: number, value: boolean): boolean[] {
  const next = [...slots];
  next[index] = value;
  return next;
}

export function reducer(state: StoreState, action: Action): StoreState {
  switch (action.type) {
    case 'appInfo':
      return { ...state, appInfo: action.info, status: 'ready' };

    case 'loaded': {
      const mirrors = state.appInfo?.ui.mirrors ?? [];
      return {
        ...state,
        status: 'ready',
        session: sessionFrom(action.result, mirrors),
        draft: viewToEditSet(action.result.view),
        story: storyDraftsFromView(action.result.view, mirrors)[action.result.view.difficulty],
        difficulty: { fixed: action.result.view.difficulty },
        validation: { errors: [], warnings: [] },
        history: { past: [], future: [], tag: null },
        lastWrite: null,
        error: null,
      };
    }

    case 'field':
      return edit(state, (draft, story) => ({ draft: { ...draft, ...action.patch }, story }), action.tag);

    case 'device':
      return edit(
        state,
        (draft, story) => {
          const device = [...draft.device];
          device[action.index] = action.id;
          // SAFETY: `device` is a clone of the binding's 30-element tuple, so it
          // keeps that length; only one element changes.
          return { draft: { ...draft, device: device as unknown as EditSet['device'] }, story };
        },
        null,
      );

    case 'bankItem':
      return edit(
        state,
        (draft, story) => {
          const bank = [...draft.bank_items];
          bank[action.index] = action.id;
          return { draft: { ...draft, bank_items: bank }, story };
        },
        null,
      );

    case 'story':
      return edit(
        state,
        (draft, story) => {
          const next: StoryDraft =
            action.kind === 'flag'
              ? { ...story, flags: withSlot(story.flags, action.index, action.value) }
              : { ...story, folders: withSlot(story.folders, action.index, action.value) };
          return { draft, story: next };
        },
        null,
      );

    case 'storyDraft':
      return edit(state, (draft) => ({ draft, story: action.story }), null);

    case 'difficulty': {
      const current = snapshotOf(state);
      if (!current || !state.session) return state;
      // Switching the selector shows that difficulty's stored story: the draft
      // is re-seeded from its baseline rather than carrying the previous
      // difficulty's edits along. Undo restores them.
      const stored = state.session.baselines[resolveDifficulty(action.difficulty, state.session)];
      return {
        ...state,
        difficulty: action.difficulty,
        story: { flags: [...stored.flags], folders: [...stored.folders] },
        history: record(state.history, current, null),
      };
    }

    case 'mode':
      return { ...state, mode: action.mode };

    case 'undo': {
      const current = snapshotOf(state);
      if (!current) return state;
      const stepped = undo(state.history, current);
      if (!stepped) return state;
      return { ...state, ...stepped.present, history: stepped.history };
    }

    case 'redo': {
      const current = snapshotOf(state);
      if (!current) return state;
      const stepped = redo(state.history, current);
      if (!stepped) return state;
      return { ...state, ...stepped.present, history: stepped.history };
    }

    case 'validation':
      return { ...state, validation: action.report };

    case 'busy':
      return { ...state, status: action.busy ? 'busy' : state.session ? 'ready' : 'loading' };

    case 'saved': {
      const mirrors = state.appInfo?.ui.mirrors ?? [];
      return {
        ...state,
        status: 'ready',
        session: sessionFrom(action.result, mirrors),
        draft: viewToEditSet(action.result.view),
        story: storyDraftsFromView(action.result.view, mirrors)[action.result.view.difficulty],
        difficulty: { fixed: action.result.view.difficulty },
        validation: { errors: [], warnings: [] },
        history: { past: [], future: [], tag: null },
        lastWrite: { path: action.result.path, message: action.message },
        error: null,
      };
    }

    case 'failed':
      return { ...state, status: state.session ? 'ready' : 'loading', error: action.message };

    case 'clearError':
      return { ...state, error: null };

    case 'theme':
      return { ...state, theme: action.theme };

    case 'section':
      return { ...state, section: action.section };
  }
}
