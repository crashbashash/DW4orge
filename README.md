# DW4orge

A save editor for **Digimon World 4** / **Digimon World X** on PS2 (USA, Europe and Japan), GameCube, and Xbox (USA and Japan).

Point it at a memory-card image or a bare save file, change your Digimon,
inventory or progress in a normal form UI, and write it back. It also ships a
headless CLI over the same core, and it never touches the network.

The original editor for this game was a Python/Tkinter script that no longer
runs on a modern desktop. DW4orge rebuilds it as a Rust core plus a React
desktop app (Tauri), and adds what that one never had: creating a memory card
from nothing, validating every edit against the game's own limits, and a
scriptable CLI.

## Using it

### Open and edit a save

Choose **Open** and select a PS2 `.ps2` memory card, a GameCube `.raw`/`.gcp`
memory card or `.gci` save, an Xbox `SaveData.dat` file or FATX memory-unit `.img`, or a bare 81,920-byte PS2/GameCube save. The editor detects the format
automatically. Choose the regional save when a card contains more than
one region, then edit the character, inventory, equipment or progress. Xbox saves
show a **Character box** selector for occupied boxes; memory units can also
contain multiple save containers. Switching boxes checks for unsaved changes.

**Save** updates the opened file. **Save As** writes a separate copy. Existing
memory cards keep their icons, filesystem and unrelated saves. GameCube cards
can also be exported to `.gci`; `.bin` exports only the game-data blocks.
Xbox Save As keeps all character boxes; `.img`
keeps the whole memory unit and `.dat` exports the selected save container.
Xbox writes re-sign both records and preserve other saves and filesystem metadata.
Normal mode packs bank items together because the Xbox loader stops at the first
empty slot; Advanced mode retains raw bank placement.
Open an extracted HDD save file; direct HDD/qcow2 image editing is not supported.
Close xemu or detach the memory unit before replacing its image.

### Create a save

Choose **New Save**, then select the save format, region, species, player name,
story preset and difficulty:

- **PS2 memory card (.ps2)** creates a complete card for PCSX2.
- **GameCube save (.gci)** creates a standalone save for import through
  Dolphin's Memory Card Manager or use in a GCI folder.
- **GameCube memory card (.raw)** creates a complete 251-block (2 MiB) card
  containing the save. Select it as Dolphin's Slot A memory card.

Click **Create**, make any further edits, then **Save As** to write the file.
No existing save or memory card is needed. New GameCube saves start with a
blank banner and icon; the game replaces them when it saves.

USA, Europe and Japan (Digimon World X) are available. Existing cards retain
their region. Player names use the game's fullwidth character encoding; the
editor's item labels are in English.

Edits go through six sections (below), with validation on every change that
points at the exact field and says how to fix it. Undo/redo spans the whole
session, and an indicator shows whether there are unsaved changes — a save you
have not written yet counts as unsaved from the moment it is created, and
closing the window asks before discarding. Saving is atomic, keeps a `.bak` of
the previous file, and verifies the result before reporting success.

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
| **Story** | Six retail-derived progression presets, per-difficulty drafts, flags, key folders and mirror-write previews |
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
dw4cli info SaveData.dat        # Xbox save and available character boxes
dw4cli info SaveData.dat --xbox-slot 'SaveData.dat#4'
dw4cli verify MemoryUnit.img    # Xbox signatures and FATX structure
dw4cli items --query "wisdom" --category mods
```

Every verb takes `--json` for scripting.

## Run it

### From a release

Download the bundle for your OS from the [Releases page](https://github.com/crashbashash/DW4orge/releases):

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

### Regional CLI examples

```bash
dw4cli new --region europe --name EUROTEST --out europe.ps2
dw4cli new --region japan --name JPTEST --out japan.ps2
dw4cli info europe.ps2           # detects Europe automatically
dw4cli verify japan.ps2         # checks save checksums and card ECC
dw4cli info mixed.ps2 --region japan  # select a save on a mixed-region card
```

New cards default to USA when `--region` is omitted. `new --card donor.ps2`
inherits the donor's region; use `--region` to select on a mixed-region donor.
Writing to an existing card requires the matching regional save to exist; the
editor does not rename or convert another region's save silently.

## Roadmap

- [x] **Xbox save support.**
- [ ] **Select additional character save boxes on GameCube.**
- [ ] **Native Wayland support.**
- [ ] **Packaging for Arch and Flatpak.**

## Documentation

- [`docs/save-format.md`](docs/save-format.md) — the save layout, memory-card
  filesystem, checksums and item-id packing.
- [`docs/xbox-save-research.md`](docs/xbox-save-research.md) — Xbox retail save
  layout, signing evidence and the experimental inspection/editing probe.
- [`docs/dw4ipc-cli-tauri-design.md`](docs/dw4ipc-cli-tauri-design.md) — the IPC
  payloads and the CLI/Tauri service layer.
- [`docs/testing.md`](docs/testing.md) — optional desktop agent controls and
  native editor/game regression runners.
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

## Disclaimer

This tool and the reverse engineering that went into it have been made with the assistance of AI.

## Licence

GPL-3.0-or-later. See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).
