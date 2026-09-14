//! Regenerate `src/ipc/fixtures/*.json`.
//!
//! Run from the workspace root: `cargo run -p dw4ipc --example gen_ui_fixtures`.

use std::path::Path;

fn main() {
    let dir = dw4ipc::fixtures::fixtures_dir();
    std::fs::create_dir_all(&dir).expect("create fixtures dir");

    let save_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../dw4core/tests/fixtures");
    for (name, body) in dw4ipc::fixtures::ui_fixtures(&save_root) {
        let path = dir.join(name);
        std::fs::write(&path, body).expect("write fixture");
        println!("wrote {}", path.display());
    }
}
