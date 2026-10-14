//! The payload types crossing the IPC boundary.
//!
//! Every one derives `ts_rs::TS`, so these are the shapes that become the
//! frontend's bindings.

use dw4core::{Cap, DifficultyChoice, FlagLabel, Item, Mirror, SaveView, Species, StoryPreset};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Where a save lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    /// A bare 81,920-byte save.
    Raw,
    /// A `.ps2` memory-card image.
    Memcard,
}

/// Everything the New Save dialog supplies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct NewSaveRequest {
    /// Species for the new character.
    pub species: Species,
    /// Player name, at most 8 fullwidth characters.
    pub name: String,
    /// A `STORY_PRESETS` name, or `None` for a storyless save.
    pub story: Option<String>,
    /// Which difficulty's mirror set to seed. `Auto` means Normal here; a
    /// brand-new save has no live mirror set to infer from.
    pub difficulty: DifficultyChoice,
}

/// The result of opening or creating a save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OpenResult {
    /// The file, or `None` for a save that has not been written yet.
    pub path: Option<String>,
    /// Which container it came from.
    pub source: SourceKind,
    /// The projected fields.
    pub view: SaveView,
}

/// One species' stored progression, for the Character section's species
/// selector.
///
/// Mirrors the per-species half of `SaveView`, so switching species can show
/// that block's values without a `get_view` round-trip (`_load_species_stats`,
/// `save_editor_gui.py:776`). Response-only: nothing deserialises it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct SpeciesStats {
    /// Level stored for this species.
    pub level: u32,
    /// EXP for this species, lifted to the level threshold in Normal mode.
    pub exp: u32,
    /// The 9 techniques, signed.
    pub tech: [i32; 9],
    /// The 11 power-ups.
    pub upcnt: [u32; 11],
}

/// Static application identity plus the tables the UI needs.
///
/// Serialize-only: `UiData` carries `FlagLabel` and `StoryPreset`, which hold
/// `'static` borrowed fields and so cannot be deserialised.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct AppInfo {
    /// Product name.
    pub name: String,
    /// Binary version.
    pub version: String,
    /// `dw4core` version, so a stale binding is detectable.
    pub core_version: String,
    /// Bytes in a save.
    pub save_size: usize,
    /// Bytes in one mirrored block.
    pub block_size: usize,
    /// Bumped whenever any payload changes shape.
    pub schema_version: u32,
    /// Static UI tables.
    pub ui: UiData,
}

/// Static tables, fetched once at startup so no keystroke costs a round-trip.
///
/// Serialize-only for the same reason as [`AppInfo`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct UiData {
    /// Every catalogue entry, ordered by base id.
    pub catalogue: Vec<Item>,
    /// Field limits, keyed by the path `validate` reports.
    pub caps: Vec<NamedCap>,
    /// The eleven power-up slots and their Normal-mode caps.
    pub powerups: Vec<PowerupLimit>,
    /// The active-to-mirror table.
    pub mirrors: Vec<Mirror>,
    /// The twelve story folder labels.
    pub folder_labels: Vec<String>,
    /// Flag labels for every story group.
    pub flag_labels: Vec<FlagLabel>,
    /// The story presets.
    pub story_presets: Vec<StoryPreset>,
}

/// One cap, keyed by the field path the validator uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct NamedCap {
    /// Validator path, e.g. `"bit"` or `"upcnt"`.
    pub field: String,
    /// The limits themselves.
    pub cap: Cap,
}

/// One power-up slot's Normal-mode limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PowerupLimit {
    /// Slot index in `BASE_UPCNT` order.
    pub slot: u8,
    /// Display name.
    pub stat: String,
    /// Largest value Normal mode accepts.
    pub normal_max: i64,
}
