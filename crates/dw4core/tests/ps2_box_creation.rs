use dw4core::document::Document;
use dw4core::{
    BLOCK, ByteOrder, Mode, Region, SaveData, SaveSpec, Species, build_save, build_save_with_order,
    fix_checksums,
};

fn seed() -> Vec<u8> {
    let mut raw = build_save(&SaveSpec::default());
    // Deleted/unused records need not be all zero; creation replaces only its target.
    for mirror in 0..2 {
        for index in 1..4 {
            let at = mirror * BLOCK + index * 0x2000 + 8;
            raw[at..at + 0x2000].fill(0x76 + index as u8);
            raw[at] = 0;
        }
        raw[mirror * BLOCK + 0x9000] = 0xa0 + mirror as u8;
    }
    fix_checksums(&mut raw);
    raw
}

#[test]
fn creating_each_empty_box_initializes_both_mirrors_and_preserves_other_bytes() {
    for number in 1..=4 {
        let mut original = seed();
        if number == 1 {
            for mirror in 0..2 {
                original[mirror * BLOCK + 8] = 0;
            }
        }
        fix_checksums(&mut original);
        let mut save = SaveData::parse(&original).unwrap();
        let mut spec = SaveSpec {
            player_name: format!("NEW{number}"),
            bit: 1234,
            ..SaveSpec::default()
        };
        spec.set_species(Species::Veemon);
        save.create_ps2_box(number, &spec).unwrap();
        assert_eq!(
            save.character_selection().as_deref(),
            Some(format!("ps2#{number}").as_str())
        );
        assert_eq!(save.player_name(), spec.player_name);
        assert_eq!(save.bit(), 1234);
        assert_eq!(save.detect_species(), Species::Veemon);
        assert_eq!(save.isuse(), 1);
        assert!(save.verify());
        assert_ne!(save.unique(), 0);
        assert_ne!(save.unique(), dw4core::UNIQUE_DEFAULT);
        for mirror in 0..2 {
            let at = mirror * BLOCK + usize::from(number - 1) * 0x2000 + 8;
            for (i, original_byte) in original
                .iter()
                .enumerate()
                .take((mirror + 1) * BLOCK)
                .skip(mirror * BLOCK + 4)
            {
                if !(at..at + 0x2000).contains(&i) {
                    assert_eq!(
                        save.as_bytes()[i],
                        *original_byte,
                        "box {number} byte {i:#x}"
                    );
                }
            }
        }
        let next = (1..=4).find(|n| save.empty_ps2_boxes().unwrap().contains(n));
        if let Some(next) = next {
            let unique = save.unique();
            save.create_ps2_box(next, &spec).unwrap();
            assert_ne!(save.unique(), unique);
        }
    }
}

#[test]
fn creation_refuses_occupied_mismatched_corrupt_unsupported_and_invalid_inputs_atomically() {
    let original = seed();
    for number in [0, 1, 5, 255] {
        let mut save = SaveData::parse(&original).unwrap();
        let before = save.clone();
        assert!(save.create_ps2_box(number, &SaveSpec::default()).is_err());
        assert_eq!(save, before);
    }
    for marker in [1, 2] {
        let mut raw = original.clone();
        raw[BLOCK + 0x2008] = marker;
        fix_checksums(&mut raw);
        let mut save = SaveData::parse(&raw).unwrap();
        let before = save.clone();
        assert!(save.create_ps2_box(2, &SaveSpec::default()).is_err());
        assert_eq!(save, before);
    }
    let mut corrupt = original.clone();
    corrupt[0x9000] ^= 1;
    let mut save = SaveData::parse(&corrupt).unwrap();
    assert!(save.create_ps2_box(2, &SaveSpec::default()).is_err());
    assert_eq!(save.as_bytes(), corrupt);
    let mut gc =
        SaveData::parse(&build_save_with_order(&SaveSpec::default(), ByteOrder::Big)).unwrap();
    let before = gc.clone();
    assert!(gc.create_ps2_box(2, &SaveSpec::default()).is_err());
    assert_eq!(gc, before);
    for name in ["", "  ", "123456789", "a\nb", "🙂", "\u{ffff}"] {
        let mut save = SaveData::parse(&original).unwrap();
        let spec = SaveSpec {
            player_name: name.into(),
            ..SaveSpec::default()
        };
        assert!(save.create_ps2_box(2, &spec).is_err());
        assert_eq!(save.as_bytes(), original);
    }
}

#[test]
fn new_boxes_save_and_export_on_all_regions_with_other_filesystem_bytes_unchanged() {
    for region in Region::ALL {
        for flat in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("source.ps2");
            let raw = seed();
            let mut image = dw4core::memcard::format_card_for_region(&raw, region).unwrap();
            if flat {
                image = image
                    .as_chunks::<528>()
                    .0
                    .iter()
                    .flat_map(|page| page[..512].iter().copied())
                    .collect();
            }
            std::fs::write(&path, &image).unwrap();
            let mut doc = Document::load(&path).unwrap();
            doc.create_ps2_box(
                4,
                &SaveSpec {
                    player_name: "FOURNEW".into(),
                    ..SaveSpec::default()
                },
            )
            .unwrap();
            let copy = dir.path().join("copy.ps2");
            doc.save(&copy).unwrap();
            assert_eq!(doc.region(), Some(region));
            doc.save(&copy).unwrap();
            assert_eq!(doc.character_selection().as_deref(), Some("ps2#4"));
            let loaded = Document::load_with_selection(&copy, None, Some("ps2#4")).unwrap();
            assert_eq!(loaded.view(Mode::Normal).name, "FOURNEW");
            assert!(loaded.view(Mode::Normal).checksum_ok);
            let after = dw4core::memcard::load_save(&copy).unwrap();
            for mirror in 0..2 {
                assert_eq!(
                    &after[mirror * BLOCK + 4..mirror * BLOCK + 0x6008],
                    &raw[mirror * BLOCK + 4..mirror * BLOCK + 0x6008]
                );
            }
            let before_card = dw4core::memcard::Ps2Memcard::from_image(image.clone()).unwrap();
            let located = before_card
                .locate(region.save_name(), region.save_name())
                .unwrap();
            let geom = *before_card.geometry();
            let copied = std::fs::read(&copy).unwrap();
            let changed_pages: std::collections::HashSet<_> = located
                .chain
                .iter()
                .flat_map(|cluster| {
                    let absolute = geom.alloc_offset + cluster;
                    (0..geom.pages_per_cluster)
                        .map(move |page| absolute as usize * geom.pages_per_cluster + page)
                })
                .collect();
            for (page, (a, b)) in image
                .chunks_exact(geom.raw_page_size)
                .zip(copied.chunks_exact(geom.raw_page_size))
                .enumerate()
            {
                if !changed_pages.contains(&page) {
                    assert_eq!(a, b, "non-save page {page}");
                }
            }
            let bare = dir.path().join("copy.bin");
            doc.save(&bare).unwrap();
            assert_eq!(
                Document::load_with_selection(&bare, None, Some("ps2#4"))
                    .unwrap()
                    .view(Mode::Normal)
                    .name,
                "FOURNEW"
            );
            assert_eq!(std::fs::read(&path).unwrap(), image);
        }
    }
}
