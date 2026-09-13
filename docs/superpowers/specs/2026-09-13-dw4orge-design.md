# DW4orge — Design

**Date:** 2026-09-13
**Status:** approved design, ready for implementation planning
**Repo:** <https://github.com/crashbashash/DW4orge> (GPL-3.0)

---

## 1. Purpose

DW4orge is a desktop save editor for **Digimon World 4** (PS2, USA,
`SLUS_208.36`). It is a rewrite of the existing Python/tkinter editor in
`Decomp/DW4/DW4_Save_Editor/` as a **Tauri 2** application (Rust core + React
frontend), with a redesigned interface and a headless CLI.

The reverse-engineering work that makes editing possible lives in
`Decomp/DW4/` (`pi-re/ghidra-projects/dw4/docs/`, `FLAG_MAP.md`,
`CRITICAL_PATH.md`, the item JSONs, and real memory-card dumps). That tree is
**read-only**: it is a source of knowledge and of golden test fixtures, never a
build or runtime dependency.

### Scope

#### In scope

- Feature parity with the Python editor: character, device folder, equipment,
  disks, story flags/folders, bank, raw edits, and from-scratch save synthesis.
- `.ps2` memory-card images (read and in-place write) and raw 81920-byte saves.
- Normal (safe) and Advanced (guard-rails off) editing modes.
- A headless CLI over the same core library.
- Windows, macOS and Linux desktop builds.

#### Out of scope

- Exposing RE data the Python editor never surfaced (no flag-xref browser, no
  enemy/item database browser, no Lua tooling).
- Save diffing, batch generation, a struct inspector, or hex editing of
  anything other than the 0xA000 save block through the existing raw-edit path.
- Editing fields the game recomputes on load (HP/MP and all derived stats).
- Any modification of the `Decomp/` tree.

---

## 2. Approaches considered

Three shapes were considered for the Rust side:

1. **Single Tauri crate with the save logic inline.** Fewest files. Rejected:
   the save logic becomes untestable without a Tauri build, and a headless CLI
   would have to duplicate or reach into the app crate.
2. **`dw4core` library + Tauri app.** Testable core, thin shell. Good, but the
   CLI would still live inside the app crate.
3. **Cargo workspace: `dw4core` + `dw4cli` + `src-tauri` (chosen).** The core
   is pure and has no Tauri, UI or I/O-policy dependency; the CLI is a
   first-class consumer; the Tauri crate is a thin command layer. This is what
   makes the golden-fixture parity tests and real local verification cheap.

---

## 3. Architecture

### 3.1 Repository layout

```text
DW4orge/
├─ Cargo.toml                     # [workspace] members
├─ package.json                   # frontend + dev scripts
├─ crates/
│  ├─ dw4core/                    # ALL game logic. No Tauri, no UI.
│  │  ├─ data/                    # vendored item DB JSON + generated flag tables
│  │  ├─ src/
│  │  │  ├─ lib.rs
│  │  │  ├─ offsets.rs            # every block offset, defined once
│  │  │  ├─ save.rs               # SaveData: typed access, mirror-both-blocks, checksum
│  │  │  ├─ codes.rs              # level formula, junk tiers, rarity, caps
│  │  │  ├─ name.rs               # fullwidth <-> ASCII player-name codec
│  │  │  ├─ item.rs               # build/split/describe item id, invalid_reason
│  │  │  ├─ catalogue.rs          # embedded data/*.json -> ItemCatalogue
│  │  │  ├─ flags.rs              # story flags, folders, mirror map, presets
│  │  │  ├─ builder.rs            # pure-synthesis new save
│  │  │  ├─ document.rs           # SaveView projection, EditSet, validate, apply
│  │  │  ├─ memcard/              # CardBackend trait + impls
│  │  │  └─ error.rs              # thiserror enum
│  │  └─ tests/                   # golden fixtures, round-trips
│  └─ dw4cli/                     # headless binary
├─ src-tauri/                     # Tauri 2 shell
│  ├─ src/{main,lib,commands,state,error}.rs
│  ├─ tauri.conf.json
│  ├─ capabilities/
│  └─ icons/
├─ src/                           # React 19 + TS + Vite
│  ├─ app/  components/  features/  lib/  bindings/  styles/
├─ tools/gen_fixtures.py          # regenerates vendored data + golden fixtures from Decomp/
├─ .github/workflows/{ci,release}.yml
└─ docs/superpowers/specs/
```

