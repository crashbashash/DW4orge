//! Session lifecycle over the committed fixtures.

mod common;

use dw4core::document::DifficultyChoice;
use dw4core::{Difficulty, EMPTY, Mode, Species};
use dw4ipc::{EditorSession, NewSaveRequest, SourceKind};

fn raw_fixture() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../dw4core/tests/fixtures/mcd001/save.raw"
    ))
}

#[test]
fn open_reads_the_raw_fixture() {
    let mut session = EditorSession::new_session();
    let result = session.open(&raw_fixture()).expect("opens");
    assert_eq!(result.source, SourceKind::Raw);
    assert_eq!(result.view.species, Species::Dorumon);
    assert!(result.view.checksum_ok);
    assert!(result.path.is_some());
}

#[test]
fn open_rejects_a_missing_file() {
    let mut session = EditorSession::new_session();
    let err = session
        .open(std::path::Path::new("/nonexistent/nope.raw"))
        .expect_err("must fail");
    assert!(matches!(err, dw4ipc::IpcError::Core { .. }), "{err:?}");
}

#[test]
fn view_needs_an_open_document() {
    let session = EditorSession::new_session();
    let err = session.view(Mode::Normal).expect_err("must fail");
    assert_eq!(err, dw4ipc::IpcError::NoOpenDocument);
}

#[test]
fn new_save_builds_the_requested_character() {
    let mut session = EditorSession::new_session();
    let req = NewSaveRequest {
        format: None,
        region: None,
        species: Species::Agumon,
        name: "abc".to_string(),
        story: None,
        difficulty: DifficultyChoice::Auto,
    };
    let result = session.new_save(&req).expect("builds");
    assert_eq!(result.path, None);
    assert_eq!(result.source, SourceKind::Raw);
    assert_eq!(result.view.species, Species::Agumon);
    assert_eq!(result.view.name, "abc");
}

#[test]
fn new_save_applies_a_story_preset() {
    let mut session = EditorSession::new_session();
    let req = NewSaveRequest {
        format: None,
        region: None,
        species: Species::Dorumon,
        name: "TST".to_string(),
        story: Some("Fresh (tutorial)".to_string()),
        difficulty: DifficultyChoice::Fixed(Difficulty::Normal),
    };
    let result = session.new_save(&req).expect("builds");
    assert!(result.view.story_flags.iter().any(|b| *b != 0));
}

#[test]
fn new_save_rejects_an_unknown_story() {
    let mut session = EditorSession::new_session();
    let req = NewSaveRequest {
        format: None,
        region: None,
        species: Species::Dorumon,
        name: "TST".to_string(),
        story: Some("nope".to_string()),
        difficulty: DifficultyChoice::Auto,
    };
    let err = session.new_save(&req).expect_err("must fail");
    assert!(
        matches!(err, dw4ipc::IpcError::Unsupported { .. }),
        "{err:?}"
    );
}

#[test]
fn the_reconstructed_card_reads_back_as_the_raw_fixture() {
    // Independent oracle: the duplicate fixture loader is correct only if the
    // card's save equals save.raw.
    let card = common::memcard_fixture();
    let save = dw4core::memcard::load_save_from_bytes(&card).expect("reads");
    let raw = std::fs::read(common::fixture("mcd001/save.raw")).expect("raw");
    assert_eq!(save, raw);
}

