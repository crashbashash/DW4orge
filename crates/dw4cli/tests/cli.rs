//! End-to-end CLI behaviour.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dw4cli"))
}

fn fixture(relative: &str) -> String {
    format!(
        "{}/../dw4core/tests/fixtures/{relative}",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[test]
fn info_json_describes_the_raw_fixture() {
    let out = bin()
        .args(["info", "--json", &fixture("mcd001/save.raw")])
        .output()
        .expect("runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(value["source"], "raw");
    assert_eq!(value["view"]["checksum_ok"], true);
    assert_eq!(value["view"]["species"], "Dorumon");
}

#[test]
fn info_human_is_not_json() {
    let out = bin()
        .args(["info", &fixture("mcd001/save.raw")])
        .output()
        .expect("runs");
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).expect("utf8");
    assert!(text.contains("container:"), "{text}");
    assert!(text.contains("checksum:"), "{text}");
}

#[test]
fn info_on_a_missing_file_exits_one() {
    let out = bin()
        .args(["info", "/nonexistent/x.raw"])
        .output()
        .expect("runs");
    assert_eq!(out.status.code(), Some(1));
    assert!(!out.stderr.is_empty());
}

#[test]
fn items_lists_the_catalogue_as_json() {
    let out = bin()
        .args(["items", "--json", "--limit", "5"])
        .output()
        .expect("runs");
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(value.as_array().expect("array").len(), 5);
}

#[test]
fn items_filters_by_category_and_query() {
    let out = bin()
        .args([
            "items",
            "--json",
            "--category",
            "weapon",
            "--query",
            "sword",
        ])
        .output()
        .expect("runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    let items = value.as_array().expect("array");
    assert!(!items.is_empty());
    assert!(items.iter().all(|i| i["category"] == "weapon"));
}

#[test]
fn dump_json_matches_the_view_shape() {
    let out = bin()
        .args(["dump", "--json", &fixture("mcd001/save.raw")])
        .output()
        .expect("runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert!(value["device"].is_array(), "{value}");
    assert_eq!(value["species"], "Dorumon");
}

#[test]
fn new_writes_a_raw_save_that_reopens() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = dir.path().join("fresh.raw");
    let out = bin()
        .args([
            "new",
            "--species",
            "Agumon",
            "--name",
            "abc",
            "--out",
            out_path.to_str().unwrap(),
        ])
        .output()
        .expect("runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(std::fs::read(&out_path).unwrap().len(), 81_920);

    let check = bin()
        .args(["info", "--json", out_path.to_str().unwrap()])
        .output()
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(value["view"]["species"], "Agumon");
    assert_eq!(value["view"]["name"], "abc");
}

#[test]
fn new_to_ps2_without_a_card_creates_one() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = dir.path().join("fresh.ps2");
    let out = bin()
        .args([
            "new",
            "--species",
            "Agumon",
            "--name",
            "abc",
            "--out",
            out_path.to_str().unwrap(),
        ])
        .output()
        .expect("runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(std::fs::read(&out_path).unwrap().len(), 8_650_752);

    // It re-opens as a card and carries the requested character.
    let check = bin()
        .args(["info", "--json", out_path.to_str().unwrap()])
        .output()
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(value["source"], "memcard");
    assert_eq!(value["view"]["species"], "Agumon");
    assert_eq!(value["view"]["name"], "abc");
}

#[test]
fn verify_reports_a_good_raw_save() {
    let out = bin()
        .args(["verify", "--json", &fixture("mcd001/save.raw")])
        .output()
        .expect("runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(value["checksum_ok"], true);
    assert_eq!(value["problems"].as_array().unwrap().len(), 0);
}

#[test]
fn verify_of_a_corrupt_save_exits_one() {
    let dir = tempfile::tempdir().unwrap();
    let mut bytes = std::fs::read(fixture("mcd001/save.raw")).unwrap();
    bytes[0x10] ^= 0xFF;
    let target = dir.path().join("corrupt.raw");
    std::fs::write(&target, &bytes).unwrap();
    let out = bin()
        .args(["verify", target.to_str().unwrap()])
        .output()
        .expect("runs");
    assert_eq!(out.status.code(), Some(1));
}