### 3.2 Responsibility boundaries

| Unit | Does | Depends on |
| --- | --- | --- |
| `SaveData` | byte-level read/write of the 81920-byte save, both mirrored blocks, checksum | nothing but `offsets`, `struct` |
| `ItemCatalogue` | id → name/category/grade lookup, validity, display strings | vendored JSON |
| `flags` | story flag/folder tables, per-difficulty mirror maps, presets | consts |
| `builder` | synthesise a complete save from a `SaveSpec` | `offsets`, `codes`, `flags` |
| `Document` | project a `SaveData` into a `SaveView`; validate and apply an `EditSet` | all of the above |
| `CardBackend` | read/write the save file inside a card image or a raw file | `ps2-memcard` (one impl) |
| `dw4cli` | headless entry point | `dw4core` |
| `src-tauri` | IPC commands, window/app lifecycle, file dialogs | `dw4core`, Tauri |
| React app | draft state, validation display, all presentation | Tauri IPC only |

Each unit is independently testable. `CardBackend` is the only seam through
which a third-party crate (`ps2-memcard`) enters, so replacing it with a native
implementation touches one file.

### 3.3 Data flow

Rust owns the file; TypeScript owns the draft.

1. **Open** — `open_save(path)` reads the card or raw file via `CardBackend`,
   parses `SaveData`, and returns a `SaveView`: file identity, checksum status,
   every editable field, the item catalogue, the caps/limits table, story
   flag+mirror state, and the detected difficulty.
2. **Edit** — the UI copies editable fields into a draft in a reducer with
   undo/redo stacks. Inline validation uses `limits` and `catalogue` from the
   view, so no round-trip per keystroke.
3. **Save** — `save(path)` / `save_as(path)` sends the draft. The Rust side
   re-validates authoritatively, applies to a fresh copy of the originally
   loaded save, fixes both checksums, writes atomically, re-reads and verifies,
   then returns the refreshed `SaveView`. On failure it returns
   `FieldError[]` and writes nothing.
4. **New** — `new_save(species, name, story, difficulty)` synthesises in the
   core. No template file is needed.

### 3.4 IPC surface

`open_save`, `new_save`, `get_view`, `validate_edits`, `save`, `save_as`,
`export_raw`, `app_info`.

All payload types derive `serde::Serialize`/`Deserialize` and `ts_rs::TS`. A
Rust test regenerates the `.ts` bindings and fails if the checked-in output
differs, so the two sides cannot drift.

### 3.5 Write safety

- Temp file in the destination directory, then atomic rename.
- A `.bak` sibling is written once before the first overwrite of an existing
  file; subsequent saves in the same session do not overwrite the `.bak`.
- After writing, the destination is re-read, parsed and checksum-verified
  before success is reported.

---

## 4. Save format

The save file is `BASLUS-20836savedata`, 81920 bytes: **two byte-identical
0xA000-byte blocks** (mirror/backup).

Per block: `checksum = Σ u32le(block+4 .. block+0xA000) mod 2³²`, stored at
`block+0`. Writes go to both blocks; checksums are fixed for both.

### 4.1 Block field map (block-relative offsets)