#[test]
fn validate_reports_field_errors() {
    let mut session = EditorSession::new_session();
    let opened = session.open(&raw_fixture()).expect("opens");
    let mut edits = opened.view.to_edit_set();
    edits.bit = 10_000_000; // above CAP_BIT.normal_max
    let err = session
        .validate(&edits, Mode::Normal)
        .expect_err("rejected");
    match err {
        dw4ipc::IpcError::Validation { fields } => {
            assert_eq!(fields[0].path, "bit");
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn a_rejected_edit_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("save.raw");
    std::fs::copy(raw_fixture(), &target).unwrap();
    let before = std::fs::read(&target).unwrap();

    let mut session = EditorSession::new_session();
    let opened = session.open(&target).unwrap();
    let mut edits = opened.view.to_edit_set();
    edits.bit = 10_000_000;
    assert!(session.validate(&edits, Mode::Normal).is_err());
    // validate is the gate; no save call happens on rejection.
    assert_eq!(std::fs::read(&target).unwrap(), before);
}

#[test]
fn save_round_trips_through_a_temp_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("save.raw");
    std::fs::copy(raw_fixture(), &target).unwrap();

    let mut session = EditorSession::new_session();
    let opened = session.open(&target).unwrap();
    let mut edits = opened.view.to_edit_set();
    edits.bit = 1234;
    edits.device[0] = EMPTY;
    session.validate(&edits, Mode::Normal).expect("valid");
    let saved = session.save(&edits, Mode::Normal).expect("saves");
    assert_eq!(saved.view.bit, 1234);
    assert!(saved.view.checksum_ok);
    assert!(target.with_extension("raw.bak").exists());
}

#[test]
fn save_without_a_path_asks_for_save_as() {
    let mut session = EditorSession::new_session();
    let opened = session
        .new_save(&NewSaveRequest {
            format: None,
            region: None,
            species: Species::Dorumon,
            name: "TST".to_string(),
            story: None,
            difficulty: DifficultyChoice::Auto,
        })
        .unwrap();
    let edits = opened.view.to_edit_set();
    let err = session.save(&edits, Mode::Normal).expect_err("must fail");
    assert!(
        matches!(err, dw4ipc::IpcError::Unsupported { .. }),
        "{err:?}"
    );
}

#[test]
fn save_as_creates_a_ps2_with_or_without_a_source_card() {
    let dir = tempfile::tempdir().unwrap();

    // No source card: a standard card is synthesised.
    let mut session = EditorSession::new_session();
    let opened = session
        .new_save(&NewSaveRequest {
            format: None,
            region: None,
            species: Species::Dorumon,
            name: "TST".to_string(),
            story: None,
            difficulty: DifficultyChoice::Auto,
        })
        .unwrap();
    let edits = opened.view.to_edit_set();
    let fresh = dir.path().join("fresh.ps2");
    let saved = session
        .save_as(&fresh, &edits, Mode::Normal)
        .expect("formats a card");
    assert_eq!(saved.source, dw4ipc::SourceKind::Memcard);
    assert_eq!(std::fs::read(&fresh).unwrap().len(), 8_650_752);

    // With a source card, the donor is copied instead.
    let card = common::card_file(dir.path());
    let mut session = EditorSession::new_session();
    let opened = session
        .new_save_on_card(
            &NewSaveRequest {
                format: None,
                region: None,
                species: Species::Agumon,
                name: "abc".to_string(),
                story: None,
                difficulty: DifficultyChoice::Auto,
            },
            &card,
        )
        .unwrap();
    let edits = opened.view.to_edit_set();
    let out = dir.path().join("copy.ps2");
    let saved = session
        .save_as(&out, &edits, Mode::Normal)
        .expect("writes a card");
    assert_eq!(saved.source, dw4ipc::SourceKind::Memcard);
    assert_eq!(std::fs::read(&out).unwrap().len(), 8_650_752);
}

#[test]
fn species_stats_read_the_requested_block_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("out.raw");

    let mut session = EditorSession::new_session();
    session.open(&raw_fixture()).expect("opens");

    // Edit Dorumon, leaving every other species' block untouched.
    let mut edits = session.view(Mode::Normal).expect("view").to_edit_set();
    edits.species = Species::Dorumon;
    edits.level = 50;
    edits.exp = u32::try_from(dw4core::level_threshold(50)).expect("fits u32");
    session
        .save_as(&target, &edits, Mode::Normal)
        .expect("saves");

    let dorumon = session
        .species_stats(Species::Dorumon, Mode::Normal)
        .expect("stats");
    assert_eq!(dorumon.level, 50);
    assert_eq!(dorumon.exp, edits.exp);

    let agumon = session
        .species_stats(Species::Agumon, Mode::Normal)
        .expect("stats");
    assert_eq!(agumon.level, 1, "another species' block must be untouched");
}

