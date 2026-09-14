//! Behaviour against the real `Mcd001.ps2`, rebuilt from the sparse fixture.
//!
//! The fixture stores only the 352 pages of the card that are not entirely
//! `0xFF`; `common::memcard_fixture()` rebuilds the full 8,650,752 bytes. The
//! generator refuses to write the fixture unless that rebuild is byte-identical
//! to the real card, so these tests run against the real thing.
mod common;

/// The first index at which two byte slices differ, if any.
///
/// Assertions on the whole image must not print it: a failure here would dump
/// 8.6 MB into the test log and bury the actual message.
fn first_difference(a: &[u8], b: &[u8]) -> Option<usize> {
    if a.len() != b.len() {
        return Some(a.len().min(b.len()));
    }
    a.iter().zip(b).position(|(x, y)| x != y)
}

#[test]
fn the_fixture_rebuilds_the_real_card() {
    let card = common::memcard_fixture();
    assert_eq!(card.len(), 8_650_752, "the real card's size");
    assert_eq!(card.len() % common::CARD_PAGE_SIZE, 0);

    // The superblock magic is the cheapest proof that the rebuild placed the
    // right pages at the right offsets.
    assert_eq!(
        &card[..28],
        b"Sony PS2 Memory Card Format ",
        "superblock magic"
    );

    // The save directory name is present.
    let needle = dw4core::memcard::SAVE_DIR.as_bytes();
    assert!(
        card.windows(needle.len()).any(|w| w == needle),
        "the save directory name must be present"
    );

    // And the erased-page assumption the fixture depends on holds.
    let page = common::CARD_PAGE_SIZE;
    let erased = (0..card.len() / page)
        .filter(|n| card[n * page..(n + 1) * page] == [0xffu8; 528][..])
        .count();
    assert_eq!(erased, 16_032, "erased pages");
}

use dw4core::memcard::{CardKind, Geometry, Superblock};

#[test]
fn the_real_superblock_parses_to_the_measured_values() {
    let card = common::memcard_fixture();
    assert_eq!(CardKind::of_size(card.len()), Some(CardKind::WithSpare));

    let sb = Superblock::parse(&card).expect("the real superblock parses");
    assert_eq!(sb.version, "1.2.0.0");
    assert_eq!(sb.page_size, 512);
    assert_eq!(sb.pages_per_cluster, 2);
    assert_eq!(sb.pages_per_block, 16);
    assert_eq!(sb.clusters_per_card, 8192);
    assert_eq!(sb.alloc_offset, 41);
    assert_eq!(sb.alloc_end, 8135);
    assert_eq!(sb.rootdir_cluster, 0);
    assert_eq!(sb.ifc_list, vec![8]);
    assert_eq!(sb.card_type, 2);
    assert_eq!(sb.card_flags, 43);

    let g = Geometry::from_superblock(&sb, CardKind::WithSpare);
    assert_eq!(g.page_size, 512);
    assert_eq!(g.spare_size, 16);
    assert_eq!(g.raw_page_size, 528);
    assert_eq!(g.cluster_size, 1024);
    assert_eq!(g.fat_per_cluster, 256);
    assert_eq!(g.total_pages(), 16_384);

    // Addresses are strided by the RAW page size (528), not the data size
    // (512): a cluster is two 528-byte pages apart, not 1024 bytes.
    assert_eq!(g.page_offset(1), 528);
    assert_eq!(g.cluster_offset(0), 41 * 2 * 528);
    assert_eq!(g.cluster_offset(2), 43 * 2 * 528);
    assert_eq!(g.total_pages() * g.raw_page_size, 8_650_752);
}

#[test]
fn a_short_or_non_card_input_is_rejected() {
    assert!(Superblock::parse(&[0u8; 100]).is_err());
    let mut card = common::memcard_fixture();
    card[0] = b'X'; // break the magic
    assert!(Superblock::parse(&card).is_err());
}

#[test]
fn card_kind_is_decided_by_length() {
    assert_eq!(CardKind::of_size(8_650_752), Some(CardKind::WithSpare));
    assert_eq!(CardKind::of_size(8_388_608), Some(CardKind::DataOnly));
    assert_eq!(CardKind::of_size(1234), None);
}

use dw4core::memcard::page_spare;

