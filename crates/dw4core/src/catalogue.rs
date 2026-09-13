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
#[derive(Debug, Clone, PartialEq, Eq)]
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

        Self { items }
    }
}

/// The process-wide catalogue, built once on first use.
#[must_use]
pub fn get_catalogue() -> &'static ItemCatalogue {
    static CATALOGUE: OnceLock<ItemCatalogue> = OnceLock::new();
    CATALOGUE.get_or_init(ItemCatalogue::load_from_embedded)
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
}
