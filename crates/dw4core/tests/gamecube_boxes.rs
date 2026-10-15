//! Four-box layout and container preservation in the GameCube serializer.
use dw4core::document::Document;
use dw4core::{
    BLOCK, ByteOrder, Mode, Region, SaveData, SaveSpec, Species, build_save_with_order,
    fix_checksums_with_order,
};

fn four_boxes() -> Vec<u8> {
    let mut raw = build_save_with_order(&SaveSpec::default(), ByteOrder::Big);
    for index in 0..4 {
        let mut spec = SaveSpec {
            player_name: ["ONE", "あい", "THREE", "FOUR"][index].into(),
            bit: 111 * (index as u32 + 1),
            ..SaveSpec::default()
        };
        spec.set_species(
            [
                Species::Agumon,
                Species::Veemon,
                Species::Girumon,
                Species::Dorumon,
            ][index],
        );
        let record = build_save_with_order(&spec, ByteOrder::Big);
        for mirror in 0..2 {
            let base = mirror * BLOCK + index * 0x2000;
            raw[base + 8..base + 0x2008].copy_from_slice(&record[8..0x2008]);
            raw[base + 0x1ff0] = 0xa0 + index as u8;
            raw[base + 9] = 0x7a; // reserved bytes do not change the byte-sized occupied marker
        }
    }
    raw[0x9000] = 0xab;
    raw[BLOCK + 0x9000] = 0xcd;
    fix_checksums_with_order(&mut raw, ByteOrder::Big);
    raw
}

#[test]
fn all_four_gamecube_boxes_decode_names_and_edit_only_selected_record() {
    let original = four_boxes();
    for number in 1..=4 {
        let mut data = SaveData::parse(&original).unwrap();
        let choices = data.character_boxes();
        assert_eq!(choices.len(), 4);
        assert_eq!(choices[1].label, "Box 2 · あい");
        data.select_character_box(Some(&format!("gamecube#{number}")))
            .unwrap();
        assert_eq!(data.bit(), 111 * number as u32);
        assert_eq!(data.version(), 4);
        assert_eq!(data.isuse(), 1);
        assert_eq!(
            data.player_name(),
            ["ONE", "あい", "THREE", "FOUR"][number - 1]
        );
        data.set_player_name("EDITあ");
        data.set_bit(765432);
        let output = data.to_bytes();
        assert!(SaveData::parse(&output).unwrap().verify());
        for mirror in 0..2 {
            let base = mirror * BLOCK;
            let start = base + (number - 1) * 0x2000;
            assert_eq!(
                u32::from_be_bytes(output[start + 0x68..start + 0x6c].try_into().unwrap()),
                765432
            );
            for offset in base + 4..base + BLOCK {
                if !(start + 0x30..start + 0x42).contains(&offset)
                    && !(start + 0x68..start + 0x6c).contains(&offset)
                {
                    assert_eq!(
                        output[offset], original[offset],
                        "box {number} at {offset:#x}"
                    );
                }
            }
        }
    }
}

#[test]
fn gamecube_holes_default_and_invalid_selection_do_not_change_state() {
    let mut raw = four_boxes();
    for mirror in 0..2 {
        for index in [0, 1, 2] {
            raw[mirror * BLOCK + index * 0x2000 + 8..mirror * BLOCK + index * 0x2000 + 12].fill(0);
        }
    }
    fix_checksums_with_order(&mut raw, ByteOrder::Big);
    let mut data = SaveData::parse(&raw).unwrap();
    data.select_character_box(None).unwrap();
    assert_eq!(data.character_selection().as_deref(), Some("gamecube#4"));
    for id in [
        "gamecube#0",
        "gamecube#1",
        "gamecube#5",
        "ps2#4",
        "SaveData.dat#4",
    ] {
        assert!(data.select_character_box(Some(id)).is_err());
        assert_eq!(data.character_selection().as_deref(), Some("gamecube#4"));
    }
}

#[test]
fn gci_card_and_bare_exports_keep_all_boxes_metadata_and_region() {
    for region in Region::ALL {
        let dir = tempfile::tempdir().unwrap();
        let raw = four_boxes();
        let gci = dir.path().join("seed.gci");
        let card = dir.path().join("seed.raw");
        let mut seed = Document::from_bytes(&raw).unwrap();
        seed.set_region(region).unwrap();
        seed.save(&gci).unwrap();
        seed.save(&card).unwrap();
        for source in [&gci, &card] {
            for number in 1..=4 {
                let id = format!("gamecube#{number}");
                let mut doc = Document::load_with_selection(source, None, Some(&id)).unwrap();
                let mut edits = doc.view(Mode::Normal).to_edit_set();
                edits.name = format!("EDIT{number}");
                edits.bit = 654321;
                edits.tech[4] = 17;
                edits.bank_bit = 4321;
                edits.disks[2] = 3;
                doc.apply(&edits, Mode::Normal).unwrap();
                let source_bytes = std::fs::read(source).unwrap();
                let ext = source.extension().unwrap().to_str().unwrap();
                let copy = dir.path().join(format!("copy-{number}-{ext}.{ext}"));
                doc.save(&copy).unwrap();
                doc.save(&copy).unwrap();
                assert_eq!(doc.character_selection().as_deref(), Some(id.as_str()));
                let after = std::fs::read(&copy).unwrap();
                if ext == "gci" {
                    assert_eq!(&after[..0x2040], &source_bytes[..0x2040]);
                } else {
                    assert_eq!(&after[..5 * 0x2000], &source_bytes[..5 * 0x2000]);
                    assert_eq!(doc.region(), Some(region));
                }
                let data = dw4core::memcard::load_save(&copy).unwrap();
                for mirror in 0..2 {
                    for other in 0..4 {
                        if other == number - 1 {
                            continue;
                        }
                        let at = mirror * BLOCK + other * 0x2000;
                        assert_eq!(&data[at + 8..at + 0x2008], &raw[at + 8..at + 0x2008]);
                    }
                }
                let bare = dir.path().join(format!("bare-{number}-{ext}.bin"));
                doc.save(&bare).unwrap();
                let loaded = Document::load_with_selection(&bare, None, Some(&id)).unwrap();
                assert_eq!(loaded.view(Mode::Normal).name, edits.name);
                assert!(loaded.view(Mode::Normal).checksum_ok);
                if ext == "raw" {
                    let exported = dir.path().join(format!("export-{number}.gci"));
                    doc.save(&exported).unwrap();
                    assert_eq!(dw4core::memcard::load_save(&exported).unwrap(), data);
                }
                assert_eq!(std::fs::read(source).unwrap(), source_bytes);
            }
        }
    }
}

#[test]
fn externally_changed_gamecube_boxes_are_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let original = four_boxes();
    for ext in ["gci", "raw", "bin"] {
        let path = dir.path().join(format!("conflict.{ext}"));
        Document::from_bytes(&original)
            .unwrap()
            .save(&path)
            .unwrap();
        let mut editing = Document::load_with_selection(&path, None, Some("gamecube#4")).unwrap();
        editing.data_mut().set_bit(444444);
        let mut other = Document::load_with_selection(&path, None, Some("gamecube#2")).unwrap();
        other.data_mut().set_bit(222222);
        other.save(&path).unwrap();
        let changed = std::fs::read(&path).unwrap();
        assert!(
            editing
                .save(&path)
                .unwrap_err()
                .to_string()
                .contains("changed since opening")
        );
        assert_eq!(std::fs::read(&path).unwrap(), changed);
    }
}