| Offset | Size | Field |
| --- | --- | --- |
| `+0x00` | u32 | checksum |
| `+0x04` | u32 | version (4) |
| `+0x08` | u32 | `ISUSE` |
| `+0x0c` | u32 | `UNIQUE` (`0x6096F82C`) |
| `+0x10` | 16 B | `DIGIMONNAME` (ASCII model stem, `p_<stem>`) |
| `+0x30` | 8 B | `PLAYERNAME` (u16 `0xFFFF` marker + up to 3 fullwidth chars) |
| `+0x50` | u32 | `LEVEL` (menu snapshot) |
| `+0x54`–`+0x60` | 4×u32 | `HP`/`MHP`/`MP`/`MMP` (**derived, never edited**) |
| `+0x64` | u32 | `XDATA` |
| `+0x68` | u32 | `BIT` |
| `+0x6c` | 36×u32 | `DEVICEFOLDER` (30 used + 6 reserved; `0xFFFFFFFF` = empty) |
| `+0xfc` | 12×u32 | `DISKFOLDER` (`(count << 16) OR (0x4000 + i)`) |
| `+0x12c` | 52 B | `CARDLIST` (bitfield; read-only in this app) |
| `+0x160` | 96×u32 | `BANKDEVICE` (`0xFFFFFFFF` = empty) |
| `+0x2e0` | u32 | `BANKBIT` |
| `+0x2e4` | 3×u32 | weapon slots (indices into the device folder) |
| `+0x2f0` | 5×u32 | weapon-mod sockets |
| `+0x304` | u32 | armor / core slot |
| `+0x308` | 5×u32 | armor-mod sockets |
| `+0x31c` | u32 | sub / board slot |
| `+0x320` | 1024 B | `BASE_FLAG` (`0x01` = set/done, `0x00` = clear) |
| `+0x720` | 12 B | `BASE_FLAGFOLDER` (`0x01` = SET) |
| `+0x72c` | 16×u32 | `BASE_COUNTER` (junk donations live at `[1]`, `+0x730`) |
| `+0x76c` | 16×u8 | `BASE_ISUSE` |
| `+0x77c` | 16×u32 | `BASE_LEVEL` (per species) |
| `+0x7bc` | 16×u32 | `BASE_EXP` (per species) |
| `+0x7fc` | 144×u32 | `BASE_SKILL` (16 species × 9 techniques, signed i32) |
| `+0xa3c` | 176×u32 | `BASE_UPCNT` (16 species × 11 power-ups) |
| `+0xafc`..`+0x9fff` | | zero padding |

The authoritative field semantics and their RE provenance are documented in
`Decomp/DW4/pi-re/ghidra-projects/dw4/docs/08-save-format.md`; this table is the
implementation's single source of truth for offsets (`offsets.rs`).

### 4.2 Species and models

Sixteen species indices map to `MODEL_NAME` stems in this order: `agumon`,
`vmon`, `girumon`, `dorumon`, `weregaru`, `hekabut`, `wargrey`, `angelr`,
`beelzeb`, `alpha`, `bwargrey`, `impdrafm`, `impdrapm`, `metalgaru`,
`dukecrim`, `susanoo`. The game derives the species from `DIGIMONNAME`; the
editor keeps the two in sync on every species change. Unknown stems fall back
to Dorumon (index 3), matching the Python editor.

### 4.3 Item encoding

An item in a folder is a u32:

```text
id      = base_id | ((seed << 4 | mod_count) << 16)
base_id = (category << 8) | index
```

- Categories: `0x00` graded weapons, `0x05` styled weapons, `0x10` cores
  (armor), `0x20` boards (sub), `0x30` mods/chips, `0x34` mod "equipped"
  variant (never a legal inventory value).
- `seed` (12 bits) is a **direct additive stat bonus** in its low 11 bits
  (`seed & 0x7FF`, `+0..+2047`); bit 11 is masked off. Rarity colour is a
  monotonic threshold of `seed & 0x7FF`:

  | colour | range |
  | --- | --- |
  | white | `0x000` |
  | blue | `0x001`–`0x00F` |
  | green | `0x010`–`0x07F` |
  | yellow | `0x080`–`0x1FF` |
  | orange | `0x200`–`0x3FF` |
  | pink | `0x400`–`0x7FF` |

- `mod_count` is 4 bits, 0–15.

