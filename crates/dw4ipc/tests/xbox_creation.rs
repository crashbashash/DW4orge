//! Synthetic Xbox containers, never retail save data.
use dw4core::{DifficultyChoice, Mode, SaveSpec, Species, build_save};
use dw4ipc::{EditorSession, NewSaveRequest, SourceKind};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::path::Path;

fn fixture() -> Vec<u8> {
    let key = [
        0xf1, 0x10, 0x0c, 0x5c, 0xac, 0xeb, 0x2f, 0xe8, 0x3c, 0x97, 0xa6, 0xb8, 0x92, 0x59, 0x9c,
        0x13,
    ];
    let mut bytes = vec![0; 0x10030];
    let raw = build_save(&SaveSpec::default());
    for base in [0, 0x8018] {
        bytes[base..base + 4].copy_from_slice(&4u32.to_le_bytes());
        bytes[base + 4..base + 4 + 0x2000].copy_from_slice(&raw[8..8 + 0x2000]);
        let mut mac = Hmac::<Sha1>::new_from_slice(&key).unwrap();
        mac.update(&bytes[base..base + 0x8004]);
        bytes[base + 0x8004..base + 0x8018].copy_from_slice(&mac.finalize().into_bytes());
    }
    bytes
}
fn request() -> NewSaveRequest {
    NewSaveRequest {
        format: Some(SourceKind::Xbox),
        region: None,
        species: Species::Veemon,
        name: "XBNEW2".into(),
        story: None,
        difficulty: DifficultyChoice::Auto,
    }
}
#[test]
fn xbox_creation_pending_save_as_reopen_and_repeated_creation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("SaveData.dat");
    let original = fixture();
    std::fs::write(&path, &original).unwrap();
    let mut session = EditorSession::new_session();
    let first = session.open(&path).unwrap();
    assert_eq!(first.empty_boxes, Some(vec![2, 3, 4]));
    for format in [
        SourceKind::Raw,
        SourceKind::Memcard,
        SourceKind::Gamecube,
        SourceKind::GamecubeCard,
    ] {
        assert!(
            session
                .create_box(
                    2,
                    &NewSaveRequest {
                        format: Some(format),
                        ..request()
                    }
                )
                .is_err()
        );
        assert_eq!(session.view(Mode::Normal).unwrap(), first.view);
    }
    let created = session.create_box(2, &request()).unwrap();
    assert!(created.pending_write);
    assert_eq!(
        created.character_selection.as_deref(),
        Some("SaveData.dat#2")
    );
    assert_eq!(created.empty_boxes, Some(vec![3, 4]));
    assert_eq!(created.view.name, "XBNEW2");
    assert_eq!(created.view.device[..3], [0x050d, 0x0513, 0x0512]);
    assert_eq!(created.view.weapons, [0, 1, 2]);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(session.create_box(1, &request()).is_err());
    assert_eq!(session.view(Mode::Normal).unwrap(), created.view);
    // Discard/recreate a pending character reloads the correct source container.
    let recreated = session.create_box(2, &request()).unwrap();
    let output = dir.path().join("output.dat");
    let saved = session
        .save_as(&output, &recreated.view.to_edit_set(), Mode::Normal)
        .unwrap();
    assert!(!saved.pending_write);
    assert_eq!(
        session
            .open_with_selection(&output, None, Some("SaveData.dat#1"))
            .unwrap()
            .view,
        first.view
    );
    assert_eq!(
        session
            .open_with_selection(&output, None, Some("SaveData.dat#2"))
            .unwrap()
            .view,
        saved.view
    );
    for number in [3, 4] {
        let result = session.create_box(number, &request()).unwrap();
        session
            .save(&result.view.to_edit_set(), Mode::Normal)
            .unwrap();
    }
    assert_eq!(session.open(&output).unwrap().empty_boxes, Some(vec![]));
    assert!(session.create_box(4, &request()).is_err());
    let backup = output.with_extension("dat.bak");
    assert!(backup.exists());
    assert_eq!(std::fs::read(&path).unwrap(), original);
}
#[test]
fn xbox_creation_rechecks_stale_target_and_refuses_corrupt_input() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("SaveData.dat");
    std::fs::write(&path, fixture()).unwrap();
    let mut session = EditorSession::new_session();
    let first = session.open(&path).unwrap();
    let mut external = EditorSession::new_session();
    external.open(&path).unwrap();
    let created = external.create_box(2, &request()).unwrap();
    external
        .save(&created.view.to_edit_set(), Mode::Normal)
        .unwrap();
    assert!(session.create_box(2, &request()).is_err());
    assert_eq!(session.view(Mode::Normal).unwrap(), first.view);
    let mut corrupt = std::fs::read(&path).unwrap();
    corrupt[123] ^= 1;
    std::fs::write(&path, corrupt).unwrap();
    assert!(session.create_box(3, &request()).is_err());
    assert_eq!(session.view(Mode::Normal).unwrap(), first.view);
    assert!(session.new_save(&request()).is_err());
    assert!(session.open(Path::new("/nonexistent.dat")).is_err());
}

