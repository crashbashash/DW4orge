//! Cross-checks the mirror table and the presets against the Python sources.
//!
//! The Python editor carries **two different partial** mirror tables, so this is
//! the one part of the format with no single oracle: the builder's map must
//! match our Normal column exactly, and the GUI's tables must agree with us
//! wherever the GUI has an opinion.
use dw4core::flags::{
    FOLDER_MIRROR_BASE, MIRRORED_FOLDERS, folder_mirror, mirror_of, preset_by_name,
};
use dw4core::{Difficulty, MIRRORS, STORY_PRESETS};
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/flags")
}

fn load(name: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(fixture_dir().join(name))
        .unwrap_or_else(|e| panic!("read {name}: {e}"));
    serde_json::from_str(&text).expect("valid json")
}

fn flags_of(value: &serde_json::Value) -> Vec<u32> {
    value
        .as_array()
        .expect("a flags array")
        .iter()
        .map(|v| v.as_u64().expect("a flag number") as u32)
        .collect()
}

#[test]
fn our_normal_column_is_exactly_the_builders_mirror_map() {
    // The builder's NORMAL_FLAG_MIRRORS is the Normal column of FLAG_MAP.md §7.
    let expected = load("mirrors.json");
    let builder = expected["builder_normal"].as_object().unwrap();

    assert_eq!(
        builder.len(),
        MIRRORS.len(),
        "the builder covers {} flags, we cover {}",
        builder.len(),
        MIRRORS.len()
    );

    for (active, mirror) in builder {
        let active: u32 = active.parse().unwrap();
        let mirror = mirror.as_u64().unwrap() as u32;
        assert_eq!(
            mirror_of(active, Difficulty::Normal),
            Some(mirror),
            "active {active}"
        );
    }
}

#[test]
fn our_table_is_a_superset_of_the_guis() {
    // The GUI knew only 10 flags per difficulty. Everything it knew, we must
    // agree with; anything beyond that is ours.
    let expected = load("mirrors.json");
    let gui = expected["gui"].as_object().unwrap();

    for difficulty in Difficulty::ALL {
        let Some(map) = gui.get(difficulty.label()).and_then(|v| v.as_object()) else {
            continue;
        };
        for (active, mirror) in map {
            let active: u32 = active.parse().unwrap();
            let mirror = mirror.as_u64().unwrap() as u32;
            assert_eq!(
                mirror_of(active, difficulty),
                Some(mirror),
                "{difficulty:?} active {active}"
            );
        }
    }
}

#[test]
fn our_folder_bands_match_the_guis() {
    let expected = load("mirrors.json");
    let gui_folders = expected["gui_folders"].as_object().unwrap();
    let bases = expected["folder_base"].as_object().unwrap();

    for difficulty in Difficulty::ALL {
        let label = difficulty.label();
        assert_eq!(
            FOLDER_MIRROR_BASE[difficulty.index()] as u64,
            bases[label].as_u64().unwrap(),
            "{label} base"
        );

        let map = gui_folders[label].as_object().unwrap();
        assert_eq!(map.len(), MIRRORED_FOLDERS, "{label} folder count");
        for (folder, mirror) in map {
            let folder: usize = folder.parse().unwrap();
            assert_eq!(
                folder_mirror(folder, difficulty),
                Some(mirror.as_u64().unwrap() as u32),
                "{label} folder {folder}"
            );
        }
    }
}

#[test]
fn our_presets_match_the_builder_exactly() {
    let expected = load("presets.json");
    let builder = expected["builder"].as_object().unwrap();
    assert_eq!(builder.len(), STORY_PRESETS.len(), "preset count");

    for preset in STORY_PRESETS {
        let theirs = builder
            .get(preset.name)
            .unwrap_or_else(|| panic!("the builder has no preset {:?}", preset.name));

        assert_eq!(
            preset.flags,
            flags_of(&theirs["flags"]).as_slice(),
            "{} flags",
            preset.name
        );
        assert_eq!(
            preset.folders,
            flags_of(&theirs["folders"]).as_slice(),
            "{} folders",
            preset.name
        );
    }
}

#[test]
fn the_gui_and_the_builder_agree_on_every_preset() {
    // This is the check that would have caught the fabricated "preset drift"
    // once claimed in the spec: the two Python dictionaries are identical, so
    // there is nothing for the canonical list to reconcile. If this ever fails,
    // the two Python sources have genuinely diverged and the canonical list
    // needs a deliberate decision rather than a silent merge.
    let expected = load("presets.json");
    let gui = expected["gui"].as_object().unwrap();
    let builder = expected["builder"].as_object().unwrap();

    assert_eq!(gui.len(), builder.len(), "preset counts differ");
    for (name, their_flags) in builder {
        let our_flags = gui
            .get(name)
            .unwrap_or_else(|| panic!("the GUI has no preset {name:?}"));
        assert_eq!(
            our_flags, their_flags,
            "the two sources disagree on {name:?}"
        );
    }

    // And our list is a third copy that must match the GUI too.
    for preset in STORY_PRESETS {
        assert_eq!(
            preset.flags,
            flags_of(&gui[preset.name]["flags"]).as_slice(),
            "{}: our list drifted from the Python sources",
            preset.name
        );
        assert_eq!(
            preset.folders,
            flags_of(&gui[preset.name]["folders"]).as_slice(),
            "{} folders: our list drifted from the Python sources",
            preset.name
        );
    }
}

#[test]
fn flag_66_is_absent_from_the_presets_that_should_not_have_it() {
    // Pins the specific error an earlier draft made: flag 66 (Apocalymon, W1)
    // was wrongly added to "After World 3" and "All worlds + keys".
    let expected = load("presets.json");
    let gui = expected["gui"].as_object().unwrap();
    for name in ["After World 3", "All worlds + keys"] {
        assert!(
            !flags_of(&gui[name]["flags"]).contains(&66),
            "the Python GUI does not set flag 66 in {name}"
        );
        assert!(
            !preset_by_name(name).unwrap().flags.contains(&66),
            "and neither should we: {name}"
        );
    }
}
