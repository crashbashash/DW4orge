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
//! Table provenance: `Decomp/DW4/FLAG_MAP.md` §7, derived from diffs of real
//! in-game checkpoint saves. It is the union of the Python editor's two partial
//! tables and supersedes both: the GUI's `FLAG_MIRRORS` knows 10 flags per
//! difficulty, and `dw4build.NORMAL_FLAG_MIRRORS` knows all 38 for Normal only.
//! Our Normal column is byte-equal to the builder's map, and the GUI's tables
//! are strict subsets of ours — both verified against the Python sources.

use crate::offsets;
use serde::{Deserialize, Serialize};

/// The save's difficulty. Normal is `SYSgetDifficulty() == -1`.
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
    /// Order matters: `detect_difficulty` keeps the first maximum, so ties
    /// resolve to Normal, exactly as the Python editor's strict `>` does.
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
/// Laid out one row per line on purpose: this is the table from `FLAG_MAP.md`
/// §7, and `rustfmt` would put each row on four lines.
#[rustfmt::skip]
pub const MIRRORS: &[Mirror] = &[
    // intro 0-4
    Mirror { active: 0,   normal: 699, hard: 6,   very_hard: 12 },
    Mirror { active: 1,   normal: 700, hard: 7,   very_hard: 13 },
    Mirror { active: 2,   normal: 701, hard: 8,   very_hard: 14 },
    Mirror { active: 3,   normal: 702, hard: 9,   very_hard: 15 },
    Mirror { active: 4,   normal: 703, hard: 10,  very_hard: 16 },
    // 18-19
    Mirror { active: 18,  normal: 705, hard: 740, very_hard: 770 },
    Mirror { active: 19,  normal: 706, hard: 741, very_hard: 771 },
    // bosses 66-68
    Mirror { active: 66,  normal: 707, hard: 73,  very_hard: 77 },
    Mirror { active: 67,  normal: 708, hard: 74,  very_hard: 78 },
    Mirror { active: 68,  normal: 709, hard: 75,  very_hard: 79 },
    // 82, 85
    Mirror { active: 82,  normal: 711, hard: 742, very_hard: 772 },
    Mirror { active: 85,  normal: 712, hard: 86,  very_hard: 87 },
    // 88-97
    Mirror { active: 88,  normal: 713, hard: 743, very_hard: 773 },
    Mirror { active: 89,  normal: 714, hard: 744, very_hard: 774 },
    Mirror { active: 90,  normal: 715, hard: 745, very_hard: 775 },
    Mirror { active: 91,  normal: 716, hard: 746, very_hard: 776 },
    Mirror { active: 92,  normal: 717, hard: 747, very_hard: 777 },
    Mirror { active: 93,  normal: 718, hard: 748, very_hard: 778 },
    Mirror { active: 94,  normal: 719, hard: 749, very_hard: 779 },
    Mirror { active: 95,  normal: 720, hard: 750, very_hard: 780 },
    Mirror { active: 96,  normal: 721, hard: 751, very_hard: 781 },
    Mirror { active: 97,  normal: 722, hard: 752, very_hard: 782 },
    // 319-325
    Mirror { active: 319, normal: 724, hard: 754, very_hard: 784 },
    Mirror { active: 320, normal: 725, hard: 755, very_hard: 785 },
    Mirror { active: 321, normal: 726, hard: 756, very_hard: 786 },
    Mirror { active: 322, normal: 727, hard: 757, very_hard: 787 },
    Mirror { active: 323, normal: 728, hard: 758, very_hard: 788 },
    Mirror { active: 324, normal: 729, hard: 759, very_hard: 789 },
    Mirror { active: 325, normal: 730, hard: 760, very_hard: 790 },
    // 328-331
    Mirror { active: 328, normal: 732, hard: 762, very_hard: 792 },
    Mirror { active: 329, normal: 733, hard: 763, very_hard: 793 },
    Mirror { active: 330, normal: 734, hard: 764, very_hard: 794 },
    Mirror { active: 331, normal: 735, hard: 765, very_hard: 795 },
    // 334
    Mirror { active: 334, normal: 737, hard: 767, very_hard: 797 },
    // the four singles
    Mirror { active: 48,  normal: 800, hard: 801, very_hard: 802 },
    Mirror { active: 382, normal: 803, hard: 804, very_hard: 805 },
    Mirror { active: 81,  normal: 806, hard: 807, very_hard: 808 },
    Mirror { active: 333, normal: 809, hard: 810, very_hard: 811 },
];

/// First flag of each difficulty's folder-mirror band: 518 / 530 / 542.
pub const FOLDER_MIRROR_BASE: [u32; 3] = [518, 530, 542];

