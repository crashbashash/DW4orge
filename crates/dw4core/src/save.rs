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

    // ---- header ----------------------------------------------------------

    /// Save format version. Real saves are `4`.
    #[must_use]
    pub fn version(&self) -> u32 {
        self.get_u32(offsets::VERSION)
    }

    /// `ISUSE`. Real saves are `1`.
    #[must_use]
    pub fn isuse(&self) -> u32 {
        self.get_u32(offsets::ISUSE)
    }

    /// `UNIQUE`, a per-save id. **Not** a constant: real saves carry different
    /// values (e.g. `0x700FFDAC`, `0x6096F82C`).
    #[must_use]
    pub fn unique(&self) -> u32 {
        self.get_u32(offsets::UNIQUE)
    }

    // ---- character -------------------------------------------------------

    /// `BIT`, the currency.
    #[must_use]
    pub fn bit(&self) -> u32 {
        self.get_u32(offsets::BIT)
    }

    /// Set `BIT`.
    pub fn set_bit(&mut self, value: u32) {
        self.set_u32(offsets::BIT, value);
    }

    /// The `XDATA` counter.
    #[must_use]
    pub fn xdata(&self) -> u32 {
        self.get_u32(offsets::XDATA)
    }

    /// Set `XDATA`.
    pub fn set_xdata(&mut self, value: u32) {
        self.set_u32(offsets::XDATA, value);
    }

    /// The menu-snapshot level. The game recomputes this on load.
    #[must_use]
    pub fn menu_level(&self) -> u32 {
        self.get_u32(offsets::MENU_LEVEL)
    }

    /// Set the menu-snapshot level.
    pub fn set_menu_level(&mut self, value: u32) {
        self.set_u32(offsets::MENU_LEVEL, value);
    }

    /// The cumulative junk-shop donation counter (`BASE_COUNTER[1]`).
    #[must_use]
    pub fn junk_counter(&self) -> u32 {
        self.get_u32(offsets::BASE_COUNTER + offsets::COUNTER_JUNK * 4)
    }

    /// Set the junk-shop donation counter.
    pub fn set_junk_counter(&mut self, value: u32) {
        self.set_u32(offsets::BASE_COUNTER + offsets::COUNTER_JUNK * 4, value);
    }

    /// The 16 raw bytes of `DIGIMONNAME`.
    #[must_use]
    pub fn digimon_name_raw(&self) -> &[u8] {
        self.get_bytes(offsets::DIGIMON_NAME, 16)
    }

    /// The ASCII model stem, e.g. `p_dorumon`, up to the first NUL.
    #[must_use]
    pub fn digimon_name(&self) -> String {
        let raw = self.digimon_name_raw();
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        String::from_utf8_lossy(&raw[..end]).into_owned()
    }

    /// Write the model stem, NUL-padded to 16 bytes and truncated if longer.
    pub fn set_digimon_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let take = bytes.len().min(16);
        let mut field = [0u8; 16];
        field[..take].copy_from_slice(&bytes[..take]);
        self.set_bytes(offsets::DIGIMON_NAME, &field);
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

    #[test]
    fn junk_counter_sits_at_counter_slot_one() {
        assert_eq!(
            offsets::BASE_COUNTER + offsets::COUNTER_JUNK * 4,
            0x730,
            "the junk counter is BASE_COUNTER[1]"
        );
    }

    #[test]
    fn header_and_currency_round_trip() {
        let mut save = SaveData::parse(&real_save()).unwrap();
        assert_eq!(save.version(), 4);
        assert_eq!(save.isuse(), 1);
        // UNIQUE is a per-save id, not a constant. The golden test pins it
        // against the oracle; here we only require that it is populated.
        assert_ne!(save.unique(), 0, "a real save carries an id here");

        save.set_bit(1_234);
        save.set_xdata(5_678);
        save.set_menu_level(42);
        save.set_junk_counter(3_956_000);
        assert_eq!(save.bit(), 1_234);
        assert_eq!(save.xdata(), 5_678);
        assert_eq!(save.menu_level(), 42);
        assert_eq!(save.junk_counter(), 3_956_000);
    }

    #[test]
    fn digimon_name_is_nul_terminated_ascii() {
        let mut save = SaveData::parse(&real_save()).unwrap();
        assert_eq!(save.digimon_name(), "p_dorumon");

        save.set_digimon_name("p_impdrapm");
        assert_eq!(save.digimon_name(), "p_impdrapm");
        assert_eq!(save.digimon_name_raw().len(), 16);
        assert_eq!(&save.digimon_name_raw()[10..], &[0u8; 6]);
    }

    #[test]
    fn set_digimon_name_truncates_to_sixteen_bytes() {
        let mut save = SaveData::parse(&real_save()).unwrap();
        save.set_digimon_name("p_this_stem_is_far_too_long");
        assert_eq!(save.digimon_name_raw().len(), 16);
        assert_eq!(save.digimon_name(), "p_this_stem_is_f");
    }
}
