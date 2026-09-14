//! Payload shapes the frontend will depend on.

use dw4core::document::DifficultyChoice;
use dw4core::{Difficulty, Mode, SaveView, Species};
use dw4ipc::{IpcError, OpenResult, SourceKind};

fn sample_view() -> SaveView {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../dw4core/tests/fixtures/mcd001/save.raw"
    ))
    .expect("fixture");
    dw4core::document::Document::from_bytes(&bytes)
        .expect("parses")
        .view(Mode::Normal)
}

#[test]
fn open_result_round_trips_through_json() {
    let result = OpenResult {
        path: Some("/tmp/example.raw".to_string()),
        source: SourceKind::Raw,
        view: sample_view(),
    };
    let json = serde_json::to_string(&result).expect("serialises");
    let back: OpenResult = serde_json::from_str(&json).expect("deserialises");
    assert_eq!(back, result);
}

#[test]
fn source_kind_is_lowercase() {
    assert_eq!(
        serde_json::to_string(&SourceKind::Memcard).unwrap(),
        "\"memcard\""
    );
    assert_eq!(serde_json::to_string(&SourceKind::Raw).unwrap(), "\"raw\"");
}

#[test]
fn ipc_error_is_tagged() {
    let err = IpcError::Validation { fields: Vec::new() };
    let json = serde_json::to_string(&err).unwrap();
    assert!(json.contains("\"kind\":\"validation\""), "{json}");

    let err = IpcError::Core {
        variant: "NoSave".to_string(),
        message: "boom".to_string(),
    };
    let json = serde_json::to_string(&err).unwrap();
    assert!(json.contains("\"variant\":\"NoSave\""), "{json}");
}

#[test]
fn core_errors_map_to_named_variants() {
    let core = dw4core::Error::BadSaveSize {
        expected: 81_920,
        actual: 4,
    };
    let err = IpcError::from(core);
    match err {
        IpcError::Core { variant, message } => {
            assert_eq!(variant, "BadSaveSize");
            assert!(message.contains("81920"), "{message}");
        }
        other => panic!("expected Core, got {other:?}"),
    }
}

#[test]
fn new_save_request_uses_core_enums() {
    let req = dw4ipc::NewSaveRequest {
        species: Species::Dorumon,
        name: "abc".to_string(),
        story: Some("Fresh (tutorial)".to_string()),
        difficulty: DifficultyChoice::Fixed(Difficulty::Hard),
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"Dorumon\""), "{json}");
}