/// How many folders are mirrored. Folder `N` mirrors to `base + N`.
///
/// Folders 10 and 11 exist in `BASE_FLAGFOLDER` but have no mirror.
pub const MIRRORED_FOLDERS: usize = 10;

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
/// largest. Ties go to the earlier entry in [`Difficulty::ALL`], so an empty or
/// unmirrored save reads as Normal — the same tie-break as the Python editor.
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

        if count > best_count {
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
        label: "Idle (no active chapter - locked if 1-6 are all off)",
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

/// Era markers. These overlap the intro mirrors deliberately.
pub const CHAPTER_FLAGS: &[FlagLabel] = &[
    FlagLabel {
        flag: 701,
        label: "World 1 era",
    },
    FlagLabel {
        flag: 702,
        label: "Post-W1 (Ophanimon key)",
    },
    FlagLabel {
        flag: 703,
        label: "World 2 era",
    },
    FlagLabel {
        flag: 704,
        label: "World 3 era",
    },
    FlagLabel {
        flag: 705,
        label: "World 4 era",
    },
    FlagLabel {
        flag: 706,
        label: "Post-W4",
    },
];

/// Boss defeats and the story-cleared markers.
pub const BOSS_FLAGS: &[FlagLabel] = &[
    FlagLabel {
        flag: 66,
        label: "Apocalymon (W1)",
    },
    FlagLabel {
        flag: 67,
        label: "BelialVamdemon (W1)",
    },
    FlagLabel {
        flag: 68,
        label: "Lucemon (W2)",
    },
    FlagLabel {
        flag: 69,
        label: "Devimon (W3)",
    },
    FlagLabel {
        flag: 82,
        label: "LordKnightmon",
    },
    FlagLabel {
        flag: 85,
        label: "Story cleared (final)",
    },
    FlagLabel {
        flag: 609,
        label: "Final scene",
    },
];

/// The quest-chain flags.
pub const QUEST_FLAGS: &[FlagLabel] = &[
    FlagLabel {
        flag: 375,
        label: "Quest 1",
    },
    FlagLabel {
        flag: 376,
        label: "Quest 2",
    },
    FlagLabel {
        flag: 377,
        label: "Quest 3",
    },
    FlagLabel {
        flag: 378,
        label: "Quest 4",
    },
    FlagLabel {
        flag: 379,
        label: "Quest 5",
    },
    FlagLabel {
        flag: 380,
        label: "Final gate",
    },
];

/// Always-on lobby and system flags, set at new game and never cleared.
pub const LOBBY_FLAGS: &[FlagLabel] = &[
    FlagLabel {
        flag: 24,
        label: "Lobby flag 24 (first-visit)",
    },
    FlagLabel {
        flag: 501,
        label: "Lobby flag 501 (save state)",
    },
    FlagLabel {
        flag: 508,
        label: "Lobby flag 508 (early state)",
    },
    FlagLabel {
        flag: 931,
        label: "Lobby flag 931 (quest init)",
    },
    FlagLabel {
        flag: 935,
        label: "Lobby flag 935 (quest init)",
    },
    FlagLabel {
        flag: 936,
        label: "Lobby flag 936 (quest init)",
    },
    FlagLabel {
        flag: 960,
        label: "Lobby flag 960 (system init)",
    },
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
    "(unused)",
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

/// The canonical story presets.
///
/// Both Python sources — the GUI's `STORY_PRESETS` and `dw4build.STORY_PRESETS`
/// — agree on all six of these, flag for flag, so there is no inconsistency to
/// resolve. Keeping one list here means the Story tab and the New Save dialog
/// cannot drift apart, and `tests/flags.rs` pins every entry against the Python
/// values.
#[rustfmt::skip]
pub const STORY_PRESETS: &[StoryPreset] = &[
    StoryPreset {
        name: "Fresh (tutorial)",
        flags: &[1, 24, 501, 508, 931, 935, 936, 960],
        folders: &[],
    },
    StoryPreset {
        name: "After World 1",
        flags: &[0, 2, 66, 67, 701],
        folders: &[0, 1, 2],
    },
    StoryPreset {
        name: "After World 2",
        flags: &[0, 4, 66, 67, 68, 703],
        folders: &[0, 1, 2, 3, 4, 5],
    },
    StoryPreset {
        name: "After World 3",
        flags: &[0, 5, 67, 68, 69, 82, 704],
        folders: &[0, 1, 2, 3, 4, 5, 6, 7, 8],
    },
    StoryPreset {
        name: "All worlds + keys",
        flags: &[0, 6, 67, 68, 69, 82, 705],
        folders: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    StoryPreset {
        name: "Story cleared (post-game)",
        flags: &[0, 67, 68, 69, 82, 85, 609],
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
/// Writes every active flag **and its difficulty mirror**, because the title
/// screen restores active ← mirror on load and an active-only edit reverts.
///
/// Explicit preset flags are re-applied last: some of them (701, 703, 705) are
/// also mirror targets, and must keep the value the preset asked for. The
/// Python builder orders its writes the same way.
#[must_use]
pub fn apply_story(preset: &StoryPreset, difficulty: Difficulty) -> StoryState {
    let mut flags = vec![0u8; offsets::FLAG_COUNT];
    let mut folders = vec![0u8; offsets::FOLDER_COUNT];

    for &flag in preset.flags {
        flags[flag as usize] = 1;
    }
    for &folder in preset.folders {
        folders[folder as usize] = 1;
    }

    // Mirror the active state for this save's difficulty.
    for row in MIRRORS {
        flags[row.mirror_for(difficulty) as usize] = flags[row.active as usize];
    }
    for (folder, &set) in folders.iter().enumerate().take(MIRRORED_FOLDERS) {
        if let Some(mirror) = folder_mirror(folder, difficulty) {
            flags[mirror as usize] = set;
        }
    }

    // Re-apply, so an explicit flag that is also a mirror target keeps its value.
    for &flag in preset.flags {
        flags[flag as usize] = 1;
    }

    StoryState { flags, folders }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offsets;

    #[test]
    fn the_table_has_one_row_per_mirrored_flag() {
        // 5 intro + 2 + 3 bosses + 2 + 10 + 7 + 4 + 1 + 4 singles = 38.
        assert_eq!(MIRRORS.len(), 38);
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
    fn the_documented_rows_are_present_and_correct() {
        let find = |active: u32| *MIRRORS.iter().find(|r| r.active == active).expect("row");

        // Intro 0-4 -> 699-703 / 6-10 / 12-16.
        let intro = find(0);
        assert_eq!((intro.normal, intro.hard, intro.very_hard), (699, 6, 12));
        let intro4 = find(4);
        assert_eq!(
            (intro4.normal, intro4.hard, intro4.very_hard),
            (703, 10, 16)
        );

        // 18-19.
        let eighteen = find(18);
        assert_eq!(
            (eighteen.normal, eighteen.hard, eighteen.very_hard),
            (705, 740, 770)
        );

        // The bosses and the final-clear flag.
        let apoc = find(66);
        assert_eq!((apoc.normal, apoc.hard, apoc.very_hard), (707, 73, 77));
        let cleared = find(85);
        assert_eq!(
            (cleared.normal, cleared.hard, cleared.very_hard),
            (712, 86, 87)
        );

        // A range row at both ends, and the four singles.
        let q = find(88);
        assert_eq!((q.normal, q.hard, q.very_hard), (713, 743, 773));
        let late = find(97);
        assert_eq!((late.normal, late.hard, late.very_hard), (722, 752, 782));
        let range319 = find(319);
        assert_eq!(
            (range319.normal, range319.hard, range319.very_hard),
            (724, 754, 784)
        );
        let range325 = find(325);
        assert_eq!(
            (range325.normal, range325.hard, range325.very_hard),
            (730, 760, 790)
        );
        let range328 = find(328);
        assert_eq!(
            (range328.normal, range328.hard, range328.very_hard),
            (732, 762, 792)
        );
        let range331 = find(331);
        assert_eq!(
            (range331.normal, range331.hard, range331.very_hard),
            (735, 765, 795)
        );
        let thirty_four = find(334);
        assert_eq!(
            (thirty_four.normal, thirty_four.hard, thirty_four.very_hard),
            (737, 767, 797)
        );

        for (active, normal, hard, very_hard) in [
            (48, 800, 801, 802),
            (382, 803, 804, 805),
            (81, 806, 807, 808),
            (333, 809, 810, 811),
        ] {
            let row = find(active);
            assert_eq!(
                (row.normal, row.hard, row.very_hard),
                (normal, hard, very_hard),
                "active {active}"
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
        // Only ten folders are mirrored; 10 and 11 are not.
        assert_eq!(folder_mirror(10, Difficulty::Normal), None);
        assert_eq!(folder_mirror(11, Difficulty::Normal), None);
    }

    #[test]
    fn mirror_lookup_finds_a_flag_or_reports_none() {
        assert_eq!(mirror_of(66, Difficulty::Normal), Some(707));
        assert_eq!(mirror_of(66, Difficulty::Hard), Some(73));
        assert_eq!(mirror_of(66, Difficulty::VeryHard), Some(77));
        assert_eq!(
            mirror_of(24, Difficulty::Normal),
            None,
            "lobby flags are not mirrored"
        );
        assert_eq!(mirror_of(1023, Difficulty::Normal), None);
    }

    #[test]
    fn detection_picks_the_difficulty_whose_mirrors_are_live() {
        // An empty save has no live mirrors anywhere; ties go to Normal.
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
    fn a_tie_between_two_difficulties_resolves_to_normal() {
        // All three columns hold 38 flags, so lighting two of them ties. The
        // Python editor keeps the first maximum in Normal/Hard/Very Hard
        // order, and so do we.
        let mut flags = vec![0u8; offsets::FLAG_COUNT];
        for row in MIRRORS {
            flags[row.normal as usize] = 1;
            flags[row.hard as usize] = 1;
        }
        assert_eq!(detect_difficulty(&flags), Difficulty::Normal);

        // Normal + Very Hard ties too.
        let mut flags = vec![0u8; offsets::FLAG_COUNT];
        for row in MIRRORS {
            flags[row.normal as usize] = 1;
            flags[row.very_hard as usize] = 1;
        }
        assert_eq!(detect_difficulty(&flags), Difficulty::Normal);
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
        assert!(state.flags.iter().all(|&b| b == 0));
        assert!(state.folders.iter().all(|&b| b == 0));
    }

    #[test]
    fn a_preset_writes_both_the_active_flag_and_its_mirror() {
        // The core rule: the active flag alone reverts on load.
        let only_intro = StoryPreset {
            name: "intro",
            flags: &[0],
            folders: &[],
        };
        for difficulty in Difficulty::ALL {
            let state = apply_story(&only_intro, difficulty);
            assert_eq!(state.flags[0], 1, "{difficulty:?} active");
            assert_eq!(
                state.flags[mirror_of(0, difficulty).unwrap() as usize],
                1,
                "{difficulty:?} mirror"
            );
        }
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
    fn explicit_preset_flags_win_over_mirror_writes() {
        // 701 is both an intro mirror (of flag 2) and the "World 1 era" chapter
        // flag. The Python builder re-applies explicit flags last for exactly
        // this reason, and so must we.
        let both = StoryPreset {
            name: "both",
            flags: &[701],
            folders: &[],
        };
        let state = apply_story(&both, Difficulty::Normal);
        assert_eq!(state.flags[701], 1, "the explicit chapter flag survives");
    }

    #[test]
    fn the_fresh_preset_matches_the_documented_flag_set() {
        let fresh = preset_by_name("Fresh (tutorial)").unwrap();
        assert_eq!(fresh.flags, &[1, 24, 501, 508, 931, 935, 936, 960]);
        assert!(fresh.folders.is_empty());

        let state = apply_story(fresh, Difficulty::Normal);
        assert_eq!(state.flags[1], 1);
        assert_eq!(state.flags[700], 1, "flag 1's Normal mirror");
        assert_eq!(state.flags[24], 1, "lobby flag 24 is not mirrored");
    }

    #[test]
    fn the_presets_match_the_python_values_exactly() {
        // Both Python sources agree on all six presets, so these are pinned
        // literally. Flag 66 is Apocalymon (W1) and belongs only to the two
        // post-W1 presets; flag 609 is the final scene and belongs only to
        // the post-game one.
        let expected: [(&str, &[u32], &[u32]); 6] = [
            (
                "Fresh (tutorial)",
                &[1, 24, 501, 508, 931, 935, 936, 960],
                &[],
            ),
            ("After World 1", &[0, 2, 66, 67, 701], &[0, 1, 2]),
            (
                "After World 2",
                &[0, 4, 66, 67, 68, 703],
                &[0, 1, 2, 3, 4, 5],
            ),
            (
                "After World 3",
                &[0, 5, 67, 68, 69, 82, 704],
                &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            ),
            (
                "All worlds + keys",
                &[0, 6, 67, 68, 69, 82, 705],
                &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            ),
            (
                "Story cleared (post-game)",
                &[0, 67, 68, 69, 82, 85, 609],
                &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            ),
        ];

        assert_eq!(STORY_PRESETS.len(), expected.len());
        for (preset, (name, flags, folders)) in STORY_PRESETS.iter().zip(expected) {
            assert_eq!(preset.name, name);
            assert_eq!(preset.flags, flags, "{name} flags");
            assert_eq!(preset.folders, folders, "{name} folders");
        }
    }

    #[test]
    fn flag_66_and_609_appear_only_where_they_belong() {
        let with = |flag: u32| -> Vec<&str> {
            STORY_PRESETS
                .iter()
                .filter(|p| p.flags.contains(&flag))
                .map(|p| p.name)
                .collect()
        };
        assert_eq!(with(66), vec!["After World 1", "After World 2"]);
        assert_eq!(with(609), vec!["Story cleared (post-game)"]);
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