#[test]
fn species_stats_need_an_open_document() {
    let session = EditorSession::new_session();
    assert_eq!(
        session
            .species_stats(Species::Dorumon, Mode::Normal)
            .unwrap_err(),
        dw4ipc::IpcError::NoOpenDocument
    );
}

#[test]
fn ps2_box_switching_failed_selection_and_save_preserve_the_other_character() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("two.ps2");
    let mut raw = dw4core::build_save(&dw4core::SaveSpec::default());
    let second = dw4core::build_save(&dw4core::SaveSpec {
        player_name: "SECOND".into(),
        bit: 222,
        ..Default::default()
    });
    for mirror in 0..2 {
        let start = mirror * dw4core::BLOCK + 0x2000;
        raw[start + 8..start + 0x2000].copy_from_slice(&second[8..0x2000]);
    }
    dw4core::fix_checksums(&mut raw);
    std::fs::write(
        &path,
        dw4core::memcard::format_card_for_region(&raw, dw4core::Region::Japan).unwrap(),
    )
    .unwrap();
    let mut session = EditorSession::new_session();
    let first = session.open(&path).unwrap();
    assert_eq!(first.character_boxes.as_ref().unwrap().len(), 2);
    assert_eq!(first.character_selection.as_deref(), Some("ps2#1"));
    let second = session
        .open_with_selection(&path, first.region, Some("ps2#2"))
        .unwrap();
    assert_eq!(second.region, Some(dw4core::Region::Japan));
    assert_eq!(second.view.name, "SECOND");
    assert_eq!(second.view.bit, 222);
    assert!(
        session
            .open_with_selection(&path, second.region, Some("ps2#4"))
            .is_err()
    );
    assert_eq!(session.view(Mode::Normal).unwrap().name, "SECOND");
    let mut edits = second.view.to_edit_set();
    edits.name = "SAVED".into();
    edits.bit = 345678;
    let saved = session.save(&edits, Mode::Normal).unwrap();
    assert_eq!(saved.character_selection.as_deref(), Some("ps2#2"));
    assert_eq!(saved.character_boxes.unwrap()[1].label, "Box 2 · SAVED");
    assert_eq!(saved.view.bit, 345678);
    let first_again = session
        .open_with_selection(&path, None, Some("ps2#1"))
        .unwrap();
    assert_eq!(first_again.view, first.view);
    let back = dw4core::memcard::load_save(&path).unwrap();
    for mirror in 0..2 {
        let start = mirror * dw4core::BLOCK;
        assert_eq!(
            &back[start + 4..start + 0x2000],
            &raw[start + 4..start + 0x2000]
        );
    }
}

#[test]
fn new_character_on_a_donor_card_preserves_other_ps2_boxes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("donor.ps2");
    let copy = dir.path().join("copy.ps2");
    let mut raw = dw4core::build_save(&dw4core::SaveSpec::default());
    for mirror in 0..2 {
        let base = mirror * dw4core::BLOCK;
        let first = raw[base + 8..base + 0x2008].to_vec();
        raw[base + 0x6008..base + 0x8008].copy_from_slice(&first);
    }
    dw4core::fix_checksums(&mut raw);
    std::fs::write(
        &path,
        dw4core::memcard::format_card_for_region(&raw, dw4core::Region::Usa).unwrap(),
    )
    .unwrap();
    let mut session = EditorSession::new_session();
    let request = NewSaveRequest {
        format: None,
        region: None,
        species: Species::Agumon,
        name: "NEWONE".into(),
        story: None,
        difficulty: DifficultyChoice::Auto,
    };
    let gc = dir.path().join("gc.bin");
    std::fs::write(
        &gc,
        dw4core::build_save_with_order(&dw4core::SaveSpec::default(), dw4core::ByteOrder::Big),
    )
    .unwrap();
    assert!(session.new_save_on_card(&request, &gc).is_err());
    let created = session.new_save_on_card(&request, &path).unwrap();
    session
        .save_as(&copy, &created.view.to_edit_set(), Mode::Normal)
        .unwrap();
    let fourth = session
        .open_with_selection(&copy, None, Some("ps2#4"))
        .unwrap();
    assert_eq!(fourth.view.name, "TST");
    let out = dw4core::memcard::load_save(&copy).unwrap();
    for mirror in 0..2 {
        let base = mirror * dw4core::BLOCK;
        assert_eq!(
            &out[base + 0x2008..base + dw4core::BLOCK],
            &raw[base + 0x2008..base + dw4core::BLOCK]
        );
    }
}

