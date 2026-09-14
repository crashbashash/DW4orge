//! Behaviour against the real `Mcd001.ps2`, rebuilt from the sparse fixture.
//!
//! The fixture stores only the 352 pages of the card that are not entirely
//! `0xFF`; `common::memcard_fixture()` rebuilds the full 8,650,752 bytes. The
//! generator refuses to write the fixture unless that rebuild is byte-identical
//! to the real card, so these tests run against the real thing.
mod common;

#[test]
fn the_fixture_rebuilds_the_real_card() {
    let card = common::memcard_fixture();
    assert_eq!(card.len(), 8_650_752, "the real card's size");
    assert_eq!(card.len() % common::CARD_PAGE_SIZE, 0);

    // The superblock magic is the cheapest proof that the rebuild placed the
    // right pages at the right offsets.
    assert_eq!(
        &card[..28],
        b"Sony PS2 Memory Card Format ",
        "superblock magic"
    );

    // The save directory name is present.
    let needle = dw4core::memcard::SAVE_DIR.as_bytes();
    assert!(
        card.windows(needle.len()).any(|w| w == needle),
        "the save directory name must be present"
    );

    // And the erased-page assumption the fixture depends on holds.
    let page = common::CARD_PAGE_SIZE;
    let erased = (0..card.len() / page)
        .filter(|n| card[n * page..(n + 1) * page] == [0xffu8; 528][..])
        .count();
    assert_eq!(erased, 16_032, "erased pages");
}

use dw4core::memcard::{CardKind, Geometry, Superblock};

#[test]
fn the_real_superblock_parses_to_the_measured_values() {
    let card = common::memcard_fixture();
    assert_eq!(CardKind::of_size(card.len()), Some(CardKind::WithSpare));

    let sb = Superblock::parse(&card).expect("the real superblock parses");
    assert_eq!(sb.version, "1.2.0.0");
    assert_eq!(sb.page_size, 512);
    assert_eq!(sb.pages_per_cluster, 2);
    assert_eq!(sb.pages_per_block, 16);
    assert_eq!(sb.clusters_per_card, 8192);
    assert_eq!(sb.alloc_offset, 41);
    assert_eq!(sb.alloc_end, 8135);
    assert_eq!(sb.rootdir_cluster, 0);
    assert_eq!(sb.ifc_list, vec![8]);
    assert_eq!(sb.card_type, 2);
    assert_eq!(sb.card_flags, 43);

    let g = Geometry::from_superblock(&sb, CardKind::WithSpare);
    assert_eq!(g.page_size, 512);
    assert_eq!(g.spare_size, 16);
    assert_eq!(g.raw_page_size, 528);
    assert_eq!(g.cluster_size, 1024);
    assert_eq!(g.fat_per_cluster, 256);
    assert_eq!(g.total_pages(), 16_384);

    // Addresses are strided by the RAW page size (528), not the data size
    // (512): a cluster is two 528-byte pages apart, not 1024 bytes.
    assert_eq!(g.page_offset(1), 528);
    assert_eq!(g.cluster_offset(0), 41 * 2 * 528);
    assert_eq!(g.cluster_offset(2), 43 * 2 * 528);
    assert_eq!(g.total_pages() * g.raw_page_size, 8_650_752);
}

#[test]
fn a_short_or_non_card_input_is_rejected() {
    assert!(Superblock::parse(&[0u8; 100]).is_err());
    let mut card = common::memcard_fixture();
    card[0] = b'X'; // break the magic
    assert!(Superblock::parse(&card).is_err());
}

#[test]
fn card_kind_is_decided_by_length() {
    assert_eq!(CardKind::of_size(8_650_752), Some(CardKind::WithSpare));
    assert_eq!(CardKind::of_size(8_388_608), Some(CardKind::DataOnly));
    assert_eq!(CardKind::of_size(1234), None);
}

use dw4core::memcard::page_spare;

