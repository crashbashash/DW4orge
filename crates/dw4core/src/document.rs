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
use crate::flags::Difficulty;
use crate::offsets::{BANK_SLOTS, DEVICE_SLOTS};
use crate::species::Species;

/// Which of the two story fields an edit targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
pub struct EditSet {
    /// Species to switch to. Rewrites `DIGIMONNAME` when it changes.
    pub species: Species,
    /// Player name, at most 3 fullwidth characters.
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
}
