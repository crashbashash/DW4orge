//! The one error type the CLI and the Tauri shell return.

use dw4core::FieldError;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Anything an IPC call or a service call can fail with.
///
/// Internally tagged because serde cannot internally tag a `Vec` variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IpcError {
    /// One or more fields were rejected. Nothing was written.
    Validation {
        /// The per-field reasons.
        fields: Vec<FieldError>,
    },
    /// A `dw4core` failure, with its variant name for branching.
    Core {
        /// The `dw4core::Error` variant name.
        variant: String,
        /// Its rendered message.
        message: String,
    },
    /// No save is open.
    NoOpenDocument,
    /// The request cannot be honoured at all.
    Unsupported {
        /// Why not.
        message: String,
    },
}

impl IpcError {
    /// Wrap a core error.
    #[must_use]
    pub fn core(err: &dw4core::Error) -> Self {
        Self::Core {
            variant: core_variant(err).to_string(),
            message: err.to_string(),
        }
    }
}

impl From<dw4core::Error> for IpcError {
    fn from(err: dw4core::Error) -> Self {
        Self::core(&err)
    }
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation { fields } => {
                write!(f, "{} field(s) rejected", fields.len())?;
                for field in fields {
                    write!(f, "\n  {}: {}", field.path, field.message)?;
                }
                Ok(())
            }
            Self::Core { variant, message } => write!(f, "{variant}: {message}"),
            Self::NoOpenDocument => write!(f, "no save is open"),
            Self::Unsupported { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for IpcError {}

/// The `dw4core::Error` variant name. Exhaustive on purpose: adding a variant
/// to the core error must break this match, not silently become "Error".
fn core_variant(err: &dw4core::Error) -> &'static str {
    match err {
        dw4core::Error::BadSaveSize { .. } => "BadSaveSize",
        dw4core::Error::FieldOutOfRange { .. } => "FieldOutOfRange",
        dw4core::Error::File { .. } => "File",
        dw4core::Error::Io(_) => "Io",
        dw4core::Error::NoSave(_) => "NoSave",
        dw4core::Error::NotAMemcard(_) => "NotAMemcard",
        dw4core::Error::BadCardGeometry(_) => "BadCardGeometry",
        dw4core::Error::BadCard(_) => "BadCard",
        dw4core::Error::SaveNotFound { .. } => "SaveNotFound",
        dw4core::Error::BadClusterChain { .. } => "BadClusterChain",
        dw4core::Error::NoFreeDeviceSlot(_) => "NoFreeDeviceSlot",
        dw4core::Error::StoryIndex { .. } => "StoryIndex",
    }
}
