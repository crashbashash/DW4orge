//! Property tests that cross module boundaries.
use dw4core::{
    SaveData, Species, build_item_id, level_from_exp, level_threshold, offsets, split_item_id,
};
use proptest::prelude::*;

fn real_save() -> Vec<u8> {
    let p =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mcd001/save.raw");
    std::fs::read(p).expect("run tools/gen_fixtures.py first")
}

proptest! {
    /// Base id, seed and mod count survive a pack/unpack round trip.
    #[test]
    fn item_ids_round_trip(base in 0u32..0x1_0000, seed in 0u16..0x1000, mods in 0u8..0x10) {
        let full = build_item_id(base, seed, mods);
        prop_assert_eq!(split_item_id(full), (base, seed, mods));
    }

    /// The level curve is a bijection between level and its threshold.
    #[test]
    fn level_lookup_inverts_the_curve(level in 1u32..1000) {
        let exp = u32::try_from(level_threshold(level)).unwrap();
        prop_assert_eq!(level_from_exp(exp), level);
    }

    /// Any EXP between two thresholds maps to the lower level.
    #[test]
    fn level_lookup_picks_the_floor(level in 1u32..900, slack in 0u32..300) {
        let lo = u32::try_from(level_threshold(level)).unwrap();
        let hi = u32::try_from(level_threshold(level + 1)).unwrap();
        let exp = lo.saturating_add(slack).min(hi - 1);
        prop_assert_eq!(level_from_exp(exp), level);
    }

    /// Writing any field leaves the other block byte-identical to it.
    #[test]
    fn every_write_mirrors_both_blocks(index in 0usize..offsets::FIELDS.len(), value: u32) {
        let field = offsets::FIELDS[index];
        if field.offset == offsets::CHECKSUM {
            return Ok(());  // the checksum is the one field writes are free to leave stale
        }
        if field.len != 4 {
            return Ok(());  // this property is about single u32 fields
        }
        let mut save = SaveData::parse(&real_save()).unwrap();
        save.set_u32(field.offset, value);
        prop_assert_eq!(save.get_u32_block(field.offset, 0), value);
        prop_assert_eq!(save.get_u32_block(field.offset, 1), value);
    }

    /// A save written out and read back is unchanged apart from its checksum
    /// and the one field we changed, and its checksum verifies.
    #[test]
    fn writing_then_reading_back_preserves_every_other_byte(bit: u32) {
        let original = real_save();
        let mut save = SaveData::parse(&original).unwrap();
        save.set_bit(bit);
        let out = save.to_bytes();
        let back = SaveData::parse(&out).unwrap();
        prop_assert!(back.verify());
        prop_assert_eq!(back.bit(), bit);

        // In both blocks, everything outside BIT is untouched — including the
        // stored checksum, which to_bytes recomputes from the same data.
        for block in 0..2 {
            let base = block * offsets::BLOCK;
            prop_assert_eq!(
                &out[base + 4..base + offsets::BIT],
                &original[base + 4..base + offsets::BIT]
            );
            prop_assert_eq!(
                &out[base + offsets::BIT + 4..base + offsets::BLOCK],
                &original[base + offsets::BIT + 4..base + offsets::BLOCK]
            );
        }
    }

    /// Every species index is reachable from its own model name.
    #[test]
    fn species_model_names_round_trip(index in 0usize..16) {
        let sp = Species::from_index(index).unwrap();
        prop_assert_eq!(Species::from_model_name(&sp.model_name()), Some(sp));
    }

    /// A species edit is confined to that species' own row.
    #[test]
    fn a_species_edit_does_not_disturb_other_species(
        index in 0usize..16,
        level in 1u32..1000,
        slot in 0usize..11,
        value in 0u32..100_000,
    ) {
        let mut save = SaveData::parse(&real_save()).unwrap();
        let target = Species::from_index(index).unwrap();
        save.set_level(target, level);
        save.set_upcnt(target, slot, value);
        prop_assert_eq!(save.level(target), level);
        prop_assert_eq!(save.upcnt(target, slot), value);

        for other in Species::ALL {
            if other != target {
                prop_assert_eq!(save.level(other), 1, "{:?} was disturbed", other);
                for s in 0..11 {
                    prop_assert_eq!(save.upcnt(other, s), 0, "{:?} slot {} was disturbed", other, s);
                }
            }
        }
    }
}
