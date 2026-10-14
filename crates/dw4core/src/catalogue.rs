//! The item catalogue, embedded from `data/*.json`.
//!
//! The five JSON files were distilled from the game's own item tables during
//! reverse engineering and are vendored verbatim; see `data/PROVENANCE.md`.
//! They are embedded rather than loaded so the crate has no runtime data
//! dependency: the CLI, the tests and the Tauri app all get the same
//! catalogue without shipping loose files.
//!
//! None of this module touches save bytes. It maps base ids to names, decides
//! whether a base id is safe to write, and renders an item id for display, all
//! matching the Python editor.

/// Graded weapons: 50 models × 5 grades + 6 unique.
pub const WEAPONS_JSON: &str = include_str!("../data/weapons.json");
/// Styled weapons, flat ids.
pub const STYLED_JSON: &str = include_str!("../data/styled.json");
/// Cores, used in the armor slot.
pub const CORES_JSON: &str = include_str!("../data/cores.json");
/// Boards, used in the sub slot.
pub const ARMOR_JSON: &str = include_str!("../data/armor.json");
/// Mod chips: families, grades and stat-variant offsets.
pub const MODS_JSON: &str = include_str!("../data/mods.json");

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::item::Category;

/// Greek letters used for a graded item's grade.
pub const GRADE_LETTER: [&str; 5] = ["\u{3b1}", "\u{3b2}", "\u{3b3}", "\u{3b4}", "\u{3b5}"];

/// Map a grade name from the data files to its index.
fn grade_index(name: &str) -> Option<u8> {
    match name.to_ascii_lowercase().as_str() {
        "alpha" | "a" => Some(0),
        "beta" | "b" => Some(1),
        "gamma" | "g" | "y" => Some(2),
        "delta" | "d" => Some(3),
        "epsilon" | "e" => Some(4),
        _ => None,
    }
}

/// One catalogue entry, keyed by its base id (the low 16 bits of a stored id).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Item {
    /// The base id: `category << 8 | index`.
    pub base_id: u32,
    /// Display name, without any grade suffix.
    pub name: String,
    /// Which slot the item belongs to.
    pub category: Category,
    /// Grade index 0-4, where the item is graded.
    pub grade: Option<u8>,
    /// Free-text caveat, where the data files carry one.
    pub note: Option<String>,
}

/// Every known item, keyed by base id.
///
/// Ordered by base id so iteration and serialisation are deterministic.
#[derive(Debug)]
pub struct ItemCatalogue {
    items: BTreeMap<u32, Item>,
}

impl ItemCatalogue {
    /// Look up a base id.
    #[must_use]
    pub fn get(&self, base_id: u32) -> Option<&Item> {
        self.items.get(&base_id)
    }

    /// How many items the catalogue holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the catalogue is empty. Never true in practice; clippy asks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Every item, ascending by base id.
    pub fn iter(&self) -> impl Iterator<Item = &Item> {
        self.items.values()
    }