#[test]
fn gamecube_box_selection_survives_gci_and_card_exports_and_failed_switches() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("boxes.gci");
    let mut raw =
        dw4core::build_save_with_order(&dw4core::SaveSpec::default(), dw4core::ByteOrder::Big);
    let fourth = dw4core::build_save_with_order(
        &dw4core::SaveSpec {
            player_name: "FOURあ".into(),
            bit: 444,
            ..Default::default()
        },
        dw4core::ByteOrder::Big,
    );
    for mirror in 0..2 {
        let at = mirror * dw4core::BLOCK + 0x6000;
        raw[at + 8..at + 0x2008].copy_from_slice(&fourth[8..0x2008]);
    }
    dw4core::fix_checksums_with_order(&mut raw, dw4core::ByteOrder::Big);
    let mut seed = dw4core::document::Document::from_bytes(&raw).unwrap();
    seed.save(&source).unwrap();
    let mut session = EditorSession::new_session();
    let opened = session.open(&source).unwrap();
    assert_eq!(opened.source, SourceKind::Gamecube);
    assert_eq!(opened.character_boxes.unwrap().len(), 2);
    let chosen = session
        .open_with_selection(&source, None, Some("gamecube#4"))
        .unwrap();
    assert_eq!(chosen.view.name, "FOURあ");
    assert!(
        session
            .open_with_selection(&source, None, Some("ps2#4"))
            .is_err()
    );
    assert_eq!(session.view(Mode::Normal).unwrap().name, "FOURあ");
    let mut edits = chosen.view.to_edit_set();
    edits.name = "SAVED4".into();
    edits.bit = 654321;
    let result = session.save(&edits, Mode::Normal).unwrap();
    assert_eq!(result.character_selection.as_deref(), Some("gamecube#4"));
    assert_eq!(result.character_boxes.unwrap()[1].label, "Box 4 · SAVED4");
    let card = dir.path().join("card.raw");
    let result = session
        .save_as_with_region(&card, &edits, Mode::Normal, Some(dw4core::Region::Japan))
        .unwrap();
    assert_eq!(result.region, Some(dw4core::Region::Japan));
    assert_eq!(result.source, SourceKind::GamecubeCard);
    assert_eq!(result.character_selection.as_deref(), Some("gamecube#4"));
    let first = session
        .open_with_selection(&card, Some(dw4core::Region::Japan), Some("gamecube#1"))
        .unwrap();
    assert_eq!(first.view, opened.view);
}

