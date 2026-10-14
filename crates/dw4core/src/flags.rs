//! Story flags, folders, and the per-difficulty mirror map.
//!
//! `BASE_FLAG` holds 1024 bytes where `0x01` means "set". The title screen
//! (`entry_0806/chunk_0006`) clears a range and then restores every story flag
//! from a **per-difficulty mirror**, so an edit only survives a load if the
//! mirror byte is written as well.
//!
//! Difficulty is **not stored in the save**. It is inferred by counting which
//! mirror set is live (see [`detect_difficulty`]).
//!
//! Table provenance: corrected retail title-screen restoration
//! (`entry_0806/chunk_0006`) and character-select save (`entry_0810/chunk_0004`).
//! The old Python tables omitted inclusive loop endpoints and included flags
//! that are not mirrored. Tests use independently extracted retail mappings.

use crate::offsets;
use serde::{Deserialize, Serialize};

/// The save's difficulty. Normal is `SYSgetDifficulty() == 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum Difficulty {
    Normal,
    Hard,
    VeryHard,
}

impl Difficulty {
    /// Every difficulty, in the order the Python editor scores them.
    ///
    /// Order is Normal → Hard → Very Hard; `detect_difficulty` keeps the last
    /// maximum, so ties resolve to the **hardest** difficulty (see that
    /// function for why).
    pub const ALL: [Difficulty; 3] = [Difficulty::Normal, Difficulty::Hard, Difficulty::VeryHard];

    /// The display label, matching the Python editor's dropdown.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
            Difficulty::VeryHard => "Very Hard",
        }
    }

    /// Column index into [`Mirror`]'s three mirror fields.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Difficulty::Normal => 0,
            Difficulty::Hard => 1,
            Difficulty::VeryHard => 2,
        }
    }
}

/// One active story flag and the mirror it is restored from, per difficulty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Mirror {
    /// The flag the game reads during play.
    pub active: u32,
    /// Where the active flag is copied from on Normal.
    pub normal: u32,
    /// Where the active flag is copied from on Hard.
    pub hard: u32,
    /// Where the active flag is copied from on Very Hard.
    pub very_hard: u32,
}

impl Mirror {
    /// The mirror flag for `difficulty`.
    #[must_use]
    pub fn mirror_for(self, difficulty: Difficulty) -> u32 {
        match difficulty {
            Difficulty::Normal => self.normal,
            Difficulty::Hard => self.hard,
            Difficulty::VeryHard => self.very_hard,
        }
    }
}

