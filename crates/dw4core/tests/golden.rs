//! Pins the Rust core against the Python editor's reading of a real save.
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mcd001")
}

#[test]
fn fixture_exists_and_is_a_save() {
    let raw = std::fs::read(fixture_dir().join("save.raw")).expect("fixture save.raw is committed");
    assert_eq!(raw.len(), dw4core::SAVE_SIZE);
}

#[test]
fn fixture_expected_json_parses() {
    let text = std::fs::read_to_string(fixture_dir().join("expected.json")).expect("expected.json");
    let v: serde_json::Value = serde_json::from_str(&text).expect("valid json");
    assert_eq!(v["version"], 4);
    assert_eq!(v["detected_species"], 3);
    assert_eq!(v["device"].as_array().unwrap().len(), 36);
    assert_eq!(v["bank_device"].as_array().unwrap().len(), 96);
}
