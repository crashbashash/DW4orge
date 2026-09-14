//! The Tauri shell: thin command wrappers over `dw4ipc`.
//!
//! Every command body is a delegation. Logic, validation and writes all live in
//! `dw4ipc`, which is tested without a webkit sysroot.

pub mod commands;
pub mod state;

use state::AppState;

/// Build and run the desktop application.
///
/// # Panics
/// If Tauri cannot start.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::open_save,
            commands::new_save,
            commands::get_view,
            commands::validate_edits,
            commands::save,
            commands::save_as,
            commands::app_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DW4orge");
}