/// The complete active → mirror map, one row per flag. See the module docs.
///
/// The retail scripts use inclusive loops; every endpoint is represented.
/// Kept on one row per line for direct comparison with the extracted oracle.
#[rustfmt::skip]
pub const MIRRORS: &[Mirror] = &[
    Mirror { active: 1, normal: 700, hard: 7, very_hard: 13 },
    Mirror { active: 2, normal: 701, hard: 8, very_hard: 14 },
    Mirror { active: 3, normal: 702, hard: 9, very_hard: 15 },
    Mirror { active: 4, normal: 703, hard: 10, very_hard: 16 },
    Mirror { active: 5, normal: 704, hard: 11, very_hard: 17 },
    Mirror { active: 6, normal: 705, hard: 12, very_hard: 18 },
    Mirror { active: 19, normal: 706, hard: 741, very_hard: 771 },
    Mirror { active: 20, normal: 707, hard: 742, very_hard: 772 },
    Mirror { active: 49, normal: 801, hard: 802, very_hard: 803 },
    Mirror { active: 67, normal: 708, hard: 74, very_hard: 78 },
    Mirror { active: 68, normal: 709, hard: 75, very_hard: 79 },
    Mirror { active: 69, normal: 710, hard: 76, very_hard: 80 },
    Mirror { active: 70, normal: 711, hard: 77, very_hard: 81 },
    Mirror { active: 82, normal: 807, hard: 808, very_hard: 809 },
    Mirror { active: 83, normal: 712, hard: 743, very_hard: 773 },
    Mirror { active: 86, normal: 713, hard: 87, very_hard: 88 },
    Mirror { active: 89, normal: 714, hard: 744, very_hard: 774 },
    Mirror { active: 90, normal: 715, hard: 745, very_hard: 775 },
    Mirror { active: 91, normal: 716, hard: 746, very_hard: 776 },
    Mirror { active: 92, normal: 717, hard: 747, very_hard: 777 },
    Mirror { active: 93, normal: 718, hard: 748, very_hard: 778 },
    Mirror { active: 94, normal: 719, hard: 749, very_hard: 779 },
    Mirror { active: 95, normal: 720, hard: 750, very_hard: 780 },
    Mirror { active: 96, normal: 721, hard: 751, very_hard: 781 },
    Mirror { active: 97, normal: 722, hard: 752, very_hard: 782 },
    Mirror { active: 98, normal: 723, hard: 753, very_hard: 783 },
    Mirror { active: 99, normal: 724, hard: 754, very_hard: 784 },
    Mirror { active: 320, normal: 725, hard: 755, very_hard: 785 },
    Mirror { active: 321, normal: 726, hard: 756, very_hard: 786 },
    Mirror { active: 322, normal: 727, hard: 757, very_hard: 787 },
    Mirror { active: 323, normal: 728, hard: 758, very_hard: 788 },
    Mirror { active: 324, normal: 729, hard: 759, very_hard: 789 },
    Mirror { active: 325, normal: 730, hard: 760, very_hard: 790 },
    Mirror { active: 326, normal: 731, hard: 761, very_hard: 791 },
    Mirror { active: 327, normal: 732, hard: 762, very_hard: 792 },
    Mirror { active: 329, normal: 733, hard: 763, very_hard: 793 },
    Mirror { active: 330, normal: 734, hard: 764, very_hard: 794 },
    Mirror { active: 331, normal: 735, hard: 765, very_hard: 795 },
    Mirror { active: 332, normal: 736, hard: 766, very_hard: 796 },
    Mirror { active: 333, normal: 737, hard: 767, very_hard: 797 },
    Mirror { active: 334, normal: 810, hard: 811, very_hard: 812 },
    Mirror { active: 335, normal: 738, hard: 768, very_hard: 798 },
    Mirror { active: 383, normal: 804, hard: 805, very_hard: 806 },
];

/// First flag of each difficulty's folder-mirror band: 518 / 530 / 542.
pub const FOLDER_MIRROR_BASE: [u32; 3] = [518, 530, 542];

/// How many folders are mirrored. Folder `N` mirrors to `base + N`.
///
/// Folders 0–10 are restored on load. Folder 11 is not restored.
pub const MIRRORED_FOLDERS: usize = 11;

/// The flag that mirrors folder `folder` on `difficulty`, if it has one.
#[must_use]
pub fn folder_mirror(folder: usize, difficulty: Difficulty) -> Option<u32> {
    if folder >= MIRRORED_FOLDERS {
        return None;
    }
    Some(FOLDER_MIRROR_BASE[difficulty.index()] + folder as u32)
}

/// The mirror flag for `active` on `difficulty`, if it is mirrored at all.
#[must_use]
pub fn mirror_of(active: u32, difficulty: Difficulty) -> Option<u32> {
    MIRRORS
        .iter()
        .find(|row| row.active == active)
        .map(|row| row.mirror_for(difficulty))
}

