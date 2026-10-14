//! Pure synthesis of a Digimon World 4 save file.
//!
//! Produces a complete 81920-byte save from field values alone, with no
//! template or reference save. Every byte of the 0xA000 block is written from
//! an explicit value or left zero; the block is then duplicated and both
//! checksums are fixed.
//!
//! This mirrors `dw4build.py` field for field, so a synthesised save is
//! byte-identical to the Python builder's for the same input. `tests/builder.rs`
//! proves that against nine Python-generated fixtures.

use crate::codes::{MAX_LEVEL, level_threshold, upcnt_safe_cap};
use crate::flags::{apply_story, preset_by_name};
use crate::name::{NAME_FIELD_LEN, encode_player_name};
use crate::offsets;
use crate::save::fix_checksums;
use crate::species::Species;
use crate::{BLOCK, Difficulty, EMPTY, SAVE_SIZE};

/// The `UNIQUE` field of a synthesised save.
///
/// A real save carries an id that varies between saves (`0x700FFDAC` and
/// `0x6096F82C` were both observed). A synthesised one has no history to
/// derive an id from, so it uses this fixed placeholder.
pub const UNIQUE_DEFAULT: u32 = 0x6096_F82C;

/// Everything needed to synthesise a save.
///
/// `Default` matches the Python builder's `fresh_spec()`: Dorumon, player name
/// `TST`, level 1 everywhere, nine techniques at 1, no items, no bits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveSpec {
    /// Save format version.
    pub version: u32,
    /// `ISUSE`.
    pub isuse: u8,
    /// Per-save id; see [`UNIQUE_DEFAULT`].
    pub unique: u32,

    /// Model stem written to `DIGIMONNAME`, e.g. `p_dorumon`.
    pub digimon_name: String,
    /// Player name, truncated to three fullwidth characters.
    pub player_name: String,
    /// The active species. Keep in step with `digimon_name` via [`SaveSpec::set_species`].
    pub species: Species,

    /// Menu-snapshot level. The game recomputes it on load.
    pub menu_level: u32,
    /// Menu-snapshot HP. Derived; the builder writes a plausible value.
    pub hp: u32,
    /// Menu-snapshot max HP. Derived.
    pub mhp: u32,
    /// Menu-snapshot MP. Derived.
    pub mp: u32,
    /// Menu-snapshot max MP. Derived.
    pub mmp: u32,

    /// The `XDATA` counter.
    pub xdata: u32,
    /// The `BIT` currency.
    pub bit: u32,

    /// Device-folder entries. Empty means every slot is `EMPTY`.
    pub device: Vec<u32>,
    /// Raw disk-folder u32s. Empty means `0x4000 + i`, zero owned.
    pub disk: Vec<u32>,
    /// The 52-byte collected-card bitfield.
    pub card_list: Vec<u8>,
    /// Bank slots. Empty means every slot is `EMPTY`.
    pub bank: Vec<u32>,
    /// Bank balance.
    pub bank_bit: u32,

    /// Weapon slots: indices into the device folder.
    pub weapons: Vec<u32>,
    /// Weapon-mod sockets: indices into the device folder.
    pub weapon_mods: Vec<u32>,
    /// The armor slot: an index into the device folder.
    pub armor: u32,
    /// Armor-mod sockets: indices into the device folder.
    pub armor_mods: Vec<u32>,
    /// The sub slot: an index into the device folder.
    pub sub: u32,

    /// The 1024 `BASE_FLAG` bytes. Empty means all clear.
    pub flags: Vec<u8>,
    /// The 12 `BASE_FLAGFOLDER` bytes. Empty means all clear.
    pub folders: Vec<u8>,
    /// `BASE_COUNTER`. Empty means zeros; `[1]` is the junk-shop total.
    pub counters: Vec<u32>,

    /// `BASE_ISUSE`. `None` marks `species` as the active one.
    pub base_isuse: Option<Vec<u8>>,

    /// Level applied to every species when `base_level` is empty.
    pub level: u32,
    /// EXP applied to every species when `base_exp` is empty.
    pub exp: u32,
    /// Technique level applied to all 144 slots when `base_skill` is empty.
    pub skill: i32,
    /// Power-up value applied to all 176 slots when `base_upcnt` is empty.
    pub upcnt: u32,

    /// Explicit per-species levels, else `[level; 16]`.
    pub base_level: Vec<u32>,
    /// Explicit per-species EXP, else `[exp; 16]`.
    pub base_exp: Vec<u32>,
    /// Explicit 144 technique levels, else `[skill; 144]`.
    pub base_skill: Vec<i32>,
    /// Explicit 176 power-up values, else `[upcnt; 176]`.
    pub base_upcnt: Vec<u32>,
}

