//! Every block-relative offset of the 0xA000-byte save block.
//!
//! One definition per field, in one place. Nothing else in the crate may
//! hardcode an offset.
//!
//! Layout provenance: `Decomp/DW4/pi-re/ghidra-projects/dw4/docs/08-save-format.md`
//! and `Decomp/DW4/FLAG_MAP.md`. Where the Python editor disagrees with the
//! arithmetic, the arithmetic wins — see `PAD_END`.

/// Size of one mirrored save block.
pub const BLOCK: usize = 0xA000;

// ---- 8-byte header -------------------------------------------------------
/// Per-block checksum: sum of u32 LE words from `+4` to `+0xA000`.
pub const CHECKSUM: usize = 0x0000;
/// Save format version. Real saves are `4`.
pub const VERSION: usize = 0x0004;
/// `ISUSE` ("slot in use"). Real saves are `1`.
pub const ISUSE: usize = 0x0008;
/// `UNIQUE`, an id that **varies between saves**. Observed `0x6096F82C` and
/// `0x700FFDAC`. Not a constant; do not assert a fixed value against a real save.
pub const UNIQUE: usize = 0x000C;

// ---- names ---------------------------------------------------------------
/// 16-byte ASCII model stem, e.g. `p_dorumon`.
pub const DIGIMON_NAME: usize = 0x0010;
/// 16 bytes of name padding.
pub const DIGIMON_NAME_PAD: usize = 0x0020;
/// 18 bytes: u16 `0xFFFF` marker, then up to 8 fullwidth chars, NUL-padded.
pub const PLAYER_NAME: usize = 0x0030;
/// 14 bytes of name padding.
pub const PLAYER_NAME_PAD: usize = 0x0042;

// ---- menu snapshot + currency -------------------------------------------
/// `LEVEL`, a menu snapshot the game recomputes.
pub const MENU_LEVEL: usize = 0x0050;
/// `HP`, derived — never written by the editor.
pub const HP: usize = 0x0054;
/// `MHP`, derived — never written by the editor.
pub const MHP: usize = 0x0058;
/// `MP`, derived — never written by the editor.
pub const MP: usize = 0x005C;
/// `MMP`, derived — never written by the editor.
pub const MMP: usize = 0x0060;
/// `XDATA` counter.
pub const XDATA: usize = 0x0064;
/// `BIT`, the currency.
pub const BIT: usize = 0x0068;

// ---- containers ----------------------------------------------------------
/// 36 × u32 device folder. Slots 0..30 are the usable inventory.
pub const DEVICE: usize = 0x006C;
/// Usable device-folder slots (3 pages × 10).
pub const DEVICE_SLOTS: usize = 30;
/// Reserved save region for the device folder, including 6 padding slots.
pub const DEVICE_SAVE_SLOTS: usize = 36;
/// 12 × u32 disk folder: `(count << 16) | (0x4000 + i)`.
pub const DISK: usize = 0x00FC;
/// Number of disk types.
pub const DISK_SLOTS: usize = 12;
/// 52-byte collected-card bitfield. Read-only in this crate.
pub const CARD_LIST: usize = 0x012C;
/// 96 × u32 bank item storage.
pub const BANK_DEVICE: usize = 0x0160;
/// Number of bank slots.
pub const BANK_SLOTS: usize = 96;
/// Bank balance.
pub const BANK_BIT: usize = 0x02E0;

// ---- equipment -----------------------------------------------------------
/// 3 × u32 weapon slots; each is an index into the device folder.
pub const WEAPON: usize = 0x02E4;
/// Number of weapon slots.
pub const WEAPON_SLOTS: usize = 3;
/// 5 × u32 weapon-mod sockets; each is an index into the device folder.
pub const WEAPON_MOD: usize = 0x02F0;
/// 1 × u32 armor / core slot; an index into the device folder.
pub const ARMOR: usize = 0x0304;
/// 5 × u32 armor-mod sockets; each is an index into the device folder.
pub const ARMOR_MOD: usize = 0x0308;
/// Number of mod sockets on each of weapon and armor.
pub const MOD_SOCKETS: usize = 5;
/// 1 × u32 sub / board slot; an index into the device folder.
pub const SUB: usize = 0x031C;