#[test]
fn computed_ecc_matches_the_real_card_on_every_in_use_page() {
    // The card stores its own ECC in each page's 16-byte spare area, so it is
    // the oracle. The known exception is page 1, inside the pre-allocated
    // superblock region, which write_save never touches.
    let card = common::memcard_fixture();
    let page = common::CARD_PAGE_SIZE;
    let data = 512usize;

    let mut matched = 0;
    let mut mismatched = Vec::new();
    for n in 0..card.len() / page {
        let at = n * page;
        if card[at..at + page] == [0xffu8; 528][..] {
            continue; // erased page: spare is 0xFF, not a code
        }
        let stored = &card[at + data..at + page];
        let computed = page_spare(&card[at..at + data]);
        if computed == stored {
            matched += 1;
        } else {
            mismatched.push(n);
        }
    }

    assert_eq!(matched, 351, "in-use pages with matching ECC");
    assert_eq!(
        mismatched,
        vec![1],
        "the only mismatch is page 1, in the superblock region"
    );
}

#[test]
fn the_trailing_four_spare_bytes_are_zero_on_in_use_pages() {
    let card = common::memcard_fixture();
    let page = common::CARD_PAGE_SIZE;
    for n in 0..card.len() / page {
        let at = n * page;
        if card[at..at + page] == [0xffu8; 528][..] {
            continue;
        }
        assert_eq!(
            &card[at + 512 + 12..at + page],
            &[0u8; 4],
            "page {n} trailing spare bytes"
        );
    }
}

use dw4core::memcard::Entry;

/// The real card's geometry, so offsets in tests cannot drift from the code.
fn real_geometry() -> Geometry {
    let card = common::memcard_fixture();
    let sb = Superblock::parse(&card).expect("superblock");
    Geometry::from_superblock(&sb, CardKind::WithSpare)
}

/// Byte offset of entry `slot` (0-based) within relative cluster `cluster`.
fn entry_offset(cluster: u32, slot: usize) -> usize {
    real_geometry().cluster_data_offset(cluster, slot * 512)
}

#[test]
fn the_root_directory_entry_decodes() {
    // Cluster 0 (relative) holds the root directory. Its `.` entry has length
    // 3 and cluster 0.
    let card = common::memcard_fixture();
    let at = entry_offset(0, 0);
    let root = Entry::parse(&card[at..at + 512]).expect("the root entry decodes");
    assert_eq!(root.name(), ".");
    assert_eq!(root.length(), 3, "the root bounds its children by this");
    assert_eq!(root.cluster(), 0);
    assert!(root.is_dir());
    assert!(root.exists());
    assert!(!root.is_file());
    assert!(root.is_dot());
}

#[test]
fn the_dot_dot_entry_decodes_and_is_filtered_as_a_dot_entry() {
    let card = common::memcard_fixture();
    let at = entry_offset(0, 1);
    let e = Entry::parse(&card[at..at + 512]).expect("decodes");
    assert_eq!(e.name(), "..");
    assert!(e.is_dir());
    assert!(
        e.is_dot(),
        "`..` must be filtered out of a directory listing like `.`"
    );
}

#[test]
fn the_save_directory_entry_decodes() {
    let card = common::memcard_fixture();
    let at = entry_offset(1, 0);
    let e = Entry::parse(&card[at..at + 512]).expect("decodes");
    assert_eq!(e.name(), "BASLUS-20836savedata");
    assert_eq!(e.cluster(), 2);
    assert_eq!(e.length(), 5, "five entries including . and ..");
    assert!(e.is_dir());
}

#[test]
fn an_entry_round_trips_through_parse_and_encode() {
    let card = common::memcard_fixture();
    let at = entry_offset(1, 0);
    let original = &card[at..at + 512];
    let e = Entry::parse(original).expect("decodes");
    let encoded = e.encode();
    let reparsed = Entry::parse(&encoded).expect("re-decodes");
    assert_eq!(e, reparsed);
    // The whole 512-byte slot survives, so timestamps and the fields this crate
    // does not interpret are preserved on write.
    assert_eq!(encoded, original[..512], "the slot round-trips verbatim");
}

use dw4core::memcard::{CHAIN_END, FatTable, UNALLOCATED};

fn real_fat(card: &[u8]) -> FatTable {
    let sb = Superblock::parse(card).expect("superblock");
    let g = Geometry::from_superblock(&sb, CardKind::WithSpare);
    FatTable::from_card(card, &sb, &g).expect("fat")
}

#[test]
fn the_fat_reports_the_measured_layout() {
    let card = common::memcard_fixture();
    let fat = real_fat(&card);
    assert_eq!(fat.fat_clusters(), &(9u32..=40).collect::<Vec<_>>());
    assert_eq!(fat.cluster_count(), 8192, "32 fat clusters x 256 entries");
}