impl Default for SaveSpec {
    fn default() -> Self {
        Self {
            version: 4,
            isuse: 1,
            unique: UNIQUE_DEFAULT,
            digimon_name: Species::DEFAULT.model_name(),
            player_name: "TST".to_owned(),
            species: Species::DEFAULT,
            menu_level: 1,
            hp: 170,
            mhp: 170,
            mp: 60,
            mmp: 60,
            xdata: 0,
            bit: 0,
            device: Vec::new(),
            disk: Vec::new(),
            card_list: Vec::new(),
            bank: Vec::new(),
            bank_bit: 0,
            weapons: Vec::new(),
            weapon_mods: Vec::new(),
            armor: EMPTY,
            armor_mods: Vec::new(),
            sub: EMPTY,
            flags: Vec::new(),
            folders: Vec::new(),
            counters: Vec::new(),
            base_isuse: None,
            level: 1,
            exp: 0,
            skill: 1,
            upcnt: 0,
            base_level: Vec::new(),
            base_exp: Vec::new(),
            base_skill: Vec::new(),
            base_upcnt: Vec::new(),
        }
    }
}

impl SaveSpec {
    /// Set the active species and keep `DIGIMONNAME` in step.
    ///
    /// The game derives the species from the model name (`FUN_003f80e0`), so
    /// changing one without the other produces a save that loads as the wrong
    /// Digimon.
    pub fn set_species(&mut self, species: Species) {
        self.species = species;
        self.digimon_name = species.model_name();
    }
}

