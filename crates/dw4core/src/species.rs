//! The 16 species, in `MODEL_NAME` table order.
//!
//! The game derives the species from the `DIGIMONNAME` string stored in the
//! save (`FUN_003f80e0` linear-searches the model-name table). So the editor
//! must keep the name and the species index in step.
//!
//! Order provenance: the ELF string table at `0x417b88`. Indices 0-3 (the
//! starters) and 12 (`p_impdrapm`) are pinned against real saves; the rest are
//! best-effort, exactly as in the Python editor.

use serde::{Deserialize, Serialize};

/// One of the 16 species. The discriminant **is** the species index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Species {
    Agumon = 0,
    Veemon = 1,
    Girumon = 2,
    Dorumon = 3,
    WereGarurumon = 4,
    HerculesKabuterimon = 5,
    WarGreymon = 6,
    AngelRimon = 7,
    Beelzemon = 8,
    Alphamon = 9,
    BlackWarGreymon = 10,
    ImperialdramonFm = 11,
    ImperialdramonPm = 12,
    MetalGarurumon = 13,
    DukeCrimson = 14,
    Susanoomon = 15,
}

/// Model-name stems, indexed by species. Written into `DIGIMONNAME` as `p_<stem>`.
const MODEL_STEMS: [&str; 16] = [
    "agumon",
    "vmon",
    "girumon",
    "dorumon",
    "weregaru",
    "hekabut",
    "wargrey",
    "angelr",
    "beelzeb",
    "alpha",
    "bwargrey",
    "impdrafm",
    "impdrapm",
    "metalgaru",
    "dukecrim",
    "susanoo",
];

/// Human labels, used for UI display only.
const DISPLAY_NAMES: [&str; 16] = [
    "Agumon",
    "Veemon",
    "Guilmon",
    "Dorumon",
    "WereGarurumon",
    "HerculesKabuterimon",
    "WarGreymon",
    "AngelRimon",
    "Beelzemon",
    "Alphamon",
    "BlackWarGreymon",
    "Imperialdramon FM",
    "Imperialdramon PM",
    "MetalGarurumon",
    "Dukemon (Crimson)",
    "Susanoomon",
];

/// Prefixes seen on `DIGIMONNAME` values in real saves.
const NAME_PREFIXES: [&str; 4] = ["p_", "m_", "q_", "s_"];

impl Species {
    /// Every species, in index order.
    pub const ALL: [Species; 16] = [
        Species::Agumon,
        Species::Veemon,
        Species::Girumon,
        Species::Dorumon,
        Species::WereGarurumon,
        Species::HerculesKabuterimon,
        Species::WarGreymon,
        Species::AngelRimon,
        Species::Beelzemon,
        Species::Alphamon,
        Species::BlackWarGreymon,
        Species::ImperialdramonFm,
        Species::ImperialdramonPm,
        Species::MetalGarurumon,
        Species::DukeCrimson,
        Species::Susanoomon,
    ];

    /// Species used when the stored model name is unrecognised.
    pub const DEFAULT: Species = Species::Dorumon;

    /// The species index, 0-15.
    #[must_use]
    pub fn index(self) -> usize {
        self as usize
    }

    /// The species for an index, or `None` if out of range.
    #[must_use]
    pub fn from_index(index: usize) -> Option<Species> {
        Species::ALL.get(index).copied()
    }

    /// The `DIGIMONNAME` stem, without the `p_` prefix.
    #[must_use]
    pub fn model_stem(self) -> &'static str {
        MODEL_STEMS[self.index()]
    }

    /// The `DIGIMONNAME` value to store, e.g. `p_dorumon`.
    #[must_use]
    pub fn model_name(self) -> String {
        format!("p_{}", self.model_stem())
    }

    /// The human label for this species.
    #[must_use]
    pub fn display(self) -> &'static str {
        DISPLAY_NAMES[self.index()]
    }

    /// The species a stored model name refers to, ignoring any `p_`/`m_`/`q_`/`s_` prefix.
    #[must_use]
    pub fn from_model_name(name: &str) -> Option<Species> {
        let stem = NAME_PREFIXES
            .iter()
            .find_map(|p| name.strip_prefix(p))
            .unwrap_or(name);
        Species::ALL.into_iter().find(|sp| sp.model_stem() == stem)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_species_round_trips_by_index() {
        for (i, sp) in Species::ALL.iter().enumerate() {
            assert_eq!(sp.index(), i);
            assert_eq!(Species::from_index(i), Some(*sp));
        }
        assert_eq!(Species::from_index(16), None);
    }

    #[test]
    fn model_stems_and_display_names_are_unique() {
        let mut stems = std::collections::BTreeSet::new();
        let mut names = std::collections::BTreeSet::new();
        for sp in Species::ALL {
            assert!(
                stems.insert(sp.model_stem()),
                "duplicate stem {}",
                sp.model_stem()
            );
            assert!(
                names.insert(sp.display()),
                "duplicate name {}",
                sp.display()
            );
        }
    }

    #[test]
    fn detection_strips_every_known_prefix() {
        assert_eq!(
            Species::from_model_name("p_dorumon"),
            Some(Species::Dorumon)
        );
        assert_eq!(Species::from_model_name("m_agumon"), Some(Species::Agumon));
        assert_eq!(Species::from_model_name("q_vmon"), Some(Species::Veemon));
        assert_eq!(
            Species::from_model_name("s_girumon"),
            Some(Species::Girumon)
        );
    }

    #[test]
    fn imperialdramon_pm_is_index_twelve() {
        // Verified against a real save: p_impdrapm = 12, whose BASE_UPCNT row
        // carried the tester's in-game upgrades.
        let pm = Species::from_model_name("p_impdrapm").unwrap();
        assert_eq!(pm.index(), 12);
        assert_eq!(pm.display(), "Imperialdramon PM");
    }

    #[test]
    fn an_unknown_stem_is_not_a_species() {
        assert_eq!(Species::from_model_name("p_notadigimon"), None);
        assert_eq!(Species::from_model_name(""), None);
    }

    #[test]
    fn the_four_starters_are_in_the_documented_order() {
        assert_eq!(Species::from_model_name("p_agumon").unwrap().index(), 0);
        assert_eq!(Species::from_model_name("p_vmon").unwrap().index(), 1);
        assert_eq!(Species::from_model_name("p_girumon").unwrap().index(), 2);
        assert_eq!(Species::from_model_name("p_dorumon").unwrap().index(), 3);
    }
}
