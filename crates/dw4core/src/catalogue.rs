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
}