/// Write `count` u32s, using `values` where present and `empty` beyond.
fn fill_list(buf: &mut [u8], offset: usize, values: &[u32], count: usize, empty: u32) {
    for i in 0..count {
        let value = values.get(i).copied().unwrap_or(empty);
        let at = offset + i * 4;
        buf[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
}

/// Write `data` at `offset`, NUL-padding to `count` bytes and truncating if longer.
fn fill_bytes(buf: &mut [u8], offset: usize, data: &[u8], count: usize) {
    let take = data.len().min(count);
    buf[offset..offset + take].copy_from_slice(&data[..take]);
    buf[offset + take..offset + count].fill(0);
}

/// Build one 0xA000-byte block from a spec.
#[must_use]
pub fn build_block(spec: &SaveSpec) -> Vec<u8> {
    let mut buf = vec![0u8; BLOCK];

    fill_bytes(&mut buf, offsets::VERSION, &spec.version.to_le_bytes(), 4);
    buf[offsets::ISUSE] = spec.isuse;
    fill_bytes(&mut buf, offsets::UNIQUE, &spec.unique.to_le_bytes(), 4);

    fill_bytes(
        &mut buf,
        offsets::DIGIMON_NAME,
        spec.digimon_name.as_bytes(),
        16,
    );
    fill_bytes(
        &mut buf,
        offsets::PLAYER_NAME,
        &encode_player_name(&spec.player_name),
        NAME_FIELD_LEN,
    );

    for (offset, value) in [
        (offsets::MENU_LEVEL, spec.menu_level),
        (offsets::HP, spec.hp),
        (offsets::MHP, spec.mhp),
        (offsets::MP, spec.mp),
        (offsets::MMP, spec.mmp),
        (offsets::XDATA, spec.xdata),
        (offsets::BIT, spec.bit),
    ] {
        fill_bytes(&mut buf, offset, &value.to_le_bytes(), 4);
    }

    fill_list(
        &mut buf,
        offsets::DEVICE,
        &spec.device,
        offsets::DEVICE_SAVE_SLOTS,
        EMPTY,
    );

    // An empty disk vector means the default folder: one entry per disk type,
    // zero owned. A caller-supplied vector is used verbatim (and padded with
    // EMPTY, matching the other containers).
    if spec.disk.is_empty() {
        let disks: Vec<u32> = (0..offsets::DISK_SLOTS as u32)
            .map(|i| 0x4000 + i)
            .collect();
        fill_list(&mut buf, offsets::DISK, &disks, offsets::DISK_SLOTS, EMPTY);
    } else {
        fill_list(
            &mut buf,
            offsets::DISK,
            &spec.disk,
            offsets::DISK_SLOTS,
            EMPTY,
        );
    }

    fill_bytes(&mut buf, offsets::CARD_LIST, &spec.card_list, 52);
    fill_list(
        &mut buf,
        offsets::BANK_DEVICE,
        &spec.bank,
        offsets::BANK_SLOTS,
        EMPTY,
    );
    fill_bytes(&mut buf, offsets::BANK_BIT, &spec.bank_bit.to_le_bytes(), 4);

    fill_list(
        &mut buf,
        offsets::WEAPON,
        &spec.weapons,
        offsets::WEAPON_SLOTS,
        EMPTY,
    );
    fill_list(
        &mut buf,
        offsets::WEAPON_MOD,
        &spec.weapon_mods,
        offsets::MOD_SOCKETS,
        EMPTY,
    );
    fill_bytes(&mut buf, offsets::ARMOR, &spec.armor.to_le_bytes(), 4);
    fill_list(
        &mut buf,
        offsets::ARMOR_MOD,
        &spec.armor_mods,
        offsets::MOD_SOCKETS,
        EMPTY,
    );
    fill_bytes(&mut buf, offsets::SUB, &spec.sub.to_le_bytes(), 4);

    fill_bytes(
        &mut buf,
        offsets::BASE_FLAG,
        &spec.flags,
        offsets::FLAG_COUNT,
    );
    fill_bytes(
        &mut buf,
        offsets::BASE_FLAG_FOLDER,
        &spec.folders,
        offsets::FOLDER_COUNT,
    );
    fill_list(
        &mut buf,
        offsets::BASE_COUNTER,
        &spec.counters,
        offsets::COUNTER_SLOTS,
        0,
    );

    // BASE_ISUSE: mark the active species unless the caller supplied the array.
    match &spec.base_isuse {
        Some(marker) => fill_bytes(
            &mut buf,
            offsets::BASE_ISUSE,
            marker,
            offsets::SPECIES_COUNT,
        ),
        None => {
            let mut marker = vec![0u8; offsets::SPECIES_COUNT];
            marker[spec.species.index()] = 1;
            fill_bytes(
                &mut buf,
                offsets::BASE_ISUSE,
                &marker,
                offsets::SPECIES_COUNT,
            );
        }
    }

    // Per-species tables, each falling back to its scalar default.
    if spec.base_level.is_empty() {
        let levels = vec![spec.level; offsets::SPECIES_COUNT];
        fill_list(
            &mut buf,
            offsets::BASE_LEVEL,
            &levels,
            offsets::SPECIES_COUNT,
            0,
        );
    } else {
        fill_list(
            &mut buf,
            offsets::BASE_LEVEL,
            &spec.base_level,
            offsets::SPECIES_COUNT,
            0,
        );
    }
    if spec.base_exp.is_empty() {
        let exps = vec![spec.exp; offsets::SPECIES_COUNT];
        fill_list(
            &mut buf,
            offsets::BASE_EXP,
            &exps,
            offsets::SPECIES_COUNT,
            0,
        );
    } else {
        fill_list(
            &mut buf,
            offsets::BASE_EXP,
            &spec.base_exp,
            offsets::SPECIES_COUNT,
            0,
        );
    }

    let skill_slots = offsets::SPECIES_COUNT * offsets::TECHNIQUE_SLOTS;
    if spec.base_skill.is_empty() {
        let skills = vec![spec.skill as u32; skill_slots];
        fill_list(&mut buf, offsets::BASE_SKILL, &skills, skill_slots, 0);
    } else {
        let skills: Vec<u32> = spec.base_skill.iter().map(|&v| v as u32).collect();
        fill_list(&mut buf, offsets::BASE_SKILL, &skills, skill_slots, 0);
    }

    let powerup_slots = offsets::SPECIES_COUNT * offsets::POWERUP_SLOTS;
    if spec.base_upcnt.is_empty() {
        let powerups = vec![spec.upcnt; powerup_slots];
        fill_list(&mut buf, offsets::BASE_UPCNT, &powerups, powerup_slots, 0);
    } else {
        fill_list(
            &mut buf,
            offsets::BASE_UPCNT,
            &spec.base_upcnt,
            powerup_slots,
            0,
        );
    }

    buf
}

/// Build a complete 81920-byte save: one block, duplicated, both checksums fixed.
#[must_use]
pub fn build_save(spec: &SaveSpec) -> Vec<u8> {
    let block = build_block(spec);
    let mut out = Vec::with_capacity(SAVE_SIZE);
    out.extend_from_slice(&block);
    out.extend_from_slice(&block);
    fix_checksums(&mut out);
    out
}

/// A fresh save carrying `preset_name`'s story state on `difficulty`.
///
/// `None` if there is no preset with that name. The species, name and every
/// other field keep [`SaveSpec::default`]'s values; set them on the returned
/// spec before building.
#[must_use]
pub fn spec_with_story(preset_name: &str, difficulty: Difficulty) -> Option<SaveSpec> {
    let preset = preset_by_name(preset_name)?;
    let story = apply_story(preset, difficulty);
    let spec = SaveSpec {
        flags: story.flags,
        folders: story.folders,
        ..SaveSpec::default()
    };
    Some(spec)
}

/// A maxed-character save carrying `preset_name`'s story state.
///
/// Sets level and EXP for every species, every technique to 9,999, and every
/// power-up to its per-slot safe cap. `menu_level` is clamped to [`MAX_LEVEL`]
/// because that is all the game displays; EXP follows the curve past it.
///
/// `None` if there is no preset with that name.
#[must_use]
pub fn spec_maxed(preset_name: &str, difficulty: Difficulty, level: u32) -> Option<SaveSpec> {
    let base = spec_with_story(preset_name, difficulty)?;

    // The EXP field is a u32 and the curve overflows it well before the level
    // cap, so truncate rather than saturate: `dw4build.py` writes
    // `level_threshold(level) & 0xFFFFFFFF` through `struct.pack_into`.
    let exp = level_threshold(level) as u64 as u32;

    Some(SaveSpec {
        level,
        exp,
        menu_level: level.min(MAX_LEVEL),
        bit: 9_999_999,
        xdata: 9_999,
        base_level: vec![level; offsets::SPECIES_COUNT],
        base_exp: vec![exp; offsets::SPECIES_COUNT],
        base_skill: vec![9_999; offsets::SPECIES_COUNT * offsets::TECHNIQUE_SLOTS],
        base_upcnt: (0..offsets::SPECIES_COUNT * offsets::POWERUP_SLOTS)
            .map(|i| upcnt_safe_cap(i % offsets::POWERUP_SLOTS) as u32)
            .collect(),
        ..base
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offsets;
    use crate::{BLOCK, SAVE_SIZE, SaveData};

    #[test]
    fn a_default_spec_has_the_documented_header() {
        let save = build_save(&SaveSpec::default());
        assert_eq!(save.len(), SAVE_SIZE);
        let parsed = SaveData::parse(&save).unwrap();
        assert!(parsed.verify(), "the builder must emit valid checksums");
        assert_eq!(parsed.version(), 4);
        assert_eq!(parsed.isuse(), 1);
        assert_eq!(parsed.unique(), UNIQUE_DEFAULT);
        assert_eq!(parsed.digimon_name(), "p_dorumon");
        assert_eq!(parsed.player_name(), "TST");
    }

    #[test]
    fn both_blocks_are_identical() {
        let save = build_save(&SaveSpec::default());
        assert_eq!(save[..BLOCK], save[BLOCK..]);
    }

    #[test]
    fn an_empty_spec_fills_containers_the_way_the_game_expects() {
        let parsed = SaveData::parse(&build_save(&SaveSpec::default())).unwrap();

        for slot in 0..offsets::DEVICE_SAVE_SLOTS {
            assert_eq!(parsed.device(slot), crate::EMPTY, "device slot {slot}");
        }
        for slot in 0..offsets::BANK_SLOTS {
            assert_eq!(parsed.bank_device(slot), crate::EMPTY, "bank slot {slot}");
        }
        for slot in 0..offsets::WEAPON_SLOTS {
            assert_eq!(parsed.weapon(slot), crate::EMPTY, "weapon {slot}");
        }
        assert_eq!(parsed.armor(), crate::EMPTY);
        assert_eq!(parsed.sub(), crate::EMPTY);

        // The disk folder defaults to one entry per disk type, zero owned.
        for disk in 0..offsets::DISK_SLOTS {
            assert_eq!(parsed.disk_raw(disk), 0x4000 + disk as u32);
            assert_eq!(parsed.disk_count(disk), 0);
        }
    }

    #[test]
    fn a_default_spec_marks_the_default_species_active() {
        let parsed = SaveData::parse(&build_save(&SaveSpec::default())).unwrap();
        let marker = parsed.get_bytes(offsets::BASE_ISUSE, offsets::SPECIES_COUNT);
        assert_eq!(
            marker[crate::Species::Dorumon.index()],
            1,
            "Dorumon is active"
        );
        assert_eq!(
            marker.iter().sum::<u8>(),
            1,
            "exactly one species is active"
        );
    }

    #[test]
    fn a_default_spec_gives_every_species_level_one_and_nine_techniques() {
        let parsed = SaveData::parse(&build_save(&SaveSpec::default())).unwrap();
        for species in crate::Species::ALL {
            assert_eq!(parsed.level(species), 1, "{species:?}");
            assert_eq!(parsed.exp(species), 0, "{species:?}");
            for slot in 0..offsets::TECHNIQUE_SLOTS {
                assert_eq!(parsed.skill(species, slot), 1, "{species:?} slot {slot}");
            }
            for slot in 0..offsets::POWERUP_SLOTS {
                assert_eq!(parsed.upcnt(species, slot), 0, "{species:?} slot {slot}");
            }
        }
    }

    #[test]
    fn the_menu_snapshot_defaults_are_plausible() {
        let parsed = SaveData::parse(&build_save(&SaveSpec::default())).unwrap();
        assert_eq!(parsed.menu_level(), 1);
        assert_eq!(parsed.get_u32(offsets::HP), 170);
        assert_eq!(parsed.get_u32(offsets::MHP), 170);
        assert_eq!(parsed.get_u32(offsets::MP), 60);
        assert_eq!(parsed.get_u32(offsets::MMP), 60);
    }

    #[test]
    fn spec_values_reach_the_save() {
        let mut spec = SaveSpec {
            bit: 1_234_567,
            xdata: 42,
            menu_level: 999,
            player_name: "Zz9".to_owned(),
            level: 999,
            base_level: vec![999; 16],
            ..SaveSpec::default()
        };
        spec.set_species(crate::Species::ImperialdramonPm);

        let parsed = SaveData::parse(&build_save(&spec)).unwrap();
        assert_eq!(parsed.bit(), 1_234_567);
        assert_eq!(parsed.xdata(), 42);
        assert_eq!(parsed.menu_level(), 999);
        assert_eq!(
            parsed.digimon_name(),
            "p_impdrapm",
            "set_species keeps the name in step"
        );
        assert_eq!(parsed.player_name(), "Zz9");
        assert_eq!(parsed.detect_species(), crate::Species::ImperialdramonPm);
        assert_eq!(parsed.level(crate::Species::ImperialdramonPm), 999);
        assert_eq!(
            parsed.get_bytes(offsets::BASE_ISUSE, offsets::SPECIES_COUNT)[12],
            1,
            "the active-species marker follows set_species"
        );
    }

    #[test]
    fn short_containers_are_padded_with_the_empty_value() {
        let spec = SaveSpec {
            device: vec![0x050D, 0x0513],
            counters: vec![7],
            ..SaveSpec::default()
        };

        let parsed = SaveData::parse(&build_save(&spec)).unwrap();
        assert_eq!(parsed.device(0), 0x050D);
        assert_eq!(parsed.device(1), 0x0513);
        assert_eq!(parsed.device(2), crate::EMPTY, "padded with EMPTY");
        assert_eq!(parsed.get_u32(offsets::BASE_COUNTER), 7);
        assert_eq!(parsed.junk_counter(), 0, "counters pad with 0, not EMPTY");
    }

    #[test]
    fn explicit_story_bytes_reach_the_save() {
        let mut flags = vec![0u8; offsets::FLAG_COUNT];
        flags[66] = 1;
        flags[707] = 1;
        let spec = SaveSpec {
            flags,
            folders: vec![1u8; offsets::FOLDER_COUNT],
            ..SaveSpec::default()
        };

        let parsed = SaveData::parse(&build_save(&spec)).unwrap();
        assert_eq!(parsed.get_bytes(offsets::BASE_FLAG + 66, 1), &[1]);
        assert_eq!(parsed.get_bytes(offsets::BASE_FLAG + 707, 1), &[1]);
        assert_eq!(parsed.get_bytes(offsets::BASE_FLAG_FOLDER, 12), &[1u8; 12]);
    }

    #[test]
    fn a_negative_technique_survives_the_round_trip() {
        let spec = SaveSpec {
            skill: -1,
            ..SaveSpec::default()
        };
        let parsed = SaveData::parse(&build_save(&spec)).unwrap();
        assert_eq!(parsed.skill(crate::Species::Agumon, 0), -1);
    }

    #[test]
    fn an_unknown_preset_is_not_a_spec() {
        assert!(spec_with_story("nope", crate::Difficulty::Normal).is_none());
        assert!(spec_maxed("nope", crate::Difficulty::Normal, 999).is_none());
    }

    #[test]
    fn a_story_spec_carries_the_presets_flags_and_folders() {
        let spec = spec_with_story("After World 1", crate::Difficulty::Normal).unwrap();
        let parsed = SaveData::parse(&build_save(&spec)).unwrap();

        // Active flags.
        for flag in [0, 2, 66, 67, 701] {
            assert_eq!(
                parsed.get_bytes(offsets::BASE_FLAG + flag, 1),
                &[1],
                "flag {flag}"
            );
        }
        // Their Normal mirrors.
        for mirror in [699usize, 701, 707, 708] {
            assert_eq!(
                parsed.get_bytes(offsets::BASE_FLAG + mirror, 1),
                &[1],
                "mirror {mirror}"
            );
        }
        // Folders and their mirrors.
        for folder in 0..3usize {
            assert_eq!(
                parsed.get_bytes(offsets::BASE_FLAG_FOLDER + folder, 1),
                &[1],
                "folder {folder}"
            );
            assert_eq!(
                parsed.get_bytes(offsets::BASE_FLAG + 518 + folder, 1),
                &[1],
                "folder {folder} mirror"
            );
        }
        // Folders 3-9 stay clear.
        for folder in 3..10usize {
            assert_eq!(
                parsed.get_bytes(offsets::BASE_FLAG_FOLDER + folder, 1),
                &[0],
                "folder {folder} should be clear"
            );
        }
    }

    #[test]
    fn the_difficulty_changes_which_mirrors_are_written() {
        let flag_bytes = |difficulty: crate::Difficulty| {
            let spec = spec_with_story("After World 1", difficulty).unwrap();
            build_save(&spec)[offsets::BASE_FLAG..][..offsets::FLAG_COUNT].to_vec()
        };
        let (n, h, v) = (
            flag_bytes(crate::Difficulty::Normal),
            flag_bytes(crate::Difficulty::Hard),
            flag_bytes(crate::Difficulty::VeryHard),
        );

        // Active flags are identical across difficulties...
        for flag in [0usize, 2, 66, 67, 701] {
            assert_eq!(n[flag], 1, "Normal active {flag}");
            assert_eq!(h[flag], 1, "Hard active {flag}");
            assert_eq!(v[flag], 1, "Very Hard active {flag}");
        }
        // ...but the mirrors are not: each difficulty writes its own column and
        // every lower one, so 66 mirrors to 707 / 707+73 / 707+73+77.
        assert_eq!((n[707], h[73], v[77]), (1, 1, 1));
        assert_eq!((n[73], n[77]), (0, 0), "Normal reaches no harder column");
        assert_eq!(
            (h[707], h[77]),
            (1, 0),
            "Hard fills Normal but not Very Hard"
        );
        assert_eq!((v[707], v[73]), (1, 1), "Very Hard fills the whole stack");

        // And the folder bands differ the same way.
        assert_eq!((n[518], h[530], v[542]), (1, 1, 1));
        assert_eq!((n[530], n[542]), (0, 0), "Normal reaches no harder band");
        assert_eq!((h[518], h[542]), (1, 0), "Hard fills Normal's band only");
        assert_eq!((v[518], v[530]), (1, 1), "Very Hard fills every band");

        // Each save reports the difficulty it was built for.
        assert_eq!(crate::detect_difficulty(&n), crate::Difficulty::Normal);
        assert_eq!(crate::detect_difficulty(&h), crate::Difficulty::Hard);
        assert_eq!(crate::detect_difficulty(&v), crate::Difficulty::VeryHard);
    }

    #[test]
    fn a_maxed_spec_fills_every_per_species_table() {
        let spec = spec_maxed("Fresh (tutorial)", crate::Difficulty::Normal, 999).unwrap();
        let parsed = SaveData::parse(&build_save(&spec)).unwrap();

        assert_eq!(parsed.bit(), 9_999_999);
        assert_eq!(parsed.xdata(), 9_999);
        assert_eq!(parsed.menu_level(), 999);
        for species in crate::Species::ALL {
            assert_eq!(parsed.level(species), 999, "{species:?} level");
            assert_eq!(parsed.exp(species), 1_133_652_152, "{species:?} exp");
            for slot in 0..offsets::TECHNIQUE_SLOTS {
                assert_eq!(
                    parsed.skill(species, slot),
                    9_999,
                    "{species:?} tech {slot}"
                );
            }
            for slot in 0..offsets::POWERUP_SLOTS {
                let want = crate::upcnt_safe_cap(slot) as u32;
                assert_eq!(
                    parsed.upcnt(species, slot),
                    want,
                    "{species:?} power-up {slot}"
                );
            }
        }
    }

    #[test]
    fn the_maxed_menu_level_is_clamped_but_the_exp_is_not() {
        // The game displays at most level 999; EXP follows the curve past it
        // and is truncated into its u32 field, exactly as the Python builder
        // does with `value & 0xFFFFFFFF`.
        let spec = spec_maxed("Fresh (tutorial)", crate::Difficulty::Normal, 2_000).unwrap();
        assert_eq!(spec.menu_level, 999);
        assert_eq!(spec.level, 2_000);

        let parsed = SaveData::parse(&build_save(&spec)).unwrap();
        assert_eq!(parsed.menu_level(), 999);
        assert_eq!(parsed.level(crate::Species::Agumon), 2_000);
    }

    #[test]
    fn a_maxed_fresh_save_still_carries_the_fresh_presets_story_flags() {
        let spec = spec_maxed("Fresh (tutorial)", crate::Difficulty::Normal, 999).unwrap();
        assert_eq!(spec.flags[1], 1, "the tutorial flag");
        assert_eq!(spec.flags[700], 1, "its Normal mirror");
        assert_eq!(spec.flags[960], 1, "lobby flag 960 is not mirrored");
    }
}
