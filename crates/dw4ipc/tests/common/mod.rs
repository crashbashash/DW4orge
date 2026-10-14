//! Shared fixture helpers.
//!
//! Compiled into every test binary in this crate, so unused helpers are dead
//! code in some of them. That is inherent to shared test support.
#![allow(dead_code)]

use std::path::PathBuf;

/// Absolute path to a file under `dw4core`'s fixtures.
pub fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../dw4core/tests/fixtures")
        .join(relative)
}

/// Page size of a PS2 memory card, including its 16-byte spare area.
pub const CARD_PAGE_SIZE: usize = 528;

/// Rebuild the full 8,650,752-byte card from the sparse fixture.
///
/// The reconstruction is pinned by an independent oracle: the save it yields
/// must equal `mcd001/save.raw` byte for byte (asserted in `session.rs`).
pub fn memcard_fixture() -> Vec<u8> {
    let meta: serde_json::Value = serde_json::from_str(
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

/// Write the reconstructed card to a temp path and return it.
pub fn card_file(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("Mcd001.ps2");
    std::fs::write(&path, memcard_fixture()).expect("write card");
    path
}
