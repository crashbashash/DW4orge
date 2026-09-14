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