// ---- story state ---------------------------------------------------------
/// 1024 × u8. `0x01` = set / event happened; `0x00` = clear.
pub const BASE_FLAG: usize = 0x0320;
/// Number of story flags.
pub const FLAG_COUNT: usize = 1024;
/// 12 × u8 folder state. `0x01` = SET.
pub const BASE_FLAG_FOLDER: usize = 0x0720;
/// Number of story folders.
pub const FOLDER_COUNT: usize = 12;
/// 16 × u32 counters. Index 1 is the junk-shop donation total (`+0x730`).
pub const BASE_COUNTER: usize = 0x072C;
/// Number of counters.
pub const COUNTER_SLOTS: usize = 16;
/// Index of the junk-shop donation counter inside `BASE_COUNTER`.
pub const COUNTER_JUNK: usize = 1;
/// 16 × u8; `1` marks the active species.
pub const BASE_ISUSE: usize = 0x076C;

// ---- per-species tables --------------------------------------------------
/// Number of species.
pub const SPECIES_COUNT: usize = 16;
/// 16 × u32 level, one per species.
pub const BASE_LEVEL: usize = 0x077C;
/// 16 × u32 EXP, one per species.
pub const BASE_EXP: usize = 0x07BC;
/// 144 × u32 = 16 species × 9 technique slots. **Signed** i32.
pub const BASE_SKILL: usize = 0x07FC;
/// Technique slots per species.
pub const TECHNIQUE_SLOTS: usize = 9;
/// 176 × u32 = 16 species × 11 power-up slots.
pub const BASE_UPCNT: usize = 0x0A3C;
/// Power-up slots per species.
pub const POWERUP_SLOTS: usize = 11;
/// First offset of the zero padding that ends the record.
///
/// 176 u32 from `BASE_UPCNT` ends at `0xCFC`. `dw4build.py` claims `0xAFC`;
/// that is wrong by `0x200` and is confirmed against a synthesised save with
/// every power-up slot filled (last non-zero byte `0xCF9`).
pub const PAD_END: usize = 0x0CFC;

/// One named field, for tests and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    /// Field name, as written in the format docs.
    pub name: &'static str,
    /// Block-relative offset.
    pub offset: usize,
    /// Length in bytes.
    pub len: usize,
}

