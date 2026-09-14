# DW4orge

A save editor for **Digimon World 4** (PS2, NTSC-U, `SLUS_208.36`).

Point it at a memory-card image or a bare save file, change your Digimon,
inventory or progress in a normal form UI, and write it back. It also ships a
headless CLI over the same core, and it never touches the network.

The original editor for this game was a Python/Tkinter script that no longer
runs on a modern desktop. DW4orge rebuilds it as a Rust core plus a React
desktop app (Tauri), and adds what that one never had: creating a memory card
from nothing, validating every edit against the game's own limits, and a
scriptable CLI.

## Using it

Open a `.ps2` memory card or a bare 81,920-byte save — the file's own bytes
decide which, not the extension. A new save can be created from nothing:
DW4orge synthesises a complete memory card that PCSX2 mounts and the game loads.

Edits go through six sections (below), with validation on every change that
points at the exact field and says how to fix it. Undo/redo spans the whole
session, and an indicator shows whether there are unsaved changes. Saving is
atomic, keeps a `.bak` of the previous file, and verifies the result before
reporting success.

Two modes keep you honest:

- **Normal** mirrors the game. Boxes are capped at what the game accepts —
  level 999, BIT 9,999,999, 9 of each disk, per-slot power-up caps — so a
  rejected value cannot be entered in the first place.
- **Advanced** lifts those caps to the field's data-type range, and exposes raw
  item ids, for experimentation.

Light and dark themes follow the OS with a manual override, including the
native window title bar.

## Features

| Section | What it edits |
| --- | --- |
| **Character** | Species (16), player name, BIT, X-Data, junk-shop tier, level, EXP, the 9 technique levels and the 11 X-Data power-ups |
| **Items** | The 30-slot device folder: searchable picker over the 539-item catalogue, rarity colour, `+N` bonus and mod count; Advanced also edits raw ids |
| **Equipment** | 3 weapons, armor, board, and the 5 weapon / 5 armor mod sockets. A mod chip that is not in the inventory is added for you |
| **Disks** | The 12 disk counts — HP/MP α–γ, Cure, Raise, Gate, Recovery, B. Pack, Key Chain |
| **Story** | Difficulty, six one-click story presets, and the 1024 story flags and folders grouped by intro, chapters, bosses, quests, lobby and folders, with a live preview of the difficulty's mirrors |
| **Bank** | Balance and the item slots |

The item catalogue and every game constant are vendored from the disassembly,
so the editor's labels, rarity colours and caps match the game rather than being
guessed.

### Headless CLI

The same core, without a window. Build it with `cargo build -p dw4cli` or run it
with `cargo run -p dw4cli --`:

```bash
dw4cli info Mcd001.ps2          # one-screen summary
dw4cli dump Mcd001.ps2          # every editable field
dw4cli new --species Dorumon --name abcdefgh -o new.ps2
dw4cli verify new.ps2           # checksums and card ECC
dw4cli items --query "wisdom" --category mods
```

Every verb takes `--json` for scripting.

## Run it

### From a release

Download the bundle for your OS from the Releases page:

| OS | Bundle |
| --- | --- |
| Linux | `.deb` (Debian/Ubuntu) or `.AppImage` (any distro) |
| Windows | `.msi` |
| macOS | universal `.dmg` |

### In a browser, no Rust build needed

The frontend runs against a fixture-backed mock backend:

```bash
npm install
npm run dev        # http://localhost:5173
```

Click **Load sample save**. This is the fastest way to work on the UI; inside
the Tauri shell the same code talks to the real commands.

### The desktop app from source

You need Rust 1.88+, Node 22+, and on Linux the webkit/GTK development packages
(`libwebkit2gtk-4.1-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
librsvg2-dev`, plus `build-essential pkg-config`).

```bash
npm install
npx tauri dev      # development build
npx tauri build    # bundles for the current OS
```

## Build and test

```bash
# Rust: format, catalogue, document, memcard, CLI, service layer
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Frontend
npm run typecheck
npm run lint
npm test
npm run build
npm run check:contrast   # palette WCAG ratios, reads src/styles/tokens.css
```

`src-tauri` is its own workspace, excluded from the root because it needs the
webkit sysroot. Build and check it from `src-tauri/`:

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
```

The checked-in TypeScript bindings and browser fixtures are generated from Rust;
regenerate them when a payload or the version changes:

```bash
cargo run -p dw4ipc --example gen_bindings
cargo run -p dw4ipc --example gen_ui_fixtures
```

The app icons are generated from the committed 1024² source artwork with
`tools/gen_icons.py` (needs Pillow).

## Roadmap

Known gaps, roughly in priority order:

- [ ] **Launch the Windows `.msi` and macOS `.dmg`.** CI builds them, but no one
  has ever run them.
- [ ] **Test the other PS2 regional releases.** Only NTSC-U is exercised. The
  NTSC-J and PAL discs may name the save on the card differently (NTSC-U is
  `BASLUS-20836savedata`) and could differ in the fields themselves; every
  measurement in `docs/save-format.md` comes from the NTSC-U image.
- [ ] **Build a GameCube save editor.** DW4orge is only setup to work with PS2.
- [ ] **Native Wayland without the compatibility hook**, on a real compositor.
- [ ] **`AppInfo.name` reports `"dw4ipc"`** — it is the service crate's package
  name, not the product name.
- [ ] **Packaging for Arch and Flatpak.** The `.deb` and `.rpm` do not install on
  Arch-based distros, so those users are limited to the AppImage.

## Documentation

- [`docs/save-format.md`](docs/save-format.md) — the save layout, memory-card
  filesystem, checksums and item-id packing.
- [`docs/dw4ipc-cli-tauri-design.md`](docs/dw4ipc-cli-tauri-design.md) — the IPC
  payloads and the CLI/Tauri service layer.
- [`docs/frontend-design.md`](docs/frontend-design.md) — the store, validation
  flow, fixtures and testing.

## Layout

```text
crates/dw4core   all game logic: format, catalogue, document, memcard
crates/dw4ipc    the payloads and service layer shared by the CLI and the app
crates/dw4cli    the headless binary (info, dump, new, verify, items)
src-tauri        the Tauri 2 shell (excluded from the root workspace)
src              the React 19 + TypeScript frontend
tools            the fixture and icon generators
```

## Licence

GPL-3.0-or-later. See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).
