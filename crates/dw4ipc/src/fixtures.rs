//! JSON fixtures the browser-runnable mock backend loads.
//!
//! Rendered from the real payloads by `examples/gen_ui_fixtures.rs` and pinned
//! by `tests/ui_fixtures.rs`, so the mock cannot drift from the Rust shapes.

use std::path::{Path, PathBuf};

use crate::{EditorSession, app_info};

/// The checked-in fixtures directory: `<repo>/src/ipc/fixtures`.
#[must_use]
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src/ipc/fixtures")
}

/// One file name and its pretty JSON body.
///
/// `save_root` is the `dw4core/tests/fixtures` directory. Panics only on a
/// corrupt fixture; the committed fixtures are already validated by the
/// `dw4core` suite.
///
/// The `path` in the `OpenResult` is replaced with a repo-relative one: the real
/// value is absolute, so a fixture rendered on one machine would never match a
/// fresh render on another and the drift test would fail by directory-name
/// length. Nothing consumes the field — the mock overrides it with the path the
/// caller asked for.
#[must_use]
pub fn ui_fixtures(save_root: &Path) -> Vec<(&'static str, String)> {
    let mut session = EditorSession::new_session();
    let mut opened = session
        .open(&save_root.join("mcd001/save.raw"))
        .expect("the raw fixture opens");
    opened.path = Some("crates/dw4core/tests/fixtures/mcd001/save.raw".to_string());

    vec![
        (
            "app_info.json",
            serde_json::to_string_pretty(&app_info()).expect("AppInfo serialises"),
        ),
        (
            "open_result.raw.json",
            serde_json::to_string_pretty(&opened).expect("OpenResult serialises"),
        ),
    ]
}