`invalid_reason(base_id)` reproduces the confirmed crash/glitch ranges so
Normal mode can reject them. Ranges are given as full base ids, with `lo`
meaning `base_id & 0xFF` within the stated category:

| id range | Verdict |
| --- | --- |
| styled `0x0515`–`0x051a` | blank item |
| styled `0x051b`–`0x053e` | crashes on load |
| styled `0x053f` | the blank "0" weapon, cannot hit |
| core `0x1020`–`0x103f` | blank; `0x1026` crashes when selected |
| board `0x202d`–`0x2035` | crashes opening the devices folder |
| board `0x2036`–`0x203e` | crashes, but displays STRENGTH 9999 |
| board `0x203f` | crashes, but displays WISDOM 9999 |
| mod `0x30b9`+ (`lo >= 0xB9`) | blank |
| any `0x34` category ("equipped" mod variant) | game-managed, never a real item |
| anything else not in the catalogue | unknown item id |

### 4.4 Level, caps and derived values

- `threshold(n) = n³ + 137n² − 77n − 61`; `level_from_exp(exp)` is the largest
  `n` whose threshold is `<= exp`. Editing level auto-syncs EXP to
  `threshold(level)` (clamped to u32), and a stale EXP below `threshold(level)`
  is lifted on load in Normal mode.
- Junk-shop tiers are cumulative donation thresholds: tier 1 at 1,000 through
  tier 9 at 3,956,000. Selecting tier *N* writes that tier's threshold value
  into the counter, exactly as the Python editor's tier dropdown does.
- Caps: Normal mode enforces the in-game limits; Advanced permits the full
  data-type range but never beyond it.

  | Field | Normal cap | Advanced range |
  | --- | --- | --- |
  | BIT | 9,999,999 | 0..`0xFFFFFFFF` |
  | Level | 999 | 0..`0xFFFFFFFF` |
  | EXP | 1,133,652,152 | 0..`0xFFFFFFFF` |
  | X-Data | 9,999 | 0..`0xFFFFFFFF` |
  | Technique | 9,999 | −2³¹..2³¹−1 (signed i32) |
  | Power-up | 9,999 (99,999 for HP/MP max) | 0..`0xFFFFFFFF` |
  | Item `+N` | 0..`0x7FF` | 0..`0x7FF` |
  | Item mod count | 0..15 | 0..15 |
  | Disk count | 0..65,535 | 0..65,535 |

- HP/MP/MHP/MMP are recomputed by the game from the loadout; the editor never
  writes them (the builder writes plausible values only because it creates a
  block from zero).

### 4.5 Story flags, folders and the mirror system

`BASE_FLAG[i] == 0x01` means "event happened / boss defeated / flag set".
`BASE_FLAGFOLDER[i] == 0x01` means "folder SET". Folders are not inverted.

The title screen restores **active ← mirror** on load, so an edit survives only
if the mirror byte is written too. The complete per-difficulty map (from
`FLAG_MAP.md` §7) is:

| active | Normal (−1) | Hard (0) | Very Hard (1) |
| --- | --- | --- | --- |
| intro 0–4 | 699–703 | 6–10 | 12–16 |
| 18–19 | 705–706 | 740–741 | 770–771 |
| 66–68 | 707–709 | 73–75 | 77–79 |
| 82 | 711 | 742 | 772 |
| 85 | 712 | 86 | 87 |
| 88–97 | 713–722 | 743–752 | 773–782 |
| 319–325 | 724–730 | 754–760 | 784–790 |
| 328–331 | 732–735 | 762–765 | 792–795 |
| 334 | 737 | 767 | 797 |
| 48 | 800 | 801 | 802 |
| 382 | 803 | 804 | 805 |
| 81 | 806 | 807 | 808 |
| 333 | 809 | 810 | 811 |

Folders: `folder[i]` mirrors to flag `518+i` (Normal), `530+i` (Hard),
`542+i` (Very Hard), for `i` in 0–9.