#[test]
fn the_save_file_chain_has_eighty_clusters_and_terminates() {
    // 81,920 bytes over a 1,024-byte cluster is exactly 80 clusters.
    let card = common::memcard_fixture();
    let fat = real_fat(&card);
    let chain = fat.chain(38).expect("the chain walks");
    assert_eq!(chain.len(), 80, "81920 / 1024");
    assert_eq!(chain[0], 38, "starts where the directory entry says");
    assert_eq!(chain[79], 117, "and runs to the measured last cluster");
    assert_eq!(
        fat.next(*chain.last().expect("non-empty")),
        None,
        "the last cluster ends the chain"
    );
    // The allocated bit is masked off when a value is read.
    assert_eq!(fat.raw_entry(38), 0x8000_0027, "cluster 38 links to 39");
    assert_eq!(fat.next(38), Some(39));
}

#[test]
fn a_free_cluster_ends_a_chain_in_both_encodings() {
    // A real card uses two spellings for a free cluster: 0xFFFFFFFF, and
    // 0x7FFFFFFF which is already CHAIN_END. Both must terminate a walk.
    assert_eq!(CHAIN_END, 0x7FFF_FFFF);
    assert_eq!(UNALLOCATED, 0xFFFF_FFFF);

    let card = common::memcard_fixture();
    let fat = real_fat(&card);
    let free = (0..fat.cluster_count())
        .find(|n| fat.raw_entry(*n) == UNALLOCATED)
        .expect("the card has free clusters");
    assert_eq!(fat.next(free), None);

    // 0x7FFFFFFF needs no masking, and also ends the chain.
    let already_end = (0..fat.cluster_count())
        .find(|n| fat.raw_entry(*n) == CHAIN_END)
        .expect("the card has CHAIN_END entries");
    assert_eq!(fat.next(already_end), None);
}

#[test]
fn a_chain_never_runs_past_the_data_area() {
    let card = common::memcard_fixture();
    let fat = real_fat(&card);
    let sb = Superblock::parse(&card).expect("superblock");
    assert!(
        fat.chain(2)
            .expect("walks")
            .iter()
            .all(|c| *c < sb.alloc_end),
        "every cluster of the save directory is inside the data area"
    );
}

#[test]
fn a_chain_that_leaves_the_data_area_is_refused() {
    let card = common::memcard_fixture();
    let fat = real_fat(&card);
    let sb = Superblock::parse(&card).expect("superblock");
    let err = fat.chain(sb.alloc_end).unwrap_err();
    assert!(
        matches!(err, dw4core::Error::BadClusterChain { .. }),
        "{err:?}"
    );
}

use dw4core::memcard::{Ps2Memcard, SAVE_DIR, SAVE_FILE};

#[test]
fn reading_the_save_out_of_the_card_matches_the_known_good_fixture() {
    // The strongest available check: the expected bytes were produced by the
    // Python editor reading this same card.
    let card = common::memcard_fixture();
    let mut mem = Ps2Memcard::from_image(card).expect("opens");
    let save = mem.read_save(SAVE_DIR, SAVE_FILE).expect("reads");
    let expected = std::fs::read(common::fixture("mcd001/save.raw")).expect("fixture");
    assert_eq!(save.len(), 81_920);
    assert_eq!(save, expected, "the save read from the card");
}

#[test]
fn the_located_file_reports_the_measured_cluster_and_length() {
    let card = common::memcard_fixture();
    let mem = Ps2Memcard::from_image(card).expect("opens");
    let located = mem.locate(SAVE_DIR, SAVE_FILE).expect("found");
    assert_eq!(located.first_cluster, 38);
    assert_eq!(located.length, 81_920);
    assert_eq!(located.chain.len(), 80);
}

#[test]
fn a_missing_save_is_reported_not_panicked() {
    let card = common::memcard_fixture();
    let mut mem = Ps2Memcard::from_image(card).expect("opens");
    let err = mem.read_save("NO-SUCH-DIR", SAVE_FILE).unwrap_err();
    assert!(
        matches!(err, dw4core::Error::SaveNotFound { .. }),
        "{err:?}"
    );
}

#[test]
fn the_other_files_in_the_save_directory_are_readable() {
    // icon.sys is 964 bytes and icon1.ico is 34156; both exercise a chain that
    // does not fill its last cluster.
    let card = common::memcard_fixture();
    let mut mem = Ps2Memcard::from_image(card).expect("opens");
    let icon = mem.read_save(SAVE_DIR, "icon1.ico").expect("reads");
    assert_eq!(icon.len(), 34_156);
    let sys = mem.read_save(SAVE_DIR, "icon.sys").expect("reads");
    assert_eq!(sys.len(), 964);
}

