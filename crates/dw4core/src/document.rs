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
}
