//! Parent both pickers explicitly so Linux portals can attach their dialogs.
use serde::Deserialize;
use tauri::Window;
use tauri_plugin_dialog::DialogExt;

#[derive(Deserialize)]
pub struct FileFilter {
    name: String,
    extensions: Vec<String>,
}

#[tauri::command]
pub async fn pick_open_path(
    window: Window,
    filters: Vec<FileFilter>,
) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = window.dialog().file().set_parent(&window);
        for filter in filters {
            let extensions: Vec<&str> = filter.extensions.iter().map(String::as_str).collect();
            dialog = dialog.add_filter(filter.name, &extensions);
        }
        dialog
            .blocking_pick_file()
            .map(|p| p.into_path().map(|p| p.to_string_lossy().into_owned()))
            .transpose()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn pick_save_path(
    window: Window,
    default_path: String,
    filters: Vec<FileFilter>,
) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = window.dialog().file().set_parent(&window);
        for filter in filters {
            let extensions: Vec<&str> = filter.extensions.iter().map(String::as_str).collect();
            dialog = dialog.add_filter(filter.name, &extensions);
        }
        let path = std::path::Path::new(&default_path);
        if let Some(parent) = path.parent().filter(|p| p.components().count() > 0) {
            dialog = dialog.set_directory(parent);
        }
        if let Some(name) = path.file_name() {
            dialog = dialog.set_file_name(name.to_string_lossy());
        }
        dialog
            .blocking_save_file()
            .map(|p| p.into_path().map(|p| p.to_string_lossy().into_owned()))
            .transpose()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
