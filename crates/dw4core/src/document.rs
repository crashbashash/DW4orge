//! The editable projection of a parsed save: [`SaveView`], [`EditSet`],
//! validation and application.
//!
//! The Python editor is the only known-good implementation. Its edit path is
//! split: `dw4save.SaveData` is runnable as an oracle, but `collect()` and
//! `apply()` live in `save_editor_gui.py`, which imports `tkinter` and cannot
//! be run here. Byte-level semantics are therefore verified against the real
//! `SaveData`, and the *ordering* of `apply` is transcribed from the GUI source
//! (`save_editor_gui.py:1197`) and pinned by the tests in
//! `tests/document_parity.rs`.

use serde::{Deserialize, Serialize};

/// How strictly edits are checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// In-game limits enforced; crash-causing item ids rejected.
    Normal,
    /// Guard-rails off. Values may use the full data-type range and item ids
    /// may be glitch/crash ids.
    Advanced,
}

impl Mode {
    /// Whether this is [`Mode::Advanced`].
    #[must_use]
    pub const fn is_advanced(self) -> bool {
        matches!(self, Mode::Advanced)
    }
}

/// How serious a reported problem is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The edit was rejected and nothing was written.
    Error,
    /// The edit was applied but may behave oddly in-game.
    Warning,
}

/// One problem with one field.
///
/// `path` is the key the frontend uses to highlight the exact control:
/// `"bit"`, `"device[3].mods"`, `"equip.armor"`, `"story.flag[66]"`,
/// `"bank_items[7]"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FieldError {
    /// Where the problem is.
    pub path: String,
    /// What is wrong, in a form fit to show the user.
    pub message: String,
    /// Whether this blocked the edit.
    pub severity: Severity,
}

impl FieldError {
    /// A problem that rejected the edit.
    #[must_use]
    pub fn error(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
            severity: Severity::Error,
        }
    }

    /// A problem that did not block the edit.
    #[must_use]
    pub fn warning(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
            severity: Severity::Warning,
        }
    }
}

/// A non-blocking report. Same shape as [`FieldError`]; the severity is always
/// [`Severity::Warning`].
pub type Warning = FieldError;

use crate::EMPTY;
use crate::catalogue::{category_label, invalid_reason};
use crate::codes::{
    CAP_BIT, CAP_DISK_COUNT, CAP_EXP, CAP_LEVEL, CAP_TECH, CAP_UPCNT, CAP_XDATA, Cap,
    POWERUP_STATS, SEED_BONUS_MASK, TECHNIQUES, junk_tier_from_counter, level_threshold,
    upcnt_safe_cap,
};
use crate::flags::{Difficulty, MIRRORED_FOLDERS, MIRRORS, detect_difficulty, folder_mirror};
use crate::item::{Category, build_item_id, category_of, split_item_id};
use crate::offsets::{BANK_SLOTS, DEVICE_SAVE_SLOTS, DEVICE_SLOTS, FLAG_COUNT, FOLDER_COUNT};
use crate::save::SaveData;
use crate::species::Species;
use std::path::{Path, PathBuf};

/// Which of the two story fields an edit targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum StoryKind {
    /// A `BASE_FLAG` byte.
    Flag,
    /// A `BASE_FLAGFOLDER` byte.
    Folder,
}

/// One story bit to write.
///
/// Mirrors one entry of the Python `collect()` story dict, which is keyed by
/// `(kind, index)` and valued `0`/`1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StoryEdit {
    /// Which field this targets.
    pub kind: StoryKind,
    /// Flag index (0-1023) or folder index (0-11).
    pub index: u16,
    /// The value to write.
    pub value: bool,
}

impl StoryEdit {
    /// An edit to `BASE_FLAG[index]`.
    #[must_use]
    pub fn flag(index: u16, value: bool) -> Self {
        Self {
            kind: StoryKind::Flag,
            index,
            value,
        }
    }

    /// An edit to `BASE_FLAGFOLDER[index]`.
    #[must_use]
    pub fn folder(index: u16, value: bool) -> Self {
        Self {
            kind: StoryKind::Folder,
            index,
            value,
        }
    }
}

/// Which difficulty's mirror set to write alongside a story edit.
///
/// The save does not store its difficulty, so `Auto` asks the editor to infer
/// it (spec 4.5). This is the `difficulty_var` of the Python GUI, whose default
/// there is `"auto"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum DifficultyChoice {
    /// Infer from which mirror set is live.
    #[default]
    Auto,
    /// Use a difficulty chosen by the user.
    Fixed(Difficulty),
}

/// Everything the UI can change, in one payload.
///
/// Mirrors the Python `collect()` dictionary field-for-field, minus the raw
/// offset edits that DW4orge drops by design (spec 5.3).
///
/// Two field groups are *not* plain values:
///
/// - `weapons`, `armor` and `sub` are **indices into `device`**, exactly as the
///   Python `_parse_equip` returns them.
/// - `wmods` and `amods` are **mod-chip base ids** (`None` = empty socket), not
///   indices. The Python editor resolves them to indices during `collect()`,
///   adding the chip to the device folder if absent; doing that at apply time
///   instead means a socket cannot carry a stale index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EditSet {
    /// Species to switch to. Rewrites `DIGIMONNAME` when it changes.
    pub species: Species,
    /// Player name, at most 8 fullwidth characters.
    pub name: String,
    /// Currency.
    pub bit: u32,
    /// X-Data currency.
    pub xdata: u32,
    /// Junk-shop donation **threshold** (not a tier number), as `collect()`
    /// writes `dict(JUNK_TIERS)[tier]`.
    pub junk: u32,
    /// Level for the edited species.
    pub level: u32,
    /// EXP for the edited species.
    pub exp: u32,
    /// The 9 technique values, signed.
    pub tech: [i32; 9],
    /// The 11 X-Data power-up values.
    pub upcnt: [u32; 11],
    /// The 30 usable device-folder slots.
    pub device: [u32; DEVICE_SLOTS],
    /// 3 weapon slots, as indices into `device`.
    pub weapons: [u32; 3],
    /// Armor slot, as an index into `device`.
    pub armor: u32,
    /// Sub/board slot, as an index into `device`.
    pub sub: u32,
    /// 5 weapon mod sockets, as mod-chip base ids.
    pub wmods: [Option<u32>; 5],
    /// 5 armor mod sockets, as mod-chip base ids.
    pub amods: [Option<u32>; 5],
    /// Story bits to write. Bits absent from this list keep their stored value.
    pub story: Vec<StoryEdit>,
    /// Which difficulty's mirrors to write for those story bits.
    pub difficulty: DifficultyChoice,
    /// Bank balance.
    pub bank_bit: u32,
    /// The 12 owned disk counts.
    pub disks: [u16; 12],
    /// The 96 bank item slots.
    ///
    /// A `Vec` rather than a 96-element array because serde only derives for
    /// arrays up to 32 elements. It also matches `builder::SaveSpec::bank` and
    /// `SaveData::get_raw_bank_slots`.
    pub bank_items: Vec<u32>,
}

