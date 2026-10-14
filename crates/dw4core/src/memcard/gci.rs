//! GameCube GCI wrapper used by Digimon World 4 / World X.
//!
//! A retail `NDMWSaveData.gci` consists of the standard 0x40-byte GCI header,
//! a 0x2000-byte banner/icon area, and the same pair of 0xA000-byte game-save
//! blocks used by the PS2 serializer. The GameCube blocks are big-endian.

use crate::{Error, Result, SAVE_SIZE};

pub const GCI_HEADER_SIZE: usize = 0x40;
pub const GCI_BANNER_SIZE: usize = 0x2000;
pub const GCI_SAVE_OFFSET: usize = GCI_HEADER_SIZE + GCI_BANNER_SIZE;
pub const GCI_MIN_SIZE: usize = GCI_SAVE_OFFSET + SAVE_SIZE;

/// Whether this is the GameCube DW4/DWX GCI file (not a memory-card image).
#[must_use]
pub fn is_gci(bytes: &[u8]) -> bool {
    if bytes.len() < GCI_MIN_SIZE || &bytes[4..6] != b"B2" {
        return false;
    }
    let game_code = &bytes[..4];
    let known_game = game_code.starts_with(b"GDJ");
    let name = &bytes[8..0x28];
    known_game && name.starts_with(b"NDMWSaveData")
}

/// Extract the two serialized save blocks from a GCI file.
pub fn read_save(bytes: &[u8]) -> Result<Vec<u8>> {
    if !is_gci(bytes) {
        return Err(Error::NoSave(
            "not a Digimon World 4 GameCube GCI file".into(),
        ));
    }
    Ok(bytes[GCI_SAVE_OFFSET..GCI_SAVE_OFFSET + SAVE_SIZE].to_vec())
}

/// Replace only the serialized save blocks, retaining GCI metadata and banner.
pub fn write_save(bytes: &[u8], save: &[u8]) -> Result<Vec<u8>> {
    if !is_gci(bytes) {
        return Err(Error::NoSave(
            "not a Digimon World 4 GameCube GCI file".into(),
        ));
    }
    if save.len() != SAVE_SIZE {
        return Err(Error::BadSaveSize {
            expected: SAVE_SIZE,
            actual: save.len(),
        });
    }
    if crate::SaveData::parse(save)?.byte_order() != crate::ByteOrder::Big {
        return Err(Error::NoSave(
            "GameCube containers require big-endian savedata".into(),
        ));
    }
    let mut out = bytes.to_vec();
    out[GCI_SAVE_OFFSET..GCI_SAVE_OFFSET + SAVE_SIZE].copy_from_slice(save);
    Ok(out)
}
