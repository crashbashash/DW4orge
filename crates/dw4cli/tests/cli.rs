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

#[test]
fn regional_cards_can_be_created_detected_verified_and_used_as_donors() {
    let dir = tempfile::tempdir().unwrap();
    for region in ["usa", "europe", "japan"] {
        let card = dir.path().join(format!("{region}.ps2"));
        let made = bin()
            .args(["new", "--region", region, "--out", card.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(
            made.status.success(),
            "{}",
            String::from_utf8_lossy(&made.stderr)
        );
        let info = bin()
            .args(["info", "--json", card.to_str().unwrap()])
            .output()
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&info.stdout).unwrap();
        assert_eq!(result["region"], region);
        assert!(
            bin()
                .args(["verify", card.to_str().unwrap()])
                .status()
                .unwrap()
                .success()
        );
        let copy = dir.path().join(format!("{region}-copy.ps2"));
        assert!(
            bin()
                .args([
                    "new",
                    "--card",
                    card.to_str().unwrap(),
                    "--name",
                    "COPY",
                    "--out",
                    copy.to_str().unwrap()
                ])
                .status()
                .unwrap()
                .success()
        );
        let info = bin()
            .args(["info", "--json", copy.to_str().unwrap()])
            .output()
            .unwrap();
        let result: serde_json::Value = serde_json::from_slice(&info.stdout).unwrap();
        assert_eq!(result["region"], region);
        assert_eq!(result["view"]["name"], "COPY");
        let data_only = dir.path().join(format!("{region}-data-only.ps2"));
        let bytes = std::fs::read(&card).unwrap();
        let bytes: Vec<u8> = bytes
            .as_chunks::<528>()
            .0
            .iter()
            .flat_map(|p| p[..512].iter().copied())
            .collect();
        std::fs::write(&data_only, bytes).unwrap();
        assert!(
            bin()
                .args(["verify", data_only.to_str().unwrap()])
                .status()
                .unwrap()
                .success()
        );
    }
}

#[test]
fn ps2_character_boxes_are_listed_selected_and_invalid_choices_fail() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boxes.bin");
    let mut data = dw4core::build_save(&dw4core::SaveSpec::default());
    for mirror in 0..2 {
        let base = mirror * dw4core::BLOCK;
        let record = data[base + 8..base + 0x2000].to_vec();
        data[base + 0x6008..base + 0x8000].copy_from_slice(&record);
        data[base + 8..base + 12].fill(0); // only box 4 is occupied
    }
    dw4core::fix_checksums(&mut data);
    std::fs::write(&path, data).unwrap();
    let out = bin()
        .args(["info", path.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["character_selection"], "ps2#4");
    assert_eq!(value["character_boxes"][0]["box_number"], 4);
    for flag in ["--box", "--xbox-slot"] {
        let out = bin()
            .args(["info", path.to_str().unwrap(), flag, "ps2#4"])
            .output()
            .unwrap();
        assert!(out.status.success());
        assert!(String::from_utf8(out.stdout).unwrap().contains("ps2#4"));
    }
    let out = bin()
        .args(["dump", path.to_str().unwrap(), "--box", "ps2#1"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("occupied PS2")
    );
}

#[test]
fn gamecube_boxes_can_be_selected_from_gci_card_and_bare_files() {
    let dir = tempfile::tempdir().unwrap();
    let mut raw =
        dw4core::build_save_with_order(&dw4core::SaveSpec::default(), dw4core::ByteOrder::Big);
    for mirror in 0..2 {
        let base = mirror * dw4core::BLOCK;
        let record = raw[base + 8..base + 0x2008].to_vec();
        raw[base + 0x6008..base + 0x8008].copy_from_slice(&record);
        raw[base + 8] = 0;
    }
    dw4core::fix_checksums_with_order(&mut raw, dw4core::ByteOrder::Big);
    let mut doc = dw4core::document::Document::from_bytes(&raw).unwrap();
    for ext in ["gci", "raw", "bin"] {
        let path = dir.path().join(format!("boxes.{ext}"));
        doc.save(&path).unwrap();
        let out = bin()
            .args([
                "info",
                path.to_str().unwrap(),
                "--box",
                "gamecube#4",
                "--json",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(value["character_selection"], "gamecube#4");
        assert_eq!(value["character_boxes"][0]["box_number"], 4);
        let bad = bin()
            .args(["dump", path.to_str().unwrap(), "--box", "gamecube#1"])
            .output()
            .unwrap();
        assert!(!bad.status.success());
    }
}

#[test]
fn create_box_cli_adds_a_character_without_overwriting_occupied_boxes_or_source_files() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.ps2");
    let output = dir.path().join("output.ps2");
    let raw = dw4core::build_save(&dw4core::SaveSpec::default());
    let original = dw4core::memcard::format_card_for_region(&raw, dw4core::Region::Europe).unwrap();
    std::fs::write(&input, &original).unwrap();
    let out = bin()
        .args([
            "create-box",
            input.to_str().unwrap(),
            "--number",
            "4",
            "--species",
            "Veemon",
            "--name",
            "CLI4",
            "--out",
            output.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["character_selection"], "ps2#4");
    assert_eq!(value["view"]["name"], "CLI4");
    assert_eq!(value["pending_write"], false);
    assert_eq!(value["region"], "europe");
    assert_eq!(std::fs::read(&input).unwrap(), original);
    let before = std::fs::read(&output).unwrap();
    for n in ["1", "4", "0", "5"] {
        let out = bin()
            .args([
                "create-box",
                output.to_str().unwrap(),
                "--number",
                n,
                "--out",
                output.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert_eq!(std::fs::read(&output).unwrap(), before);
    }
    let out = bin()
        .args([
            "create-box",
            output.to_str().unwrap(),
            "--number",
            "2",
            "--name",
            "NEXT2",
            "--out",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(output.with_extension("ps2.bak").exists());
}

#[test]
fn create_box_cli_supports_all_gamecube_containers_and_leaves_occupied_boxes_untouched() {
    let dir = tempfile::tempdir().unwrap();
    for ext in ["gci", "raw", "bin"] {
        let source = dir.path().join(format!("input.{ext}"));
        let output = dir.path().join(format!("output.{ext}"));
        let raw =
            dw4core::build_save_with_order(&dw4core::SaveSpec::default(), dw4core::ByteOrder::Big);
        let mut doc = dw4core::document::Document::from_bytes(&raw).unwrap();
        doc.save(&source).unwrap();
        let original = std::fs::read(&source).unwrap();
        let out = bin()
            .args([
                "create-box",
                source.to_str().unwrap(),
                "--number",
                "4",
                "--name",
                "CLI4",
                "--species",
                "Veemon",
                "--out",
                output.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(value["character_selection"], "gamecube#4");
        assert_eq!(value["view"]["name"], "CLI4");
        assert_eq!(value["pending_write"], false);
        let before = std::fs::read(&output).unwrap();
        for n in ["0", "1", "4", "5"] {
            let bad = bin()
                .args([
                    "create-box",
                    output.to_str().unwrap(),
                    "--number",
                    n,
                    "--out",
                    output.to_str().unwrap(),
                ])
                .output()
                .unwrap();
            assert!(!bad.status.success());
            assert_eq!(std::fs::read(&output).unwrap(), before);
        }
        assert_eq!(std::fs::read(&source).unwrap(), original);
    }
}