impl Default for EditSet {
    /// The state of a freshly built save (`builder::SaveSpec::default()`):
    /// Dorumon named TST at level 1, everything else empty.
    fn default() -> Self {
        Self {
            species: Species::Dorumon,
            name: "TST".to_string(),
            bit: 0,
            xdata: 0,
            junk: 0,
            level: 1,
            exp: 0,
            tech: [1; 9],
            upcnt: [0; 11],
            device: [EMPTY; DEVICE_SLOTS],
            weapons: [EMPTY; 3],
            armor: EMPTY,
            sub: EMPTY,
            wmods: [None; 5],
            amods: [None; 5],
            story: Vec::new(),
            difficulty: DifficultyChoice::Auto,
            bank_bit: 0,
            disks: [0; 12],
            bank_items: vec![EMPTY; BANK_SLOTS],
        }
    }
}

/// Everything the UI needs to render, read out of a save.
///
/// This is a *projection*, not a second source of truth: it is rebuilt from the
/// bytes on every open and every save, and never written back wholesale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SaveView {
    /// Species derived from `DIGIMONNAME`.
    pub species: Species,
    /// Player name.
    pub name: String,
    /// Currency.
    pub bit: u32,
    /// X-Data currency.
    pub xdata: u32,
    /// Junk-shop donation counter as stored.
    pub junk_counter: u32,
    /// Tier that counter reaches.
    pub junk_tier: u32,
    /// Level for `species`.
    pub level: u32,
    /// EXP for `species`, lifted to the level threshold in Normal mode.
    pub exp: u32,
    /// The 9 techniques, signed.
    pub tech: [i32; 9],
    /// The 11 power-ups.
    pub upcnt: [u32; 11],
    /// The 30 usable device slots.
    pub device: [u32; DEVICE_SLOTS],
    /// 3 weapon slots, as device indices.
    pub weapons: [u32; 3],
    /// Armor slot, as a device index.
    pub armor: u32,
    /// Sub slot, as a device index.
    pub sub: u32,
    /// 5 weapon mod sockets, as device indices.
    pub wmods: [u32; 5],
    /// 5 armor mod sockets, as device indices.
    pub amods: [u32; 5],
    /// The 1024 raw `BASE_FLAG` bytes, so the Story tab can render every
    /// checkbox without a second call.
    pub story_flags: Vec<u8>,
    /// The 12 raw `BASE_FLAGFOLDER` bytes.
    pub story_folders: Vec<u8>,
    /// Difficulty inferred from the live mirror set.
    pub difficulty: Difficulty,
    /// Bank balance.
    pub bank_bit: u32,
    /// The 12 owned disk counts.
    pub disks: [u16; 12],
    /// The 96 bank slots.
    pub bank_items: Vec<u32>,
    /// Whether the loaded bytes carried valid checksums.
    pub checksum_ok: bool,
}

impl SaveView {
    /// Seed a draft from this view.
    ///
    /// `wmods`/`amods` convert from device indices back to mod-chip base ids,
    /// which is the inverse of the resolution `apply` performs.
    #[must_use]
    pub fn to_edit_set(&self) -> EditSet {
        EditSet {
            species: self.species,
            name: self.name.clone(),
            bit: self.bit,
            xdata: self.xdata,
            junk: self.junk_counter,
            level: self.level,
            exp: self.exp,
            tech: self.tech,
            upcnt: self.upcnt,
            device: self.device,
            weapons: self.weapons,
            armor: self.armor,
            sub: self.sub,
            wmods: self.wmods.map(|i| socket_base_id(&self.device, i)),
            amods: self.amods.map(|i| socket_base_id(&self.device, i)),
            story: Vec::new(),
            difficulty: DifficultyChoice::Fixed(self.difficulty),
            bank_bit: self.bank_bit,
            disks: self.disks,
            bank_items: self.bank_items.clone(),
        }
    }
}

/// The mod-chip base id a socket index points at, or `None` if empty or out of range.
fn socket_base_id(device: &[u32; DEVICE_SLOTS], index: u32) -> Option<u32> {
    let slot = device.get(index as usize)?;
    if *slot == EMPTY {
        return None;
    }
    Some(*slot & 0xFFFF)
}

/// A save being edited, plus where it came from.
#[derive(Debug, Clone)]
pub struct Document {
    data: SaveData,
    path: Option<PathBuf>,
    /// The card this was loaded from, if it was one. A new `.ps2` is created by
    /// copying this, which is the Python `save_memcard` semantics.
    source_card: Option<PathBuf>,
    /// Whether the bytes as loaded had valid checksums.
    loaded_checksums_ok: bool,
}

impl Document {
    /// Wrap already-parsed save bytes.
    ///
    /// # Errors
    /// [`crate::Error::BadSaveSize`] if `bytes` is not [`crate::SAVE_SIZE`] long.
    pub fn from_bytes(bytes: &[u8]) -> crate::Result<Self> {
        let data = SaveData::parse(bytes)?;
        let loaded_checksums_ok = data.verify();
        Ok(Self {
            data,
            path: None,
            source_card: None,
            loaded_checksums_ok,
        })
    }

    /// Load a save from a card image or a bare file, deciding by content.
    ///
    /// A `.ps2` memory card and an 81,920-byte raw save go through the same
    /// entry point; the file's own bytes decide which it is.
    ///
    /// # Errors
    /// [`crate::Error::File`] if unreadable, or whatever the container reports.
    pub fn load(path: &Path) -> crate::Result<Self> {
        let bytes = crate::memcard::load_save(path)?;
        let mut doc = Self::from_bytes(&bytes)?;
        doc.path = Some(path.to_path_buf());
        // Remember the card, so Save As to a new .ps2 can copy it.
        if crate::memcard::is_memcard(&fs::read(path).map_err(|source| crate::Error::File {
            path: path.to_path_buf(),
            source,
        })?) {
            doc.source_card = Some(path.to_path_buf());
        }
        Ok(doc)
    }

    /// The underlying save.
    #[must_use]
    pub fn data(&self) -> &SaveData {
        &self.data
    }

    /// The underlying save, mutably. Used by tests and, later, the card backend.
    pub fn data_mut(&mut self) -> &mut SaveData {
        &mut self.data
    }