    /// Build the catalogue from the embedded data files.
    ///
    /// # Panics
    ///
    /// Panics if the embedded data does not parse or is missing a field. The
    /// data is checked in and covered by tests, so a failure here is a build
    /// defect, not a runtime condition.
    fn load_from_embedded() -> Self {
        let mut items = BTreeMap::new();

        // Graded weapons: `items` carry a grade, `unique` do not.
        let weapons: serde_json::Value =
            serde_json::from_str(WEAPONS_JSON).expect("weapons.json parses");
        for entry in weapons["items"].as_array().expect("items array") {
            let base_id = entry["id"].as_u64().expect("id") as u32;
            let name = entry["name"].as_str().expect("name").to_owned();
            let grade = entry
                .get("grade")
                .and_then(|g| g.as_str())
                .and_then(grade_index);
            items.insert(
                base_id,
                Item {
                    base_id,
                    name,
                    category: Category::Weapon,
                    grade,
                    note: None,
                },
            );
        }
        for (id, name) in weapons["unique"].as_object().expect("unique object") {
            let base_id: u32 = id.parse().expect("unique keys are decimal ids");
            items.insert(
                base_id,
                Item {
                    base_id,
                    name: name.as_str().expect("name").to_owned(),
                    category: Category::Weapon,
                    grade: None,
                    note: None,
                },
            );
        }

        // Styled weapons: flat, no grade.
        let styled: serde_json::Value =
            serde_json::from_str(STYLED_JSON).expect("styled.json parses");
        for entry in styled["items"].as_array().expect("items array") {
            let base_id = entry["id"].as_u64().expect("id") as u32;
            items.insert(
                base_id,
                Item {
                    base_id,
                    name: entry["name"].as_str().expect("name").to_owned(),
                    category: Category::Styled,
                    grade: None,
                    note: None,
                },
            );
        }

        // Cores (armor) and boards (sub slot): both graded.
        for (raw, category) in [(CORES_JSON, Category::Core), (ARMOR_JSON, Category::Board)] {
            let data: serde_json::Value = serde_json::from_str(raw).expect("json parses");
            for entry in data["items"].as_array().expect("items array") {
                let base_id = entry["id"].as_u64().expect("id") as u32;
                let grade = entry
                    .get("grade")
                    .and_then(|g| g.as_str())
                    .and_then(grade_index);
                items.insert(
                    base_id,
                    Item {
                        base_id,
                        name: entry["name"].as_str().expect("name").to_owned(),
                        category,
                        grade,
                        note: None,
                    },
                );
            }
        }

        // Mod chips: families of graded blocks, with stat variants in the
        // 3-grade shape only.
        let mods: serde_json::Value = serde_json::from_str(MODS_JSON).expect("mods.json parses");
        let variant_seq: Vec<u32> = mods["variant_seq"]
            .as_array()
            .expect("variant_seq array")
            .iter()
            .map(|v| v.as_u64().expect("variant is a number") as u32)
            .collect();
        for family in mods["families"].as_array().expect("families array") {
            let block = family["block"].as_u64().expect("block") as u32;
            let grades = family["grades"].as_u64().expect("grades") as u32;
            let name = family["name"].as_str().expect("name");

            // 3-grade families give each grade a 5-base block of stat variants.
            // 5-grade families give each grade exactly one base.
            let stride = if grades == 3 { 5 } else { 1 };
            for grade in 0..grades {
                let grade_letter = GRADE_LETTER[grade as usize];
                for offset in 0..stride {
                    let base = 0x3000 | (block + grade * stride + offset);
                    let display = if offset == 0 {
                        format!("{name} {grade_letter}")
                    } else {
                        // Offsets 1..=4 index variant_seq from this grade's slot.
                        let variant = variant_seq
                            .get((grade + offset - 1) as usize)
                            .copied()
                            .expect("variant sequence covers every 3-grade offset");
                        format!("{name} {grade_letter}+{variant}")
                    };
                    items.insert(
                        base,
                        Item {
                            base_id: base,
                            name: display,
                            category: Category::Mod,
                            grade: None,
                            note: None,
                        },
                    );
                }
            }
        }

        Self { items }
    }
}

/// The process-wide catalogue, built once on first use.
#[must_use]
pub fn get_catalogue() -> &'static ItemCatalogue {
    static CATALOGUE: OnceLock<ItemCatalogue> = OnceLock::new();
    CATALOGUE.get_or_init(ItemCatalogue::load_from_embedded)
}

use crate::codes::{SEED_BONUS_MASK, color_for_seed};
use crate::item::{
    CAT_BOARD, CAT_CORE, CAT_MOD, CAT_MOD_EQUIPPED, CAT_STYLED, CAT_WEAPON, split_item_id,
};

/// A human label for a raw category byte, for ids outside the catalogue.
#[must_use]
pub fn category_label(category: u8) -> String {
    match category {
        CAT_WEAPON => "Weapon".to_owned(),
        CAT_STYLED => "Weapon (styled)".to_owned(),
        CAT_CORE => "Core (armor)".to_owned(),
        CAT_BOARD => "Board (sub)".to_owned(),
        CAT_MOD => "Mod (chip)".to_owned(),
        other => format!("cat 0x{other:02x}"),
    }
}

