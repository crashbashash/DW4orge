//! Shared fixture-path helper.
//!
//! Lives in a subdirectory so cargo does not compile it as its own test binary.

use std::path::PathBuf;

/// Absolute path to a file under `tests/fixtures`.
pub fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative)
}
