//! Parity against the Python editor, via fixtures from `tools/gen_fixtures.py`.
//!
//! `dw4save.SaveData` is the oracle for byte semantics and for the cap table.
//! The *order* of writes is transcribed from `save_editor_gui.py:1197`, which
//! cannot be imported here (tkinter), so the mirror comparison below is
//! restricted to the flags the Python GUI's partial table actually covers.
use dw4core::document::{Document, EditSet, Mode, StoryEdit};
use dw4core::flags::Difficulty;
use serde_json::Value;

mod common;

fn json(relative: &str) -> Value {
    let text = std::fs::read_to_string(common::fixture(relative)).expect("fixture");
    serde_json::from_str(&text).expect("valid json")
}

fn u32s(v: &Value) -> Vec<u32> {
    v.as_array()
        .expect("array")
        .iter()
        .map(|x| x.as_u64().expect("u64") as u32)
        .collect()
}

/// The scripted edit set, matching `dump_document`'s exactly.
fn scripted_edits() -> EditSet {
    let mut device = [dw4core::EMPTY; 30];
    device[0] = dw4core::item::build_item_id(0x0001, 5, 2);
    let mut bank_items = vec![dw4core::EMPTY; 96];
    bank_items[0] = dw4core::item::build_item_id(0x3000, 1, 0);
    EditSet {
        species: dw4core::Species::Dorumon,
        name: "TST".to_string(),
        bit: 123_456,
        xdata: 789,
        junk: 26_000,
        level: 42,
        exp: dw4core::codes::level_threshold(42) as u32,
        tech: [1, 2, 3, 4, 5, 6, 7, 8, 9],
        upcnt: [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110],
        device,
        weapons: [0, dw4core::EMPTY, dw4core::EMPTY],
        armor: dw4core::EMPTY,
        sub: dw4core::EMPTY,
        wmods: [None; 5],
        amods: [None; 5],
        story: vec![
            StoryEdit::flag(Difficulty::Normal, 0, true),
            StoryEdit::flag(Difficulty::Normal, 1, true),
            StoryEdit::folder(Difficulty::Normal, 0, true),
        ],
        bank_bit: 55_555,
        disks: std::array::from_fn(|i| (i * 3) as u16),
        bank_items,
    }
}

#[test]
fn applying_the_scripted_edits_is_byte_identical_to_python() {
    let original = std::fs::read(common::fixture("mcd001/save.raw")).expect("card fixture");
    let expected = std::fs::read(common::fixture("document/applied.raw")).expect("oracle");

    let mut doc = Document::from_bytes(&original).expect("parses");
    // Advanced mode: the scripted disk counts run 0..33, beyond the in-game
    // Normal cap of 9. This test is about apply's bytes matching Python, not
    // about the cap, so it uses the range that can hold the whole script.
    doc.apply(&scripted_edits(), Mode::Advanced)
        .expect("the scripted edits are valid");

    let got = doc.data().to_bytes();
    assert_eq!(got.len(), expected.len());
    if got != expected {
        let first = got
            .iter()
            .zip(&expected)
            .position(|(a, b)| a != b)
            .expect("lengths already checked");
        let differing = got.iter().zip(&expected).filter(|(a, b)| a != b).count();
        panic!(
            "first difference at 0x{first:04X} (block offset 0x{:04X}): rust 0x{:02X}, python 0x{:02X}; \
             {differing} bytes differ in total",
            first % dw4core::BLOCK,
            got[first],
            expected[first]
        );
    }
}

