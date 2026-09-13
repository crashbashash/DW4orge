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
pub mod codes;
pub mod error;
pub mod name;
pub mod offsets;
pub mod save;
pub mod species;

pub use codes::{
    CAP_BIT, CAP_DISK_COUNT, CAP_EXP, CAP_ITEM_BONUS, CAP_ITEM_MODS, CAP_LEVEL, CAP_TECH,
    CAP_UPCNT, CAP_XDATA, Cap, MAX_LEVEL, POWERUP_STATS, Rarity, TECHNIQUES, color_for_seed,
    level_from_exp, level_threshold, upcnt_safe_cap,
};
pub use error::{Error, Result};
pub use name::{NAME_CHARS, decode_player_name, encode_player_name};
pub use save::{SaveData, block_checksum, fix_checksums};
pub use species::Species;

/// Size of one mirrored save block.
pub const BLOCK: usize = offsets::BLOCK;

/// Size of the whole save data file: two mirrored blocks.
pub const SAVE_SIZE: usize = BLOCK * 2;

/// The "no item / no slot" sentinel. Never `0`.
pub const EMPTY: u32 = 0xFFFF_FFFF;
