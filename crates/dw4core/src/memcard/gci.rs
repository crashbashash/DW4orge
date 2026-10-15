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

pub(super) fn region(bytes: &[u8]) -> Option<crate::Region> {
    if !is_gci(bytes) {
        return None;
    }
    match bytes[3] {
        b'E' => Some(crate::Region::Usa),
        b'P' => Some(crate::Region::Europe),
        b'J' => Some(crate::Region::Japan),
        _ => None,
    }
}

pub(super) fn game_code(region: crate::Region) -> &'static [u8; 4] {
    match region {
        crate::Region::Usa => b"GDJE",
        crate::Region::Europe => b"GDJP",
        crate::Region::Japan => b"GDJJ",
    }
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
/// Create an original DW4 GCI without a donor save or retail artwork.
///
/// The native CI8 banner and icon are opaque black. The game replaces them
/// when the player saves. The directory entry uses the selected disc region.
///
/// # Errors
/// Returns an error if savedata has the wrong size or byte order.
pub fn create_save(save: &[u8], region: crate::Region) -> Result<Vec<u8>> {
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
    let mut out = vec![0; GCI_MIN_SIZE];
    out[..6].copy_from_slice(match region {
        crate::Region::Usa => b"GDJEB2",
        crate::Region::Europe => b"GDJPB2",
        crate::Region::Japan => b"GDJJB2",
    });
    out[6] = 0xff;
    out[7] = 1;
    out[8..20].copy_from_slice(b"NDMWSaveData");
    out[0x28..0x2c].copy_from_slice(&(super::gamecube::format_time() as u32).to_be_bytes());
    out[0x2c..0x30].copy_from_slice(&0x40u32.to_be_bytes());
    out[0x30..0x32].copy_from_slice(&1u16.to_be_bytes());
    out[0x32..0x34].copy_from_slice(&2u16.to_be_bytes());
    out[0x34] = 4;
    out[0x38..0x3a].copy_from_slice(&11u16.to_be_bytes());
    out[0x3a..0x3c].fill(0xff);
    out[0x40..0x4d].copy_from_slice(b"DigimonWorld4");
    out[0x60..0x68].copy_from_slice(b"SaveData");
    for palette in [0xc40, 0x1240] {
        for pixel in (GCI_HEADER_SIZE + palette..GCI_HEADER_SIZE + palette + 0x200).step_by(2) {
            out[pixel..pixel + 2].copy_from_slice(&0x8000u16.to_be_bytes());
        }
    }
    out[GCI_SAVE_OFFSET..].copy_from_slice(save);
    Ok(out)
}
