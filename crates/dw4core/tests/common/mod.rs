//! Shared fixture-path helper.
//!
//! Lives in a subdirectory so cargo does not compile it as its own test binary.

// Each test binary compiles this module separately, so a helper used by only
// some of them is dead code in the others. That is inherent to shared test
// support, not a mistake to fix per-helper.
#![allow(dead_code)]

use std::path::PathBuf;

/// Absolute path to a file under `tests/fixtures`.
pub fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative)
}

use serde_json::Value;

/// Page size of a PS2 memory card, including its 16-byte spare area.
pub const CARD_PAGE_SIZE: usize = 528;

/// Rebuild the full 8,650,752-byte card from the sparse fixture.
///
/// 16,032 of the card's 16,384 pages are entirely `0xFF`; only the other 352
/// are stored. `tools/gen_fixtures.py` refuses to write the fixture unless this
/// reconstruction is byte-identical to the real card, so tests may treat the
/// result as the card itself.
pub fn memcard_fixture() -> Vec<u8> {
    let meta: Value = serde_json::from_str(
        &std::fs::read_to_string(fixture("mcd001/card/card.json")).expect("card.json"),
    )
    .expect("valid json");

    let page_size = meta["page_size"].as_u64().expect("page_size") as usize;
    let total_pages = meta["total_pages"].as_u64().expect("total_pages") as usize;
    let fill = meta["fill"].as_u64().expect("fill") as u8;
    let indices: Vec<usize> = meta["pages"]
        .as_array()
        .expect("pages")
        .iter()
        .map(|v| v.as_u64().expect("index") as usize)
        .collect();

    let blob = std::fs::read(fixture("mcd001/card/pages.bin")).expect("pages.bin");
    assert_eq!(blob.len(), indices.len() * page_size, "pages.bin size");
    assert_eq!(page_size, CARD_PAGE_SIZE, "fixture page size");

    let mut card = vec![fill; total_pages * page_size];
    for (n, page) in indices.iter().enumerate() {
        card[page * page_size..(page + 1) * page_size]
            .copy_from_slice(&blob[n * page_size..(n + 1) * page_size]);
    }
    card
}
