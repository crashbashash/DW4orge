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

    /// A save could not be produced or verified, with the reason why.
    #[error("{0}")]
    NoSave(String),

    /// The input is not a PS2 memory-card image.
    #[error("not a PS2 memory card: {0}")]
    NotAMemcard(String),

    /// The superblock's geometry is self-inconsistent.
    #[error("bad memory-card geometry: {0}")]
    BadCardGeometry(String),

    /// A structure inside the card is malformed.
    #[error("malformed memory card: {0}")]
    BadCard(String),

    /// The save is not present inside the card.
    #[error("{dir}/{file} not found in the card")]
    SaveNotFound { dir: String, file: String },

    /// A cluster chain is broken, loops, or runs past the data area.
    #[error("cluster chain from {start} is invalid: {reason}")]
    BadClusterChain { start: u32, reason: String },

    /// Every device-folder slot is occupied, so a mod chip could not be added.
    #[error("no free device-folder slot to add the mod chip 0x{0:04X}")]
    NoFreeDeviceSlot(u32),

    /// A story edit named a flag or folder outside its field.
    #[error("{kind} index {index} is out of range (0..{limit})")]
    StoryIndex {
        kind: &'static str,
        index: usize,
        limit: usize,
    },
}
