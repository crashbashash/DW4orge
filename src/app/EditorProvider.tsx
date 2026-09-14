import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  type ReactNode,
} from 'react';
import type { EditSet, NewSaveRequest, OpenResult, Species, SpeciesStats, StoryKind } from '../bindings';
import { useBackend } from '../ipc/context';
import { describeIpcError, fieldErrors } from '../ipc/errors';
import type { StoryDraft } from '../lib/story';
import {
  buildEditSet,
  initialState,
  reducer,
  type SectionId,
  type StoreState,
} from './store';
import { applyTheme, detectTheme, storeTheme, type Theme } from './theme';

/** How long an edit settles before the draft is validated. */
const VALIDATE_DEBOUNCE_MS = 200;

export type EditorApi = {
  state: StoreState;
  setField(patch: Partial<EditSet>, tag: string): void;
  setDevice(index: number, id: number): void;
  setBankItem(index: number, id: number): void;
  setStory(kind: StoryKind, index: number, value: boolean): void;
  setStoryDraft(story: StoryDraft): void;
  setDifficulty(difficulty: StoreState['difficulty']): void;
  setMode(mode: StoreState['mode']): void;
  setSection(section: SectionId): void;
  clearError(): void;
  undo(): void;
  redo(): void;
  toggleTheme(): void;
  open(): Promise<void>;
  openSample(): Promise<void>;
  canLoadSample: boolean;
  newSave(request: NewSaveRequest): Promise<void>;
  save(): Promise<void>;
  saveAs(): Promise<void>;
  speciesStats(species: Species, mode: StoreState['mode']): Promise<SpeciesStats>;
};

const EditorContext = createContext<EditorApi | null>(null);

export function EditorProvider({ children }: { children: ReactNode }) {
  const backend = useBackend();
  const [state, dispatch] = useReducer(reducer, initialState, (base) => ({
    ...base,
    theme: detectTheme(),
  }));
  const validationSeq = useRef(0);

  useEffect(() => {
    let alive = true;
    backend
      .appInfo()
      .then((info) => {
        if (alive) dispatch({ type: 'appInfo', info });
      })
      .catch((err: unknown) => {
        if (alive) dispatch({ type: 'failed', message: describeIpcError(err) });
      });
    return () => {
      alive = false;
    };
  }, [backend]);

  const edits = useMemo(() => {
    if (!state.draft || !state.story || !state.session) return null;
    return buildEditSet(state.draft, state.story, state.session.baselineStory, state.difficulty);
  }, [state.draft, state.story, state.difficulty, state.session]);

  // Authoritative validation, debounced. The local cap checks in the sections
  // give instant feedback; this is the single source of truth for the rest.
  useEffect(() => {
    if (!edits) return undefined;
    const seq = (validationSeq.current += 1);
    const mode = state.mode;
    const handle = window.setTimeout(() => {
      backend
        .validateEdits(edits, mode)
        .then((report) => {
          if (seq === validationSeq.current) dispatch({ type: 'validation', report });
        })
        .catch((err: unknown) => {
          if (seq === validationSeq.current) {
            dispatch({ type: 'failed', message: describeIpcError(err) });
          }
        });
    }, VALIDATE_DEBOUNCE_MS);
    return () => window.clearTimeout(handle);
  }, [backend, edits, state.mode]);

  const reportFailure = useCallback((err: unknown) => {
    const fields = fieldErrors(err);
    if (fields) dispatch({ type: 'validation', report: { errors: fields, warnings: [] } });
    else dispatch({ type: 'failed', message: describeIpcError(err) });
  }, []);

  const load = useCallback(
    async (run: () => Promise<OpenResult>) => {
      dispatch({ type: 'busy', busy: true });
      try {
        dispatch({ type: 'loaded', result: await run() });
      } catch (err: unknown) {
        reportFailure(err);
      } finally {
        dispatch({ type: 'busy', busy: false });
      }
    },
    [reportFailure],
  );

  const open = useCallback(async () => {
    const path = await backend.pickOpenPath();
    if (!path) return;
    await load(() => backend.openSave(path));
  }, [backend, load]);

  const openSample = useCallback(async () => {
    const sample = backend.openSample;
    if (!sample) return;
    await load(() => sample());
  }, [backend, load]);

  const newSave = useCallback(
    async (request: NewSaveRequest) => {
      await load(() => backend.newSave(request));
    },
    [backend, load],
  );

  const write = useCallback(
    async (run: () => Promise<OpenResult>) => {
      dispatch({ type: 'busy', busy: true });
      try {
        const result = await run();
        dispatch({ type: 'saved', result, message: 'Saved' });
      } catch (err: unknown) {
        reportFailure(err);
      } finally {
        dispatch({ type: 'busy', busy: false });
      }
    },
    [reportFailure],
  );

  const saveAs = useCallback(async () => {
    if (!edits) return;
    const path = await backend.pickSavePath('save.raw');
    if (!path) return;
    await write(() => backend.saveAs(path, edits, state.mode));
  }, [backend, edits, state.mode, write]);

  const save = useCallback(async () => {
    if (!edits || !state.session) return;
    if (!state.session.path) {
      await saveAs();
      return;
    }
    await write(() => backend.save(edits, state.mode));
  }, [backend, edits, saveAs, state.mode, state.session, write]);

  const speciesStats = useCallback(
    (species: Species, mode: StoreState['mode']) => backend.speciesStats(species, mode),
    [backend],
  );

  const toggleTheme = useCallback(() => {
    const next: Theme = state.theme === 'dark' ? 'light' : 'dark';
    applyTheme(next, document.documentElement);
    storeTheme(next);
    dispatch({ type: 'theme', theme: next });
  }, [state.theme]);

  const api = useMemo<EditorApi>(
    () => ({
      state,
      setField: (patch, tag) => dispatch({ type: 'field', patch, tag }),
      setDevice: (index, id) => dispatch({ type: 'device', index, id }),
      setBankItem: (index, id) => dispatch({ type: 'bankItem', index, id }),
      setStory: (kind, index, value) => dispatch({ type: 'story', kind, index, value }),
      setStoryDraft: (story) => dispatch({ type: 'storyDraft', story }),
      setDifficulty: (difficulty) => dispatch({ type: 'difficulty', difficulty }),
      setMode: (mode) => dispatch({ type: 'mode', mode }),
      setSection: (section) => dispatch({ type: 'section', section }),
      clearError: () => dispatch({ type: 'clearError' }),
      undo: () => dispatch({ type: 'undo' }),
      redo: () => dispatch({ type: 'redo' }),
      toggleTheme,
      open,
      openSample,
      canLoadSample: typeof backend.openSample === 'function',
      newSave,
      save,
      saveAs,
      speciesStats,
    }),
    [backend.openSample, newSave, open, openSample, save, saveAs, speciesStats, state, toggleTheme],
  );

  return <EditorContext.Provider value={api}>{children}</EditorContext.Provider>;
}

export function useEditor(): EditorApi {
  const api = useContext(EditorContext);
  if (!api) throw new Error('useEditor must be used inside EditorProvider');
  return api;
}
