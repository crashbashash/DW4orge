//! Errors produced by the save-format layer.
use std::path::PathBuf;
use thiserror::Error;

/// Convenience alias so signatures read `Result<T>`.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    /// The byte slice handed to `SaveData::parse` was the wrong length.
    #[error("a save is {expected} bytes, got {actual}")]
    BadSaveSize { expected: usize, actual: usize },

    /// A field would extend past the end of its 0xA000-byte block.
    #[error("field at 0x{offset:04X} (+{len} bytes) runs past the {block}-byte block")]
    FieldOutOfRange {
        offset: usize,
        len: usize,
        block: usize,
    },

    /// Filesystem failure, with the path that caused it.
    #[error("{path}: {source}")]
    File {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Filesystem failure with no meaningful path attached.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
