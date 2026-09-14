//! The static tables must actually contain the tables.

#[test]
fn ui_data_carries_every_table() {
    let ui = dw4ipc::ui_data();
    assert_eq!(ui.catalogue.len(), 539);
    assert_eq!(ui.mirrors.len(), 38);
    assert_eq!(ui.folder_labels.len(), 12);
    assert_eq!(ui.powerups.len(), 11);
    assert_eq!(ui.story_presets.len(), 6);
    assert_eq!(ui.caps.len(), 10);
    assert_eq!(
        ui.flag_labels.len(),
        dw4core::INTRO_FLAGS.len()
            + dw4core::CHAPTER_FLAGS.len()
            + dw4core::BOSS_FLAGS.len()
            + dw4core::QUEST_FLAGS.len()
            + dw4core::LOBBY_FLAGS.len()
    );
}

#[test]
fn powerups_use_the_per_slot_cap() {
    let ui = dw4ipc::ui_data();
    assert_eq!(ui.powerups[0].stat, "HP max");
    assert_eq!(ui.powerups[0].normal_max, 99_999);
    assert_eq!(ui.powerups[2].normal_max, 9_999);
}

#[test]
fn app_info_reports_both_versions() {
    let info = dw4ipc::app_info();
    assert_eq!(info.save_size, 81_920);
    assert_eq!(info.block_size, 40_960);
    assert_eq!(info.core_version, dw4core::VERSION);
    assert_eq!(info.schema_version, 2);
    assert!(!info.name.is_empty());
}
