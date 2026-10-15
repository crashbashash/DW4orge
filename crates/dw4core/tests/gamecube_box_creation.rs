use dw4core::document::Document;
use dw4core::{
    BLOCK, ByteOrder, Mode, Region, SaveData, SaveSpec, Species, build_save_with_order,
    fix_checksums_with_order,
};

fn seed() -> Vec<u8> {
    let mut raw = build_save_with_order(&SaveSpec::default(), ByteOrder::Big);
    for mirror in 0..2 {
        raw[mirror * BLOCK + 9] = 0x66; // occupied is one byte, not a big-endian word
        for index in 1..4 {
            let at = mirror * BLOCK + index * 0x2000 + 8;
            raw[at..at + 0x2000].fill(0x70 + index as u8);
            raw[at] = 0;
        }
        raw[mirror * BLOCK + 0x9000] = 0xa0 + mirror as u8;
    }
    fix_checksums_with_order(&mut raw, ByteOrder::Big);
    raw
}

#[test]
fn each_gamecube_box_is_initialized_big_endian_without_changing_other_records() {
    for number in 1..=4 {
        let mut original = seed();
        if number == 1 {
            original[8] = 0;
            original[BLOCK + 8] = 0;
        }
        fix_checksums_with_order(&mut original, ByteOrder::Big);
        let mut save = SaveData::parse(&original).unwrap();
        let mut spec = SaveSpec {
            player_name: format!("NEWあ{number}"),
            bit: 765432,
            ..SaveSpec::default()
        };
        spec.set_species(Species::Veemon);
        save.create_box(number, &spec).unwrap();
        assert_eq!(save.byte_order(), ByteOrder::Big);
        assert!(save.verify());
        assert_eq!(save.player_name(), spec.player_name);
        assert_eq!(save.bit(), 765432);
        assert_eq!(save.detect_species(), Species::Veemon);
        assert_eq!(save.isuse(), 1);
        assert_ne!(save.unique(), 0);
        assert_ne!(save.unique(), dw4core::UNIQUE_DEFAULT);
        assert_eq!(
            save.character_selection(),
            Some(format!("gamecube#{number}"))
        );
        let id = save.unique();
        for mirror in 0..2 {
            let start = mirror * BLOCK + usize::from(number - 1) * 0x2000 + 8;
            assert_eq!(
                u32::from_be_bytes(save.as_bytes()[start + 4..start + 8].try_into().unwrap()),
                id
            );
            for (i, &before) in original
                .iter()
                .enumerate()
                .take((mirror + 1) * BLOCK)
                .skip(mirror * BLOCK + 4)
            {
                if !(start..start + 0x2000).contains(&i) {
                    assert_eq!(save.as_bytes()[i], before, "box {number} byte {i:#x}");
                }
            }
        }
    }
}

#[test]
fn gamecube_empty_full_mirror_conflict_checksum_and_invalid_target_guards_are_atomic() {
    let original = seed();
    let mut save = SaveData::parse(&original).unwrap();
    assert_eq!(save.empty_boxes(), Some(vec![2, 3, 4]));
    let mut identities = vec![save.unique()];
    for n in [2, 3, 4] {
        save.create_box(n, &SaveSpec::default()).unwrap();
        assert!(!identities.contains(&save.unique()));
        identities.push(save.unique());
    }
    assert_eq!(save.empty_boxes(), Some(vec![]));
    let before = save.clone();
    assert!(save.create_box(4, &SaveSpec::default()).is_err());
    assert_eq!(save, before);
    for n in [0, 1, 5, 255] {
        let mut s = SaveData::parse(&original).unwrap();
        assert!(s.create_box(n, &SaveSpec::default()).is_err());
        assert_eq!(s.as_bytes(), original);
    }
    for flag in [1, 2] {
        let mut bytes = original.clone();
        bytes[BLOCK + 0x2008] = flag;
        fix_checksums_with_order(&mut bytes, ByteOrder::Big);
        let mut s = SaveData::parse(&bytes).unwrap();
        assert!(s.create_box(2, &SaveSpec::default()).is_err());
        assert_eq!(s.as_bytes(), bytes);
    }
    let mut bytes = original.clone();
    bytes[0x9000] ^= 1;
    let mut s = SaveData::parse(&bytes).unwrap();
    assert!(s.create_box(2, &SaveSpec::default()).is_err());
    assert_eq!(s.as_bytes(), bytes);
    let mut s = SaveData::parse(&original).unwrap();
    assert!(
        s.create_box(
            2,
            &SaveSpec {
                player_name: "123456789".into(),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(s.as_bytes(), original);
    // The pre-0.8 PS2-only API keeps its platform guard.
    assert!(s.create_ps2_box(2, &SaveSpec::default()).is_err());
}

#[test]
fn gci_card_and_bare_creation_preserve_wrappers_other_boxes_and_all_regional_identities() {
    for region in Region::ALL {
        let dir = tempfile::tempdir().unwrap();
        let original = seed();
        let source_gci = dir.path().join("source.gci");
        let source_raw = dir.path().join("source.raw");
        let source_bin = dir.path().join("source.bin");
        let mut doc = Document::from_bytes(&original).unwrap();
        doc.set_region(region).unwrap();
        doc.save(&source_gci).unwrap();
        let mut gci = std::fs::read(&source_gci).unwrap();
        gci[0x1000..0x1080].fill(0xab);
        std::fs::write(&source_gci, &gci).unwrap();
        // Conversion uses the donor GCI's banner and region even when the new
        // document has no explicit regional override.
        let mut donor = Document::load(&source_gci).unwrap();
        donor.save(&source_raw).unwrap();
        assert_eq!(donor.region(), Some(region));
        let card =
            dw4core::memcard::GameCubeMemcard::from_image(std::fs::read(&source_raw).unwrap())
                .unwrap();
        let exported = card.export_gci(region).unwrap();
        assert_eq!(&exported[0x40..0x2040], &gci[0x40..0x2040]);
        doc.save(&source_bin).unwrap();
        for source in [&source_gci, &source_raw, &source_bin] {
            for number in 2..=4 {
                let ext = source.extension().unwrap().to_str().unwrap();
                let before = std::fs::read(source).unwrap();
                let mut editing = Document::load(source).unwrap();
                editing
                    .create_box(
                        number,
                        &SaveSpec {
                            player_name: format!("NEW{number}"),
                            ..Default::default()
                        },
                    )
                    .unwrap();
                let copy = dir.path().join(format!("copy-{number}.{ext}"));
                editing.save(&copy).unwrap();
                editing.save(&copy).unwrap();
                let after = std::fs::read(&copy).unwrap();
                if ext == "gci" {
                    assert_eq!(&after[..0x2040], &before[..0x2040]);
                }
                if ext == "raw" {
                    assert_eq!(&after[..5 * 8192], &before[..5 * 8192]);
                    assert_eq!(editing.region(), Some(region));
                }
                let raw = dw4core::memcard::load_save(&copy).unwrap();
                for mirror in 0..2 {
                    for other in 0..4 {
                        if other == usize::from(number - 1) {
                            continue;
                        }
                        let at = mirror * BLOCK + other * 0x2000 + 8;
                        assert_eq!(&raw[at..at + 0x2000], &original[at..at + 0x2000]);
                    }
                }
                let reopened =
                    Document::load_with_selection(&copy, None, Some(&format!("gamecube#{number}")))
                        .unwrap();
                assert_eq!(reopened.view(Mode::Normal).name, format!("NEW{number}"));
                assert!(reopened.view(Mode::Normal).checksum_ok);
                assert_eq!(std::fs::read(source).unwrap(), before);
            }
        }
    }
}
