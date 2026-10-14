//! The service layer shared by `dw4cli` and the Tauri shell.
//!
//! There is no Tauri dependency here on purpose: this crate compiles and is
//! tested without a webkit sysroot. The Tauri crate is a thin wrapper.
//!
//! It re-exports the `dw4core` types its payloads are built from, so a consumer
//! speaks the IPC protocol through this crate alone — see
//! `docs/dw4ipc-cli-tauri-design.md` §2.2, where the shell depends on `dw4ipc`
//! and Tauri and not on the core directly.

pub mod bindings;
pub mod catalogue_query;
pub mod error;
pub mod fixtures;
pub mod payload;
pub mod session;
pub mod ui;
pub mod verify;

/// The product name: what the title bar, installers and `AppInfo` report.
pub const PRODUCT_NAME: &str = "DW4orge";

// The core types that appear in command signatures and payloads, re-exported so
// the CLI and the shell need one dependency.
pub use dw4core::{EditSet, Mode, SaveView, Species, Warning};

pub use catalogue_query::catalogue_search;
pub use error::IpcError;
pub use payload::{
    AppInfo, NamedCap, NewSaveRequest, OpenResult, PowerupLimit, SourceKind, SpeciesStats, UiData,
};
pub use session::EditorSession;
pub use ui::{app_info, ui_data};
pub use verify::{CardReport, VerifyProblem, VerifyReport, verify_path};
