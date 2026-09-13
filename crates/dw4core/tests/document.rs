//! End-to-end behaviour of the document layer: load, edit, save, reload.
use dw4core::builder::{SaveSpec, build_save};
use dw4core::document::{DifficultyChoice, Document, EditSet, Mode, StoryEdit};
use dw4core::flags::Difficulty;

mod common;

fn real_save() -> Vec<u8> {
    std::fs::read(common::fixture("mcd001/save.raw")).expect("the real card fixture")
}

#[test]
fn loading_the_real_card_save_produces_a_valid_view() {
    let doc = Document::from_bytes(&real_save()).expect("parses");
    let v = doc.view(Mode::Normal);
    assert_eq!(v.name, "abc");
    assert_eq!(v.level, 1);
    assert!(v.checksum_ok);
    assert_eq!(v.device[0], 0x0000_050D);
    assert_eq!(v.device[1], 0x0000_0513);
    assert_eq!(v.device[2], 0x0000_0512);
    assert_eq!(v.weapons, [0, 1, 2]);
}

#[test]
fn a_raw_save_round_trips_byte_for_byte() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("save.raw");
    let original = real_save();
    std::fs::write(&path, &original).expect("write");

    let mut doc = Document::load_raw(&path).expect("load");
    doc.save(&path).expect("save");
    assert_eq!(std::fs::read(&path).expect("read back"), original);
}

#[test]
fn edits_survive_a_save_and_reload() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("save.raw");
    std::fs::write(&path, build_save(&SaveSpec::default())).expect("write");

    let mut doc = Document::load_raw(&path).expect("load");
    let edits = EditSet {
        bit: 42_424,
        name: "ZZZ".to_string(),
        story: vec![StoryEdit::flag(0, true)],
        difficulty: DifficultyChoice::Fixed(Difficulty::Normal),
        ..Default::default()
    };
    doc.apply(&edits, Mode::Normal).expect("apply");
    doc.save(&path).expect("save");

    let reloaded = Document::load_raw(&path).expect("reload");
    assert_eq!(reloaded.data().bit(), 42_424);
    assert_eq!(reloaded.data().player_name(), "ZZZ");
    assert_eq!(reloaded.data().raw_flags()[0], 1);
    assert_eq!(reloaded.data().raw_flags()[699], 1);
    assert!(reloaded.loaded_checksums_ok());
}

#[test]
fn saving_leaves_a_backup_of_the_previous_contents() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("save.raw");
    let original = build_save(&SaveSpec::default());
    std::fs::write(&path, &original).expect("write");

    let mut doc = Document::load_raw(&path).expect("load");
    doc.apply(
        &EditSet {
            bit: 1,
            ..Default::default()
        },
        Mode::Normal,
    )
    .expect("apply");
    doc.save(&path).expect("save");

    let bak = dir.path().join("save.raw.bak");
    assert!(bak.exists(), "a .bak sibling is written on first overwrite");
    assert_eq!(std::fs::read(&bak).expect("read bak"), original);
}

#[test]
fn a_second_save_does_not_overwrite_the_original_backup() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("save.raw");
    let original = build_save(&SaveSpec::default());
    std::fs::write(&path, &original).expect("write");

    let mut doc = Document::load_raw(&path).expect("load");
    doc.apply(
        &EditSet {
            bit: 1,
            ..Default::default()
        },
        Mode::Normal,
    )
    .expect("apply");
    doc.save(&path).expect("first save");
    doc.apply(
        &EditSet {
            bit: 2,
            ..Default::default()
        },
        Mode::Normal,
    )
    .expect("apply");
    doc.save(&path).expect("second save");

    let bak = dir.path().join("save.raw.bak");
    assert_eq!(
        std::fs::read(&bak).expect("read bak"),
        original,
        "the .bak still holds the pre-session contents"
    );
}

#[test]
fn loading_a_wrong_sized_file_is_an_error() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("short.raw");
    std::fs::write(&path, vec![0u8; 100]).expect("write");
    assert!(Document::load_raw(&path).is_err());
}
