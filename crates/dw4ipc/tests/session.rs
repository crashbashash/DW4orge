//! Session lifecycle over the committed fixtures.

mod common;

use dw4core::document::DifficultyChoice;
use dw4core::{Difficulty, EMPTY, Mode, Species};
use dw4ipc::{EditorSession, NewSaveRequest, SourceKind};

fn raw_fixture() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../dw4core/tests/fixtures/mcd001/save.raw"
    ))
}

#[test]
fn open_reads_the_raw_fixture() {
    let mut session = EditorSession::new_session();
    let result = session.open(&raw_fixture()).expect("opens");
    assert_eq!(result.source, SourceKind::Raw);
    assert_eq!(result.view.species, Species::Dorumon);
    assert!(result.view.checksum_ok);
    assert!(result.path.is_some());
}

#[test]
fn open_rejects_a_missing_file() {
    let mut session = EditorSession::new_session();
    let err = session
        .open(std::path::Path::new("/nonexistent/nope.raw"))
        .expect_err("must fail");
    assert!(matches!(err, dw4ipc::IpcError::Core { .. }), "{err:?}");
}

#[test]
fn view_needs_an_open_document() {
    let session = EditorSession::new_session();
    let err = session.view(Mode::Normal).expect_err("must fail");
    assert_eq!(err, dw4ipc::IpcError::NoOpenDocument);
}

#[test]
fn new_save_builds_the_requested_character() {
    let mut session = EditorSession::new_session();
    let req = NewSaveRequest {
        species: Species::Agumon,
        name: "abc".to_string(),
        story: None,
        difficulty: DifficultyChoice::Auto,
    };
    let result = session.new_save(&req).expect("builds");
    assert_eq!(result.path, None);
    assert_eq!(result.source, SourceKind::Raw);
    assert_eq!(result.view.species, Species::Agumon);
    assert_eq!(result.view.name, "abc");
}

#[test]
fn new_save_applies_a_story_preset() {
    let mut session = EditorSession::new_session();
    let req = NewSaveRequest {
        species: Species::Dorumon,
        name: "TST".to_string(),
        story: Some("Fresh (tutorial)".to_string()),
        difficulty: DifficultyChoice::Fixed(Difficulty::Normal),
    };
    let result = session.new_save(&req).expect("builds");
    assert!(result.view.story_flags.iter().any(|b| *b != 0));
}

#[test]
fn new_save_rejects_an_unknown_story() {
    let mut session = EditorSession::new_session();
    let req = NewSaveRequest {
        species: Species::Dorumon,
        name: "TST".to_string(),
        story: Some("nope".to_string()),
        difficulty: DifficultyChoice::Auto,
    };
    let err = session.new_save(&req).expect_err("must fail");
    assert!(
        matches!(err, dw4ipc::IpcError::Unsupported { .. }),
        "{err:?}"
    );
}

#[test]
fn the_reconstructed_card_reads_back_as_the_raw_fixture() {
    // Independent oracle: the duplicate fixture loader is correct only if the
    // card's save equals save.raw.
    let card = common::memcard_fixture();
    let save = dw4core::memcard::load_save_from_bytes(&card).expect("reads");
    let raw = std::fs::read(common::fixture("mcd001/save.raw")).expect("raw");
    assert_eq!(save, raw);
}

#[test]
fn validate_reports_field_errors() {
    let mut session = EditorSession::new_session();
    let opened = session.open(&raw_fixture()).expect("opens");
    let mut edits = opened.view.to_edit_set();
    edits.bit = 10_000_000; // above CAP_BIT.normal_max
    let err = session
        .validate(&edits, Mode::Normal)
        .expect_err("rejected");
    match err {
        dw4ipc::IpcError::Validation { fields } => {
            assert_eq!(fields[0].path, "bit");
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn a_rejected_edit_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("save.raw");
    std::fs::copy(raw_fixture(), &target).unwrap();
    let before = std::fs::read(&target).unwrap();

    let mut session = EditorSession::new_session();
    let opened = session.open(&target).unwrap();
    let mut edits = opened.view.to_edit_set();
    edits.bit = 10_000_000;
    assert!(session.validate(&edits, Mode::Normal).is_err());
    // validate is the gate; no save call happens on rejection.
    assert_eq!(std::fs::read(&target).unwrap(), before);
}

#[test]
fn save_round_trips_through_a_temp_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("save.raw");
    std::fs::copy(raw_fixture(), &target).unwrap();

    let mut session = EditorSession::new_session();
    let opened = session.open(&target).unwrap();
    let mut edits = opened.view.to_edit_set();
    edits.bit = 1234;
    edits.device[0] = EMPTY;
    session.validate(&edits, Mode::Normal).expect("valid");
    let saved = session.save(&edits, Mode::Normal).expect("saves");
    assert_eq!(saved.view.bit, 1234);
    assert!(saved.view.checksum_ok);
    assert!(target.with_extension("raw.bak").exists());
}

#[test]
fn save_without_a_path_asks_for_save_as() {
    let mut session = EditorSession::new_session();
    let opened = session
        .new_save(&NewSaveRequest {
            species: Species::Dorumon,
            name: "TST".to_string(),
            story: None,
            difficulty: DifficultyChoice::Auto,
        })
        .unwrap();
    let edits = opened.view.to_edit_set();
    let err = session.save(&edits, Mode::Normal).expect_err("must fail");
    assert!(
        matches!(err, dw4ipc::IpcError::Unsupported { .. }),
        "{err:?}"
    );
}

#[test]
fn save_as_creates_a_new_ps2_only_from_a_source_card() {
    let dir = tempfile::tempdir().unwrap();

    // No source card: creating a new .ps2 must be refused.
    let mut session = EditorSession::new_session();
    let opened = session
        .new_save(&NewSaveRequest {
            species: Species::Dorumon,
            name: "TST".to_string(),
            story: None,
            difficulty: DifficultyChoice::Auto,
        })
        .unwrap();
    let edits = opened.view.to_edit_set();
    let err = session
        .save_as(&dir.path().join("fresh.ps2"), &edits, Mode::Normal)
        .expect_err("must fail");
    match err {
        dw4ipc::IpcError::Core { variant, .. } => assert_eq!(variant, "NoSave"),
        other => panic!("expected Core NoSave, got {other:?}"),
    }

    // With a source card, the same write succeeds and stays a card.
    let card = common::card_file(dir.path());
    let mut session = EditorSession::new_session();
    let opened = session
        .new_save_on_card(
            &NewSaveRequest {
                species: Species::Agumon,
                name: "abc".to_string(),
                story: None,
                difficulty: DifficultyChoice::Auto,
            },
            &card,
        )
        .unwrap();
    let edits = opened.view.to_edit_set();
    let out = dir.path().join("copy.ps2");
    let saved = session
        .save_as(&out, &edits, Mode::Normal)
        .expect("writes a card");
    assert_eq!(saved.source, dw4ipc::SourceKind::Memcard);
    assert_eq!(std::fs::read(&out).unwrap().len(), 8_650_752);
}