#[test]
fn ps2_creation_is_pending_until_saved_and_refuses_overwriting_existing_characters() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.ps2");
    let original = dw4core::memcard::format_card_for_region(
        &dw4core::build_save(&dw4core::SaveSpec::default()),
        dw4core::Region::Japan,
    )
    .unwrap();
    std::fs::write(&path, &original).unwrap();
    let mut session = EditorSession::new_session();
    let opened = session.open(&path).unwrap();
    assert_eq!(opened.empty_boxes, Some(vec![2, 3, 4]));
    assert!(!opened.pending_write);
    let request = NewSaveRequest {
        format: Some(SourceKind::Memcard),
        region: Some(dw4core::Region::Japan),
        species: Species::Veemon,
        name: "NEW3".into(),
        story: None,
        difficulty: DifficultyChoice::Auto,
    };
    let created = session.create_ps2_box(3, &request).unwrap();
    assert!(created.pending_write);
    assert_eq!(created.path, opened.path);
    assert_eq!(created.region, opened.region);
    assert_eq!(created.character_selection.as_deref(), Some("ps2#3"));
    assert_eq!(created.empty_boxes, Some(vec![2, 4]));
    assert_eq!(created.view.name, "NEW3");
    assert_eq!(created.view.species, Species::Veemon);
    assert_eq!(created.view.device[..3], [0x050d, 0x0513, 0x0512]);
    assert_eq!(created.view.weapons, [0, 1, 2]);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(session.create_ps2_box(1, &request).is_err());
    assert_eq!(session.view(Mode::Normal).unwrap(), created.view);
    let copy = dir.path().join("copy.ps2");
    let saved = session
        .save_as(&copy, &created.view.to_edit_set(), Mode::Normal)
        .unwrap();
    assert!(!saved.pending_write);
    assert_eq!(saved.character_selection, created.character_selection);
    assert_eq!(
        session
            .open_with_selection(&copy, None, Some("ps2#1"))
            .unwrap()
            .view,
        opened.view
    );
    assert_eq!(
        session
            .open_with_selection(&copy, None, Some("ps2#3"))
            .unwrap()
            .view,
        created.view
    );
    let created2 = session.create_ps2_box(2, &request).unwrap();
    let saved = session
        .save(&created2.view.to_edit_set(), Mode::Normal)
        .unwrap();
    assert!(!saved.pending_write);
    assert_eq!(saved.empty_boxes, Some(vec![4]));
    assert!(copy.with_extension("ps2.bak").exists());
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn creation_rechecks_disk_state_and_rejects_wrong_platform_region_and_invalid_request() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.bin");
    let raw = dw4core::build_save(&dw4core::SaveSpec::default());
    std::fs::write(&path, &raw).unwrap();
    let request = NewSaveRequest {
        format: None,
        region: None,
        species: Species::Agumon,
        name: "NEW2".into(),
        story: Some("After World 1".into()),
        difficulty: DifficultyChoice::Fixed(Difficulty::Hard),
    };
    let mut session = EditorSession::new_session();
    assert!(session.create_ps2_box(2, &request).is_err());
    let first = session.open(&path).unwrap();
    let mut external = dw4core::document::Document::load(&path).unwrap();
    external
        .create_ps2_box(2, &dw4core::SaveSpec::default())
        .unwrap();
    external.save(&path).unwrap();
    assert!(session.create_ps2_box(2, &request).is_err());
    assert_eq!(session.view(Mode::Normal).unwrap(), first.view);
    for bad in [
        NewSaveRequest {
            name: "123456789".into(),
            ..request.clone()
        },
        NewSaveRequest {
            story: Some("not a preset".into()),
            ..request.clone()
        },
        NewSaveRequest {
            format: Some(SourceKind::Gamecube),
            ..request.clone()
        },
    ] {
        assert!(session.create_ps2_box(4, &bad).is_err());
        assert_eq!(session.view(Mode::Normal).unwrap(), first.view);
    }
    let created = session.create_ps2_box(4, &request).unwrap();
    assert_eq!(created.character_boxes.unwrap().len(), 3); // incorporates the external box 2
    let gc = dir.path().join("gc.bin");
    std::fs::write(
        &gc,
        dw4core::build_save_with_order(&dw4core::SaveSpec::default(), dw4core::ByteOrder::Big),
    )
    .unwrap();
    session.open(&gc).unwrap();
    assert!(session.create_ps2_box(2, &request).is_err());
    let card = dir.path().join("japan.ps2");
    std::fs::write(
        &card,
        dw4core::memcard::format_card_for_region(&raw, dw4core::Region::Japan).unwrap(),
    )
    .unwrap();
    session.open(&card).unwrap();
    assert!(
        session
            .create_ps2_box(
                2,
                &NewSaveRequest {
                    region: Some(dw4core::Region::Europe),
                    ..request
                }
            )
            .is_err()
    );
}

