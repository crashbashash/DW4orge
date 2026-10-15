import type {
  AppInfo,
  EditSet,
  FieldError,
  Mode,
  NewSaveRequest,
  OpenResult,
  Region,
  SaveView,
  Species,
  SpeciesStats,
} from '../bindings';

/**
 * A validation pass is a normal state, not an exception: errors and warnings
 * are both outcomes, and only a genuine failure rejects.
 */
export type ValidationReport = {
  errors: FieldError[];
  warnings: FieldError[];
};

/** Everything the UI can ask of the shell. */
export interface Backend {
  appInfo(): Promise<AppInfo>;
  openSave(path: string, region?: Region, characterSelection?: string): Promise<OpenResult>;
  newSave(request: NewSaveRequest): Promise<OpenResult>;
  getView(mode: Mode): Promise<SaveView>;
  validateEdits(edits: EditSet, mode: Mode): Promise<ValidationReport>;
  save(edits: EditSet, mode: Mode): Promise<OpenResult>;
  saveAs(path: string, edits: EditSet, mode: Mode, region?: Region): Promise<OpenResult>;
  speciesStats(species: Species, mode: Mode): Promise<SpeciesStats>;
  pickOpenPath(): Promise<string | null>;
  pickSavePath(defaultName: string, source?: import('../bindings').SourceKind): Promise<string | null>;
  /**
   * Register a handler for the shell's window-close request (the title bar's
   * close button, Alt+F4). The handler returns `true` to keep the window open
   * — the caller shows its own confirmation — and `false` to let the close
   * proceed. Returns an unsubscribe.
   */
  onCloseRequested(handler: () => boolean): Promise<() => void>;
  /** Force the window closed, bypassing {@link onCloseRequested}. */
  closeWindow(): Promise<void>;
  /**
   * Present only on the browser mock. The shell offers a one-click sample load
   * when it is defined, so `npm run dev` has something to show without a file
   * dialog (and without a webkit build).
   */
  openSample?: () => Promise<OpenResult>;
}

/** The open dialog lists every container first, then each kind alone. */
export const OPEN_FILTERS = [
  { name: 'Supported saves and memory cards', extensions: ['gci', 'gcp', 'ps2', 'raw', 'bin', 'dat', 'img'] },
  { name: 'GameCube memory card', extensions: ['raw', 'gcp'] },
  { name: 'PS2 memory card', extensions: ['ps2'] },
  { name: 'GameCube GCI save', extensions: ['gci'] },
  { name: 'Xbox HDD save', extensions: ['dat'] },
  { name: 'Xbox memory unit', extensions: ['img'] },
  { name: 'Raw save', extensions: ['raw', 'bin'] },
];

/** Container choices; the native dialog puts the suggested extension first. */
export const SAVE_FILTERS = [
  { name: 'Xbox HDD save', extensions: ['dat'] },
  { name: 'Xbox memory unit', extensions: ['img'] },
  { name: 'PS2 memory card', extensions: ['ps2'] },
  { name: 'GameCube GCI save', extensions: ['gci'] },
  { name: 'GameCube memory card', extensions: ['raw', 'gcp'] },
  { name: 'Bare save data', extensions: ['bin'] },
];
