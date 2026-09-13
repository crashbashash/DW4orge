//! The save document: parse, mirror-both-blocks access, checksum.
use crate::error::{Error, Result};
use crate::offsets;
use crate::{BLOCK, SAVE_SIZE};

/// A parsed save file: two mirrored 0xA000-byte blocks.
///
/// The invariant is `raw.len() == SAVE_SIZE`. Every `set_*` writes to **both**
/// blocks, so the mirror the game restores from cannot drift.
#[derive(Clone, PartialEq, Eq)]
pub struct SaveData {
    raw: Vec<u8>,
}

impl std::fmt::Debug for SaveData {
    /// Summarised: printing 80 KB of hex helps nobody. `Result::unwrap_err`
    /// needs `Debug`, which is the only reason this exists.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SaveData")
            .field("len", &self.raw.len())
            .field("checksum", &self.get_u32(offsets::CHECKSUM))
            .finish()
    }
}

impl SaveData {
    /// Parse a save from exactly `SAVE_SIZE` bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != SAVE_SIZE {
            return Err(Error::BadSaveSize {
                expected: SAVE_SIZE,
                actual: bytes.len(),
            });
        }
        Ok(Self {
            raw: bytes.to_vec(),
        })
    }

    /// The raw bytes as loaded — checksums **not** recomputed.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw
    }

    /// A copy of the save with both block checksums recomputed.
    ///
    /// Returns a `Vec` rather than a fixed-size array: an 80 KB array would be
    /// returned on the stack.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.raw.clone();
        fix_checksums(&mut out);
        out
    }

    /// Whether both stored checksums match their blocks' contents.
    #[must_use]
    pub fn verify(&self) -> bool {
        (0..2).all(|b| block_checksum(&self.raw, b) == self.get_u32_block(offsets::CHECKSUM, b))
    }

    // ---- raw access ------------------------------------------------------

    /// A u32 from block 0. Use [`Self::get_u32_block`] to choose a block.
    #[must_use]
    pub fn get_u32(&self, offset: usize) -> u32 {
        self.get_u32_block(offset, 0)
    }

    /// A u32 at `offset` in `block` (0 or 1).
    #[must_use]
    pub fn get_u32_block(&self, offset: usize, block: usize) -> u32 {
        read_u32(&self.raw, block * BLOCK + offset)
    }

    /// Write a u32 to `offset` in **both** blocks.
    pub fn set_u32(&mut self, offset: usize, value: u32) {
        for block in 0..2 {
            write_u32(&mut self.raw, block * BLOCK + offset, value);
        }
    }

    /// `len` bytes at `offset` from block 0.
    #[must_use]
    pub fn get_bytes(&self, offset: usize, len: usize) -> &[u8] {
        &self.raw[offset..offset + len]
    }

    /// Write bytes at `offset` in **both** blocks.
    ///
    /// # Panics
    ///
    /// Panics if the write would extend past the end of a block. Every caller
    /// passes a fixed-size destination field, so this is a programming error.
    pub fn set_bytes(&mut self, offset: usize, data: &[u8]) {
        assert!(
            offset + data.len() <= BLOCK,
            "field at 0x{offset:04X} (+{} bytes) runs past the {BLOCK}-byte block",
            data.len()
        );
        for block in 0..2 {
            let at = block * BLOCK + offset;
            self.raw[at..at + data.len()].copy_from_slice(data);
        }
    }
}

/// `Σ u32le(block+4 .. block+0xA000) mod 2³²`.
#[must_use]
pub fn block_checksum(raw: &[u8], block: usize) -> u32 {
    let base = block * BLOCK;
    (base + 4..base + BLOCK)
        .step_by(4)
        .fold(0u32, |acc, i| acc.wrapping_add(read_u32(raw, i)))
}

/// Recompute and store both blocks' checksums in place.
pub fn fix_checksums(raw: &mut [u8]) {
    for block in 0..2 {
        let sum = block_checksum(raw, block);
        write_u32(raw, block * BLOCK + offsets::CHECKSUM, sum);
    }
}

fn read_u32(raw: &[u8], offset: usize) -> u32 {
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&raw[offset..offset + 4]);
    u32::from_le_bytes(bytes)
}

fn write_u32(raw: &mut [u8], offset: usize, value: u32) {
    raw[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offsets;
    use std::path::PathBuf;

    fn real_save() -> Vec<u8> {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mcd001/save.raw");
        std::fs::read(p).expect("run tools/gen_fixtures.py first")
    }

    #[test]
    fn parse_rejects_the_wrong_length() {
        let err = SaveData::parse(&[0u8; 10]).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::BadSaveSize {
                expected: 81920,
                actual: 10
            }
        ));
    }

    #[test]
    fn the_real_save_verifies() {
        let save = SaveData::parse(&real_save()).unwrap();
        assert!(
            save.verify(),
            "the committed fixture's checksum must verify"
        );
    }

    #[test]
    fn both_blocks_are_byte_identical_in_the_fixture() {
        let raw = real_save();
        assert_eq!(raw[..crate::BLOCK], raw[crate::BLOCK..]);
    }

    #[test]
    fn a_corrupted_checksum_fails_verification() {
        let mut raw = real_save();
        raw[offsets::CHECKSUM] ^= 0xFF;
        assert!(!SaveData::parse(&raw).unwrap().verify());
    }

    #[test]
    fn setting_a_field_updates_both_blocks() {
        let mut save = SaveData::parse(&real_save()).unwrap();
        save.set_u32(offsets::BIT, 0x1234_5678);
        assert_eq!(save.get_u32_block(offsets::BIT, 0), 0x1234_5678);
        assert_eq!(save.get_u32_block(offsets::BIT, 1), 0x1234_5678);
    }

    #[test]
    fn set_bytes_updates_both_blocks() {
        const AGUMON: &[u8; 16] = b"p_agumon\0\0\0\0\0\0\0\0";
        let mut save = SaveData::parse(&real_save()).unwrap();
        save.set_bytes(offsets::DIGIMON_NAME, AGUMON);
        assert_eq!(save.get_bytes(offsets::DIGIMON_NAME, 16), &AGUMON[..]);
        let block1 = crate::BLOCK + offsets::DIGIMON_NAME;
        assert_eq!(&save.as_bytes()[block1..block1 + 16], &AGUMON[..]);
    }

    #[test]
    fn to_bytes_fixes_the_checksum_of_a_dirty_save() {
        let mut save = SaveData::parse(&real_save()).unwrap();
        save.set_u32(offsets::BIT, 42);
        assert!(
            !save.verify(),
            "the in-memory checksum is stale until to_bytes"
        );

        let out = save.to_bytes();
        assert_eq!(out.len(), crate::SAVE_SIZE);
        let reparsed = SaveData::parse(&out).unwrap();
        assert!(reparsed.verify());
        assert_eq!(reparsed.get_u32(offsets::BIT), 42);
    }

    #[test]
    fn checksums_of_the_two_blocks_are_equal_for_a_clean_save() {
        let raw = real_save();
        assert_eq!(block_checksum(&raw, 0), block_checksum(&raw, 1));
    }

    #[test]
    fn fix_checksums_is_idempotent() {
        let mut raw = real_save();
        fix_checksums(&mut raw);
        let once = raw.clone();
        fix_checksums(&mut raw);
        assert_eq!(raw, once);
    }
}