#[test]
fn gamecube_creation_gci_card_and_bare_sessions_preserve_existing_characters_and_pending_state() {
    let dir = tempfile::tempdir().unwrap();
    for ext in ["gci", "raw", "bin"] {
        let path = dir.path().join(format!("source.{ext}"));
        let raw =
            dw4core::build_save_with_order(&dw4core::SaveSpec::default(), dw4core::ByteOrder::Big);
        let mut doc = dw4core::document::Document::from_bytes(&raw).unwrap();
        doc.set_region(dw4core::Region::Japan).unwrap();
        doc.save(&path).unwrap();
        let original = std::fs::read(&path).unwrap();
        let mut session = EditorSession::new_session();
        let first = session.open(&path).unwrap();
        assert_eq!(first.empty_boxes, Some(vec![2, 3, 4]));
        let request = NewSaveRequest {
            format: None,
            region: None,
            species: Species::Veemon,
            name: "GCNEWあ".into(),
            story: None,
            difficulty: DifficultyChoice::Auto,
        };
        let created = session.create_box(3, &request).unwrap();
        assert!(created.pending_write);
        assert_eq!(created.path, first.path);
        assert_eq!(created.character_selection.as_deref(), Some("gamecube#3"));
        assert_eq!(created.empty_boxes, Some(vec![2, 4]));
        assert_eq!(created.view.name, "GCNEWあ");
        assert_eq!(created.view.device[..3], [0x050d, 0x0513, 0x0512]);
        assert_eq!(created.view.weapons, [0, 1, 2]);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert!(session.create_box(1, &request).is_err());
        assert_eq!(session.view(Mode::Normal).unwrap(), created.view);
        let copy = dir.path().join(format!("copy.{ext}"));
        let saved = session
            .save_as(&copy, &created.view.to_edit_set(), Mode::Normal)
            .unwrap();
        assert!(!saved.pending_write);
        assert_eq!(saved.character_selection, created.character_selection);
        assert_eq!(
            session
                .open_with_selection(&copy, None, Some("gamecube#1"))
                .unwrap()
                .view,
            first.view
        );
        let reopened = session
            .open_with_selection(&copy, None, Some("gamecube#3"))
            .unwrap();
        assert_eq!(reopened.view, created.view);
        session.create_box(2, &request).unwrap();
        let edits = session.view(Mode::Normal).unwrap().to_edit_set();
        assert!(!session.save(&edits, Mode::Normal).unwrap().pending_write);
        assert!(copy.with_extension(format!("{ext}.bak")).exists());
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
}

#[test]
fn shared_creation_refuses_platform_conversion_and_rechecks_a_stale_gamecube_target() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.bin");
    let raw =
        dw4core::build_save_with_order(&dw4core::SaveSpec::default(), dw4core::ByteOrder::Big);
    std::fs::write(&path, &raw).unwrap();
    let mut session = EditorSession::new_session();
    let first = session.open(&path).unwrap();
    let request = NewSaveRequest {
        format: None,
        region: None,
        species: Species::Agumon,
        name: "NEW4".into(),
        story: Some("After World 1".into()),
        difficulty: DifficultyChoice::Fixed(Difficulty::Normal),
    };
    for format in [SourceKind::Memcard, SourceKind::Xbox, SourceKind::XboxMu] {
        assert!(
            session
                .create_box(
                    2,
                    &NewSaveRequest {
                        format: Some(format),
                        ..request.clone()
                    }
                )
                .is_err()
        );
        assert_eq!(session.view(Mode::Normal).unwrap(), first.view);
    }
    let mut external = dw4core::document::Document::load(&path).unwrap();
    external
        .create_box(4, &dw4core::SaveSpec::default())
        .unwrap();
    external.save(&path).unwrap();
    assert!(session.create_box(4, &request).is_err());
    assert_eq!(session.view(Mode::Normal).unwrap(), first.view);
    assert_eq!(
        session
            .create_box(2, &request)
            .unwrap()
            .character_boxes
            .unwrap()
            .len(),
        3
    );
    let card = dir.path().join("japan.raw");
    let mut doc = dw4core::document::Document::from_bytes(&raw).unwrap();
    doc.set_region(dw4core::Region::Japan).unwrap();
    doc.save(&card).unwrap();
    session.open(&card).unwrap();
    assert!(
        session
            .create_box(
                3,
                &NewSaveRequest {
                    region: Some(dw4core::Region::Europe),
                    ..request
                }
            )
            .is_err()
    );
}
