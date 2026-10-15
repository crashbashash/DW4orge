//! The Tauri shell: thin command wrappers over `dw4ipc`.
//!
//! Every command body is a delegation. Logic, validation and writes all live in
//! `dw4ipc`, which is tested without a webkit sysroot.

#[cfg(feature = "agent-control")]
mod agent;

pub mod commands;
pub mod state;

use state::AppState;

/// Build and run the desktop application.
///
/// # Panics
/// If Tauri cannot start.
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default());
    #[cfg(feature = "agent-control")]
    let builder = builder
        .manage(agent::AgentState::default())
        .append_invoke_initialization_script(format!(
            "window.__DW4_AGENT_CONTROL__ = {};",
            std::env::var_os("DW4_AGENT_TOKEN").is_some()
        ))
        .setup(|app| agent::start(app.handle().clone()));

    #[cfg(feature = "agent-control")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::open_save,
        commands::new_save,
        commands::create_box,
        commands::get_view,
        commands::validate_edits,
        commands::save,
        commands::save_as,
        commands::app_info,
        commands::species_stats,
        agent::agent_reply,
        agent::agent_queue_dialog,
        agent::agent_pick_path,
        agent::agent_close,
    ]);
    #[cfg(not(feature = "agent-control"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::open_save,
        commands::new_save,
        commands::create_box,
        commands::get_view,
        commands::validate_edits,
        commands::save,
        commands::save_as,
        commands::app_info,
        commands::species_stats,
    ]);
    builder
        .run(tauri::generate_context!())
        .expect("error while running DW4orge");
}