fn multi_save_mu() -> Vec<u8> {
    fn put(image: &mut [u8], at: usize, value: u32) {
        image[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn entry(image: &mut [u8], at: usize, name: &str, dir: bool, cluster: u32, size: u32) {
        image[at] = name.len() as u8;
        image[at + 1] = if dir { 0x10 } else { 0x20 };
        image[at + 2..at + 2 + name.len()].copy_from_slice(name.as_bytes());
        put(image, at + 44, cluster);
        put(image, at + 48, size);
    }
    let offset = |cluster: usize| 8192 + (cluster - 1) * 16384;
    let mut image = vec![0; 8 * 1024 * 1024];
    image[..4].copy_from_slice(b"FATX");
    put(&mut image, 8, 4);
    put(&mut image, 12, 1);
    for cluster in 0..=14 {
        image[4096 + cluster * 2..4098 + cluster * 2].copy_from_slice(&0xffffu16.to_le_bytes());
    }
    entry(&mut image, offset(1), "42410002", true, 2, 0);
    entry(&mut image, offset(2), "FIRST", true, 3, 0);
    entry(&mut image, offset(2) + 64, "SECOND", true, 4, 0);
    let file = fixture();
    for (dir, first) in [(3, 5), (4, 10)] {
        entry(
            &mut image,
            offset(dir),
            "SaveData.dat",
            false,
            first as u32,
            0x10030,
        );
        for index in 0..5 {
            let cluster = first + index;
            if index < 4 {
                image[4096 + cluster * 2..4098 + cluster * 2]
                    .copy_from_slice(&((cluster + 1) as u16).to_le_bytes());
            }
            let start = index * 16384;
            let len = (file.len() - start).min(16384);
            image[offset(cluster)..offset(cluster) + len]
                .copy_from_slice(&file[start..start + len]);
        }
    }
    image
}

#[test]
fn memory_unit_creation_reloads_selected_container_and_exports_selected_box() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("card.img");
    let original = multi_save_mu();
    std::fs::write(&path, &original).unwrap();
    let mut session = EditorSession::new_session();
    session
        .open_with_selection(&path, None, Some("42410002/SECOND/SaveData.dat#1"))
        .unwrap();
    let req = NewSaveRequest {
        format: Some(SourceKind::XboxMu),
        ..request()
    };
    let created = session.create_box(3, &req).unwrap();
    assert_eq!(
        created.character_selection.as_deref(),
        Some("42410002/SECOND/SaveData.dat#3")
    );
    // Retry while pending still targets SECOND.
    let created = session.create_box(3, &req).unwrap();
    let copy = dir.path().join("copy.img");
    session
        .save_as(&copy, &created.view.to_edit_set(), Mode::Normal)
        .unwrap();
    assert_eq!(
        session
            .open_with_selection(&copy, None, Some("42410002/FIRST/SaveData.dat#1"))
            .unwrap()
            .empty_boxes,
        Some(vec![2, 3, 4])
    );
    let reopened = session
        .open_with_selection(&copy, None, Some("42410002/SECOND/SaveData.dat#3"))
        .unwrap();
    assert_eq!(reopened.empty_boxes, Some(vec![2, 4]));
    let exported = dir.path().join("export.dat");
    let result = session
        .save_as(&exported, &reopened.view.to_edit_set(), Mode::Normal)
        .unwrap();
    assert_eq!(
        result.character_selection.as_deref(),
        Some("SaveData.dat#3")
    );
    assert_eq!(
        session
            .open_with_selection(&exported, None, Some("SaveData.dat#3"))
            .unwrap()
            .view,
        created.view
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
}
