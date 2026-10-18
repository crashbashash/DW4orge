//! Follow GTK's tiling state for native title-bar decorations.
use gtk::{gdk::WindowState, prelude::*};
use tauri::Manager;

fn update(window: &gtk::ApplicationWindow, state: WindowState) {
    let tiled = state.intersects(
        WindowState::TILED
            | WindowState::TOP_TILED
            | WindowState::RIGHT_TILED
            | WindowState::BOTTOM_TILED
            | WindowState::LEFT_TILED,
    );
    let decorated = !tiled;
    if window.is_decorated() != decorated {
        window.set_decorated(decorated);
    }
}

pub fn install(app: &tauri::App) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        let native = window.gtk_window()?;
        if let Some(surface) = native.window() {
            update(&native, surface.state());
        }
        native.connect_window_state_event(|window, event| {
            update(window, event.new_window_state());
            gtk::glib::Propagation::Proceed
        });
    }
    Ok(())
}
