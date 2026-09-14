import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { EditSet, FieldError, Mode, OpenResult, Species } from '../bindings';
import type { Backend, ValidationReport } from './backend';
import { OPEN_FILTERS, SAVE_FILTERS } from './backend';
import { fieldErrors } from './errors';

export const tauriBackend: Backend = {
  appInfo: () => invoke('app_info'),
  openSave: (path) => invoke('open_save', { path }),
  newSave: (request) => invoke('new_save', { request }),
  getView: (mode) => invoke('get_view', { mode }),
  async validateEdits(edits: EditSet, mode: Mode): Promise<ValidationReport> {
    try {
      const warnings = await invoke<FieldError[]>('validate_edits', { edits, mode });
      return { errors: [], warnings };
    } catch (err) {
      const fields = fieldErrors(err);
      if (fields) return { errors: fields, warnings: [] };
      throw err;
    }
  },
  save: (edits, mode): Promise<OpenResult> => invoke('save', { edits, mode }),
  saveAs: (path, edits, mode): Promise<OpenResult> => invoke('save_as', { path, edits, mode }),
  speciesStats: (species: Species, mode: Mode) => invoke('species_stats', { species, mode }),
  async pickOpenPath() {
    const picked = await open({ multiple: false, directory: false, filters: OPEN_FILTERS });
    return typeof picked === 'string' ? picked : null;
  },
  pickSavePath: (defaultName) => save({ defaultPath: defaultName, filters: SAVE_FILTERS }),
};
