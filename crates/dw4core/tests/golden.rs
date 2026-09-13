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

#[test]
fn player_name_matches_the_python_oracle() {
    let e = expected();
    let save =
        dw4core::SaveData::parse(&std::fs::read(fixture_dir().join("save.raw")).unwrap()).unwrap();
    assert_eq!(save.player_name(), e["player_name"].as_str().unwrap());
    assert_eq!(save.player_name(), "abc");
}

#[test]
fn setting_a_player_name_survives_a_round_trip_through_bytes() {
    let mut save =
        dw4core::SaveData::parse(&std::fs::read(fixture_dir().join("save.raw")).unwrap()).unwrap();
    save.set_player_name("Zz9");
    let reparsed = dw4core::SaveData::parse(&save.to_bytes()).unwrap();
    assert_eq!(reparsed.player_name(), "Zz9");
}

fn u32s(v: &serde_json::Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u32)
        .collect()
}

#[test]
fn containers_match_the_python_oracle() {
    let e = expected();
    let save =
        dw4core::SaveData::parse(&std::fs::read(fixture_dir().join("save.raw")).unwrap()).unwrap();

    assert_eq!(save.get_raw_device_slots().len(), 36);
    assert_eq!(save.get_raw_device_slots(), u32s(&e["device"]));
    assert_eq!(save.get_raw_weapon_slots(), u32s(&e["weapon"]));
    assert_eq!(save.get_raw_weapon_mod_slots(), u32s(&e["weapon_mod"]));
    assert_eq!(save.armor() as u64, e["armor"].as_u64().unwrap());
    assert_eq!(save.get_raw_armor_mod_slots(), u32s(&e["armor_mod"]));
    assert_eq!(save.sub() as u64, e["sub"].as_u64().unwrap());
    assert_eq!(save.bank_bit() as u64, e["bank_bit"].as_u64().unwrap());
    assert_eq!(save.get_raw_bank_slots(), u32s(&e["bank_device"]));

    // The disk folder is compared as raw u32s: the count lives in the high
    // half and the type id (0x4000 + i) in the low half.
    let expected_disks: Vec<u32> = (0..12u32)
        .map(|i| {
            (e["disk"].as_array().unwrap()[i as usize].as_u64().unwrap() as u32) << 16
                | (0x4000 + i)
        })
        .collect();
    assert_eq!(save.get_raw_disk_slots(), expected_disks);

    let counts: Vec<u16> = (0..12).map(|i| save.disk_count(i)).collect();
    let expected_counts: Vec<u16> = e["disk"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u16)
        .collect();
    assert_eq!(counts, expected_counts);
}

#[test]
fn nicknames_agree_with_the_python_item_catalogue() {
    // The catalogue itself arrives in plan 2; here we only prove the ids match.
    let e = expected();
    let save =
        dw4core::SaveData::parse(&std::fs::read(fixture_dir().join("save.raw")).unwrap()).unwrap();
    let nicknames = e["nicknames"].as_object().unwrap();
    for (base, name) in nicknames {
        let base: u32 = base.parse().unwrap();
        assert!(
            (0..36).any(|i| save.device(i) & 0xFFFF == base),
            "base id 0x{base:04X} ({name}) is not in the device folder"
        );
    }
}
