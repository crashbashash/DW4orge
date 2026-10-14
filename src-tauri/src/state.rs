//! The one piece of mutable state the commands share.

use std::sync::Mutex;

use dw4ipc::EditorSession;

/// Wrapped so a command can hold the session for the duration of a call.
pub struct AppState {
    /// The open save, if any.
    pub session: Mutex<EditorSession>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session: Mutex::new(EditorSession::new_session()),
        }
    }
}
