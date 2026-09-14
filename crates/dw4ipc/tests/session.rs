//! Session lifecycle over the committed fixtures.

use dw4core::document::DifficultyChoice;
use dw4core::{Difficulty, Mode, Species};
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
