//! Layout measured from a retail card before/after saving a second character.
use dw4core::document::Document;
use dw4core::{
    BLOCK, ByteOrder, Mode, Region, SaveData, SaveSpec, Species, build_save, build_save_with_order,
    fix_checksums,
};

fn four_boxes() -> Vec<u8> {
    let mut raw = build_save(&SaveSpec::default());
    for index in 0..4 {
        let mut spec = SaveSpec {
            player_name: format!("BOX{}", index + 1),
            bit: 100 + index as u32,
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
        let record = build_save(&spec);
        for mirror in 0..2 {
            let at = mirror * BLOCK + index * 0x2000;
            raw[at + 8..at + 0x2000].copy_from_slice(&record[8..0x2000]);
            // Unknown bytes in every box and the trailing block must survive.
            raw[at + 0x1ff0] = 0x90 + index as u8;
        }
    }
    raw[0x9000] = 0xda;
    raw[BLOCK + 0x9000] = 0xdb;
    fix_checksums(&mut raw);
    raw
}

#[test]
fn each_box_reads_and_edits_only_its_record_in_both_mirrors() {
    let original = four_boxes();
    for index in 0..4 {
        let mut data = SaveData::parse(&original).unwrap();
        assert_eq!(data.character_boxes().len(), 4);
        data.select_character_box(Some(&format!("ps2#{}", index + 1)))
            .unwrap();
        assert_eq!(data.player_name(), format!("BOX{}", index + 1));
        assert_eq!(data.bit(), 100 + index as u32);
        assert_eq!(data.version(), 4); // version remains the shared wrapper word
        data.set_bit(654321);
        data.set_player_name("EDITED");
        let output = data.to_bytes();
        assert!(SaveData::parse(&output).unwrap().verify());
        for mirror in 0..2 {
            let base = mirror * BLOCK;
            let start = base + index * 0x2000;
            for offset in base + 4..base + BLOCK {
                let field = (start + 0x30..start + 0x42).contains(&offset)
                    || (start + 0x68..start + 0x6c).contains(&offset);
                if !field {
                    assert_eq!(
                        output[offset], original[offset],
                        "box {index}, byte {offset:#x}"
                    );
                }
            }
            assert_eq!(
                u32::from_le_bytes(output[start + 0x68..start + 0x6c].try_into().unwrap()),
                654321
            );
        }
    }
}

#[test]
fn holes_empty_boxes_invalid_ids_and_gamecube_selection() {
    let mut raw = four_boxes();
    for mirror in 0..2 {
        for index in [0, 2] {
            raw[mirror * BLOCK + index * 0x2000 + 8..mirror * BLOCK + index * 0x2000 + 12].fill(0);
        }
    }
    fix_checksums(&mut raw);
    let mut data = SaveData::parse(&raw).unwrap();
    assert_eq!(
        data.character_boxes()
            .iter()
            .map(|b| b.box_number)
            .collect::<Vec<_>>(),
        [2, 4]
    );
    data.select_character_box(None).unwrap();
    assert_eq!(data.character_selection().as_deref(), Some("ps2#2"));
    for id in ["ps2#1", "ps2#0", "ps2#5", "ps2#-1", "SaveData.dat#2"] {
        assert!(data.select_character_box(Some(id)).is_err());
        assert_eq!(data.character_selection().as_deref(), Some("ps2#2"));
    }
    let mut gc =
        SaveData::parse(&build_save_with_order(&SaveSpec::default(), ByteOrder::Big)).unwrap();
    assert_eq!(gc.character_boxes()[0].id, "gamecube#1");
    assert!(gc.select_character_box(Some("ps2#1")).is_err());
}

#[test]
fn card_and_raw_save_as_preserve_all_boxes_regions_and_selected_edits() {
    for region in Region::ALL {
        let dir = tempfile::tempdir().unwrap();
        let card_path = dir.path().join("original.ps2");
        let original = four_boxes();
        let image = dw4core::memcard::format_card_for_region(&original, region).unwrap();
        std::fs::write(&card_path, &image).unwrap();
        for number in 1..=4 {
            let id = format!("ps2#{number}");
            let mut doc = Document::load_with_selection(&card_path, None, Some(&id)).unwrap();
            assert_eq!(doc.region(), Some(region));
            let mut edits = doc.view(Mode::Normal).to_edit_set();
            edits.name = format!("EDIT{number}");
            edits.bit = 123456;
            edits.bank_bit = 987654;
            edits.tech[2] = 17;
            edits.upcnt[0] = 9;
            edits.disks[0] = 7;
            doc.apply(&edits, Mode::Normal).unwrap();
            let copy = dir.path().join(format!("copy-{number}.ps2"));
            doc.save(&copy).unwrap();
            assert_eq!(doc.character_selection().as_deref(), Some(id.as_str()));
            let mut loaded = Document::load_with_selection(&copy, None, Some(&id)).unwrap();
            assert_eq!(loaded.view(Mode::Normal).name, edits.name);
            assert_eq!(loaded.view(Mode::Normal).bit, 123456);
            assert_eq!(loaded.view(Mode::Normal).bank_bit, 987654);
            let output = loaded.data().as_bytes();
            for mirror in 0..2 {
                for other in 0..4 {
                    if other == number - 1 {
                        continue;
                    }
                    let start = mirror * BLOCK + other * 0x2000;
                    assert_eq!(
                        &output[start + 8..start + 0x2000],
                        &original[start + 8..start + 0x2000]
                    );
                }
            }
            // Saving again, then exporting to bare data keeps the selection and all boxes.
            loaded.save(&copy).unwrap();
            let bare = dir.path().join(format!("copy-{number}.bin"));
            loaded.save(&bare).unwrap();
            assert_eq!(loaded.character_selection().as_deref(), Some(id.as_str()));
            assert_eq!(
                Document::load_with_selection(&bare, None, Some(&id))
                    .unwrap()
                    .view(Mode::Normal)
                    .name,
                edits.name
            );
            assert_eq!(std::fs::read(&card_path).unwrap(), image);
        }
    }
}

#[test]
fn external_edits_to_another_box_are_refused_before_overwriting() {
    let dir = tempfile::tempdir().unwrap();
    for extension in ["bin", "ps2"] {
        let path = dir.path().join(format!("conflict.{extension}"));
        let original = four_boxes();
        let container = |data: &[u8]| {
            if extension == "ps2" {
                dw4core::memcard::format_card_for_region(data, Region::Usa).unwrap()
            } else {
                data.to_vec()
            }
        };
        std::fs::write(&path, container(&original)).unwrap();
        let mut doc = Document::load_with_selection(&path, None, Some("ps2#2")).unwrap();
        doc.data_mut().set_bit(2222);
        let mut external = SaveData::parse(&original).unwrap();
        external.select_character_box(Some("ps2#4")).unwrap();
        external.set_bit(4444);
        let changed = container(&external.to_bytes());
        std::fs::write(&path, &changed).unwrap();
        let error = doc.save(&path).unwrap_err().to_string();
        assert!(error.contains("changed since opening"), "{error}");
        assert_eq!(std::fs::read(&path).unwrap(), changed);
        assert!(!path.with_extension(format!("{extension}.bak")).exists());
    }
}