    /// The file this was loaded from, if any.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Whether the bytes as loaded carried valid checksums.
    #[must_use]
    pub fn loaded_checksums_ok(&self) -> bool {
        self.loaded_checksums_ok
    }

    /// Project the save for the UI.
    ///
    /// In [`Mode::Normal`] a stale EXP - one below `level_threshold(level)` - is
    /// lifted to the threshold, reproducing `_load_species_stats`
    /// (`save_editor_gui.py:784`). Advanced mode reports what is stored.
    #[must_use]
    pub fn view(&self, mode: Mode) -> SaveView {
        let species = self.data.detect_species();
        let level = self.data.level(species);
        let mut exp = self.data.exp(species);
        if !mode.is_advanced() {
            let threshold = level_threshold(level);
            if i64::from(exp) < threshold && threshold <= i64::from(u32::MAX) {
                exp = threshold as u32;
            }
        }

        SaveView {
            species,
            name: self.data.player_name(),
            bit: self.data.bit(),
            xdata: self.data.xdata(),
            junk_counter: self.data.junk_counter(),
            junk_tier: junk_tier_from_counter(self.data.junk_counter()),
            level,
            exp,
            tech: std::array::from_fn(|i| self.data.skill(species, i)),
            upcnt: std::array::from_fn(|i| self.data.upcnt(species, i)),
            // `get_raw_device_slots` returns all 36 slots (30 usable + 6
            // reserved); the view exposes only the usable 30.
            device: std::array::from_fn(|i| self.data.device(i)),
            weapons: [
                self.data.weapon(0),
                self.data.weapon(1),
                self.data.weapon(2),
            ],
            armor: self.data.armor(),
            sub: self.data.sub(),
            wmods: self
                .data
                .get_raw_weapon_mod_slots()
                .try_into()
                .expect("5 slots"),
            amods: self
                .data
                .get_raw_armor_mod_slots()
                .try_into()
                .expect("5 slots"),
            story_flags: self.data.raw_flags().to_vec(),
            story_folders: self.data.raw_folders().to_vec(),
            difficulty: detect_difficulty(self.data.raw_flags()),
            bank_bit: self.data.bank_bit(),
            disks: std::array::from_fn(|i| self.data.disk_count(i)),
            bank_items: self.data.get_raw_bank_slots(),
            checksum_ok: self.loaded_checksums_ok,
        }
    }

    /// Check an edit set, returning non-blocking warnings or every blocking
    /// error.
    ///
    /// The Python `collect()` raises on the first problem. Reporting all of them
    /// is deliberate: the frontend highlights one control per error, and fixing
    /// fields one round-trip at a time is worse.
    ///
    /// # Errors
    /// A non-empty `Vec<FieldError>` when any edit was rejected. Nothing is
    /// written in that case.
    pub fn validate(&self, edits: &EditSet, mode: Mode) -> Result<Vec<Warning>, Vec<FieldError>> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let adv = mode.is_advanced();

        // Numeric scalars.
        for (path, label, cap, value, safe_max) in [
            (
                "bit",
                "BIT",
                CAP_BIT,
                i64::from(edits.bit),
                CAP_BIT.normal_max,
            ),
            (
                "xdata",
                "X-Data",
                CAP_XDATA,
                i64::from(edits.xdata),
                CAP_XDATA.normal_max,
            ),
            (
                "level",
                "Level",
                CAP_LEVEL,
                i64::from(edits.level),
                CAP_LEVEL.normal_max,
            ),
            (
                "exp",
                "EXP",
                CAP_EXP,
                i64::from(edits.exp),
                CAP_EXP.normal_max,
            ),
            (
                "bank_bit",
                "Bank balance",
                CAP_BIT,
                i64::from(edits.bank_bit),
                CAP_BIT.normal_max,
            ),
        ] {
            if let Some(e) = check_cap(path, label, cap, value, mode, safe_max) {
                errors.push(e);
            }
        }

        for (i, v) in edits.tech.iter().enumerate() {
            if let Some(e) = check_cap(
                &format!("tech[{i}]"),
                TECHNIQUES[i],
                CAP_TECH,
                i64::from(*v),
                mode,
                CAP_TECH.normal_max,
            ) {
                errors.push(e);
            }
        }
        for (i, v) in edits.upcnt.iter().enumerate() {
            if let Some(e) = check_cap(
                &format!("upcnt[{i}]"),
                POWERUP_STATS[i],
                CAP_UPCNT,
                i64::from(*v),
                mode,
                upcnt_safe_cap(i),
            ) {
                errors.push(e);
            }
        }

        // Disk counts. Unreachable while the field is `u16`, which cannot
        // exceed the cap; kept so widening the field cannot silently lose the
        // check.
        for (i, c) in edits.disks.iter().enumerate() {
            if let Some(e) = check_cap(
                &format!("disks[{i}]"),
                &format!("Disk {i}"),
                CAP_DISK_COUNT,
                i64::from(*c),
                mode,
                CAP_DISK_COUNT.normal_max,
            ) {
                errors.push(e);
            }
        }

        // Item slots.
        for (i, id) in edits.device.iter().enumerate() {
            validate_item(*id, &format!("device[{i}]"), mode, &mut errors);
        }
        for (i, id) in edits.bank_items.iter().enumerate() {
            validate_item(*id, &format!("bank_items[{i}]"), mode, &mut errors);
        }

        // Mod sockets: resolution is what can fail, so run it once and reuse.
        let resolved = match resolve_mods(edits) {
            Ok(r) => Some(r),
            Err(e) => {
                errors.push(e);
                None
            }
        };

        // Equipment index range.
        for (label, index) in equip_indices(edits, resolved.as_ref()) {
            if index != EMPTY && (index as usize) >= DEVICE_SLOTS && !adv {
                errors.push(FieldError::error(
                    format!("equip.{label}"),
                    format!("slot {index} out of range 0..{}", DEVICE_SLOTS - 1),
                ));
            }
        }

        // Category sanity. Warnings only, and Normal mode only, matching the
        // Python "these may behave oddly in-game" behaviour.
        if !adv && let Some(r) = &resolved {
            for (label, index) in equip_indices(edits, Some(r)) {
                if index == EMPTY || (index as usize) >= DEVICE_SLOTS {
                    continue;
                }
                let fid = r.device[index as usize];
                if fid == EMPTY {
                    continue;
                }
                let Some((_, want)) = EQUIP_CATEGORIES.iter().find(|(l, _)| *l == label) else {
                    continue;
                };
                let cat = category_of(fid);
                if !want.contains(&cat) {
                    warnings.push(FieldError::warning(
                        format!("equip.{label}"),
                        format!(
                            "{label} points at a {} (expected {})",
                            category_label(cat.byte()),
                            want.iter()
                                .map(|c| category_label(c.byte()))
                                .collect::<Vec<_>>()
                                .join("/")
                        ),
                    ));
                }
            }
        }

