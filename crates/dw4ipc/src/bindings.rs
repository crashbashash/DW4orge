//! The one list of exported TypeScript types.
//!
//! Both `examples/gen_bindings.rs` and `tests/bindings.rs` call this, so the
//! generation and the drift check can never disagree about which types exist.

use dw4core::{
    Cap, Category, Difficulty, DifficultyChoice, EditSet, FieldError, FlagLabel, Item, Mirror,
    Mode, SaveView, Severity, Species, StoryEdit, StoryKind, StoryPreset,
};
use ts_rs::{Config, ExportError, TS};

use crate::error::IpcError;
use crate::payload::{
    AppInfo, NamedCap, NewSaveRequest, OpenResult, PowerupLimit, SourceKind, SpeciesStats, UiData,
};

/// Every payload rendered to TypeScript, keyed by file stem.
///
/// # Errors
/// [`ExportError`] if a type cannot be exported.
pub fn exported_types(cfg: &Config) -> Result<Vec<(&'static str, String)>, ExportError> {
    Ok(vec![
        ("AppInfo", AppInfo::export_to_string(cfg)?),
        ("Cap", Cap::export_to_string(cfg)?),
        ("Category", Category::export_to_string(cfg)?),
        ("Difficulty", Difficulty::export_to_string(cfg)?),
        ("DifficultyChoice", DifficultyChoice::export_to_string(cfg)?),
        ("EditSet", EditSet::export_to_string(cfg)?),
        ("FieldError", FieldError::export_to_string(cfg)?),
        ("FlagLabel", FlagLabel::export_to_string(cfg)?),
        ("IpcError", IpcError::export_to_string(cfg)?),
        ("Item", Item::export_to_string(cfg)?),
        ("Mirror", Mirror::export_to_string(cfg)?),
        ("Mode", Mode::export_to_string(cfg)?),
        ("NamedCap", NamedCap::export_to_string(cfg)?),
        ("NewSaveRequest", NewSaveRequest::export_to_string(cfg)?),
        ("OpenResult", OpenResult::export_to_string(cfg)?),
        ("PowerupLimit", PowerupLimit::export_to_string(cfg)?),
        ("SaveView", SaveView::export_to_string(cfg)?),
        ("Severity", Severity::export_to_string(cfg)?),
        ("SourceKind", SourceKind::export_to_string(cfg)?),
        ("Species", Species::export_to_string(cfg)?),
        ("SpeciesStats", SpeciesStats::export_to_string(cfg)?),
        ("StoryEdit", StoryEdit::export_to_string(cfg)?),
        ("StoryKind", StoryKind::export_to_string(cfg)?),
        ("StoryPreset", StoryPreset::export_to_string(cfg)?),
        ("UiData", UiData::export_to_string(cfg)?),
    ])
}

/// The configuration every generation and check uses.
///
/// `large_int = "number"` because JSON sends numbers: without it an `i64` cap
/// renders as `bigint` in TypeScript, which would not match what arrives.
#[must_use]
pub fn binding_config() -> Config {
    Config::default().with_large_int("number")
}
