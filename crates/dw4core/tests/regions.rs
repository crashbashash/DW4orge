use dw4core::document::Document;
use dw4core::memcard::{Ps2Memcard, format_card_for_region, load_save_from_bytes, page_spare};
use dw4core::{Error, Region, SaveSpec, build_save};

#[test]
fn every_region_creates_a_discoverable_card_with_valid_ecc_and_metadata() {
    let save = build_save(&SaveSpec::default());
    for region in Region::ALL {
        let image = format_card_for_region(&save, region).unwrap();
        let mut card = Ps2Memcard::from_image(image.clone()).unwrap();
        assert_eq!(card.save_region(None).unwrap(), region);
        assert_eq!(load_save_from_bytes(&image).unwrap(), save);
        let icon = card.read_save(region.save_name(), "icon.sys").unwrap();
        if region == Region::Japan {
            assert_eq!(&icon[6..8], &18u16.to_le_bytes());
            assert_eq!(&icon[192..196], &[0x83, 0x66, 0x83, 0x57]);
        } else {
            assert_eq!(icon, include_bytes!("../data/card/icon.sys"));
        }
        for page in image
            .as_chunks::<528>()
            .0
            .iter()
            .filter(|p| **p != [255; 528])
        {
            assert_eq!(page_spare(&page[..512]), &page[512..]);
        }
    }
}

#[test]
fn editing_and_save_as_preserve_the_card_region_icons_and_backups() {
    let dir = tempfile::tempdir().unwrap();
    for region in Region::ALL {
        let src = dir.path().join(format!("{}.ps2", region.label()));
        let dst = dir.path().join(format!("{}-copy.ps2", region.label()));
        let image = format_card_for_region(&build_save(&SaveSpec::default()), region).unwrap();
        std::fs::write(&src, &image).unwrap();
        let mut doc = Document::load(&src).unwrap();
        assert_eq!(doc.region(), Some(region));
        assert!(
            doc.set_region(Region::ALL.into_iter().find(|r| *r != region).unwrap())
                .is_err()
        );
        doc.data_mut().set_bit(12345);
        doc.save(&src).unwrap();
        assert_eq!(
            std::fs::read(src.with_file_name(format!("{}.ps2.bak", region.label()))).unwrap(),
            image
        );
        doc.save(&dst).unwrap();
        let result = Document::load(&dst).unwrap();
        assert_eq!(result.region(), Some(region));
        assert_eq!(result.data().bit(), 12345);
        let mut before = Ps2Memcard::from_image(image).unwrap();
        let mut after = Ps2Memcard::open(&dst).unwrap();
        for file in ["icon.sys", "icon1.ico"] {
            assert_eq!(
                before.read_save(region.save_name(), file).unwrap(),
                after.read_save(region.save_name(), file).unwrap()
            );
        }
    }
}

// Combine two formatted cards with independent directory and file chains.
fn mixed_card() -> Vec<u8> {
    let save = build_save(&SaveSpec::default());
    let usa = format_card_for_region(&save, Region::Usa).unwrap();
    let eu = format_card_for_region(&save, Region::Europe).unwrap();
    let mut data: Vec<u8> = usa
        .as_chunks::<528>()
        .0
        .iter()
        .flat_map(|p| p[..512].iter().copied())
        .collect();
    let donor: Vec<u8> = eu
        .as_chunks::<528>()
        .0
        .iter()
        .flat_map(|p| p[..512].iter().copied())
        .collect();
    let put =
        |data: &mut [u8], at: usize, n: u32| data[at..at + 4].copy_from_slice(&n.to_le_bytes());
    let shift = 128u32;
    // Copy EU's data area and rebase allocated FAT entries.
    for rel in 2..120usize {
        let target = (41 + rel + shift as usize) * 1024;
        data[target..target + 1024].copy_from_slice(&donor[(41 + rel) * 1024..(42 + rel) * 1024]);
        let fat_at = 9 * 1024 + rel * 4;
        let entry = u32::from_le_bytes(donor[fat_at..fat_at + 4].try_into().unwrap());
        let moved = if entry == 0xffffffff {
            entry
        } else {
            0x80000000 | ((entry & 0x7fffffff) + shift)
        };
        put(&mut data, 9 * 1024 + (rel + shift as usize) * 4, moved);
    }
    // Rebase EU save-directory entry references, including its own '.' cluster.
    for (rel, slot) in [(2, 0), (2, 1), (3, 0), (3, 1), (118, 0)] {
        let at = (41 + rel + shift as usize) * 1024 + slot * 512;
        if rel == 2 && slot == 0 {
            put(&mut data, at + 0x14, 2 + shift);
        }
        if rel != 2 {
            let old = u32::from_le_bytes(data[at + 16..at + 20].try_into().unwrap());
            put(&mut data, at + 16, old + shift);
        }
    }
    let root_slot = 42 * 1024;
    data[root_slot + 512..root_slot + 1024].copy_from_slice(&donor[root_slot..root_slot + 512]);
    put(&mut data, root_slot + 512 + 16, 2 + shift);
    put(&mut data, 41 * 1024 + 4, 4);
    data.as_chunks::<512>()
        .0
        .iter()
        .flat_map(|page| {
            let mut out = page.to_vec();
            out.extend(page_spare(page));
            out
        })
        .collect()
}

#[test]
fn a_mixed_card_requires_selection_and_never_overwrites_the_other_region() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mixed.ps2");
    let image = mixed_card();
    let mut before = Ps2Memcard::from_image(image.clone()).unwrap();
    let usa = before
        .read_save(Region::Usa.save_name(), Region::Usa.save_name())
        .unwrap();
    std::fs::write(&path, &image).unwrap();
    assert!(matches!(
        Document::load(&path),
        Err(Error::AmbiguousRegion { .. })
    ));
    assert_eq!(std::fs::read(&path).unwrap(), image);
    let mut doc = Document::load_with_region(&path, Some(Region::Europe)).unwrap();
    doc.data_mut().set_bit(54321);
    doc.save(&path).unwrap();
    let mut after = Ps2Memcard::open(&path).unwrap();
    assert_eq!(
        after
            .read_save(Region::Usa.save_name(), Region::Usa.save_name())
            .unwrap(),
        usa
    );
    assert_eq!(
        Document::load_with_region(&path, Some(Region::Europe))
            .unwrap()
            .data()
            .bit(),
        54321
    );
}

#[test]
fn raw_data_has_no_detected_region_but_can_export_a_selected_card() {
    let dir = tempfile::tempdir().unwrap();
    let mut doc = Document::from_bytes(&build_save(&SaveSpec::default())).unwrap();
    assert_eq!(doc.region(), None);
    doc.set_region(Region::Japan).unwrap();
    let path = dir.path().join("japan.ps2");
    doc.save(&path).unwrap();
    assert_eq!(Document::load(&path).unwrap().region(), Some(Region::Japan));
}