#[test]
fn computed_ecc_matches_the_real_card_on_every_in_use_page() {
    // The card stores its own ECC in each page's 16-byte spare area, so it is
    // the oracle. The known exception is page 1, inside the pre-allocated
    // superblock region, which write_save never touches.
    let card = common::memcard_fixture();
    let page = common::CARD_PAGE_SIZE;
    let data = 512usize;

    let mut matched = 0;
    let mut mismatched = Vec::new();
    for n in 0..card.len() / page {
        let at = n * page;
        if card[at..at + page] == [0xffu8; 528][..] {
            continue; // erased page: spare is 0xFF, not a code
        }
        let stored = &card[at + data..at + page];
        let computed = page_spare(&card[at..at + data]);
        if computed == stored {
            matched += 1;
        } else {
            mismatched.push(n);
        }
    }

    assert_eq!(matched, 351, "in-use pages with matching ECC");
    assert_eq!(
        mismatched,
        vec![1],
        "the only mismatch is page 1, in the superblock region"
    );
}

#[test]
fn the_trailing_four_spare_bytes_are_zero_on_in_use_pages() {
    let card = common::memcard_fixture();
    let page = common::CARD_PAGE_SIZE;
    for n in 0..card.len() / page {
        let at = n * page;
        if card[at..at + page] == [0xffu8; 528][..] {
            continue;
        }
        assert_eq!(
            &card[at + 512 + 12..at + page],
            &[0u8; 4],
            "page {n} trailing spare bytes"
        );
    }
}

use dw4core::memcard::Entry;

/// The real card's geometry, so offsets in tests cannot drift from the code.
fn real_geometry() -> Geometry {
    let card = common::memcard_fixture();
    let sb = Superblock::parse(&card).expect("superblock");
    Geometry::from_superblock(&sb, CardKind::WithSpare)
}

/// Byte offset of entry `slot` (0-based) within relative cluster `cluster`.
fn entry_offset(cluster: u32, slot: usize) -> usize {
    real_geometry().cluster_data_offset(cluster, slot * 512)
}

#[test]
fn the_root_directory_entry_decodes() {
    // Cluster 0 (relative) holds the root directory. Its `.` entry has length
    // 3 and cluster 0.
    let card = common::memcard_fixture();
    let at = entry_offset(0, 0);
    let root = Entry::parse(&card[at..at + 512]).expect("the root entry decodes");
    assert_eq!(root.name(), ".");
    assert_eq!(root.length(), 3, "the root bounds its children by this");
    assert_eq!(root.cluster(), 0);
    assert!(root.is_dir());
    assert!(root.exists());
    assert!(!root.is_file());
    assert!(root.is_dot());
}

#[test]
fn the_dot_dot_entry_decodes_and_is_filtered_as_a_dot_entry() {
    let card = common::memcard_fixture();
    let at = entry_offset(0, 1);
    let e = Entry::parse(&card[at..at + 512]).expect("decodes");
    assert_eq!(e.name(), "..");
    assert!(e.is_dir());
    assert!(
        e.is_dot(),
        "`..` must be filtered out of a directory listing like `.`"
    );
}

#[test]
fn the_save_directory_entry_decodes() {
    let card = common::memcard_fixture();
    let at = entry_offset(1, 0);
    let e = Entry::parse(&card[at..at + 512]).expect("decodes");
    assert_eq!(e.name(), "BASLUS-20836savedata");
    assert_eq!(e.cluster(), 2);
    assert_eq!(e.length(), 5, "five entries including . and ..");
    assert!(e.is_dir());
}

#[test]
fn an_entry_round_trips_through_parse_and_encode() {
    let card = common::memcard_fixture();
    let at = entry_offset(1, 0);
    let original = &card[at..at + 512];
    let e = Entry::parse(original).expect("decodes");
    let encoded = e.encode();
    let reparsed = Entry::parse(&encoded).expect("re-decodes");
    assert_eq!(e, reparsed);
    // The whole 512-byte slot survives, so timestamps and the fields this crate
    // does not interpret are preserved on write.
    assert_eq!(encoded, original[..512], "the slot round-trips verbatim");
}
