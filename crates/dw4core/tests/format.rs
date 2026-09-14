//! A card synthesised from nothing must read back as a real card.

mod common;

use dw4core::memcard::{Geometry, Ps2Memcard, SAVE_DIR, SAVE_FILE, format_card};

fn save_bytes() -> Vec<u8> {
    std::fs::read(common::fixture("mcd001/save.raw")).expect("fixture")
}

#[test]
fn the_save_round_trips_through_a_synthesised_card() {
    let save = save_bytes();
    let card = format_card(&save).expect("formats");
    assert_eq!(card.len(), 8_650_752);

    let mut memcard = Ps2Memcard::from_image(card).expect("parses as a card");
    assert_eq!(memcard.read_save(SAVE_DIR, SAVE_FILE).expect("reads"), save);
}

#[test]
fn a_wrong_sized_save_is_refused() {
    let err = format_card(&[0u8; 4]).expect_err("must refuse");
    assert!(matches!(err, dw4core::Error::BadSaveSize { .. }), "{err:?}");
}

#[test]
fn page_zero_matches_the_reference_card() {
    use dw4core::memcard::page_spare;
    let reference = common::memcard_fixture();
    let ours = format_card(&save_bytes()).expect("formats");
    assert_eq!(&ours[..512], &reference[..512], "page 0 data");
    assert_eq!(&ours[512..528], &reference[512..528], "page 0 spare");
    assert_eq!(page_spare(&ours[..512]).as_slice(), &ours[512..528]);
}

#[test]
fn page_one_is_left_erased() {
    let ours = format_card(&save_bytes()).expect("formats");
    assert!(ours[528..1056].iter().all(|b| *b == 0xFF));
}

#[test]
fn every_in_use_page_has_matching_ecc() {
    let card = format_card(&save_bytes()).expect("formats");
    let geom = Geometry::from_superblock(
        &dw4core::memcard::Superblock::parse(&card).expect("superblock"),
        dw4core::memcard::CardKind::WithSpare,
    );
    let mut checked = 0;
    for n in 0..card.len() / geom.raw_page_size {
        let at = geom.page_offset(n);
        let page = &card[at..at + geom.raw_page_size];
        if page.iter().all(|b| *b == 0xFF) {
            continue;
        }
        checked += 1;
        let data = &page[..geom.page_size];
        let spare = &page[geom.page_size..];
        assert_eq!(
            dw4core::memcard::page_spare(data).as_slice(),
            spare,
            "page {n}"
        );
        assert_eq!(
            &spare[12..16],
            &[0, 0, 0, 0],
            "trailing spare bytes on page {n}"
        );
    }
    assert!(checked > 100, "only {checked} in-use pages");
}

#[test]
fn the_icons_are_the_committed_assets() {
    let card = format_card(&save_bytes()).expect("formats");
    let mut memcard = Ps2Memcard::from_image(card).expect("parses");
    let icons = memcard.read_save(SAVE_DIR, "icon1.ico").expect("icon1.ico");
    assert_eq!(icons.as_slice(), include_bytes!("../data/card/icon1.ico"));
    let sys = memcard.read_save(SAVE_DIR, "icon.sys").expect("icon.sys");
    assert_eq!(sys.as_slice(), include_bytes!("../data/card/icon.sys"));
}
