//! Regression oracle extracted independently from corrected retail load/save Lua.
use dw4core::flags::{MIRRORED_FOLDERS, apply_story, folder_mirror, preset_by_name};
use dw4core::{Difficulty, MIRRORS, STORY_PRESETS};

fn retail() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/flags/retail.json")).unwrap()
}

#[test]
fn every_mirror_matches_both_retail_scripts_including_loop_endpoints() {
    assert_eq!(serde_json::to_value(MIRRORS).unwrap(), retail()["mirrors"]);
    assert_eq!(
        MIRRORED_FOLDERS as u64,
        retail()["restored_folders"].as_u64().unwrap()
    );
}

#[test]
fn progressed_presets_include_the_complete_retail_checkpoints() {
    for (name, expected) in retail()["checkpoints"].as_object().unwrap() {
        let preset = preset_by_name(name).unwrap();
        assert_eq!(
            serde_json::to_value(preset.flags).unwrap(),
            expected["flags"],
            "{name}"
        );
        assert_eq!(
            serde_json::to_value(preset.folders).unwrap(),
            expected["folders"],
            "{name}"
        );
    }
}

#[test]
fn every_preset_retains_one_playable_chapter_after_a_retail_load_on_each_supported_mode() {
    let oracle = retail();
    for preset in STORY_PRESETS {
        for difficulty in Difficulty::ALL {
            let state = apply_story(preset, difficulty);
            for playable in &Difficulty::ALL[..=difficulty.index()] {
                let mut restored = state.flags.clone();
                for row in oracle["mirrors"].as_array().unwrap() {
                    let column = ["normal", "hard", "very_hard"][playable.index()];
                    restored[row["active"].as_u64().unwrap() as usize] =
                        state.flags[row[column].as_u64().unwrap() as usize];
                }
                let chapters = (1..=6).filter(|&f| restored[f] == 1).count();
                assert_eq!(
                    chapters + usize::from(restored[86] == 1),
                    1,
                    "{} built for {difficulty:?}, loaded on {playable:?}",
                    preset.name
                );
                for folder in 0..MIRRORED_FOLDERS {
                    assert_eq!(
                        state.folders[folder],
                        state.flags[folder_mirror(folder, *playable).unwrap() as usize],
                        "{} folder {folder} on {playable:?}",
                        preset.name
                    );
                }
                assert_eq!(restored[960], 1, "player initialization survives");
            }
        }
    }
}

#[test]
fn presets_never_set_raw_mirror_targets_or_the_old_boss_ids() {
    for preset in STORY_PRESETS {
        for flag in preset.flags {
            assert!(
                !MIRRORS
                    .iter()
                    .any(|r| [r.normal, r.hard, r.very_hard].contains(flag)),
                "{} includes raw mirror {flag}",
                preset.name
            );
        }
        assert!(!preset.flags.contains(&66));
        assert!(!preset.flags.contains(&85));
        assert!(!preset.flags.contains(&609));
    }
}