#[test]
fn rewriting_the_same_save_leaves_the_card_byte_identical() {
    // The strongest write test available without hardware: read, write the
    // identical bytes back, and require all 8.6 MB to be unchanged. This is
    // what proves the ECC and spare-area handling reproduce the original.
    let card = common::memcard_fixture();
    let mut mem = Ps2Memcard::from_image(card.clone()).expect("opens");
    let save = mem.read_save(SAVE_DIR, SAVE_FILE).expect("reads");
    mem.write_save(SAVE_DIR, SAVE_FILE, &save).expect("writes");
    if let Some(at) = first_difference(mem.image(), &card) {
        panic!(
            "an unchanged rewrite altered the card at 0x{at:X} (page {}): {} vs {}",
            at / common::CARD_PAGE_SIZE,
            mem.image()[at],
            card[at]
        );
    }
}

#[test]
fn a_modified_save_writes_and_reads_back() {
    let card = common::memcard_fixture();
    let mut mem = Ps2Memcard::from_image(card).expect("opens");

    let mut save = mem.read_save(SAVE_DIR, SAVE_FILE).expect("reads");
    save[0x68] ^= 0xFF; // the BIT field, to be sure the bytes moved
    mem.write_save(SAVE_DIR, SAVE_FILE, &save).expect("writes");

    let back = Ps2Memcard::from_image(mem.image().to_vec())
        .expect("reopens")
        .read_save(SAVE_DIR, SAVE_FILE)
        .expect("re-reads");
    assert_eq!(back, save);
}

#[test]
fn writing_the_save_disturbs_nothing_outside_its_own_clusters() {
    let card = common::memcard_fixture();
    let mut mem = Ps2Memcard::from_image(card.clone()).expect("opens");
    let located = mem.locate(SAVE_DIR, SAVE_FILE).expect("found");

    // Every page holding the save's own clusters.
    let geometry = *mem.geometry();
    let mut own_pages = std::collections::HashSet::new();
    for cluster in &located.chain {
        let absolute = geometry.alloc_offset + cluster;
        let first = absolute as usize * geometry.pages_per_cluster;
        for p in 0..geometry.pages_per_cluster {
            own_pages.insert(first + p);
        }
    }

    let mut save = mem.read_save(SAVE_DIR, SAVE_FILE).expect("reads");
    save[0x68] ^= 0xFF;
    mem.write_save(SAVE_DIR, SAVE_FILE, &save).expect("writes");

    let page = geometry.raw_page_size;
    let mut checked = 0usize;
    for n in 0..card.len() / page {
        if own_pages.contains(&n) {
            continue;
        }
        if let Some(at) = first_difference(
            &card[n * page..(n + 1) * page],
            &mem.image()[n * page..(n + 1) * page],
        ) {
            panic!(
                "page {n} is outside the save but changed at +0x{at:X}: {} vs {}",
                card[n * page + at],
                mem.image()[n * page + at]
            );
        }
        checked += 1;
    }
    assert_eq!(
        checked,
        16_384 - own_pages.len(),
        "every other page checked"
    );
}

#[test]
fn a_save_that_does_not_fit_the_chain_is_refused() {
    let card = common::memcard_fixture();
    let mut mem = Ps2Memcard::from_image(card.clone()).expect("opens");
    // One cluster longer than the file's 80-cluster chain.
    let too_big = vec![0u8; 81 * 1024];
    let err = mem.write_save(SAVE_DIR, SAVE_FILE, &too_big).unwrap_err();
    assert!(matches!(err, dw4core::Error::BadCard(_)), "{err:?}");
    if let Some(at) = first_difference(mem.image(), &card) {
        panic!("a refused write changed the card at 0x{at:X}");
    }
}

#[test]
fn the_written_pages_carry_valid_ecc() {
    let card = common::memcard_fixture();
    let mut mem = Ps2Memcard::from_image(card).expect("opens");
    let located = mem.locate(SAVE_DIR, SAVE_FILE).expect("found");

    let mut save = mem.read_save(SAVE_DIR, SAVE_FILE).expect("reads");
    save[0x10] = b'X'; // change the model name, so ECC must change
    mem.write_save(SAVE_DIR, SAVE_FILE, &save).expect("writes");

    let geometry = *mem.geometry();
    for cluster in &located.chain {
        let absolute = geometry.alloc_offset + cluster;
        let first = absolute as usize * geometry.pages_per_cluster;
        for p in 0..geometry.pages_per_cluster {
            let at = geometry.page_offset(first + p);
            let data = &mem.image()[at..at + geometry.page_size];
            let spare = &mem.image()[at + geometry.page_size..at + geometry.raw_page_size];
            assert_eq!(
                dw4core::memcard::page_spare(data),
                spare,
                "page {} of a rewritten save must carry matching ECC",
                first + p
            );
        }
    }
}