/// Why a base id is unsafe to write, or `None` if it is fine.
///
/// Only catalogue entries are accepted. Everything else is described by the
/// confirmed glitch ranges, which is what keeps Normal mode from writing an
/// item that blanks a menu or crashes the game on load.
#[must_use]
pub fn invalid_reason(base_id: u32) -> Option<String> {
    if get_catalogue().get(base_id).is_some() {
        return None;
    }

    let category = ((base_id >> 8) & 0xFF) as u8;
    let low = (base_id & 0xFF) as u8;

    match category {
        CAT_STYLED => {
            if (0x15..=0x1A).contains(&low) {
                return Some("styled weapon: blank item".to_owned());
            }
            if (0x1B..=0x3E).contains(&low) {
                return Some("styled weapon: CRASHES ON LOAD".to_owned());
            }
            if low == 0x3F {
                return Some("styled weapon: blank '0' weapon (cannot hit)".to_owned());
            }
        }
        CAT_CORE => {
            if (0x20..=0x3F).contains(&low) {
                return Some("core: blank/crash (0x26 crashes when selected)".to_owned());
            }
        }
        CAT_BOARD => {
            if (0x2D..=0x35).contains(&low) {
                return Some("board: crashes opening devices folder".to_owned());
            }
            if (0x36..=0x3E).contains(&low) {
                return Some("board: crash, but STRENGTH shows 9999".to_owned());
            }
            if low == 0x3F {
                return Some("board: crash, but WISDOM shows 9999".to_owned());
            }
        }
        CAT_MOD if low >= 0xB9 => {
            return Some("mod: blank".to_owned());
        }
        CAT_MOD_EQUIPPED => {
            return Some(
                "mod: 'equipped' category byte (game-managed, not a real item)".to_owned(),
            );
        }
        _ => {}
    }

    Some(format!("unknown item id 0x{base_id:04x}"))
}

