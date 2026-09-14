//! The IPC commands. Bodies delegate to `dw4ipc`.

use dw4core::{EditSet, Mode, SaveView, Species, Warning};
use dw4ipc::{
    AppInfo, EditorSession, IpcError, NewSaveRequest, OpenResult, SpeciesStats,
};
use tauri::State;

use crate::state::AppState;

/// Lock the shared session.
fn session(state: &State<'_, AppState>) -> std::sync::MutexGuard<'_, EditorSession> {
    state.session.lock().expect("session mutex poisoned")
}

/// Open a card or raw save. `kind` is decided by content, not extension.
#[tauri::command]
pub fn open_save(state: State<'_, AppState>, path: String) -> Result<OpenResult, IpcError> {
    session(&state).open(std::path::Path::new(&path))
}

/// Synthesise a new save in memory; nothing is written until `save_as`.
#[tauri::command]
pub fn new_save(
    state: State<'_, AppState>,
    request: NewSaveRequest,
) -> Result<OpenResult, IpcError> {
    session(&state).new_save(&request)
}

/// Re-project the open save for `mode`.
#[tauri::command]
pub fn get_view(state: State<'_, AppState>, mode: Mode) -> Result<SaveView, IpcError> {
    session(&state).view(mode)
}

/// Validate a draft without writing anything.
#[tauri::command]
pub fn validate_edits(
    state: State<'_, AppState>,
    edits: EditSet,
    mode: Mode,
) -> Result<Vec<Warning>, IpcError> {
    session(&state).validate(&edits, mode)
}

/// Validate and apply `edits`, then write over the loaded path.
#[tauri::command]
pub fn save(
    state: State<'_, AppState>,
    edits: EditSet,
    mode: Mode,
) -> Result<OpenResult, IpcError> {
    session(&state).save(&edits, mode)
}

/// Validate and apply `edits`, then write to `path`.
#[tauri::command]
pub fn save_as(
    state: State<'_, AppState>,
    path: String,
    edits: EditSet,
    mode: Mode,
) -> Result<OpenResult, IpcError> {
    session(&state).save_as(std::path::Path::new(&path), &edits, mode)
}

/// Static app identity plus the UI tables. Never fails.
#[tauri::command]
#[must_use]
pub fn app_info() -> AppInfo {
    dw4ipc::app_info()
}

/// The stored progression for one species, for the Character selector.
#[tauri::command]
pub fn species_stats(
    state: State<'_, AppState>,
    species: Species,
    mode: Mode,
) -> Result<SpeciesStats, IpcError> {
    session(&state).species_stats(species, mode)
}
