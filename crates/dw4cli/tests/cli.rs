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
