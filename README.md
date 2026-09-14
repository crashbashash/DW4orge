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
```

The checked-in TypeScript bindings and mock fixtures are generated from Rust;
regenerate them with:

```bash
cargo run -p dw4ipc --example gen_bindings
cargo run -p dw4ipc --example gen_ui_fixtures
```

## Verifying a save in-game

Whether the game accepts an edited save can only be checked in an emulator or on
hardware. Load the written `.ps2` in PCSX2 and confirm the game lists and loads
it; the automated suite cannot see that failure.
