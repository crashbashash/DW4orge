//! The PS2 memory-card filesystem, implemented natively.
//!
//! `ps2-memcard` 0.2.2 was evaluated and rejected (spec 6.1): it sizes a
//! directory from the `.` entry's `length`, which is 0 on this card, where the
//! working reference bounds by the *parent* entry's length. Its `CardBuilder`
//! also rebuilds whole cards rather than writing in place.
//!
//! Everything here was validated against the real `Mcd001.ps2` before this
//! module was written; the measured values are recorded in the implementation
//! plan and asserted by `tests/memcard.rs`.

mod ecc;
mod entry;
mod fat;
mod geometry;

pub use ecc::{ECC_CHUNK, ECC_CHUNK_BYTES, ecc_chunk, page_spare};
pub use entry::{ENTRY_SIZE, Entry, MODE_DIR, MODE_EXISTS, MODE_FILE, MODE_HIDDEN};
pub use fat::{ALLOCATED_BIT, CHAIN_END, FatTable, UNALLOCATED};
pub use geometry::{CardKind, Geometry, SB_MAGIC, SB_SIZE, Superblock};

/// The directory holding the save, inside the card.
pub const SAVE_DIR: &str = "BASLUS-20836savedata";

/// The save file's name inside that directory.
pub const SAVE_FILE: &str = "BASLUS-20836savedata";