        // Story index range.
        for edit in &edits.story {
            let (kind, limit) = match edit.kind {
                StoryKind::Flag => ("flag", FLAG_COUNT),
                StoryKind::Folder => ("folder", FOLDER_COUNT),
            };
            if (edit.index as usize) >= limit {
                errors.push(FieldError::error(
                    format!("story.{kind}[{}]", edit.index),
                    format!(
                        "{kind} index {} is out of range 0..{}",
                        edit.index,
                        limit - 1
                    ),
                ));
            }
        }

        if errors.is_empty() {
            Ok(warnings)
        } else {
            Err(errors)
        }
    }

    /// Validate, then apply. Nothing is written unless every edit is accepted.
    ///
    /// # Errors
    /// The same `Vec<FieldError>` as [`Document::validate`]; the save is
    /// untouched.
    pub fn apply(&mut self, edits: &EditSet, mode: Mode) -> Result<Vec<Warning>, Vec<FieldError>> {
        let warnings = self.validate(edits, mode)?;
        apply_to(&mut self.data, edits);
        Ok(warnings)
    }
}

/// Write an edit set into a save.
///
/// The caller must have validated `edits` first. The write order follows
/// `App.apply` (`save_editor_gui.py:1197`): species and model name, scalars,
/// the per-species block, the device folder (with mods resolved into it),
/// equipment, story with mirrors, then bank and disks.
///
/// HP/MP/MHP/MMP are never written: the game recomputes them from the loadout.
fn apply_to(data: &mut SaveData, edits: &EditSet) {
    let species = edits.species;

    // Species -> DIGIMONNAME, only when it actually differs.
    let model = species.model_name();
    if data.digimon_name() != model {
        data.set_digimon_name(&model);
    }

    data.set_bit(edits.bit);
    data.set_xdata(edits.xdata);
    data.set_junk_counter(edits.junk);
    data.set_level(species, edits.level);
    data.set_menu_level(edits.level);
    data.set_exp(species, edits.exp);
    for (i, v) in edits.tech.iter().enumerate() {
        data.set_skill(species, i, *v);
    }
    for (i, v) in edits.upcnt.iter().enumerate() {
        data.set_upcnt(species, i, *v);
    }
    data.set_player_name(&edits.name);

    // Device folder, with mod sockets resolved against it. `resolve_mods`
    // cannot fail here: `validate` already ran the same pass and returned early
    // on error. Fall back to the un-resolved device list rather than panicking
    // if that invariant is ever broken.
    let resolved = resolve_mods(edits).ok();
    let device = resolved.as_ref().map_or(edits.device, |r| r.device);
    for (i, fid) in device.iter().enumerate() {
        data.set_device(i, *fid);
    }
    // The reserved slots 30-35 are cleared, as the Python apply() does.
    for i in DEVICE_SLOTS..DEVICE_SAVE_SLOTS {
        data.set_device(i, EMPTY);
    }

    for (i, v) in edits.weapons.iter().enumerate() {
        data.set_weapon(i, *v);
    }
    data.set_armor(edits.armor);
    data.set_sub(edits.sub);
    for (i, socket) in resolved
        .as_ref()
        .map_or([None; 5], |r| r.wmods)
        .iter()
        .enumerate()
    {
        data.set_weapon_mod(i, socket.unwrap_or(EMPTY));
    }
    for (i, socket) in resolved
        .as_ref()
        .map_or([None; 5], |r| r.amods)
        .iter()
        .enumerate()
    {
        data.set_armor_mod(i, socket.unwrap_or(EMPTY));
    }

    if !edits.story.is_empty() {
        mirror_story(data, edits);
    }

    data.set_bank_bit(edits.bank_bit);
    for (i, c) in edits.disks.iter().enumerate() {
        data.set_disk_count(i, *c);
    }
    for (i, fid) in edits.bank_items.iter().enumerate() {
        data.set_bank_device(i, *fid);
    }
}

/// Write the story edits into the live flag/folder bytes, then write each edited
/// bit's difficulty mirror.
///
/// The title screen restores active <- mirror on load, so an active-only edit
/// reverts. Unlike the Python `apply`, which mirrors through the GUI's partial
/// 10-flag table, this mirrors the **complete** table (spec 5.1) - the
/// deliberate divergence.
fn mirror_story(data: &mut SaveData, edits: &EditSet) {
    let difficulty = match edits.difficulty {
        DifficultyChoice::Fixed(d) => d,
        DifficultyChoice::Auto => detect_difficulty(data.raw_flags()),
    };

    let mut flags = data.raw_flags().to_vec();
    let mut folders = data.raw_folders().to_vec();

    for edit in &edits.story {
        let value = u8::from(edit.value);
        match edit.kind {
            StoryKind::Flag => flags[edit.index as usize] = value,
            StoryKind::Folder => folders[edit.index as usize] = value,
        }
    }

    for edit in &edits.story {
        let value = u8::from(edit.value);
        match edit.kind {
            StoryKind::Flag => {
                if let Some(row) = MIRRORS.iter().find(|r| r.active == u32::from(edit.index)) {
                    flags[row.mirror_for(difficulty) as usize] = value;
                }
            }
            StoryKind::Folder => {
                if (edit.index as usize) < MIRRORED_FOLDERS
                    && let Some(target) = folder_mirror(edit.index as usize, difficulty)
                {
                    flags[target as usize] = value;
                }
            }
        }
    }

    data.set_raw_flags(&flags);
    data.set_raw_folders(&folders);
}

use std::fs;
use std::io::Write as _;

impl Document {
    /// Read a bare 81,920-byte save file.
    ///
    /// Memory-card images are handled by the card backend (next plan); this is
    /// the raw-file path only.
    ///
    /// # Errors
    /// [`crate::Error::File`] if the file cannot be read, or
    /// [`crate::Error::BadSaveSize`] if it is the wrong length.
    pub fn load_raw(path: &Path) -> crate::Result<Self> {
        let bytes = fs::read(path).map_err(|source| crate::Error::File {
            path: path.to_path_buf(),
            source,
        })?;
        let mut doc = Self::from_bytes(&bytes)?;
        doc.path = Some(path.to_path_buf());
        Ok(doc)
    }

