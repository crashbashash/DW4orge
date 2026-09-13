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

/// The save's difficulty. Normal is `SYSgetDifficulty() == -1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}
