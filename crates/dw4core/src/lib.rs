//! Digimon World 4 (USA, `SLUS_208.36`) save-format engine.
//!
//! The save file is `BASLUS-20836savedata`, 81,920 bytes: two byte-identical
//! 0xA000-byte blocks (a mirror/backup). Every write here updates both, so the
//! mirror cannot drift.
//!
//! Offsets in this crate are **block-relative** unless a function says
//! otherwise. All multi-byte values are little-endian.
//!
//! ```ignore
//! # use dw4core::SaveData;
//! let mut save = SaveData::parse(&vec![0u8; dw4core::SAVE_SIZE]).unwrap();
//! save.set_bit(9_999_999);
//! assert_eq!(save.bit(), 9_999_999);
//! assert_eq!(save.device(0), dw4core::EMPTY);
//! ```
pub mod error;

pub use error::{Error, Result};

/// Size of one mirrored save block.
///
/// Moves into `offsets` in Task 2, which becomes the single home for every
/// block-relative offset.
pub const BLOCK: usize = 0xA000;

/// Size of the whole save data file: two mirrored blocks.
pub const SAVE_SIZE: usize = BLOCK * 2;

/// The "no item / no slot" sentinel. Never `0`.
pub const EMPTY: u32 = 0xFFFF_FFFF;
