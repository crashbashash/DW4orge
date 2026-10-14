import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { EditSet, FieldError, Mode, OpenResult, Species } from '../bindings';
import type { Backend, ValidationReport } from './backend';
import { OPEN_FILTERS, SAVE_FILTERS } from './backend';
import { fieldErrors } from './errors';

function agentControlsEnabled(): boolean {
  return (window as Window & { __DW4_AGENT_CONTROL__?: boolean }).__DW4_AGENT_CONTROL__ === true;
}

export const tauriBackend: Backend = {
  appInfo: () => invoke('app_info'),
  openSave: (path, region) => invoke('open_save', { path, region: region ?? null }),
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
  saveAs: (path, edits, mode, region): Promise<OpenResult> => invoke('save_as', { path, edits, mode, region: region ?? null }),
  speciesStats: (species: Species, mode: Mode) => invoke('species_stats', { species, mode }),
  async pickOpenPath() {
    if (agentControlsEnabled()) return invoke<string | null>('agent_pick_path', { kind: 'open' });
    const picked = await open({ multiple: false, directory: false, filters: OPEN_FILTERS });
    return typeof picked === 'string' ? picked : null;
  },
  pickSavePath: (defaultName) => agentControlsEnabled()
    ? invoke<string | null>('agent_pick_path', { kind: 'save' })
    : save({ defaultPath: defaultName, filters: [...SAVE_FILTERS].sort((a, b) => {
      const extension = defaultName?.split('.').pop()?.toLowerCase() ?? 'ps2';
      return Number(b.extensions.includes(extension)) - Number(a.extensions.includes(extension));
    }) }),
  async onCloseRequested(handler) {
    // Imported lazily, like `theme.ts`, so the window module stays out of the
    // initial bundle.
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    return getCurrentWindow().onCloseRequested((event) => {
      if (handler()) event.preventDefault();
    });
  },
  async closeWindow() {
    // `destroy` skips the close event, so the guard is not asked again.
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().destroy();
  },
};