    /// Write to `path`, atomically, keeping a `.bak` of the previous contents.
    ///
    /// The container is chosen from `path`, as [`crate::memcard::render_container`]
    /// describes: an existing card is rewritten in place, a new `.ps2` is created
    /// by copying the card this document was loaded from, and anything else is a
    /// bare 81,920-byte save.
    ///
    /// The `.bak` is written once - before the first overwrite of an existing
    /// file - and is never replaced by a later save in the same session, so it
    /// holds the state the session started from (spec 3.5).
    ///
    /// # Errors
    /// [`crate::Error::File`] if the file cannot be written,
    /// [`crate::Error::NoSave`] if a new `.ps2` has no source card or the
    /// post-write verification fails.
    pub fn save(&mut self, path: &Path) -> crate::Result<()> {
        let save = self.data.to_bytes();
        let bytes = crate::memcard::render_container(path, self.source_card.as_deref(), &save)?;

        if path.exists() {
            let bak = backup_path(path);
            if !bak.exists() {
                fs::copy(path, &bak).map_err(|source| crate::Error::File {
                    path: bak.clone(),
                    source,
                })?;
            }
        }

        write_atomically(path, &bytes)?;

        // Re-read through the same content dispatch, so a card is verified as a
        // card and a raw file as a raw file.
        let written = crate::memcard::load_save(path)?;
        let check = SaveData::parse(&written).map_err(|e| crate::Error::NoSave(e.to_string()))?;
        if !check.verify() {
            return Err(crate::Error::NoSave(format!(
                "{}: checksum verification failed after writing",
                path.display()
            )));
        }

        self.path = Some(path.to_path_buf());
        self.loaded_checksums_ok = true;
        Ok(())
    }
}

/// `save.raw` -> `save.raw.bak` (the extension is appended, not replaced).
fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".bak");
    path.with_file_name(name)
}

/// Write via a temp file in the destination directory, then rename.
fn write_atomically(path: &Path, bytes: &[u8]) -> crate::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = dir.join(format!(
        ".{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    {
        let mut f = fs::File::create(&tmp).map_err(|source| crate::Error::File {
            path: tmp.clone(),
            source,
        })?;
        f.write_all(bytes).map_err(|source| crate::Error::File {
            path: tmp.clone(),
            source,
        })?;
        f.sync_all().map_err(|source| crate::Error::File {
            path: tmp.clone(),
            source,
        })?;
    }
    fs::rename(&tmp, path).map_err(|source| crate::Error::File {
        path: path.to_path_buf(),
        source,
    })
}

/// Category an equipment slot requires.
const WEAPON_CATEGORIES: [Category; 2] = [Category::Weapon, Category::Styled];

/// The equipment slots holding a `device` index, with the categories each
/// accepts. Used for the mismatch warning.
const EQUIP_CATEGORIES: [(&str, &[Category]); 15] = [
    ("weapon0", &WEAPON_CATEGORIES),
    ("weapon1", &WEAPON_CATEGORIES),
    ("weapon2", &WEAPON_CATEGORIES),
    ("armor", &[Category::Core]),
    ("sub", &[Category::Board]),
    ("wmod0", &[Category::Mod]),
    ("wmod1", &[Category::Mod]),
    ("wmod2", &[Category::Mod]),
    ("wmod3", &[Category::Mod]),
    ("wmod4", &[Category::Mod]),
    ("amod0", &[Category::Mod]),
    ("amod1", &[Category::Mod]),
    ("amod2", &[Category::Mod]),
    ("amod3", &[Category::Mod]),
    ("amod4", &[Category::Mod]),
];

/// Check one value against a cap, returning the message the user should see.
fn check_cap(
    path: &str,
    label: &str,
    cap: Cap,
    value: i64,
    mode: Mode,
    safe_max: i64,
) -> Option<FieldError> {
    if mode.is_advanced() {
        if cap.allows_advanced(value) {
            return None;
        }
        return Some(FieldError::error(
            path,
            format!(
                "{label}: {value} out of range {}..0x{:X} (data-type range)",
                cap.dtype_min, cap.dtype_max
            ),
        ));
    }
    if (0..=safe_max).contains(&value) {
        return None;
    }
    Some(FieldError::error(
        path,
        format!("{label}: {value} exceeds normal cap {safe_max}. Enable Advanced to go higher."),
    ))
}

/// Check one item id: the `+N` bonus, and (Normal mode) whether it is a known
/// non-crashing id.
fn validate_item(id: u32, path: &str, mode: Mode, errors: &mut Vec<FieldError>) {
    if id == EMPTY {
        return;
    }
    let (base_id, seed, _mods) = split_item_id(id);
    if seed > SEED_BONUS_MASK {
        errors.push(FieldError::error(
            format!("{path}.bonus"),
            format!("+N {seed} out of range 0..{SEED_BONUS_MASK}"),
        ));
    }
    if !mode.is_advanced()
        && let Some(reason) = invalid_reason(base_id)
    {
        errors.push(FieldError::error(
            path.to_string(),
            format!("{reason}. Enable Advanced to assign anyway."),
        ));
    }
}

/// A [`EditSet`] with mod sockets resolved to device indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMods {
    /// The device folder, with any auto-added chip in place.
    pub device: [u32; DEVICE_SLOTS],
    /// Resolved weapon mod sockets.
    pub wmods: [Option<u32>; 5],
    /// Resolved armor mod sockets.
    pub amods: [Option<u32>; 5],
}

/// Resolve mod sockets to device indices, adding absent chips to the device
/// folder.
///
/// This is `_find_or_add_mod` (`save_editor_gui.py:1011`): reuse a slot already
/// holding the chip, else fill the first empty slot, else fail. `validate` and
/// `apply` both call this, so an accepted edit cannot be applied differently
/// from the way it was checked.
///
/// # Errors
/// [`FieldError`] naming the socket when no device slot is free.
pub fn resolve_mods(edits: &EditSet) -> Result<ResolvedMods, FieldError> {
    let mut device = edits.device;
    let mut wmods = [None; 5];
    let mut amods = [None; 5];

    for (i, socket) in edits.wmods.iter().enumerate() {
        if let Some(base_id) = socket {
            let path = format!("equip.wmod{i}");
            wmods[i] = Some(find_or_add_mod(&mut device, *base_id, &path)?);
        }
    }
    for (i, socket) in edits.amods.iter().enumerate() {
        if let Some(base_id) = socket {
            let path = format!("equip.amod{i}");
            amods[i] = Some(find_or_add_mod(&mut device, *base_id, &path)?);
        }
    }

    Ok(ResolvedMods {
        device,
        wmods,
        amods,
    })
}