#[test]
fn the_view_matches_python_field_for_field() {
    let expected = json("document/view.json");
    let bytes = std::fs::read(common::fixture("document/applied.raw")).expect("oracle");
    let doc = Document::from_bytes(&bytes).expect("parses");
    // Advanced mode: the fixture records stored values, not the Normal-mode
    // EXP lift.
    let v = doc.view(Mode::Advanced);

    assert_eq!(
        v.species.index() as u64,
        expected["species"].as_u64().unwrap()
    );
    assert_eq!(v.name, expected["name"].as_str().unwrap());
    assert_eq!(v.bit as u64, expected["bit"].as_u64().unwrap());
    assert_eq!(v.xdata as u64, expected["xdata"].as_u64().unwrap());
    assert_eq!(
        v.junk_counter as u64,
        expected["junk_counter"].as_u64().unwrap()
    );
    assert_eq!(v.junk_tier as u64, expected["junk_tier"].as_u64().unwrap());
    assert_eq!(v.level as u64, expected["level"].as_u64().unwrap());
    assert_eq!(v.exp as u64, expected["exp"].as_u64().unwrap());
    assert_eq!(v.device.to_vec(), u32s(&expected["device"]));
    assert_eq!(v.weapons.to_vec(), u32s(&expected["weapons"]));
    assert_eq!(v.armor, expected["armor"].as_u64().unwrap() as u32);
    assert_eq!(v.sub, expected["sub"].as_u64().unwrap() as u32);
    assert_eq!(v.wmods.to_vec(), u32s(&expected["wmods"]));
    assert_eq!(v.amods.to_vec(), u32s(&expected["amods"]));
    assert_eq!(v.bank_bit as u64, expected["bank_bit"].as_u64().unwrap());
    assert_eq!(v.bank_items, u32s(&expected["bank_items"]));
    assert_eq!(
        v.disks.iter().map(|d| u64::from(*d)).collect::<Vec<_>>(),
        expected["disks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap())
            .collect::<Vec<_>>()
    );
    let want_tech: Vec<i64> = expected["tech"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_i64().unwrap())
        .collect();
    assert_eq!(
        v.tech.iter().map(|t| i64::from(*t)).collect::<Vec<_>>(),
        want_tech
    );
    assert_eq!(v.upcnt.to_vec(), u32s(&expected["upcnt"]));
}

#[test]
fn the_cap_table_matches_python() {
    let expected = json("document/validate.json");
    let pairs: [(&str, dw4core::codes::Cap); 6] = [
        ("bit", dw4core::codes::CAP_BIT),
        ("xdata", dw4core::codes::CAP_XDATA),
        ("level", dw4core::codes::CAP_LEVEL),
        ("exp", dw4core::codes::CAP_EXP),
        ("tech", dw4core::codes::CAP_TECH),
        ("upcnt", dw4core::codes::CAP_UPCNT),
    ];
    for (name, cap) in pairs {
        assert_eq!(
            cap.normal_max,
            expected[name]["normal_max"].as_i64().unwrap(),
            "{name} normal_max"
        );
        assert_eq!(
            cap.dtype_max,
            expected[name]["dtype_max"].as_i64().unwrap(),
            "{name} dtype_max"
        );
        assert_eq!(
            cap.dtype_min,
            expected[name]["dtype_min"].as_i64().unwrap(),
            "{name} dtype_min"
        );
    }
    let want = u32s(&expected["upcnt_safe_cap"]);
    let got: Vec<u32> = (0..11)
        .map(|i| dw4core::codes::upcnt_safe_cap(i) as u32)
        .collect();
    assert_eq!(got, want, "per-slot power-up caps");
}

#[test]
fn the_python_gui_mirror_table_is_a_subset_of_ours() {
    // Documented divergence (spec 5.1): the Python apply() mirrors through a
    // partial 10-flag table, ours through the complete 38-row table. Comparing
    // the shared entries proves the scripted story edits agree.
    let mirrors = json("flags/mirrors.json");
    let gui = mirrors["gui"].as_object().expect("gui table");

    for (difficulty, table) in gui {
        let rows = table.as_object().expect("rows");
        for (active, mirror) in rows {
            let active: u32 = active.parse().expect("flag index");
            let mirror = mirror.as_u64().expect("mirror index") as u32;
            let found = dw4core::flags::MIRRORS
                .iter()
                .find(|r| r.active == active)
                .unwrap_or_else(|| panic!("{difficulty}: flag {active} missing from our table"));
            let ours = match difficulty.as_str() {
                "Normal" => found.normal,
                "Hard" => found.hard,
                "Very Hard" => found.very_hard,
                other => panic!("unexpected difficulty {other}"),
            };
            assert_eq!(ours, mirror, "{difficulty} flag {active}");
        }
    }
    assert_eq!(dw4core::flags::MIRRORS.len(), 38, "our table covers all 38");
}