/// The complete field map, ascending by offset, with no gaps before `PAD_END`.
///
/// `tests::the_whole_record_is_covered_by_known_fields` enforces both
/// properties, so this table cannot silently drift from the consts above.
///
/// Laid out one field per line on purpose: this table *is* the format
/// documentation, and `rustfmt` would expand each entry to four lines.
#[rustfmt::skip]
pub const FIELDS: &[Field] = &[
    Field { name: "CHECKSUM", offset: CHECKSUM, len: 4 },
    Field { name: "VERSION", offset: VERSION, len: 4 },
    Field { name: "ISUSE", offset: ISUSE, len: 4 },
    Field { name: "UNIQUE", offset: UNIQUE, len: 4 },
    Field { name: "DIGIMON_NAME", offset: DIGIMON_NAME, len: 16 },
    Field { name: "DIGIMON_NAME_PAD", offset: DIGIMON_NAME_PAD, len: 16 },
    Field { name: "PLAYER_NAME", offset: PLAYER_NAME, len: 18 },
    Field { name: "PLAYER_NAME_PAD", offset: PLAYER_NAME_PAD, len: 14 },
    Field { name: "MENU_LEVEL", offset: MENU_LEVEL, len: 4 },
    Field { name: "HP", offset: HP, len: 4 },
    Field { name: "MHP", offset: MHP, len: 4 },
    Field { name: "MP", offset: MP, len: 4 },
    Field { name: "MMP", offset: MMP, len: 4 },
    Field { name: "XDATA", offset: XDATA, len: 4 },
    Field { name: "BIT", offset: BIT, len: 4 },
    Field { name: "DEVICE", offset: DEVICE, len: DEVICE_SAVE_SLOTS * 4 },
    Field { name: "DISK", offset: DISK, len: DISK_SLOTS * 4 },
    Field { name: "CARD_LIST", offset: CARD_LIST, len: 52 },
    Field { name: "BANK_DEVICE", offset: BANK_DEVICE, len: BANK_SLOTS * 4 },
    Field { name: "BANK_BIT", offset: BANK_BIT, len: 4 },
    Field { name: "WEAPON", offset: WEAPON, len: WEAPON_SLOTS * 4 },
    Field { name: "WEAPON_MOD", offset: WEAPON_MOD, len: MOD_SOCKETS * 4 },
    Field { name: "ARMOR", offset: ARMOR, len: 4 },
    Field { name: "ARMOR_MOD", offset: ARMOR_MOD, len: MOD_SOCKETS * 4 },
    Field { name: "SUB", offset: SUB, len: 4 },
    Field { name: "BASE_FLAG", offset: BASE_FLAG, len: FLAG_COUNT },
    Field { name: "BASE_FLAG_FOLDER", offset: BASE_FLAG_FOLDER, len: FOLDER_COUNT },
    Field { name: "BASE_COUNTER", offset: BASE_COUNTER, len: COUNTER_SLOTS * 4 },
    Field { name: "BASE_ISUSE", offset: BASE_ISUSE, len: SPECIES_COUNT },
    Field { name: "BASE_LEVEL", offset: BASE_LEVEL, len: SPECIES_COUNT * 4 },
    Field { name: "BASE_EXP", offset: BASE_EXP, len: SPECIES_COUNT * 4 },
    Field { name: "BASE_SKILL", offset: BASE_SKILL, len: SPECIES_COUNT * TECHNIQUE_SLOTS * 4 },
    Field { name: "BASE_UPCNT", offset: BASE_UPCNT, len: SPECIES_COUNT * POWERUP_SLOTS * 4 },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn fields_are_sorted_disjoint_and_inside_the_block() {
        let mut prev_end = 0usize;
        for f in FIELDS {
            assert!(
                f.offset >= prev_end,
                "field {} at 0x{:04X} overlaps the previous field, which ends at 0x{prev_end:04X}",
                f.name,
                f.offset
            );
            assert!(
                f.offset + f.len <= BLOCK,
                "field {} at 0x{:04X} (+{} bytes) runs past the block",
                f.name,
                f.offset,
                f.len
            );
            assert!(f.len > 0, "field {} has zero length", f.name);
            prev_end = f.offset + f.len;
        }
    }

    #[test]
    fn field_names_are_unique() {
        let mut seen = BTreeSet::new();
        for f in FIELDS {
            assert!(seen.insert(f.name), "duplicate field name {}", f.name);
        }
    }

    #[test]
    fn base_upcnt_ends_at_0xcfc_not_0xafc() {
        // dw4build.py documents 0xafc; 176 u32 from 0xa3c provably ends at 0xcfc.
        assert_eq!(BASE_UPCNT + 176 * 4, 0x0CFC);
        assert_eq!(PAD_END, 0x0CFC);
    }

    #[test]
    fn the_whole_record_is_covered_by_known_fields() {
        // The header + record runs from 0 to PAD_END with no gaps: every gap in
        // this range is an explicitly named padding field.
        let mut prev_end = 0usize;
        for f in FIELDS {
            assert_eq!(
                f.offset, prev_end,
                "unclaimed bytes 0x{prev_end:04X}..0x{:04X} before field {}",
                f.offset, f.name
            );
            prev_end = f.offset + f.len;
        }
        assert_eq!(prev_end, PAD_END);
    }
}
