import type {
  AppInfo,
  EditSet,
  FieldError,
  Mode,
  NewSaveRequest,
  OpenResult,
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

/** Everything the UI can ask of the Rust side. */
export interface Backend {
  appInfo(): Promise<AppInfo>;
  openSave(path: string): Promise<OpenResult>;
  newSave(request: NewSaveRequest): Promise<OpenResult>;
  getView(mode: Mode): Promise<SaveView>;
  validateEdits(edits: EditSet, mode: Mode): Promise<ValidationReport>;
  save(edits: EditSet, mode: Mode): Promise<OpenResult>;
  saveAs(path: string, edits: EditSet, mode: Mode): Promise<OpenResult>;
  speciesStats(species: Species, mode: Mode): Promise<SpeciesStats>;
  pickOpenPath(): Promise<string | null>;
  pickSavePath(defaultName: string): Promise<string | null>;
  /**
   * Present only on the browser mock. The shell offers a one-click sample load
   * when it is defined, so `npm run dev` has something to show without a file
   * dialog (and without a webkit build).
   */
  openSample?: () => Promise<OpenResult>;
}

export const SAVE_FILTERS = [
  { name: 'PS2 memory card / raw save', extensions: ['ps2', 'raw', 'bin'] },
  { name: 'PS2 memory card', extensions: ['ps2'] },
  { name: 'Raw save', extensions: ['raw', 'bin'] },
];