fn find_or_add_mod(
    device: &mut [u32; DEVICE_SLOTS],
    base_id: u32,
    path: &str,
) -> Result<u32, FieldError> {
    for (i, slot) in device.iter().enumerate() {
        if *slot != EMPTY && (*slot & 0xFFFF) == base_id {
            return Ok(i as u32);
        }
    }
    for (i, slot) in device.iter_mut().enumerate() {
        if *slot == EMPTY {
            *slot = build_item_id(base_id, 0, 0);
            return Ok(i as u32);
        }
    }
    Err(FieldError::error(
        path,
        format!("no free device-folder slot to add the mod chip 0x{base_id:04X}"),
    ))
}

/// The `(label, index)` pairs for every equipment slot, using resolved mod
/// indices when available.
fn equip_indices(edits: &EditSet, resolved: Option<&ResolvedMods>) -> Vec<(&'static str, u32)> {
    let mut out = vec![
        ("weapon0", edits.weapons[0]),
        ("weapon1", edits.weapons[1]),
        ("weapon2", edits.weapons[2]),
        ("armor", edits.armor),
        ("sub", edits.sub),
    ];
    let wmods = resolved.map_or([None; 5], |r| r.wmods);
    let amods = resolved.map_or([None; 5], |r| r.amods);
    for (i, m) in wmods.iter().enumerate() {
        out.push((
            ["wmod0", "wmod1", "wmod2", "wmod3", "wmod4"][i],
            m.unwrap_or(EMPTY),
        ));
    }
    for (i, m) in amods.iter().enumerate() {
        out.push((
            ["amod0", "amod1", "amod2", "amod3", "amod4"][i],
            m.unwrap_or(EMPTY),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_error_carries_a_path_a_message_and_a_severity() {
        let e = FieldError::error("bit", "BIT: 10000000 exceeds normal cap 9999999");
        assert_eq!(e.path, "bit");
        assert!(e.message.contains("9999999"));
        assert_eq!(e.severity, Severity::Error);
    }

    #[test]
    fn a_warning_is_a_field_error_marked_as_a_warning() {
        let w = FieldError::warning("equip.armor", "armor points at a weapon");
        assert_eq!(w.severity, Severity::Warning);
    }

    #[test]
    fn mode_reports_whether_it_is_advanced() {
        assert!(!Mode::Normal.is_advanced());
        assert!(Mode::Advanced.is_advanced());
    }

    #[test]
    fn a_default_edit_set_matches_a_fresh_save() {
        let e = EditSet::default();
        assert_eq!(e.species, crate::species::Species::Dorumon);
        assert_eq!(e.name, "TST");
        assert_eq!(e.level, 1);
        assert_eq!(e.exp, 0);
        assert_eq!(e.bit, 0);
        assert_eq!(e.xdata, 0);
        assert_eq!(e.junk, 0);
        assert_eq!(e.bank_bit, 0);
        assert_eq!(e.tech, [1; 9]);
        assert_eq!(e.upcnt, [0; 11]);
        assert_eq!(e.device, [crate::EMPTY; DEVICE_SLOTS]);
        assert_eq!(e.weapons, [crate::EMPTY; 3]);
        assert_eq!(e.armor, crate::EMPTY);
        assert_eq!(e.sub, crate::EMPTY);
        assert_eq!(e.wmods, [None; 5]);
        assert_eq!(e.amods, [None; 5]);
        assert!(e.story.is_empty());
        assert_eq!(e.disks, [0u16; 12]);
        assert_eq!(e.bank_items, vec![crate::EMPTY; BANK_SLOTS]);
        assert_eq!(e.difficulty, DifficultyChoice::Auto);
    }

    #[test]
    fn a_story_edit_names_its_kind_index_and_value() {
        let s = StoryEdit::flag(66, true);
        assert_eq!(s.kind, StoryKind::Flag);
        assert_eq!(s.index, 66);
        assert!(s.value);
        let f = StoryEdit::folder(3, false);
        assert_eq!(f.kind, StoryKind::Folder);
        assert!(!f.value);
    }

    use crate::builder::{SaveSpec, build_save};

    fn fresh_document() -> Document {
        let bytes = build_save(&SaveSpec::default());
        Document::from_bytes(&bytes).expect("a built save parses")
    }

    #[test]
    fn a_view_of_a_fresh_save_reports_its_defaults() {
        let doc = fresh_document();
        let v = doc.view(Mode::Normal);
        assert_eq!(v.species, Species::Dorumon);
        assert_eq!(v.name, "TST");
        assert_eq!(v.level, 1);
        assert_eq!(v.exp, 0);
        assert_eq!(v.bit, 0);
        assert_eq!(v.junk_counter, 0);
        assert_eq!(v.junk_tier, 0);
        assert_eq!(v.tech, [1; 9]);
        assert_eq!(v.device, [EMPTY; DEVICE_SLOTS]);
        assert_eq!(v.bank_items, vec![EMPTY; BANK_SLOTS]);
        assert!(v.checksum_ok);
    }

    #[test]
    fn the_view_lifts_a_stale_exp_to_the_level_threshold_in_normal_mode() {
        // Level 10 with EXP 0 is inconsistent: the GUI's _load_species_stats
        // lifts it so the field shows a value matching the level.
        let spec = SaveSpec {
            level: 10,
            exp: 0,
            ..SaveSpec::default()
        };
        let doc = Document::from_bytes(&build_save(&spec)).expect("parses");

        let normal = doc.view(Mode::Normal);
        assert_eq!(normal.exp, crate::codes::level_threshold(10) as u32);

        // Advanced shows the stored value untouched.
        let advanced = doc.view(Mode::Advanced);
        assert_eq!(advanced.exp, 0);
    }

    #[test]
    fn a_view_round_trips_through_an_edit_set() {
        let doc = fresh_document();
        let v = doc.view(Mode::Normal);
        let e = v.to_edit_set();
        assert_eq!(e.species, v.species);
        assert_eq!(e.name, v.name);
        assert_eq!(e.level, v.level);
        assert_eq!(e.exp, v.exp);
        assert_eq!(e.tech, v.tech);
        assert_eq!(e.upcnt, v.upcnt);
        assert_eq!(e.device, v.device);
        assert_eq!(e.weapons, v.weapons);
        assert_eq!(e.armor, v.armor);
        assert_eq!(e.sub, v.sub);
        assert_eq!(e.bank_bit, v.bank_bit);
        assert_eq!(e.disks, v.disks);
        assert_eq!(e.bank_items, v.bank_items);
    }

    #[test]
    fn a_cap_above_the_normal_limit_is_an_error_at_its_own_path() {
        let doc = fresh_document();
        let edits = EditSet {
            bit: 10_000_000,
            ..Default::default()
        };
        let errs = doc.validate(&edits, Mode::Normal).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].path, "bit");
        assert_eq!(errs[0].severity, Severity::Error);
    }

    #[test]
    fn the_same_cap_is_accepted_in_advanced_mode() {
        let doc = fresh_document();
        let edits = EditSet {
            bit: 10_000_000,
            ..Default::default()
        };
        assert!(doc.validate(&edits, Mode::Advanced).is_ok());
    }

    #[test]
    fn every_bad_field_is_reported_not_just_the_first() {
        // The Python collect() raises on the first problem. Reporting all of
        // them is deliberate: the frontend highlights each path.
        let doc = fresh_document();
        let edits = EditSet {
            bit: 20_000_000,
            level: 5_000,
            xdata: 20_000,
            ..Default::default()
        };
        let errs = doc.validate(&edits, Mode::Normal).unwrap_err();
        let paths: Vec<&str> = errs.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"bit"), "{paths:?}");
        assert!(paths.contains(&"level"), "{paths:?}");
        assert!(paths.contains(&"xdata"), "{paths:?}");
    }

    #[test]
    fn a_power_up_slot_uses_its_own_cap() {
        // Slots 0 and 1 are HP/MP max: 99999. Every other slot is 9999.
        let doc = fresh_document();
        let ok = EditSet {
            upcnt: [
                99_999, 99_999, 9_999, 9_999, 9_999, 9_999, 9_999, 9_999, 9_999, 9_999, 9_999,
            ],
            ..Default::default()
        };
        assert!(doc.validate(&ok, Mode::Normal).is_ok());

        let bad = EditSet {
            upcnt: [1, 1, 10_000, 0, 0, 0, 0, 0, 0, 0, 0],
            ..Default::default()
        };
        let errs = doc.validate(&bad, Mode::Normal).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].path, "upcnt[2]");
    }

    #[test]
    fn a_technique_may_be_negative_in_advanced_mode_only() {
        let doc = fresh_document();
        let neg = EditSet {
            tech: [0, 0, -1, 0, 0, 0, 0, 0, 0],
            ..Default::default()
        };
        assert_eq!(
            doc.validate(&neg, Mode::Normal).unwrap_err()[0].path,
            "tech[2]"
        );
        assert!(doc.validate(&neg, Mode::Advanced).is_ok());
    }

    #[test]
    fn a_crash_item_is_rejected_in_normal_mode_and_allowed_in_advanced() {
        let doc = fresh_document();
        let mut device = [EMPTY; DEVICE_SLOTS];
        device[0] = 0x0000_051B; // styled weapon: crashes on load
        let edits = EditSet {
            device,
            ..Default::default()
        };
        let errs = doc.validate(&edits, Mode::Normal).unwrap_err();
        assert_eq!(errs[0].path, "device[0]");
        assert!(doc.validate(&edits, Mode::Advanced).is_ok());
    }

    #[test]
    fn a_device_slot_bonus_beyond_eleven_bits_is_an_error_in_both_modes() {
        let doc = fresh_document();
        let mut device = [EMPTY; DEVICE_SLOTS];
        device[2] = crate::item::build_item_id(0x0000, 0x800, 0);
        let edits = EditSet {
            device,
            ..Default::default()
        };
        assert_eq!(
            doc.validate(&edits, Mode::Normal).unwrap_err()[0].path,
            "device[2].bonus"
        );
        assert_eq!(
            doc.validate(&edits, Mode::Advanced).unwrap_err()[0].path,
            "device[2].bonus"
        );
    }

    #[test]
    fn an_equipment_category_mismatch_is_a_warning_not_an_error() {
        let doc = fresh_document();
        let mut device = [EMPTY; DEVICE_SLOTS];
        device[0] = 0x0000_0010; // a graded weapon
        // armor must be a core (0x10); point it at the weapon instead
        let edits = EditSet {
            device,
            armor: 0,
            ..Default::default()
        };
        let warnings = doc.validate(&edits, Mode::Normal).expect("accepted");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].path, "equip.armor");
        assert_eq!(warnings[0].severity, Severity::Warning);
    }

    #[test]
    fn a_mod_socket_whose_chip_is_absent_is_added_to_the_first_free_slot() {
        let mut device = [EMPTY; DEVICE_SLOTS];
        device[5] = 0x0000_0100; // occupy slot 5 so 0 is the first free slot
        let edits = EditSet {
            device,
            wmods: [Some(0x30A0), None, None, None, None],
            ..Default::default()
        };
        let resolved = resolve_mods(&edits).expect("room available");
        assert_eq!(resolved.wmods[0], Some(0));
        assert_eq!(resolved.device[0], crate::item::build_item_id(0x30A0, 0, 0));
    }

    #[test]
    fn a_mod_socket_reuses_an_existing_chip_rather_than_duplicating_it() {
        let mut device = [EMPTY; DEVICE_SLOTS];
        device[7] = crate::item::build_item_id(0x30A0, 3, 2);
        let edits = EditSet {
            device,
            wmods: [Some(0x30A0), None, None, None, None],
            ..Default::default()
        };
        let resolved = resolve_mods(&edits).expect("room available");
        assert_eq!(resolved.wmods[0], Some(7));
        assert_eq!(resolved.device[7], crate::item::build_item_id(0x30A0, 3, 2));
    }

    #[test]
    fn a_mod_socket_with_no_free_slot_is_an_error() {
        let edits = EditSet {
            device: [0x0000_0010; DEVICE_SLOTS],
            wmods: [Some(0x30A0), None, None, None, None],
            ..Default::default()
        };
        let err = resolve_mods(&edits).unwrap_err();
        assert_eq!(err.path, "equip.wmod0");
        assert!(err.message.contains("no free device-folder slot"));
    }

    #[test]
    fn applying_writes_the_header_fields() {
        let mut doc = fresh_document();
        let edits = EditSet {
            species: Species::Agumon,
            name: "ABC".to_string(),
            bit: 12_345,
            xdata: 678,
            level: 42,
            exp: crate::codes::level_threshold(42) as u32,
            ..Default::default()
        };
        assert!(doc.apply(&edits, Mode::Normal).is_ok());
        let d = doc.data();
        assert_eq!(d.bit(), 12_345);
        assert_eq!(d.xdata(), 678);
        assert_eq!(d.level(Species::Agumon), 42);
        assert_eq!(
            d.menu_level(),
            42,
            "menu_level is written from the edited level"
        );
        assert_eq!(
            d.exp(Species::Agumon),
            crate::codes::level_threshold(42) as u32
        );
        assert_eq!(d.player_name(), "ABC");
        assert_eq!(d.detect_species(), Species::Agumon);
    }

    #[test]
    fn changing_species_rewrites_the_model_name() {
        let mut doc = fresh_document();
        assert_eq!(doc.data().digimon_name(), "p_dorumon");
        let edits = EditSet {
            species: Species::Veemon,
            ..Default::default()
        };
        assert!(doc.apply(&edits, Mode::Normal).is_ok());
        assert_eq!(doc.data().digimon_name(), "p_vmon");
    }

    #[test]
    fn applying_clears_the_reserved_device_slots() {
        // The Python apply() forces slots 30-35 (DEVICE_SAVE_SLOTS) to EMPTY.
        // `SaveSpec::device` is an empty Vec by default, so set the reserved
        // slot on the built save instead of indexing into the spec.
        let bytes = crate::builder::build_save(&crate::builder::SaveSpec::default());
        let mut doc = Document::from_bytes(&bytes).expect("parses");
        doc.data_mut().set_device(33, 0x0000_0010);
        assert_eq!(doc.data().device(33), 0x0000_0010);

        assert!(doc.apply(&EditSet::default(), Mode::Normal).is_ok());
        assert_eq!(doc.data().device(33), EMPTY);
        assert_eq!(doc.data().device(34), EMPTY);
        assert_eq!(doc.data().device(35), EMPTY);
    }

    #[test]
    fn applying_a_mod_socket_adds_the_chip_and_points_at_it() {
        let mut doc = fresh_document();
        let edits = EditSet {
            wmods: [Some(0x30A0), None, None, None, None],
            ..Default::default()
        };
        assert!(doc.apply(&edits, Mode::Normal).is_ok());
        assert_eq!(doc.data().weapon_mod(0), 0, "the chip landed in slot 0");
        assert_eq!(
            doc.data().device(0),
            crate::item::build_item_id(0x30A0, 0, 0)
        );
    }

    #[test]
    fn applying_never_writes_the_derived_stats() {
        // HP/MP/MHP/MMP are recomputed by the game; the editor must not touch
        // them. They live at +0x54..+0x63.
        let mut doc = fresh_document();
        let before = doc.data().to_bytes();
        let edits = EditSet {
            species: Species::Susanoomon,
            level: 999,
            bit: 9_999_999,
            ..Default::default()
        };
        assert!(doc.apply(&edits, Mode::Normal).is_ok());
        let after = doc.data().to_bytes();

        for block in 0..2 {
            let base = block * crate::BLOCK;
            for off in 0x54..0x64 {
                assert_eq!(
                    before[base + off],
                    after[base + off],
                    "byte +0x{off:02X} of block {block} must not change"
                );
            }
        }
    }

    #[test]
    fn a_story_edit_writes_the_active_byte_and_its_mirror() {
        // Flag 0 is in every difficulty's mirror set. Normal mirrors it to 699.
        let mut doc = fresh_document();
        let edits = EditSet {
            story: vec![StoryEdit::flag(0, true)],
            difficulty: DifficultyChoice::Fixed(Difficulty::Normal),
            ..Default::default()
        };
        assert!(doc.apply(&edits, Mode::Normal).is_ok());
        assert_eq!(doc.data().raw_flags()[0], 1);
        assert_eq!(doc.data().raw_flags()[699], 1);
    }

    #[test]
    fn story_edits_mirror_into_the_chosen_difficulty() {
        let mut doc = fresh_document();
        let edits = EditSet {
            story: vec![StoryEdit::flag(0, true)],
            difficulty: DifficultyChoice::Fixed(Difficulty::VeryHard),
            ..Default::default()
        };
        assert!(doc.apply(&edits, Mode::Normal).is_ok());
        assert_eq!(doc.data().raw_flags()[0], 1);
        assert_eq!(doc.data().raw_flags()[12], 1, "Very Hard mirror of flag 0");
        assert_eq!(doc.data().raw_flags()[699], 0, "Normal mirror untouched");
    }

    #[test]
    fn a_folder_edit_mirrors_to_its_flag() {
        let mut doc = fresh_document();
        let edits = EditSet {
            story: vec![StoryEdit::folder(2, true)],
            difficulty: DifficultyChoice::Fixed(Difficulty::Normal),
            ..Default::default()
        };
        assert!(doc.apply(&edits, Mode::Normal).is_ok());
        assert_eq!(doc.data().raw_folders()[2], 1);
        assert_eq!(doc.data().raw_flags()[518 + 2], 1);
    }

    #[test]
    fn a_story_bit_that_is_not_edited_keeps_its_stored_value() {
        let mut doc = fresh_document();
        let seed = EditSet {
            story: vec![StoryEdit::flag(100, true), StoryEdit::flag(200, true)],
            difficulty: DifficultyChoice::Fixed(Difficulty::Normal),
            ..Default::default()
        };
        assert!(doc.apply(&seed, Mode::Normal).is_ok());

        let partial = EditSet {
            story: vec![StoryEdit::flag(100, false)],
            difficulty: DifficultyChoice::Fixed(Difficulty::Normal),
            ..Default::default()
        };
        assert!(doc.apply(&partial, Mode::Normal).is_ok());
        assert_eq!(doc.data().raw_flags()[100], 0);
        assert_eq!(doc.data().raw_flags()[200], 1, "untouched flag survives");
    }

    #[test]
    fn apply_rejects_a_bad_edit_set_without_writing_anything() {
        let mut doc = fresh_document();
        let before = doc.data().to_bytes();
        let edits = EditSet {
            bit: 999_999_999,
            ..Default::default()
        };
        assert!(doc.apply(&edits, Mode::Normal).is_err());
        assert_eq!(
            doc.data().to_bytes(),
            before,
            "a rejected edit writes nothing"
        );
    }

    #[test]
    fn both_mirrored_blocks_stay_identical_after_apply() {
        let mut doc = fresh_document();
        let edits = EditSet {
            bit: 5_000,
            story: vec![StoryEdit::flag(0, true)],
            difficulty: DifficultyChoice::Auto,
            ..Default::default()
        };
        assert!(doc.apply(&edits, Mode::Normal).is_ok());
        let bytes = doc.data().to_bytes();
        assert_eq!(
            &bytes[..crate::BLOCK],
            &bytes[crate::BLOCK..],
            "the two blocks must not drift"
        );
        // `to_bytes` returns a copy with the checksums fixed; `apply` itself
        // does not recompute them, so verify the bytes that would be written.
        let written = crate::SaveData::parse(&bytes).expect("reparses");
        assert!(written.verify());
    }
}
