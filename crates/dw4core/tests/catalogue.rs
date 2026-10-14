//! Compares the whole item catalogue against the Python editor's reading of it.
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/catalogue")
}

fn load(name: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(fixture_dir().join(name))
        .unwrap_or_else(|e| panic!("read {name}: {e}"));
    serde_json::from_str(&text).expect("valid json")
}

#[test]
fn the_catalogue_has_the_same_size_as_the_oracle() {
    let expected = load("items.json");
    assert_eq!(
        dw4core::get_catalogue().len(),
        expected.as_array().unwrap().len()
    );
}

#[test]
fn every_catalogue_entry_matches_the_oracle() {
    let expected = load("items.json");
    let cat = dw4core::get_catalogue();
    let mut checked = 0usize;

    for entry in expected.as_array().unwrap() {
        let base = entry["base"].as_u64().unwrap() as u32;
        let item = cat
            .get(base)
            .unwrap_or_else(|| panic!("oracle has 0x{base:04X}, the crate does not"));
        assert_eq!(
            item.name,
            entry["name"].as_str().unwrap(),
            "0x{base:04X} name"
        );
        assert_eq!(
            item.category.byte() as u64,
            entry["cat"].as_u64().unwrap(),
            "0x{base:04X} category"
        );
        let expected_grade = entry["grade"].as_u64().map(|g| g as u8);
        assert_eq!(item.grade, expected_grade, "0x{base:04X} grade");
        checked += 1;
    }

    assert_eq!(
        checked,
        cat.len(),
        "the oracle and the crate agree on count"
    );
}

#[test]
fn every_description_matches_the_oracle() {
    let expected = load("describe.json");
    let map = expected.as_object().unwrap();
    assert!(map.len() > 500, "the fixture should cover the catalogue");

    for (id, want) in map {
        let id: u32 = id.parse().expect("decimal id keys");
        assert_eq!(
            dw4core::describe_item_id(id),
            want.as_str().unwrap(),
            "describe_item_id(0x{id:08X})"
        );
    }
}

#[test]
fn every_validity_verdict_matches_the_oracle() {
    let expected = load("invalid.json");
    let map = expected.as_object().unwrap();

    for (base, want) in map {
        let base: u32 = base.parse().expect("decimal id keys");
        let got = dw4core::invalid_reason(base);
        match want {
            serde_json::Value::Null => {
                assert_eq!(got, None, "0x{base:04X} should be valid, got {got:?}");
            }
            serde_json::Value::String(s) => {
                assert_eq!(
                    got.as_deref(),
                    Some(s.as_str()),
                    "invalid_reason(0x{base:04X})"
                );
            }
            other => panic!("unexpected fixture value {other:?}"),
        }
    }
}
