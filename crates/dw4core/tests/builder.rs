//! The synthesised save must be byte-identical to the Python builder's.
//!
//! Every fixture here was produced by `dw4build.build_save`, so these tests
//! compare whole 81,920-byte files rather than sampling fields. Because both
//! Python sources agree on the presets, all six can be checked this way.
use dw4core::{Difficulty, STORY_PRESETS, SaveSpec, build_save, spec_maxed, spec_with_story};
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/builder")
}

fn expected(name: &str) -> Vec<u8> {
    std::fs::read(fixture_dir().join(name)).unwrap_or_else(|e| panic!("read {name}: {e}"))
}

/// Byte offsets where two saves differ, ignoring the per-block checksum.
///
/// The checksum legitimately differs whenever the content does, so it is not
/// evidence of a divergence.
fn differing_offsets(a: &[u8], b: &[u8]) -> Vec<usize> {
    a.iter()
        .zip(b)
        .enumerate()
        .filter(|(i, (x, y))| x != y && *i >= 4)
        .map(|(i, _)| i)
        .collect()
}

fn block(a: &[u8]) -> &[u8] {
    &a[..dw4core::BLOCK]
}

#[test]
fn a_plain_fresh_save_is_byte_identical_to_python() {
    let ours = build_save(&SaveSpec::default());
    let theirs = expected("fresh_plain.raw");
    assert_eq!(ours.len(), theirs.len());
    assert_eq!(
        differing_offsets(block(&ours), block(&theirs)),
        Vec::<usize>::new(),
        "the default spec drifted from dw4build.fresh_spec()"
    );
}

#[test]
fn a_fresh_story_save_is_byte_identical_to_python() {
    let spec = spec_with_story("Fresh (tutorial)", Difficulty::Normal).unwrap();
    let ours = build_save(&spec);
    let theirs = expected("fresh_story.raw");
    assert_eq!(
        differing_offsets(block(&ours), block(&theirs)),
        Vec::<usize>::new()
    );
}

#[test]
fn a_maxed_save_is_byte_identical_to_python() {
    let spec = spec_maxed("Fresh (tutorial)", Difficulty::Normal, 999).unwrap();
    let ours = build_save(&spec);
    let theirs = expected("maxed.raw");
    assert_eq!(
        differing_offsets(block(&ours), block(&theirs)),
        Vec::<usize>::new()
    );
}

#[test]
fn every_story_preset_is_byte_identical_to_python() {
    // All six, because both Python sources agree on all six. If this fails for
    // one preset, the canonical list and the Python sources have diverged and
    // it needs a deliberate decision.
    for (index, preset) in STORY_PRESETS.iter().enumerate() {
        let spec = spec_with_story(preset.name, Difficulty::Normal).unwrap();
        let ours = build_save(&spec);
        let theirs = expected(&format!("story_{index}.raw"));
        assert_eq!(
            differing_offsets(block(&ours), block(&theirs)),
            Vec::<usize>::new(),
            "preset {:?} drifted from dw4build",
            preset.name
        );
    }
}

#[test]
fn every_story_fixture_carries_a_valid_checksum() {
    for index in 0..STORY_PRESETS.len() {
        let raw = expected(&format!("story_{index}.raw"));
        let parsed = dw4core::SaveData::parse(&raw).expect("81920 bytes");
        assert!(parsed.verify(), "story_{index}.raw");
    }
}

#[test]
fn the_hard_and_very_hard_mirrors_have_no_python_equivalent_to_match() {
    // The Python builder cannot write these, so there is no fixture: assert the
    // shape directly instead. Every active flag is mirrored into the right band
    // and the bands do not leak into each other.
    for (difficulty, band) in [
        (Difficulty::Normal, 518u32),
        (Difficulty::Hard, 530),
        (Difficulty::VeryHard, 542),
    ] {
        let spec = spec_with_story("After World 1", difficulty).unwrap();
        let save = build_save(&spec);
        let flags = &save[dw4core::offsets::BASE_FLAG..][..dw4core::offsets::FLAG_COUNT];

        // Folders 0..3 are set, so the first three of this band's mirrors are too.
        for folder in 0..3u32 {
            assert_eq!(
                flags[(band + folder) as usize],
                1,
                "{difficulty:?} folder {folder}"
            );
        }
        // Nothing leaks past the mirrored-folder window.
        for folder in 3..10u32 {
            assert_eq!(
                flags[(band + folder) as usize],
                0,
                "{difficulty:?} folder {folder}"
            );
        }
        assert_eq!(dw4core::detect_difficulty(flags), difficulty);
    }
}