/// Infer the save's difficulty from which mirror set is live.
///
/// Counts how many bytes in each difficulty's mirror set are set and takes the
/// largest. Ties go to the **hardest** entry in [`Difficulty::ALL`], so a save
/// whose columns an editor copy-down filled to equality still names the
/// difficulty the user was working on. An empty or unmirrored save reads as
/// Normal, because no count above zero ever appears.
///
/// The Python editor instead keeps the first (Normal) maximum, which can never
/// name a copy-down-filled save; this is the deliberate divergence in spec 5.8.
///
/// # Panics
///
/// Panics if `flags` is not `FLAG_COUNT` bytes. Callers pass the `BASE_FLAG`
/// slice straight out of a parsed save, which is always that size.
#[must_use]
pub fn detect_difficulty(flags: &[u8]) -> Difficulty {
    assert_eq!(
        flags.len(),
        offsets::FLAG_COUNT,
        "BASE_FLAG is {} bytes",
        offsets::FLAG_COUNT
    );

    let mut best = Difficulty::Normal;
    let mut best_count = 0usize;

    for difficulty in Difficulty::ALL {
        let mut count = MIRRORS
            .iter()
            .filter(|row| flags[row.mirror_for(difficulty) as usize] == 1)
            .count();
        count += (0..MIRRORED_FOLDERS)
            .filter(|&folder| {
                folder_mirror(folder, difficulty).is_some_and(|flag| flags[flag as usize] == 1)
            })
            .count();

        // `>=` walks up to the hardest tied entry; `count > 0` keeps a wholly
        // unmirrored save at the Normal default.
        if count > 0 && count >= best_count {
            best = difficulty;
            best_count = count;
        }
    }

    best
}

/// A flag and the label the UI shows for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FlagLabel {
    /// The flag index.
    pub flag: u32,
    /// Human label.
    pub label: &'static str,
}

/// The intro/chapter selector. Exactly one of 1-6 is set in a normal save.
pub const INTRO_FLAGS: &[FlagLabel] = &[
    FlagLabel {
        flag: 0,
        label: "Conversation state (transient)",
    },
    FlagLabel {
        flag: 1,
        label: "Tutorial (fresh start)",
    },
    FlagLabel {
        flag: 2,
        label: "Chapter: World 1 access",
    },
    FlagLabel {
        flag: 3,
        label: "Chapter: post-W1 (Ophanimon key)",
    },
    FlagLabel {
        flag: 4,
        label: "Chapter: World 2",
    },
    FlagLabel {
        flag: 5,
        label: "Chapter: World 3",
    },
    FlagLabel {
        flag: 6,
        label: "Chapter: World 4",
    },
];

/// Persistent story milestones, separate from the current chapter selector.
#[rustfmt::skip]
pub const CHAPTER_FLAGS: &[FlagLabel] = &[
    FlagLabel { flag: 19, label: "Story milestone 19" },
    FlagLabel { flag: 20, label: "Leomon rescue completed" },
    FlagLabel { flag: 320, label: "Tutorial completed" },
    FlagLabel { flag: 322, label: "Leomon rescue briefing completed" },
    FlagLabel { flag: 326, label: "World 3 briefing completed" },
    FlagLabel { flag: 327, label: "World 4 briefing completed" },
];

/// Boss defeats and the story-cleared markers.
pub const BOSS_FLAGS: &[FlagLabel] = &[
    FlagLabel {
        flag: 67,
        label: "Apocalymon (W1)",
    },
    FlagLabel {
        flag: 68,
        label: "BelialVamdemon (W2)",
    },
    FlagLabel {
        flag: 69,
        label: "Lucemon (W3)",
    },
    FlagLabel {
        flag: 70,
        label: "Mecha Rogue X (final)",
    },
    FlagLabel {
        flag: 83,
        label: "LordKnightmon",
    },
    FlagLabel {
        flag: 86,
        label: "Story cleared (final)",
    },
    FlagLabel {
        flag: 610,
        label: "Final scene (transient)",
    },
];

/// The quest-chain flags.
pub const QUEST_FLAGS: &[FlagLabel] = &[
    FlagLabel {
        flag: 376,
        label: "Quest 1",
    },
    FlagLabel {
        flag: 377,
        label: "Quest 2",
    },
    FlagLabel {
        flag: 378,
        label: "Quest 3",
    },
    FlagLabel {
        flag: 379,
        label: "Quest 4",
    },
    FlagLabel {
        flag: 380,
        label: "Quest 5",
    },
    FlagLabel {
        flag: 381,
        label: "Final gate",
    },
];

