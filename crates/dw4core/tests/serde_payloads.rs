//! The data types the IPC layer serialises must actually serialise.

#[test]
fn catalogue_items_serialise() {
    let catalogue = dw4core::get_catalogue();
    let item = catalogue.iter().next().expect("catalogue is not empty");
    let json = serde_json::to_string(item).expect("Item serialises");
    assert!(json.contains("\"base_id\""), "{json}");
    assert!(json.contains("\"category\""), "{json}");
}

#[test]
fn cap_serialises_both_modes_ranges() {
    let json = serde_json::to_string(&dw4core::CAP_BIT).expect("Cap serialises");
    assert!(json.contains("\"normal_max\""), "{json}");
    assert!(json.contains("\"dtype_max\""), "{json}");
}

#[test]
fn mirror_and_preset_serialise() {
    let mirror = serde_json::to_string(&dw4core::MIRRORS[0]).expect("Mirror serialises");
    assert!(mirror.contains("\"very_hard\""), "{mirror}");

    let preset = serde_json::to_string(&dw4core::STORY_PRESETS[0]).expect("StoryPreset serialises");
    assert!(preset.contains("\"folders\""), "{preset}");

    let label = serde_json::to_string(&dw4core::INTRO_FLAGS[0]).expect("FlagLabel serialises");
    assert!(label.contains("\"label\""), "{label}");
}
