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
    let needle = b"BASLUS-20836savedata";
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
