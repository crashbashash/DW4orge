//! The service layer shared by `dw4cli` and the Tauri shell.
//!
//! There is no Tauri dependency here on purpose: this crate compiles and is
//! tested without a webkit sysroot. The Tauri crate is a thin wrapper.

pub mod error;
pub mod payload;
pub mod ui;

pub use error::IpcError;
pub use payload::{
    AppInfo, NamedCap, NewSaveRequest, OpenResult, PowerupLimit, SourceKind, UiData,
};
pub use ui::{app_info, ui_data};
