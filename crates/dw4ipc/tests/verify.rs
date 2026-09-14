//! Container verification.

mod common;

use dw4core::Category;
use dw4ipc::SourceKind;

#[test]
fn a_raw_fixture_verifies() {
    let report = dw4ipc::verify_path(&common::fixture("mcd001/save.raw")).expect("runs");
    assert_eq!(report.source, SourceKind::Raw);
    assert!(report.checksum_ok);
    assert!(report.problems.is_empty(), "{:?}", report.problems);
}

#[test]
fn a_card_verifies_including_ecc() {
    let dir = tempfile::tempdir().unwrap();
    let card = common::card_file(dir.path());
    let report = dw4ipc::verify_path(&card).expect("runs");
    assert_eq!(report.source, SourceKind::Memcard);
    assert!(report.checksum_ok);

    let card_report = report.card.expect("card section");
    assert_eq!(card_report.version, "1.2.0.0");
    assert_eq!(card_report.clusters, 8192);
    assert!(card_report.ecc_checked > 300, "{}", card_report.ecc_checked);
    // Page 1 is the documented superblock exception, not a failure.
    assert_eq!(card_report.ecc_mismatched, vec![1]);
}

#[test]
fn a_flipped_byte_fails_the_checksum() {
    let dir = tempfile::tempdir().unwrap();
    let mut bytes = std::fs::read(common::fixture("mcd001/save.raw")).unwrap();
    bytes[0x10] ^= 0xFF;
    let target = dir.path().join("corrupt.raw");
    std::fs::write(&target, &bytes).unwrap();
    let report = dw4ipc::verify_path(&target).expect("runs");
    assert!(!report.checksum_ok);
    assert!(!report.problems.is_empty());
}

#[test]
fn catalogue_search_filters_by_name_and_category() {
    let all = dw4ipc::catalogue_search(None, None, 1000);
    assert_eq!(all.len(), 539);

    let weapons = dw4ipc::catalogue_search(None, Some(Category::Weapon), 1000);
    assert!(!weapons.is_empty());
    assert!(weapons.iter().all(|i| i.category == Category::Weapon));

    let limited = dw4ipc::catalogue_search(None, None, 10);
    assert_eq!(limited.len(), 10);

    let query = dw4ipc::catalogue_search(Some("sword"), None, 1000);
    assert!(
        query
            .iter()
            .all(|i| i.name.to_lowercase().contains("sword"))
    );
}
