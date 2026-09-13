//! The save document: parse, mirror-both-blocks access, checksum.
use crate::error::{Error, Result};
use crate::offsets;
use crate::species::Species;
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

    /// The species implied by the stored model name.
    ///
    /// Unrecognised names fall back to [`Species::DEFAULT`], matching the
    /// Python editor.
    #[must_use]
    pub fn detect_species(&self) -> Species {
        Species::from_model_name(&self.digimon_name()).unwrap_or(Species::DEFAULT)
    }

    // ---- device folder ---------------------------------------------------

    /// A device-folder entry. Slots `0..DEVICE_SLOTS` are the usable inventory.
    #[must_use]
    pub fn device(&self, slot: usize) -> u32 {
        assert!(
            slot < offsets::DEVICE_SAVE_SLOTS,
            "device slot {slot} out of range"
        );
        self.get_u32(offsets::DEVICE + slot * 4)
    }

    /// Write a device-folder entry.
    pub fn set_device(&mut self, slot: usize, value: u32) {
        assert!(
            slot < offsets::DEVICE_SAVE_SLOTS,
            "device slot {slot} out of range"
        );
        self.set_u32(offsets::DEVICE + slot * 4, value);
    }

    /// Every device-folder slot, including the 6 reserved padding slots.
    #[must_use]
    pub fn get_raw_device_slots(&self) -> Vec<u32> {
        (0..offsets::DEVICE_SAVE_SLOTS)
            .map(|i| self.device(i))
            .collect()
    }

    // ---- equipment -------------------------------------------------------

    /// A weapon slot: an index into the device folder.
    #[must_use]
    pub fn weapon(&self, index: usize) -> u32 {
        assert!(
            index < offsets::WEAPON_SLOTS,
            "weapon slot {index} out of range"
        );
        self.get_u32(offsets::WEAPON + index * 4)
    }

    /// Write a weapon slot.
    pub fn set_weapon(&mut self, index: usize, value: u32) {
        assert!(
            index < offsets::WEAPON_SLOTS,
            "weapon slot {index} out of range"
        );
        self.set_u32(offsets::WEAPON + index * 4, value);
    }

    /// Every weapon slot.
    #[must_use]
    pub fn get_raw_weapon_slots(&self) -> Vec<u32> {
        (0..offsets::WEAPON_SLOTS).map(|i| self.weapon(i)).collect()
    }

    /// A weapon-mod socket: an index into the device folder.
    #[must_use]
    pub fn weapon_mod(&self, index: usize) -> u32 {
        assert!(
            index < offsets::MOD_SOCKETS,
            "weapon mod {index} out of range"
        );
        self.get_u32(offsets::WEAPON_MOD + index * 4)
    }

    /// Write a weapon-mod socket.
    pub fn set_weapon_mod(&mut self, index: usize, value: u32) {
        assert!(
            index < offsets::MOD_SOCKETS,
            "weapon mod {index} out of range"
        );
        self.set_u32(offsets::WEAPON_MOD + index * 4, value);
    }

    /// Every weapon-mod socket.
    #[must_use]
    pub fn get_raw_weapon_mod_slots(&self) -> Vec<u32> {
        (0..offsets::MOD_SOCKETS)
            .map(|i| self.weapon_mod(i))
            .collect()
    }

    /// The armor / core slot: an index into the device folder.
    #[must_use]
    pub fn armor(&self) -> u32 {
        self.get_u32(offsets::ARMOR)
    }

    /// Write the armor slot.
    pub fn set_armor(&mut self, value: u32) {
        self.set_u32(offsets::ARMOR, value);
    }

    /// An armor-mod socket: an index into the device folder.
    #[must_use]
    pub fn armor_mod(&self, index: usize) -> u32 {
        assert!(
            index < offsets::MOD_SOCKETS,
            "armor mod {index} out of range"
        );
        self.get_u32(offsets::ARMOR_MOD + index * 4)
    }

    /// Write an armor-mod socket.
    pub fn set_armor_mod(&mut self, index: usize, value: u32) {
        assert!(
            index < offsets::MOD_SOCKETS,
            "armor mod {index} out of range"
        );
        self.set_u32(offsets::ARMOR_MOD + index * 4, value);
    }

    /// Every armor-mod socket.
    #[must_use]
    pub fn get_raw_armor_mod_slots(&self) -> Vec<u32> {
        (0..offsets::MOD_SOCKETS)
            .map(|i| self.armor_mod(i))
            .collect()
    }

    /// The sub / board slot: an index into the device folder.
    #[must_use]
    pub fn sub(&self) -> u32 {
        self.get_u32(offsets::SUB)
    }

    /// Write the sub slot.
    pub fn set_sub(&mut self, value: u32) {
        self.set_u32(offsets::SUB, value);
    }

    // ---- disks -----------------------------------------------------------

    /// The raw disk-folder u32: `(count << 16) | (0x4000 + type)`.
    #[must_use]
    pub fn disk_raw(&self, disk_type: usize) -> u32 {
        assert!(
            disk_type < offsets::DISK_SLOTS,
            "disk type {disk_type} out of range"
        );
        self.get_u32(offsets::DISK + disk_type * 4)
    }

    /// How many of `disk_type` are owned.
    #[must_use]
    pub fn disk_count(&self, disk_type: usize) -> u16 {
        (self.disk_raw(disk_type) >> 16) as u16
    }

    /// Set how many of `disk_type` are owned, preserving the type id.
    pub fn set_disk_count(&mut self, disk_type: usize, count: u16) {
        let id = 0x4000 + disk_type as u32;
        self.set_u32(offsets::DISK + disk_type * 4, (u32::from(count) << 16) | id);
    }

    /// Every disk-folder u32.
    #[must_use]
    pub fn get_raw_disk_slots(&self) -> Vec<u32> {
        (0..offsets::DISK_SLOTS).map(|i| self.disk_raw(i)).collect()
    }

    // ---- bank ------------------------------------------------------------

    /// The bank balance.
    #[must_use]
    pub fn bank_bit(&self) -> u32 {
        self.get_u32(offsets::BANK_BIT)
    }

    /// Set the bank balance.
    pub fn set_bank_bit(&mut self, value: u32) {
        self.set_u32(offsets::BANK_BIT, value);
    }

    /// A bank storage slot.
    #[must_use]
    pub fn bank_device(&self, slot: usize) -> u32 {
        assert!(slot < offsets::BANK_SLOTS, "bank slot {slot} out of range");
        self.get_u32(offsets::BANK_DEVICE + slot * 4)
    }

    /// Write a bank storage slot.
    pub fn set_bank_device(&mut self, slot: usize, value: u32) {
        assert!(slot < offsets::BANK_SLOTS, "bank slot {slot} out of range");
        self.set_u32(offsets::BANK_DEVICE + slot * 4, value);
    }

    /// Every bank slot.
    #[must_use]
    pub fn get_raw_bank_slots(&self) -> Vec<u32> {
        (0..offsets::BANK_SLOTS)
            .map(|i| self.bank_device(i))
            .collect()
    }

    // ---- per-species tables ---------------------------------------------

    /// The level stored for `species`.
    #[must_use]
    pub fn level(&self, species: Species) -> u32 {
        self.get_u32(offsets::BASE_LEVEL + species.index() * 4)
    }

    /// Set the level for `species`.
    pub fn set_level(&mut self, species: Species, value: u32) {
        self.set_u32(offsets::BASE_LEVEL + species.index() * 4, value);
    }

    /// The EXP stored for `species`.
    #[must_use]
    pub fn exp(&self, species: Species) -> u32 {
        self.get_u32(offsets::BASE_EXP + species.index() * 4)
    }

    /// Set the EXP for `species`.
    pub fn set_exp(&mut self, species: Species, value: u32) {
        self.set_u32(offsets::BASE_EXP + species.index() * 4, value);
    }

    /// The level of technique `slot` for `species`.
    ///
    /// **Signed**: the field is an i32, so `0xFFFFFFFF` reads as `-1`.
    #[must_use]
    pub fn skill(&self, species: Species, slot: usize) -> i32 {
        assert!(
            slot < offsets::TECHNIQUE_SLOTS,
            "technique slot {slot} out of range"
        );
        let at = offsets::BASE_SKILL + (species.index() * offsets::TECHNIQUE_SLOTS + slot) * 4;
        self.get_u32(at) as i32
    }

    /// Set the level of technique `slot` for `species`.
    pub fn set_skill(&mut self, species: Species, slot: usize, value: i32) {
        assert!(
            slot < offsets::TECHNIQUE_SLOTS,
            "technique slot {slot} out of range"
        );
        let at = offsets::BASE_SKILL + (species.index() * offsets::TECHNIQUE_SLOTS + slot) * 4;
        self.set_u32(at, value as u32);
    }

    /// The power-up value in `slot` for `species`.
    #[must_use]
    pub fn upcnt(&self, species: Species, slot: usize) -> u32 {
        assert!(
            slot < offsets::POWERUP_SLOTS,
            "power-up slot {slot} out of range"
        );
        self.get_u32(offsets::BASE_UPCNT + (species.index() * offsets::POWERUP_SLOTS + slot) * 4)
    }

    /// Set the power-up value in `slot` for `species`.
    pub fn set_upcnt(&mut self, species: Species, slot: usize, value: u32) {
        assert!(
            slot < offsets::POWERUP_SLOTS,
            "power-up slot {slot} out of range"
        );
        self.set_u32(
            offsets::BASE_UPCNT + (species.index() * offsets::POWERUP_SLOTS + slot) * 4,
            value,
        );
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

    #[test]
    fn container_strides_are_four_bytes() {
        assert_eq!(
            offsets::DEVICE + offsets::DEVICE_SAVE_SLOTS * 4,
            offsets::DISK
        );
        assert_eq!(offsets::DISK + offsets::DISK_SLOTS * 4, offsets::CARD_LIST);
        assert_eq!(
            offsets::BANK_DEVICE + offsets::BANK_SLOTS * 4,
            offsets::BANK_BIT
        );
        assert_eq!(
            offsets::WEAPON + offsets::WEAPON_SLOTS * 4,
            offsets::WEAPON_MOD
        );
        assert_eq!(
            offsets::WEAPON_MOD + offsets::MOD_SOCKETS * 4,
            offsets::ARMOR
        );
        assert_eq!(offsets::ARMOR_MOD + offsets::MOD_SOCKETS * 4, offsets::SUB);
    }

    #[test]
    fn the_fixture_devices_decode_as_expected() {
        let save = SaveData::parse(&real_save()).unwrap();
        assert_eq!(save.device(0), 0x050D);
        assert_eq!(save.device(1), 0x0513);
        assert_eq!(save.device(2), 0x0512);
        assert_eq!(save.device(3), crate::EMPTY);
        assert_eq!(save.device(35), crate::EMPTY);
    }

    #[test]
    fn equipment_slots_are_device_indices() {
        let mut save = SaveData::parse(&real_save()).unwrap();
        assert_eq!(save.weapon(0), 0);
        assert_eq!(save.weapon(1), 1);
        assert_eq!(save.weapon(2), 2);
        assert_eq!(save.armor(), crate::EMPTY);
        assert_eq!(save.sub(), crate::EMPTY);

        save.set_weapon(0, 7);
        save.set_armor(9);
        save.set_sub(11);
        save.set_weapon_mod(0, 12);
        save.set_armor_mod(4, 13);
        assert_eq!(save.weapon(0), 7);
        assert_eq!(save.armor(), 9);
        assert_eq!(save.sub(), 11);
        assert_eq!(save.weapon_mod(0), 12);
        assert_eq!(save.armor_mod(4), 13);
    }

    #[test]
    fn disk_count_uses_the_high_half_and_keeps_its_type_id() {
        let mut save = SaveData::parse(&real_save()).unwrap();
        assert_eq!(save.disk_raw(0), 0x4000);
        assert_eq!(save.disk_count(0), 0);

        save.set_disk_count(3, 99);
        assert_eq!(save.disk_raw(3), (99 << 16) | 0x4003);
        assert_eq!(save.disk_count(3), 99);

        save.set_disk_count(3, 0);
        assert_eq!(
            save.disk_raw(3),
            0x4003,
            "the type id survives a count of zero"
        );
    }

    #[test]
    fn bank_slots_and_balance_round_trip() {
        let mut save = SaveData::parse(&real_save()).unwrap();
        assert_eq!(save.bank_device(0), crate::EMPTY);
        assert_eq!(save.bank_bit(), 0);

        save.set_bank_device(95, 0x3000);
        save.set_bank_bit(1_234_567);
        assert_eq!(save.bank_device(95), 0x3000);
        assert_eq!(save.bank_bit(), 1_234_567);
    }

    #[test]
    fn a_container_write_is_mirrored_too() {
        let mut save = SaveData::parse(&real_save()).unwrap();
        save.set_device(29, 0xDEAD_BEEF);
        assert_eq!(save.get_u32_block(offsets::DEVICE + 29 * 4, 1), 0xDEAD_BEEF);
    }

    #[test]
    fn per_species_tables_are_indexed_by_species_and_slot() {
        assert_eq!(offsets::BASE_LEVEL + 16 * 4, offsets::BASE_EXP);
        assert_eq!(offsets::BASE_EXP + 16 * 4, offsets::BASE_SKILL);
        assert_eq!(offsets::BASE_SKILL + 16 * 9 * 4, offsets::BASE_UPCNT);
        assert_eq!(offsets::BASE_UPCNT + 16 * 11 * 4, offsets::PAD_END);
    }

    #[test]
    fn a_fresh_save_has_level_one_everywhere() {
        let save = SaveData::parse(&real_save()).unwrap();
        for sp in crate::Species::ALL {
            assert_eq!(save.level(sp), 1, "{sp:?}");
            assert_eq!(save.exp(sp), 0, "{sp:?}");
        }
    }

    #[test]
    fn writing_one_species_leaves_the_others_alone() {
        let mut save = SaveData::parse(&real_save()).unwrap();
        save.set_level(crate::Species::ImperialdramonPm, 999);
        save.set_exp(crate::Species::ImperialdramonPm, 1_133_652_152);
        assert_eq!(save.level(crate::Species::ImperialdramonPm), 999);
        assert_eq!(save.exp(crate::Species::ImperialdramonPm), 1_133_652_152);
        assert_eq!(save.level(crate::Species::Dorumon), 1);
        assert_eq!(save.exp(crate::Species::Dorumon), 0);
    }

    #[test]
    fn techniques_round_trip_through_the_signed_type() {
        let mut save = SaveData::parse(&real_save()).unwrap();
        let dorumon = crate::Species::Dorumon;
        assert_eq!(save.skill(dorumon, 0), 1);

        for (slot, value) in [(0, -1), (1, 9_999), (2, i32::MAX), (3, i32::MIN), (4, 0)] {
            save.set_skill(dorumon, slot, value);
            assert_eq!(save.skill(dorumon, slot), value, "slot {slot}");
        }
    }

    #[test]
    fn a_minus_one_technique_is_ffffffff_in_the_block() {
        let mut save = SaveData::parse(&real_save()).unwrap();
        // Agumon is species 0, so its first technique slot is exactly BASE_SKILL.
        save.set_skill(crate::Species::Agumon, 0, -1);
        assert_eq!(
            save.get_u32(offsets::BASE_SKILL),
            0xFFFF_FFFF,
            "a signed -1 is stored as all ones"
        );
    }

    #[test]
    fn power_ups_are_indexed_per_species() {
        let mut save = SaveData::parse(&real_save()).unwrap();
        let veemon = crate::Species::Veemon;
        for slot in 0..11 {
            assert_eq!(save.upcnt(veemon, slot), 0, "slot {slot}");
        }
        save.set_upcnt(veemon, 0, 99_999);
        save.set_upcnt(veemon, 10, 9_999);
        assert_eq!(save.upcnt(veemon, 0), 99_999);
        assert_eq!(save.upcnt(veemon, 10), 9_999);
        assert_eq!(save.upcnt(crate::Species::Dorumon, 0), 0);
    }
}