Difficulty is **not stored in the save**. It is inferred by scoring each
difficulty's mirror set against the live bytes and taking the highest; the
result is shown as a badge and can be overridden in the UI.

---

## 5. Correcting the Python implementation

The Python editor is the only known-good implementation, but it contains three
internal inconsistencies. DW4orge deliberately resolves all three, and the
divergence is recorded here and in the README.

1. **Mirror table.** `dw4build.NORMAL_FLAG_MIRRORS` covers ~30 flags;
   `save_editor_gui.FLAG_MIRRORS` covers 8 and omits 18/19, 88–97, 319–331,
   334, 48, 382, 81 and 333. DW4orge implements the complete table in §4.5 for
   all three difficulties.
2. **Story presets.** The two preset dictionaries have drifted: the builder's
   "All worlds + keys" sets flag 66 while the GUI's does not; the GUI's
   "Story cleared" omits 609 while the builder's includes it. DW4orge uses one
   canonical preset list shared by the Story tab and the New Save dialog:

   | Preset | Folders set | Flags set |
   | --- | --- | --- |
   | Fresh (tutorial) | — | 1, 24, 501, 508, 931, 935, 936, 960 |
   | After World 1 | 0–2 | 0, 2, 66, 67, 701 |
   | After World 2 | 0–5 | 0, 4, 66, 67, 68, 703 |
   | After World 3 | 0–8 | 0, 5, 66, 67, 68, 69, 82, 704 |
   | All worlds + keys | 0–10 | 0, 6, 66, 67, 68, 69, 82, 705 |
   | Story cleared (post-game) | 0–10 | 0, 67, 68, 69, 82, 85, 609 |

   The two disagreements are resolved in favour of the builder's more complete
   sets (flag 66 present in "All worlds + keys"; 609 present in "Story
   cleared"), which match the real checkpoint saves the presets were derived
   from.

3. **New-save difficulty.** The Python New dialog never asks, and the builder
   always writes Normal mirrors, silently locking new saves to Normal. DW4orge
   adds a difficulty selector to New Save and writes the matching mirror set,
   defaulting to Normal.

Everything else is intended to be behaviourally identical to the Python editor.

---

## 6. Core API

```rust
pub const BLOCK: usize = 0xA000;
pub const SAVE_SIZE: usize = BLOCK * 2;
pub const EMPTY: u32 = 0xFFFF_FFFF;

pub struct SaveData { /* raw: [u8; SAVE_SIZE] */ }

impl SaveData {
    pub fn parse(bytes: &[u8]) -> Result<Self>;
    pub fn to_bytes(&self) -> [u8; SAVE_SIZE];   // checksums fixed
    pub fn verify(&self) -> bool;

    // header / character
    pub fn bit(&self) -> u32;                 pub fn set_bit(&mut self, v: u32);
    pub fn xdata(&self) -> u32;               pub fn set_xdata(&mut self, v: u32);
    pub fn menu_level(&self) -> u32;          pub fn set_menu_level(&mut self, v: u32);
    pub fn junk_counter(&self) -> u32;        pub fn set_junk_counter(&mut self, v: u32);
    pub fn digimon_name(&self) -> String;     pub fn set_digimon_name(&mut self, s: &str);
    pub fn player_name(&self) -> String;      pub fn set_player_name(&mut self, s: &str);
    pub fn detect_species(&self) -> Species;

    // per-species
    pub fn level(&self, sp: Species) -> u32;      pub fn set_level(&mut self, sp: Species, v: u32);
    pub fn exp(&self, sp: Species) -> u32;        pub fn set_exp(&mut self, sp: Species, v: u32);
    pub fn skill(&self, sp: Species, slot: usize) -> i32;
    pub fn set_skill(&mut self, sp: Species, slot: usize, v: i32);
    pub fn upcnt(&self, sp: Species, slot: usize) -> u32;
    pub fn set_upcnt(&mut self, sp: Species, slot: usize, v: u32);

    // containers
    pub fn device(&self, i: usize) -> u32;    pub fn set_device(&mut self, i: usize, v: u32);
    pub fn weapon(&self, i: usize) -> u32;    pub fn set_weapon(&mut self, i: usize, v: u32);
    pub fn wmod(&self, i: usize) -> u32;      pub fn set_wmod(&mut self, i: usize, v: u32);
    pub fn armor(&self) -> u32;               pub fn set_armor(&mut self, v: u32);
    pub fn amod(&self, i: usize) -> u32;      pub fn set_amod(&mut self, i: usize, v: u32);
    pub fn sub(&self) -> u32;                 pub fn set_sub(&mut self, v: u32);
    pub fn disk_count(&self, i: usize) -> u16; pub fn set_disk_count(&mut self, i: usize, v: u16);
    pub fn bank_bit(&self) -> u32;            pub fn set_bank_bit(&mut self, v: u32);
    pub fn bank_device(&self, i: usize) -> u32;
    pub fn set_bank_device(&mut self, i: usize, v: u32);
}

pub struct Document { /* data: SaveData, source, path, dirty baseline */ }

impl Document {
    pub fn load(path: &Path) -> Result<Self>;
    pub fn view(&self, cat: &ItemCatalogue) -> SaveView;
    pub fn validate(&self, edits: &EditSet, mode: Mode) -> Result<Vec<Warning>, Vec<FieldError>>;
    pub fn apply(&mut self, edits: &EditSet, mode: Mode) -> Result<Vec<Warning>, Vec<FieldError>>;
    pub fn save(&mut self, path: &Path) -> Result<()>;
    pub fn export_raw(&self, path: &Path) -> Result<()>;
}

pub enum Mode { Normal, Advanced }
pub enum Difficulty { Normal, Hard, VeryHard }

/// One of the 16 `MODEL_NAME` species. `from_model_stem` maps a `DIGIMONNAME`
/// value to its index, falling back to `Dorumon`.
pub enum Species { Agumon, Veemon, Girumon, Dorumon, WereGaruru, Hekabut,
                   WarGrey, AngelR, Beelzeb, Alpha, BWarGrey, ImpdraFm,
                   ImpdraPm, MetalGaru, DukeCrim, Susanoo }

pub enum Severity { Error, Warning }

pub struct FieldError {
    pub path: String,      // e.g. "device[3].mods", "equip.armor", "story.flag[66]"
    pub message: String,
    pub severity: Severity,
}
```

`EditSet` mirrors the Python `collect()` dictionary field-for-field, and
`Document::apply` reproduces its application order and quirks:

- species change rewrites `DIGIMONNAME` to `p_<stem>`;
- `menu_level` is written from the edited level;
- device slots 30–35 are forced to `EMPTY`;
- mod sockets pointing at a chip not already in the device folder cause the
  chip to be added to the first free slot (error if none is free);
- level edits write both `BASE_LEVEL` and the EXP threshold;
- story edits write the active byte *and* its difficulty-mirrored partner;
- HP/MP/MHP/MMP are never written.

Validation error `path`s are JSON-pointer keys (`device[3].mods`,
`equip.armor`, `story.flag[66]`) so the frontend can highlight the exact
control. Category mismatches on equipment are warnings, not errors, matching
the Python tool's "these may behave oddly in-game" behaviour.

### 6.1 Memcard backend

```rust
pub trait CardBackend {
    fn open(path: &Path) -> Result<Self> where Self: Sized;
    fn read_save(&mut self, dir: &str, file: &str) -> Result<Vec<u8>>;
    fn write_save(&mut self, dir: &str, file: &str, data: &[u8]) -> Result<()>;
}
```

Implementations:

- `RawFile` — a bare 81920-byte file; no filesystem.
- `Ps2Memcard` — wraps `ps2-memcard` 0.2.2 (Apache-2.0) for superblock, indirect
  FAT, directory tree and spare-area ECC.

Path selection reproduces the Python semantics: an existing `.ps2` is written in
place; a non-existent `.ps2` is created by copying the source card image and
swapping in the save data; anything else is written as a raw file. Before
committing to `ps2-memcard`, it is validated against the real 8.2 MB
`Decomp/DW4/memcards/Mcd001.ps2` and against the Python tool's `.ps2` output.
If it fails that validation, a native implementation replaces it behind the
same trait.

### 6.2 Builder

`builder.rs` is a pure function of a `SaveSpec` producing one 0xA000 block
duplicated with both checksums, byte-identical to `dw4build.build_save` for the
same spec apart from the deliberate corrections in §5. Defaults: version 4,
`ISUSE` 1, `UNIQUE = 0x6096F82C`, all species level 1 / EXP 0 / 9 techniques at
1 / power-ups 0, zero padding, disks initialised to `0x4000 + i`.

`SaveSpec` keeps the Python builder's extension points: explicit per-species
`BASE_LEVEL`/`BASE_EXP`/`BASE_SKILL`/`BASE_UPCNT` arrays, a `maxed()`
convenience (level 999, BIT 9,999,999, X-Data 9,999, maxed skills and power-ups),
and story-preset composition.

---

## 7. Frontend

### 7.1 Stack

- **React 19 + TypeScript + Vite**, built to static assets and served by Tauri.
- **`react-aria-components`** for the hard primitives — combobox, select,
  dialog, tabs — unstyled and accessible by default. All visuals are ours.
- **Hand-written CSS** with custom properties; no utility framework.
- No state library: a single `useReducer` draft store with typed actions and
  undo/redo stacks.

### 7.2 Shell

- Top bar: open, save, save as, dirty indicator, Normal↔Advanced toggle,
  light/dark toggle (defaults to the OS preference).
- Left sidebar: Character, Items, Equipment, Disks, Story, Bank, Advanced.
- Summary card: species, player name, level, BIT, X-Data, junk tier, detected
  difficulty with override, and checksum OK/mismatch.
- Status bar: open file path, last write result, error count.
- Keyboard: `Ctrl+O` open, `Ctrl+S` save, `Ctrl+Shift+S` save as, `Ctrl+Z` /
  `Ctrl+Shift+Z` draft undo/redo.

### 7.3 Sections

| Section | Contents |
| --- | --- |
| Character | species, player name, BIT, X-Data, junk tier, level, EXP, 9 techniques, 11 power-ups with per-slot safe caps |
| Items | device folder, 30 slots in 3 pages × 10 rows: bucket, searchable item picker, rarity colour and seed-derived `+N`, mod count; Advanced allows raw hex IDs |
| Equipment | 3 weapons, armor, board, 5 weapon mods, 5 armor mods; pickers filtered to the correct category, live mismatch warnings, mod chips auto-added to inventory |
| Disks | 12 owned counts, 0–65535 |
| Story | difficulty selector with detected badge, preset dropdown, grouped flag/folder checkboxes (intro, chapters, bosses, quests, lobby, folders) and a live mirror preview |
| Bank | balance plus 96 slots, reusing the Items row widget |
| Advanced | raw `offset = value` (`:u8` suffix supported) editor plus a read-only hex view of the 0xA000 block with jump-to-offset |

New Save is a dialog (species, name, story preset, difficulty). Destructive and
unsaved-changes actions use confirm dialogs; validation failures surface both
inline on the offending control and in the status bar.

---

## 8. Testing

#### Rust

- Unit tests per module.
- Property tests: `build_item_id`/`split_item_id` round-trip; `level_from_exp`
  is the inverse of `level_threshold`; the checksum is independent of write
  ordering; rarity bands are contiguous and cover `0..0x7FF`.
- Golden fixtures: a frozen set of saves and expected dumps generated once by
  `tools/gen_fixtures.py` from the Python editor and the real card dumps. The
  Rust core must reproduce them byte-for-byte.
- Round-trip: load the real `Mcd001.ps2`, edit nothing, save, re-read —
  byte-identical. Same for every fixture.
- `.ps2` integration: write a synthesised save into a copied card, re-read it,
  and confirm the card's other entries are untouched.
- Builder determinism and byte-identity against the Python builder for shared
  specs.
- Bindings drift: regenerating `ts-rs` output must not change the checked-in
  files.

#### TypeScript

- Vitest for the reducer, validation mapping and value formatting.
- Testing Library for each section against a mocked IPC layer.
- One integration test that drives the store through open → edit → save with a
  stubbed invoke, asserting the `EditSet` payload shape.

**Not automated:** whether the game accepts an edited save. That requires an
emulator or hardware and is a manual check, documented in the README.

Python is never required to build or test DW4orge; it is only used to
regenerate fixtures on demand.

---

## 9. Continuous integration and release

- **`ci.yml`** (push and PR, ubuntu): `cargo fmt --check`,
  `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`,
  `tsc --noEmit`, ESLint, `vitest run`, `vite build`.
- **`release.yml`** (tag `v*`): `tauri-apps/tauri-action` matrix over
  `ubuntu-22.04`, `windows-latest`, `macos-latest`, producing
  `.deb` + `.AppImage`, `.msi`, and a universal `.dmg`, attached to a draft
  GitHub release.

Bundle identity: product name `DW4orge`, identifier
`io.github.crashbashash.dw4orge`, version `0.1.0`.

---

## 10. Licensing and attribution

DW4orge is GPL-3.0 (matching the existing `LICENSE`). `ps2-memcard` is
Apache-2.0, which is GPL-compatible.

A `NOTICE` file credits the reverse-engineering findings and their provenance,
and records that `crates/dw4core/data/*.json` is vendored verbatim from
`Decomp/DW4/DW4_Save_Editor/data/`.

---

## 11. Risks

| Risk | Mitigation |
| --- | --- |
| `ps2-memcard` is young (111 downloads, 4 releases) and its API may be unsuitable | Behind `CardBackend`; validated against a real 8.2 MB card and the Python tool's output before adoption; native fallback is a contained change |
| `react-aria-components` API churn | Confined to four primitive components behind our own wrappers |
| Species 4–15 identification is best-effort (only the four starters are pinned) | Identical limitation to the Python editor; documented; defaults to Dorumon as before |
| Writing an edited save to a real memory card is destructive if we get the FAT/ECC wrong | Atomic write, `.bak`, post-write verification, and a live round-trip test against a real card image |
| Rarity seed→colour mapping is a threshold model with one known anomaly | Six verified colours only, exactly as the Python editor; arbitrary colours remain unavailable |

---

## 12. Milestones

1. **Workspace skeleton** — Cargo workspace, Vite/React/Tauri scaffolding, CI
   green on an empty app.
2. **`dw4core`: format layer** — offsets, `SaveData`, checksum, name codec,
   codes (level/junk/rarity/caps), item encoding; unit and property tests.
3. **`dw4core`: catalogue and data** — vendor the item JSONs, `ItemCatalogue`,
   `invalid_reason`; `tools/gen_fixtures.py`.
4. **`dw4core`: document layer** — `SaveView`, `EditSet`, validation, apply,
   story flags and the full mirror table, presets, builder.
5. **`dw4core`: memcard** — `CardBackend`, `RawFile`, `Ps2Memcard`, atomic
   writes and backups; round-trip tests against the real card.
6. **Golden fixtures** — generate, wire into the test suite, prove parity.
7. **`dw4cli`** — `info`, `dump`, `new`, `verify`, `items`.
8. **Tauri shell** — commands, state, capabilities, error mapping, file
   dialogs, `ts-rs` bindings.
9. **Frontend shell** — layout, theming, draft store, undo/redo, IPC layer.
10. **Frontend sections** — Character, Items, Equipment, Disks, Story, Bank,
    Advanced; New Save dialog; confirm dialogs.
11. **Polish** — keyboard shortcuts, inline validation, empty/error states,
    accessibility pass.
12. **Release** — icons, README, NOTICE, `release.yml`, first tagged build for
    Windows, macOS and Linux.
