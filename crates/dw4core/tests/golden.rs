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

fn expected() -> serde_json::Value {
    let text = std::fs::read_to_string(fixture_dir().join("expected.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn header_and_character_match_the_python_oracle() {
    let e = expected();
    let save =
        dw4core::SaveData::parse(&std::fs::read(fixture_dir().join("save.raw")).unwrap()).unwrap();

    assert!(save.verify());
    assert_eq!(save.version() as u64, e["version"].as_u64().unwrap());
    assert_eq!(save.isuse() as u64, e["isuse"].as_u64().unwrap());
    assert_eq!(save.unique() as u64, e["unique"].as_u64().unwrap());
    assert_eq!(save.digimon_name(), e["digimon_name"].as_str().unwrap());
    assert_eq!(save.bit() as u64, e["bit"].as_u64().unwrap());
    assert_eq!(save.xdata() as u64, e["xdata"].as_u64().unwrap());
    assert_eq!(save.menu_level() as u64, e["menu_level"].as_u64().unwrap());
    assert_eq!(
        save.junk_counter() as u64,
        e["junk_counter"].as_u64().unwrap()
    );
    assert_eq!(
        save.get_u32(dw4core::offsets::CHECKSUM) as u64,
        e["checksum_block"].as_u64().unwrap()
    );
}

#[test]
fn detected_species_matches_the_python_oracle() {
    let e = expected();
    let save =
        dw4core::SaveData::parse(&std::fs::read(fixture_dir().join("save.raw")).unwrap()).unwrap();
    assert_eq!(
        save.detect_species().index() as u64,
        e["detected_species"].as_u64().unwrap()
    );
}
