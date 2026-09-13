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

    let g = Geometry::from_superblock(&sb);
    assert_eq!(g.page_size, 512);
    assert_eq!(g.spare_size, 16);
    assert_eq!(g.raw_page_size, 528);
    assert_eq!(g.cluster_size, 1024);
    assert_eq!(g.fat_per_cluster, 256);
    assert_eq!(g.total_pages(), 16_384);

    // Addressing: page 1 starts one raw page in; cluster 0 is the first data
    // cluster, at alloc_offset.
    assert_eq!(g.page_offset(1), 528);
    assert_eq!(g.cluster_offset(0), 41 * 1024);
    assert_eq!(g.cluster_offset(2), 43 * 1024);
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