/// Tutorial and lobby progress. These are state, not unconditional init bits.
#[rustfmt::skip]
pub const LOBBY_FLAGS: &[FlagLabel] = &[
    FlagLabel { flag: 501, label: "Terminal Area introduction completed" },
    FlagLabel { flag: 502, label: "Commander Room introduction completed" },
    FlagLabel { flag: 503, label: "Practice stage completed" },
    FlagLabel { flag: 504, label: "Tutorial milestone 504" },
    FlagLabel { flag: 505, label: "Tutorial milestone 505" },
    FlagLabel { flag: 506, label: "Tutorial milestone 506" },
    FlagLabel { flag: 507, label: "Tutorial milestone 507" },
];

/// The twelve `BASE_FLAGFOLDER` slots, in order.
pub const FOLDER_LABELS: [&str; 12] = [
    "Humid Cave (W1, Blosso ID)",
    "Cliff Dungeon (W1, Mammothmon ID)",
    "Leomon rescue (W1)",
    "Ophanimon key",
    "Sand Labyrinth (W2, SkullGrey ID)",
    "Ancient Ruins (W2, Scorpio ID)",
    "Gecko Path (W3, Shogun ID)",
    "Vine Tunnel (W3, MRS04 ID)",
    "Vein (W3, Diaboro ID)",
    "Mecha Nest (W4, MRE05 ID)",
    "Electro Mine (W4, LordKnight ID)",
    "Sidequest completion (not restored on load)",
];

/// A complete story state: which flags and folders to set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StoryPreset {
    /// Label shown in the preset dropdown.
    pub name: &'static str,
    /// Active flags to set.
    pub flags: &'static [u32],
    /// Folders to set.
    pub folders: &'static [u32],
}

/// Canonical presets based on the retail debug progression checkpoints.
/// Fresh starts retain only the tutorial selector and the player init bit.
/// Progressed presets include prerequisite tutorial and briefing state; a chapter
/// flag alone is not a complete playable checkpoint.
#[rustfmt::skip]
pub const STORY_PRESETS: &[StoryPreset] = &[
    StoryPreset {
        name: "Fresh (tutorial)",
        flags: &[1, 960],
        folders: &[],
    },
    StoryPreset {
        name: "After World 1",
        flags: &[3, 20, 67, 320, 322, 383, 501, 502, 503, 504, 505, 506, 507],
        folders: &[0, 1, 2, 3],
    },
    StoryPreset {
        name: "After World 2",
        flags: &[5, 20, 50, 67, 68, 82, 320, 321, 322, 323, 324, 325, 326, 329, 330, 331, 332, 335, 383, 501, 502, 503, 504, 505, 506, 507],
        folders: &[0, 1, 2, 3, 4, 5],
    },
    StoryPreset {
        name: "After World 3",
        flags: &[6, 20, 50, 56, 57, 67, 68, 69, 82, 320, 321, 322, 323, 324, 325, 326, 327, 329, 330, 331, 332, 333, 335, 383, 501, 502, 503, 504, 505, 506, 507, 904],
        folders: &[0, 1, 2, 3, 4, 5, 6, 7, 8],
    },
    StoryPreset {
        name: "All worlds + keys",
        flags: &[6, 20, 50, 56, 57, 67, 68, 69, 82, 83, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331, 332, 333, 335, 383, 501, 502, 503, 504, 505, 506, 507, 904],
        folders: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    StoryPreset {
        name: "Story cleared (post-game)",
        flags: &[20, 50, 56, 57, 67, 68, 69, 70, 82, 83, 86, 320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331, 332, 333, 335, 383, 501, 502, 503, 504, 505, 506, 507, 904],
        folders: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
];

/// The preset with this exact name.
#[must_use]
pub fn preset_by_name(name: &str) -> Option<&'static StoryPreset> {
    STORY_PRESETS.iter().find(|p| p.name == name)
}

/// A resolved story state, ready to be written into a save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryState {
    /// The 1024 `BASE_FLAG` bytes.
    pub flags: Vec<u8>,
    /// The 12 `BASE_FLAGFOLDER` bytes.
    pub folders: Vec<u8>,
}

