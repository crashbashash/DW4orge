//! Checked-in bindings must match what the Rust types render.

use std::path::PathBuf;

fn bindings_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src/bindings")
}

#[test]
fn checked_in_bindings_match_the_rust_types() {
    let cfg = dw4ipc::bindings::binding_config();
    let dir = bindings_dir();
    let mut problems = Vec::new();

    for (name, expected) in dw4ipc::bindings::exported_types(&cfg).expect("export") {
        let path = dir.join(format!("{name}.ts"));
        match std::fs::read_to_string(&path) {
            Ok(actual) if actual == expected => {}
            Ok(actual) => problems.push(format!(
                "{name}.ts differs ({} vs {} bytes, first difference at byte {:?})",
                actual.len(),
                expected.len(),
                first_difference(actual.as_bytes(), expected.as_bytes())
            )),
            Err(err) => problems.push(format!("{name}.ts: {err}")),
        }
    }

    assert!(
        problems.is_empty(),
        "checked-in bindings are stale; run `cargo run -p dw4ipc --example gen_bindings`\n{}",
        problems.join("\n")
    );
}

/// Byte offset of the first difference, so the failure names the spot.
fn first_difference(a: &[u8], b: &[u8]) -> Option<usize> {
    a.iter().zip(b).position(|(x, y)| x != y).or_else(|| {
        if a.len() == b.len() {
            None
        } else {
            Some(a.len().min(b.len()))
        }
    })
}
