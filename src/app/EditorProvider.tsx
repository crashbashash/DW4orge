import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import type { EditSet, NewSaveRequest, OpenResult, Region, Species, SpeciesStats, StoryKind } from '../bindings';
import { RegionDialog } from '../components/RegionDialog';
import { useBackend } from '../ipc/context';
import { describeIpcError, fieldErrors, isIpcError } from '../ipc/errors';
import type { StoryDraft } from '../lib/story';
import {
  buildEditSet,
  defaultSaveName,
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
  const [regionRequest, setRegionRequest] = useState<{ regions: Region[]; message: string } | null>(null);
  const regionResolve = useRef<((region: Region | null) => void) | null>(null);
  const chooseRegion = useCallback((regions: Region[], message: string) => new Promise<Region | null>(resolve => {
    regionResolve.current = resolve;
    setRegionRequest({ regions, message });
  }), []);
  const finishRegion = useCallback((region: Region | null) => {
    regionResolve.current?.(region);
    regionResolve.current = null;
    setRegionRequest(null);
  }, []);
  useEffect(() => () => { regionResolve.current?.(null); }, []);

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

  // Every difficulty's draft goes into the one payload; each story edit names
  // its own difficulty, so changing the selector never drops a pending edit.
  const edits = useMemo(() => {
    if (!state.draft || !state.session) return null;
    return buildEditSet(state.draft, state.stories, state.session.baselines);
  }, [state.draft, state.session, state.stories]);

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
    async (run: () => Promise<OpenResult | null>) => {
      dispatch({ type: 'busy', busy: true });
      try {
        const result = await run();
        if (result) dispatch({ type: 'loaded', result });
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
    await load(async () => {
      try { return await backend.openSave(path); }
      catch (err: unknown) {
        if (!isIpcError(err) || err.kind !== 'region_choice') throw err;
        const region = await chooseRegion(err.regions, 'This card contains saves from multiple regions. Choose the save to edit.');
        if (!region) return null;
        return backend.openSave(path, region);
      }
    });
  }, [backend, load, chooseRegion]);

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
    const path = await backend.pickSavePath(defaultSaveName(state.session?.path ?? null));
    if (!path) return;
    let region: Region | undefined;
    if (/\.ps2$/i.test(path) && state.session?.region == null) {
      const picked = await chooseRegion(['usa', 'europe', 'japan'], 'Raw saves do not identify their region. Choose the game that will use this memory card.');
      if (!picked) return;
      region = picked;
    }
    await write(() => region ? backend.saveAs(path, edits, state.mode, region) : backend.saveAs(path, edits, state.mode));
  }, [backend, edits, state.mode, state.session, write, chooseRegion]);

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

  return <EditorContext.Provider value={api}>{children}<RegionDialog request={regionRequest} finish={finishRegion} /></EditorContext.Provider>;
}

export function useEditor(): EditorApi {
  const api = useContext(EditorContext);
  if (!api) throw new Error('useEditor must be used inside EditorProvider');
  return api;
}