/// Resolve a preset into flag and folder bytes for `difficulty`.
///
/// Writes every active flag **and its difficulty mirror and every lower
/// difficulty's mirror**, because the title screen restores active ← mirror on
/// load and an active-only edit reverts. Copying down matches
/// `Document::apply`: a save played on a harder difficulty still has to satisfy
/// the lower difficulties' unlock state.
///
/// Presets contain active flags only; mirror values are derived from them.
#[must_use]
pub fn apply_story(preset: &StoryPreset, difficulty: Difficulty) -> StoryState {
    let mut flags = vec![0u8; offsets::FLAG_COUNT];
    flags[960] = 1;
    let mut folders = vec![0u8; offsets::FOLDER_COUNT];

    for &flag in preset.flags {
        flags[flag as usize] = 1;
    }
    for &folder in preset.folders {
        folders[folder as usize] = 1;
    }

    // `Difficulty::ALL` is ordered Normal, Hard, Very Hard, so the slice up to
    // and including `difficulty` is that difficulty and everything below it.
    let down_to = &Difficulty::ALL[..=difficulty.index()];
    for row in MIRRORS {
        let active = flags[row.active as usize];
        for target in down_to {
            flags[row.mirror_for(*target) as usize] = active;
        }
    }
    for (folder, &set) in folders.iter().enumerate().take(MIRRORED_FOLDERS) {
        for target in down_to {
            if let Some(mirror) = folder_mirror(folder, *target) {
                flags[mirror as usize] = set;
            }
        }
    }

    StoryState { flags, folders }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offsets;

    #[test]
    fn the_table_has_one_row_per_mirrored_flag() {
        // Inclusive retail loops: 6 + 2 + 4 + 2 + 11 + 8 + 5 + 1 + 4 = 43.
        assert_eq!(MIRRORS.len(), 43);
    }

    #[test]
    fn active_flags_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for row in MIRRORS {
            assert!(
                seen.insert(row.active),
                "active {} appears twice",
                row.active
            );
        }
    }

    #[test]
    fn every_mirror_lands_inside_the_flag_array() {
        for row in MIRRORS {
            for mirror in [row.normal, row.hard, row.very_hard] {
                assert!(
                    (mirror as usize) < offsets::FLAG_COUNT,
                    "mirror {mirror} for active {} is past the flag array",
                    row.active
                );
            }
        }
    }

    #[test]
    fn the_three_difficulty_columns_never_collide() {
        // A Normal mirror and a Hard mirror must be different flags, or the
        // restore would read the same byte for two difficulties.
        let mut normals = std::collections::BTreeSet::new();
        let mut hards = std::collections::BTreeSet::new();
        let mut very_hards = std::collections::BTreeSet::new();
        for row in MIRRORS {
            assert!(normals.insert(row.normal), "normal {} twice", row.normal);
            assert!(hards.insert(row.hard), "hard {} twice", row.hard);
            assert!(
                very_hards.insert(row.very_hard),
                "very hard {} twice",
                row.very_hard
            );
        }
        assert!(normals.is_disjoint(&hards));
        assert!(normals.is_disjoint(&very_hards));
        assert!(hards.is_disjoint(&very_hards));
    }

    #[test]
    fn folders_mirror_into_their_per_difficulty_band() {
        assert_eq!(folder_mirror(0, Difficulty::Normal), Some(518));
        assert_eq!(folder_mirror(9, Difficulty::Normal), Some(527));
        assert_eq!(folder_mirror(0, Difficulty::Hard), Some(530));
        assert_eq!(folder_mirror(9, Difficulty::VeryHard), Some(551));
        // All eleven progression folders are restored; sidequest folder 11 is not.
        assert_eq!(folder_mirror(10, Difficulty::Normal), Some(528));
        assert_eq!(folder_mirror(11, Difficulty::Normal), None);
    }

    #[test]
    fn mirror_lookup_finds_a_flag_or_reports_none() {
        assert_eq!(mirror_of(67, Difficulty::Normal), Some(708));
        assert_eq!(mirror_of(67, Difficulty::Hard), Some(74));
        assert_eq!(mirror_of(67, Difficulty::VeryHard), Some(78));
        assert_eq!(
            mirror_of(24, Difficulty::Normal),
            None,
            "lobby flags are not mirrored"
        );
        assert_eq!(mirror_of(1023, Difficulty::Normal), None);
    }

    #[test]
    fn detection_picks_the_difficulty_whose_mirrors_are_live() {
        // An empty save has no live mirrors anywhere, so nothing beats the
        // Normal default.
        let mut flags = vec![0u8; offsets::FLAG_COUNT];
        assert_eq!(detect_difficulty(&flags), Difficulty::Normal);

        // Each difficulty's own mirror set, in isolation, is detected as that
        // difficulty. The three columns are the same size, so lighting more
        // than one would tie - hence the reset inside the loop.
        for difficulty in Difficulty::ALL {
            flags.fill(0);
            for row in MIRRORS {
                flags[row.mirror_for(difficulty) as usize] = 1;
            }
            assert_eq!(detect_difficulty(&flags), difficulty);
        }
    }

    #[test]
    fn a_tie_between_difficulties_resolves_to_the_harder_one() {
        // Two equal columns cannot say which difficulty is playing, and a
        // copy-down edit fills all three equally. Taking the hardest is the
        // only choice that names the difficulty the user was working on.
        let mut flags = vec![0u8; offsets::FLAG_COUNT];
        for row in MIRRORS {
            flags[row.normal as usize] = 1;
            flags[row.hard as usize] = 1;
        }
        assert_eq!(detect_difficulty(&flags), Difficulty::Hard);

        // Normal + Very Hard ties too, and the hardest entry wins.
        let mut flags = vec![0u8; offsets::FLAG_COUNT];
        for row in MIRRORS {
            flags[row.normal as usize] = 1;
            flags[row.very_hard as usize] = 1;
        }
        assert_eq!(detect_difficulty(&flags), Difficulty::VeryHard);

        // All three equal, as after an editor copy-down.
        let mut flags = vec![0u8; offsets::FLAG_COUNT];
        for row in MIRRORS {
            for mirror in [row.normal, row.hard, row.very_hard] {
                flags[mirror as usize] = 1;
            }
        }
        assert_eq!(detect_difficulty(&flags), Difficulty::VeryHard);
    }

    #[test]
    fn detection_reads_the_real_card() {
        let raw = std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/mcd001/save.raw"),
        )
        .expect("the plan-1 fixture");
        let flags = &raw[offsets::BASE_FLAG..offsets::BASE_FLAG + offsets::FLAG_COUNT];
        // The fixture is the human's Normal-difficulty save.
        assert_eq!(detect_difficulty(flags), Difficulty::Normal);
    }

    #[test]
    fn there_are_six_presets_with_unique_names() {
        assert_eq!(STORY_PRESETS.len(), 6);
        let mut names = std::collections::BTreeSet::new();
        for p in STORY_PRESETS {
            assert!(names.insert(p.name), "duplicate preset {}", p.name);
        }
        assert_eq!(
            preset_by_name("Fresh (tutorial)").map(|p| p.name),
            Some("Fresh (tutorial)")
        );
        assert!(preset_by_name("nope").is_none());
    }

    #[test]
    fn every_labelled_flag_is_in_range() {
        for section in [
            INTRO_FLAGS,
            CHAPTER_FLAGS,
            BOSS_FLAGS,
            QUEST_FLAGS,
            LOBBY_FLAGS,
        ] {
            for entry in section {
                assert!(
                    (entry.flag as usize) < offsets::FLAG_COUNT,
                    "flag {} is past the array",
                    entry.flag
                );
                assert!(!entry.label.is_empty(), "flag {} has no label", entry.flag);
            }
        }
        assert_eq!(FOLDER_LABELS.len(), offsets::FOLDER_COUNT);
    }

    #[test]
    fn an_empty_preset_leaves_everything_clear() {
        let empty = StoryPreset {
            name: "empty",
            flags: &[],
            folders: &[],
        };
        let state = apply_story(&empty, Difficulty::Normal);
        assert!(
            state
                .flags
                .iter()
                .enumerate()
                .all(|(i, &b)| b == u8::from(i == 960))
        );
        assert!(state.folders.iter().all(|&b| b == 0));
    }

    #[test]
    fn a_preset_writes_both_the_active_flag_and_its_mirror() {
        // The core rule: the active flag alone reverts on load.
        let only_intro = StoryPreset {
            name: "intro",
            flags: &[1],
            folders: &[],
        };
        for difficulty in Difficulty::ALL {
            let state = apply_story(&only_intro, difficulty);
            assert_eq!(state.flags[1], 1, "{difficulty:?} active");
            assert_eq!(
                state.flags[mirror_of(1, difficulty).unwrap() as usize],
                1,
                "{difficulty:?} mirror"
            );
        }
    }

    #[test]
    fn a_preset_copies_down_to_the_lower_difficulties() {
        let only_intro = StoryPreset {
            name: "intro",
            flags: &[1],
            folders: &[0],
        };

        let very_hard = apply_story(&only_intro, Difficulty::VeryHard);
        assert_eq!(very_hard.flags[13], 1, "Very Hard mirror");
        assert_eq!(very_hard.flags[7], 1, "Hard mirror");
        assert_eq!(very_hard.flags[700], 1, "Normal mirror");
        assert_eq!(very_hard.flags[542], 1, "Very Hard folder mirror");
        assert_eq!(very_hard.flags[530], 1, "Hard folder mirror");
        assert_eq!(very_hard.flags[518], 1, "Normal folder mirror");

        let normal = apply_story(&only_intro, Difficulty::Normal);
        assert_eq!(normal.flags[700], 1, "Normal mirror");
        assert_eq!(normal.flags[7], 0, "Normal reaches no harder column");
        assert_eq!(normal.flags[13], 0, "Normal reaches no harder column");
    }

    #[test]
    fn folders_mirror_into_the_flag_array() {
        let with_folders = StoryPreset {
            name: "f",
            flags: &[],
            folders: &[0, 9],
        };
        let state = apply_story(&with_folders, Difficulty::Normal);
        assert_eq!(&state.folders[..10], &[1, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(state.flags[518], 1, "folder 0 mirror");
        assert_eq!(state.flags[527], 1, "folder 9 mirror");
    }

    #[test]
    fn the_fresh_preset_matches_the_documented_flag_set() {
        let fresh = preset_by_name("Fresh (tutorial)").unwrap();
        assert_eq!(fresh.flags, &[1, 960]);
        assert!(fresh.folders.is_empty());

        let state = apply_story(fresh, Difficulty::Normal);
        assert_eq!(state.flags[1], 1);
        assert_eq!(state.flags[700], 1, "flag 1's Normal mirror");
        assert_eq!(state.flags[960], 1, "player initialization bit");
    }

    #[test]
    fn every_preset_flag_is_a_real_flag_index() {
        for p in STORY_PRESETS {
            for &f in p.flags {
                assert!((f as usize) < offsets::FLAG_COUNT, "{}: flag {f}", p.name);
            }
            for &f in p.folders {
                assert!(
                    (f as usize) < offsets::FOLDER_COUNT,
                    "{}: folder {f}",
                    p.name
                );
            }
        }
    }
}