/// Render a stored item id for display, valid or not.
///
/// A graded catalogue item gets its grade letter; a non-zero seed or mod count
/// is appended as `(rarity +N, M mods)`.
#[must_use]
pub fn describe_item_id(full_id: u32) -> String {
    if full_id == crate::EMPTY {
        return "(empty)".to_owned();
    }

    let (base_id, seed, mod_count) = split_item_id(full_id);
    let suffix = if seed == 0 && mod_count == 0 {
        String::new()
    } else {
        format!(
            " ({} +{}, {} mods)",
            color_for_seed(seed).label(),
            seed & SEED_BONUS_MASK,
            mod_count
        )
    };

    match get_catalogue().get(base_id) {
        Some(item) => {
            let grade = item
                .grade
                .map(|g| format!(" {}", GRADE_LETTER[g as usize]))
                .unwrap_or_default();
            format!("{}{grade}{suffix}", item.name)
        }
        None => {
            let category = ((base_id >> 8) & 0xFF) as u8;
            format!(
                "[invalid] {} 0x{full_id:08x}{suffix}",
                category_label(category)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_five_data_files_are_embedded_and_parse() {
        for raw in [WEAPONS_JSON, STYLED_JSON, CORES_JSON, ARMOR_JSON, MODS_JSON] {
            let v: serde_json::Value =
                serde_json::from_str(raw).expect("vendored data must be valid JSON");
            assert!(v.is_object(), "each data file is an object");
        }
    }

    #[test]
    fn the_embedded_data_has_the_expected_shape() {
        let w: serde_json::Value = serde_json::from_str(WEAPONS_JSON).unwrap();
        assert_eq!(w["items"].as_array().unwrap().len(), 256);
        assert_eq!(w["unique"].as_object().unwrap().len(), 6);

        let s: serde_json::Value = serde_json::from_str(STYLED_JSON).unwrap();
        assert_eq!(s["items"].as_array().unwrap().len(), 21);

        let c: serde_json::Value = serde_json::from_str(CORES_JSON).unwrap();
        assert_eq!(c["items"].as_array().unwrap().len(), 32);

        let a: serde_json::Value = serde_json::from_str(ARMOR_JSON).unwrap();
        assert_eq!(a["items"].as_array().unwrap().len(), 45);

        let m: serde_json::Value = serde_json::from_str(MODS_JSON).unwrap();
        assert_eq!(m["families"].as_array().unwrap().len(), 15);
    }

    #[test]
    fn a_graded_weapon_has_a_grade_and_a_model_name() {
        let cat = get_catalogue();
        let hawk = cat.get(0x0000).expect("Battle Hawk alpha");
        assert_eq!(hawk.name, "Battle Hawk");
        assert_eq!(hawk.category, crate::Category::Weapon);
        assert_eq!(hawk.grade, Some(0));

        let beta = cat.get(0x0001).expect("Battle Hawk beta");
        assert_eq!(beta.name, "Battle Hawk");
        assert_eq!(beta.grade, Some(1));
    }

    #[test]
    fn the_six_unique_weapons_sit_above_the_graded_block_and_have_no_grade() {
        let cat = get_catalogue();
        let judgment = cat.get(0x00FA).expect("Judgment is id 250");
        assert_eq!(judgment.name, "Judgment");
        assert_eq!(judgment.grade, None);

        let lance = cat.get(0x00FF).expect("Nightmare Lance is id 255");
        assert_eq!(lance.name, "Nightmare Lance");
        assert_eq!(lance.grade, None);
    }

    #[test]
    fn styled_weapons_are_flat_and_include_the_appraisal_item() {
        let cat = get_catalogue();
        let katana = cat.get(0x050D).expect("Bash Katana");
        assert_eq!(katana.name, "Bash Katana");
        assert_eq!(katana.category, crate::Category::Styled);
        assert_eq!(katana.grade, None);

        let unknown = cat.get(0x0514).expect("the appraisal weapon");
        assert!(unknown.name.starts_with("??????"), "got {:?}", unknown.name);
    }

    #[test]
    fn cores_and_boards_are_graded_three_ways() {
        let cat = get_catalogue();
        let core = cat.get(0x1000).expect("Brave Core alpha");
        assert_eq!(core.name, "Brave Core");
        assert_eq!(core.category, crate::Category::Core);
        assert_eq!(core.grade, Some(0));

        let board = cat.get(0x2000).expect("Achilles Board alpha");
        assert_eq!(board.name, "Achilles Board");
        assert_eq!(board.category, crate::Category::Board);
        assert_eq!(board.grade, Some(0));
    }

    #[test]
    fn the_categories_that_are_not_in_the_catalogue_are_absent() {
        let cat = get_catalogue();
        assert!(cat.get(0x1020).is_none(), "cores stop at 0x101f");
        assert!(cat.get(0x202D).is_none(), "boards stop at 0x202c");
    }

    #[test]
    fn every_catalogue_entry_keys_to_its_own_base_id() {
        for item in get_catalogue().iter() {
            let found = get_catalogue().get(item.base_id).expect("round trip");
            assert_eq!(found.base_id, item.base_id);
        }
    }

    #[test]
    fn a_three_grade_family_lays_out_as_block_plus_grade_times_five() {
        // Wisdom Chip: block 0, 3 grades, 5 stat variants per grade.
        let cat = get_catalogue();
        assert_eq!(cat.get(0x3000).unwrap().name, "Wisdom Chip \u{3b1}");
        assert_eq!(cat.get(0x3005).unwrap().name, "Wisdom Chip \u{3b2}");
        assert_eq!(cat.get(0x300A).unwrap().name, "Wisdom Chip \u{3b3}");
        for base in [0x3000, 0x3005, 0x300A] {
            assert_eq!(cat.get(base).unwrap().category, crate::Category::Mod);
        }
    }

    #[test]
    fn stat_variants_are_suffixed_by_the_variant_sequence() {
        // alpha: +30, +60, +120, +240. beta starts 30 higher.
        let cat = get_catalogue();
        assert_eq!(cat.get(0x3001).unwrap().name, "Wisdom Chip \u{3b1}+30");
        assert_eq!(cat.get(0x3002).unwrap().name, "Wisdom Chip \u{3b1}+60");
        assert_eq!(cat.get(0x3003).unwrap().name, "Wisdom Chip \u{3b1}+120");
        assert_eq!(cat.get(0x3004).unwrap().name, "Wisdom Chip \u{3b1}+240");
        assert_eq!(cat.get(0x3006).unwrap().name, "Wisdom Chip \u{3b2}+60");
    }

    #[test]
    fn a_five_grade_family_has_one_base_per_grade_and_no_variants() {
        // Drain Ram: block 105, 5 grades, no stat variants.
        let cat = get_catalogue();
        assert_eq!(cat.get(0x3069).unwrap().name, "Drain Ram \u{3b1}");
        assert_eq!(cat.get(0x306A).unwrap().name, "Drain Ram \u{3b2}");
        assert_eq!(cat.get(0x306D).unwrap().name, "Drain Ram \u{3b5}");
    }

    #[test]
    fn the_last_mod_family_ends_at_0x30b8() {
        // Defence ROM: block 180, 5 grades.
        let cat = get_catalogue();
        let last = cat.get(0x30B8).expect("Defence ROM epsilon");
        assert_eq!(last.name, "Defence ROM \u{3b5}");
        assert!(cat.get(0x30B9).is_none(), "mods stop at 0x30b8");
    }

    #[test]
    fn every_mod_id_from_the_data_files_is_in_the_catalogue() {
        let cat = get_catalogue();
        let mods: serde_json::Value = serde_json::from_str(MODS_JSON).unwrap();
        let mut expected = 0usize;
        for family in mods["families"].as_array().unwrap() {
            let block = family["block"].as_u64().unwrap() as u32;
            let grades = family["grades"].as_u64().unwrap() as u32;
            let span = if grades == 3 { 15 } else { 5 };
            for offset in 0..span {
                let base = 0x3000 | (block + offset);
                assert!(
                    cat.get(base).is_some(),
                    "base 0x{base:04X} from family {} is missing",
                    family["name"]
                );
                expected += 1;
            }
        }
        assert_eq!(expected, 185, "the data files describe 185 mod ids");
    }

    #[test]
    fn the_catalogue_holds_539_items() {
        // 256 graded + 6 unique weapons, 21 styled, 32 cores, 45 boards,
        // 185 mods. Task 5 checks this against the oracle's own count; this
        // one fails with a number instead of a fixture mismatch.
        assert_eq!(get_catalogue().len(), 539);
    }

    #[test]
    fn a_catalogue_entry_is_never_invalid() {
        for item in get_catalogue().iter() {
            assert_eq!(
                invalid_reason(item.base_id),
                None,
                "0x{:04X} ({}) is in the catalogue but reported invalid",
                item.base_id,
                item.name
            );
        }
    }

    #[test]
    fn the_styled_weapon_glitch_ranges_match_the_oracle() {
        assert_eq!(
            invalid_reason(0x0515).as_deref(),
            Some("styled weapon: blank item")
        );
        assert_eq!(
            invalid_reason(0x051A).as_deref(),
            Some("styled weapon: blank item")
        );
        assert_eq!(
            invalid_reason(0x051B).as_deref(),
            Some("styled weapon: CRASHES ON LOAD")
        );
        assert_eq!(
            invalid_reason(0x053E).as_deref(),
            Some("styled weapon: CRASHES ON LOAD")
        );
        assert_eq!(
            invalid_reason(0x053F).as_deref(),
            Some("styled weapon: blank '0' weapon (cannot hit)")
        );
        // 0x0514 is the appraisal weapon and is safe.
        assert_eq!(invalid_reason(0x0514), None);
    }

    #[test]
    fn the_core_board_and_mod_ranges_match_the_oracle() {
        assert_eq!(
            invalid_reason(0x1020).as_deref(),
            Some("core: blank/crash (0x26 crashes when selected)")
        );
        assert_eq!(
            invalid_reason(0x202D).as_deref(),
            Some("board: crashes opening devices folder")
        );
        assert_eq!(
            invalid_reason(0x2036).as_deref(),
            Some("board: crash, but STRENGTH shows 9999")
        );
        assert_eq!(
            invalid_reason(0x203F).as_deref(),
            Some("board: crash, but WISDOM shows 9999")
        );
        assert_eq!(invalid_reason(0x30B9).as_deref(), Some("mod: blank"));
        assert_eq!(
            invalid_reason(0x3400).as_deref(),
            Some("mod: 'equipped' category byte (game-managed, not a real item)")
        );
        assert_eq!(
            invalid_reason(0x44FF).as_deref(),
            Some("unknown item id 0x44ff")
        );
    }

    #[test]
    fn describing_an_empty_slot_says_so() {
        assert_eq!(describe_item_id(crate::EMPTY), "(empty)");
    }

    #[test]
    fn describing_a_plain_weapon_appends_its_grade() {
        assert_eq!(describe_item_id(0x0000_0000), "Battle Hawk \u{3b1}");
        assert_eq!(describe_item_id(0x0000_0004), "Battle Hawk \u{3b5}");
        assert_eq!(describe_item_id(0x0000_00FA), "Judgment");
    }

    #[test]
    fn describing_a_decorated_item_shows_rarity_and_mods() {
        assert_eq!(
            describe_item_id(0x0005_0000),
            "Battle Hawk \u{3b1} (white +0, 5 mods)"
        );
        // 0x0123_4000 -> base 0x4000, seed 0x012 (18, green), 3 mods.
        assert_eq!(
            describe_item_id(0x0123_4000),
            "[invalid] cat 0x40 0x01234000 (green +18, 3 mods)"
        );
    }

    #[test]
    fn describing_an_unknown_base_id_falls_back_to_its_category_byte() {
        assert_eq!(
            describe_item_id(0x0000_0100),
            "[invalid] cat 0x01 0x00000100"
        );
        assert_eq!(
            describe_item_id(0x0000_4400),
            "[invalid] cat 0x44 0x00004400"
        );
        // The "Weapon" label is unreachable in this branch: category 0x00
        // covers 0x0000..=0x00FF in full, so every weapon id resolves to a
        // catalogue entry before the fallback is consulted.
        assert_eq!(describe_item_id(0x0000_00FF), "Nightmare Lance");
    }
}
