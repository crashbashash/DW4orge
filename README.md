# DW4orge

A desktop save editor for **Digimon World 4** (PS2, USA, `SLUS_208.36`), written
in Rust with a React frontend. It reads and writes raw 81,920-byte saves and
`.ps2` memory-card images, and ships a headless CLI over the same core library.

The format, the card filesystem and the architecture are documented in
[`docs/save-format.md`](docs/save-format.md). The public API layered on top is
[`docs/dw4ipc-cli-tauri-design.md`](docs/dw4ipc-cli-tauri-design.md), and the
frontend is [`docs/frontend-design.md`](docs/frontend-design.md).

## Layout

```text
crates/dw4core   all game logic: format, catalogue, document, memcard
crates/dw4ipc    the payloads and service layer shared by the CLI and the app
crates/dw4cli    the headless binary (info, dump, new, verify, items)
src-tauri        the Tauri 2 shell (excluded from the root workspace)
src              the React 19 + TypeScript frontend
tools            the fixture generator
```

## Rust

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`src-tauri` needs `libwebkit2gtk-4.1-dev`, `libsoup-3.0-dev`,
`libjavascriptcoregtk-4.1-dev` and `librsvg2-dev`, so it is excluded from the
root workspace and built separately from `src-tauri/` on a host that has them.

## Frontend

```bash
npm install
npm run dev        # http://localhost:5173
```

In a plain browser the app runs against a fixture-backed mock backend (generated
from `dw4ipc`), so no Rust build or webkit is needed to develop the UI; use
**Load sample save** to open the bundled save. Inside the Tauri shell the same
code talks to the real commands.

```bash
npm run typecheck
npm run lint
npm test
npm run build
npm run check:contrast   # palette WCAG ratios, reads src/styles/tokens.css
```

The checked-in TypeScript bindings and mock fixtures are generated from Rust;
regenerate them with:

```bash
cargo run -p dw4ipc --example gen_bindings
cargo run -p dw4ipc --example gen_ui_fixtures
```

The app icons in `src-tauri/icons/` and the browser favicon in `public/` are
generated from the committed source artwork (square, 1024×1024, RGBA):

```bash
python3 -m venv /tmp/iconvenv
/tmp/iconvenv/bin/pip install Pillow
/tmp/iconvenv/bin/python tools/gen_icons.py
```

## Release

`ci.yml` gates every push and pull request. `release.yml` builds the desktop
bundles when a `v*` tag is pushed — `.deb` + `.AppImage`, `.msi` and a universal
`.dmg`, attached to a draft GitHub release:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

The bundles are **unsigned**, so macOS Gatekeeper and Windows SmartScreen will
warn on first run.

## Verifying a save in-game

Whether the game accepts an edited save can only be checked in an emulator or on
hardware. Load the written `.ps2` in PCSX2 and confirm the game lists and loads
it; the automated suite cannot see that failure.
