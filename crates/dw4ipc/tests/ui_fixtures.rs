//! The committed frontend fixtures must match what Rust renders today.

use std::path::PathBuf;

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src/ipc/fixtures")
}

fn save_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dw4core/tests/fixtures")
}

#[test]
fn checked_in_ui_fixtures_match_rust() {
    let dir = fixtures_root();
    let mut problems = Vec::new();
    for (name, expected) in dw4ipc::fixtures::ui_fixtures(&save_root()) {
        let path = dir.join(name);
        match std::fs::read_to_string(&path) {
            Ok(actual) if actual == expected => {}
            Ok(actual) => problems.push(format!(
                "{name} differs ({} vs {} bytes)",
                actual.len(),
                expected.len()
            )),
            Err(err) => problems.push(format!("{name}: {err}")),
        }
    }
    assert!(
        problems.is_empty(),
        "frontend fixtures are stale; run `cargo run -p dw4ipc --example gen_ui_fixtures`\n{}",
        problems.join("\n")
    );
}

/// A fixture must not embed a path that depends on where it was rendered, or
/// the drift test passes on the machine that generated it and fails everywhere
/// else (CI caught exactly that: the render differed by 15 bytes, the length of
/// the runner's checkout path).
#[test]
fn ui_fixtures_do_not_embed_a_machine_specific_path() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for (name, body) in dw4ipc::fixtures::ui_fixtures(&save_root()) {
        assert!(
            !body.contains(manifest_dir),
            "{name} embeds the manifest directory {manifest_dir}"
        );
    }
}
