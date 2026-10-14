//! Regenerate `src/bindings/*.ts`.
//!
//! Run from the workspace root: `cargo run -p dw4ipc --example gen_bindings`.

use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/bindings");
    std::fs::create_dir_all(&dir).expect("create bindings dir");

    let cfg = dw4ipc::bindings::binding_config();
    for (name, body) in dw4ipc::bindings::exported_types(&cfg).expect("export") {
        let path = dir.join(format!("{name}.ts"));
        std::fs::write(&path, body).expect("write binding");
        println!("wrote {}", path.display());
    }
}
