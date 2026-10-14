# DW4orge — save format and design reference

How DW4orge talks to Digimon World 4 save files: the on-disk layout, the item
encoding, the story-flag mirror system, and the places where DW4orge
deliberately differs from the original Python editor.

**Repo:** <https://github.com/crashbashash/DW4orge> (GPL-3.0)

---

## 1. Purpose

DW4orge is a desktop save editor for **Digimon World 4** / **Digimon World X** (PS2, USA, Europe and Japan). It rewrites the original Python/tkinter editor as a
**Tauri 2** application (Rust core + React frontend), with a redesigned
interface and a headless CLI.

Reverse-engineering findings, item tables and reference memory-card images
provide the format knowledge and golden test fixtures. The committed data is
sufficient to build and test DW4orge; external reference material is not a
build or runtime dependency.

### Scope

#### In scope

- Feature parity with the Python editor: character, device folder, equipment,
  disks, story flags/folders, bank, and from-scratch save synthesis.
- `.ps2` memory-card images (read and in-place write) and raw 81920-byte saves.
- Normal (safe) and Advanced (guard-rails off) editing modes.
- A headless CLI over the same core library.
- Windows, macOS and Linux desktop builds.

#### Out of scope

- Exposing RE data the Python editor never surfaced (no flag-xref browser, no
  enemy/item database browser, no Lua tooling).
- Save diffing, batch generation, struct or hex inspection, and raw offset
  editing. Raw offset editing existed in the Python tool purely as a
  development aid while the format was being reverse-engineered, and is
  deliberately not carried over.
- Editing fields the game recomputes on load (HP/MP and all derived stats).
- Any modification of external reference material.

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
├─ tools/gen_fixtures.py          # regenerates vendored data + golden fixtures from external references
├─ .github/workflows/{ci,release}.yml
└─ docs/save-format.md            # this document
```

### 3.2 Responsibility boundaries

| Unit | Does | Depends on |
| --- | --- | --- |
| `SaveData` | byte-level read/write of the 81920-byte save, both mirrored blocks, checksum | nothing but `offsets`, `struct` |
| `ItemCatalogue` | id → name/category/grade lookup, validity, display strings | vendored JSON |
| `flags` | story flag/folder tables, per-difficulty mirror maps, presets | consts |
| `builder` | synthesise a complete save from a `SaveSpec` | `offsets`, `codes`, `flags` |
| `Document` | project a `SaveData` into a `SaveView`; validate and apply an `EditSet` | all of the above |
| `CardBackend` | read/write the save file inside a card image or a raw file | nothing (native) |
| `dw4cli` | headless entry point | `dw4core` |
| `src-tauri` | IPC commands, window/app lifecycle, file dialogs | `dw4ipc`, Tauri |
| React app | draft state, validation display, all presentation | Tauri IPC only |

Each unit is independently testable. `dw4core` takes no third-party **format or
filesystem** dependency: the PS2 memory-card filesystem is implemented natively
(§6.1), so there is no external format layer to work around. Its only external
crates are `serde`/`serde_json` (item catalogue, IPC payloads) and `thiserror`.

### 3.3 Data flow

Rust owns the file; TypeScript owns the draft.

1. **Open** — `open_save(path)` reads the card or raw file via `CardBackend`,
   parses `SaveData`, and returns an `OpenResult`: the file identity, the
   container kind, and a `SaveView` (checksum status, every editable field,
   story flag/folder state, the detected difficulty). The item catalogue, the
   caps/limits table, the mirror table and the presets are **static** and
   arrive once from `app_info()` as `UiData`, not on every open.
2. **Edit** — the UI copies editable fields into a draft in a reducer with
   undo/redo stacks. Inline validation uses the `caps` and `catalogue` from the
   startup `UiData`, so no round-trip per keystroke.
3. **Save** — `save(path)` / `save_as(path)` sends the draft. The Rust side
   re-validates authoritatively, applies to a fresh copy of the originally
   loaded save, fixes both checksums, writes atomically, re-reads and verifies,
   then returns the refreshed `SaveView`. On failure it returns
   `FieldError[]` and writes nothing.
4. **New** — `new_save(species, name, story, difficulty)` synthesises in the
   core. No template file is needed.

### 3.4 IPC surface

`open_save`, `new_save`, `get_view`, `validate_edits`, `save`, `save_as`,
`app_info`, `species_stats`.

`species_stats(species, mode) -> SpeciesStats` reads one species' stored block
from the open document, applying the same Normal-mode EXP lift as `get_view`.
It exists so the Character section can show another species' level, EXP,
techniques and power-ups when the selector changes (`_load_species_stats` parity),
which no other command can express.

Payloads are defined in `crates/dw4ipc` and derive
`serde::Serialize`/`Deserialize` plus `ts_rs::TS`; `dw4core`'s data types gain
the derives they need behind an optional `ts` feature. `IpcError` is the only
error shape crossing IPC: internally tagged, so the frontend branches on
`kind` (`validation` carries `FieldError[]`, then `core`, `no_open_document`,
`unsupported`).

The generated bindings live in `src/bindings/*.ts` and are checked in.
`cargo run -p dw4ipc --example gen_bindings` regenerates them; a test renders
each type and byte-compares it against the checked-in file, so the two sides
cannot drift.

The concrete payload types, error model, session semantics and binding strategy
are specified in [`docs/dw4ipc-cli-tauri-design.md`](dw4ipc-cli-tauri-design.md).

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
| `+0x0c` | u32 | `UNIQUE`, a per-save id — **varies** (`0x6096F82C`, `0x700FFDAC` observed) |
| `+0x10` | 16 B | `DIGIMONNAME` (ASCII model stem, `p_<stem>`) |
| `+0x30` | 18 B | `PLAYERNAME` (u16 `0xFFFF` marker + up to 8 fullwidth chars) |
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
| `+0xa3c` | 176×u32 | `BASE_UPCNT` (16 species × 11 power-ups), ends `+0xcfc` |
| `+0xcfc`..`+0x9fff` | | zero padding |

> **Erosion note.** `dw4build.py` documents this padding as starting at
> `+0x0afc` and defines `O_PAD_END = 0xAFC`. That is off by `0x200`: 176 u32
> from `0xa3c` ends at `0xcfc`. Confirmed empirically against an `op.ps2`
> synthesised with every power-up slot filled — its last non-zero byte is
> `0xcf9`, and 278 non-zero bytes lie beyond `0xafc`. The constant is unused
> (the builder starts from a zeroed buffer), so nothing is currently broken,
> but `offsets.rs` must use `0xcfc`.

The field semantics were established through reverse engineering. This table
is the implementation's single source of truth for offsets (`offsets.rs`).

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
  | Disk count | 0..9 | 0..65,535 |

- HP/MP/MHP/MMP are recomputed by the game from the loadout; the editor never
  writes them (the builder writes plausible values only because it creates a
  block from zero).

### 4.5 Story flags, folders and the mirror system

`BASE_FLAG[i] == 0x01` means "event happened / boss defeated / flag set".
`BASE_FLAGFOLDER[i] == 0x01` means "folder SET". Folders are not inverted.

The title screen restores **active ← mirror** on load. The retail load script
(`entry_0806/chunk_0006`) and save script (`entry_0810/chunk_0004`) agree on
these **43** mappings. An earlier Lua decoder omitted inclusive loop endpoints
and shifted several IDs; the Python story tables inherited those errors.

| active | Normal (0) | Hard (1) | Very Hard (2) |
| --- | --- | --- | --- |
| intro/chapter 1–6 | 700–705 | 7–12 | 13–18 |
| 19–20 | 706–707 | 741–742 | 771–772 |
| bosses 67–70 | 708–711 | 74–77 | 78–81 |
| 83 | 712 | 743 | 773 |
| story cleared 86 | 713 | 87 | 88 |
| 89–99 | 714–724 | 744–754 | 774–784 |
| 320–327 | 725–732 | 755–762 | 785–792 |
| 329–333 | 733–737 | 763–767 | 793–797 |
| 335 | 738 | 768 | 798 |
| 49 | 801 | 802 | 803 |
| 383 | 804 | 805 | 806 |
| 82 | 807 | 808 | 809 |
| 334 | 810 | 811 | 812 |

Folders 0–10 restore from `518+i` (Normal), `530+i` (Hard), or `542+i`
(Very Hard). Folder 11 is saved but cleared without restoration on load.
Quest flags 376–381 and final-scene flag 610 are also cleared on load.


Difficulty is **not stored in the save**. It is inferred by scoring each
difficulty's mirror set against the live bytes and taking the highest; a tie
goes to the hardest difficulty, so a save whose columns a copy-down edit filled
equally still names the difficulty it was edited for (an unmirrored save reads
as Normal). The result is shown as a badge and can be overridden in the UI.

Because a difficulty's mirror column *is* that difficulty's stored story, the
editor reads and writes the columns like this:

- **Reading.** The Story tab shows the **selected difficulty's** story: the live
  bytes with that difficulty's mirror column overlaid. Mirrored bits come from
  the column; everything else (lobby flags, quests, chapters) is shared and
  stays live. Selecting a different difficulty therefore changes the checkboxes.
- **Writing.** Each `StoryEdit` names the difficulty it was made on. It is
  written to the live bytes, to that difficulty's column, **and to every lower
  difficulty's column** (Normal ≤ Hard ≤ Very Hard). A save that is played on a
  harder difficulty still has to satisfy the lower difficulties' unlock state,
  so a higher difficulty must never leave a lower one behind. An edit never
  reaches a *higher* difficulty: Very Hard stays empty while the user works on
  Normal. Because the editor keeps a draft per difficulty, one save can carry
  edits made on several difficulties; they are applied in ascending difficulty
  order, so a harder edit wins when two disagree about the same bit.

---

## 5. Deliberate divergences from the Python implementation

The Python editor remains a byte-layout oracle for non-story fields. Story
semantics are verified against corrected retail scripts and actual game loads.
DW4orge differs in the following ways.

1. **Retail-derived story state.** The old Python mirror tables are incomplete
   and their presets omit prerequisite state. DW4orge uses all 43 mappings in
   §4.5 and complete progression checkpoints extracted from the retail debug
   scripts. Tests preserve Python parity for non-story bytes while independently
   checking game restoration of story flags and folders.

2. **New-save difficulty.** The Python New dialog never asks for a difficulty,
   and `dw4build._build_story` hardcodes the Normal mirror map, silently
   locking every synthesised save to Normal. DW4orge adds a difficulty selector
   to New Save and writes that difficulty's mirror set — and every lower one
   (item 7) — defaulting to Normal. Explicit story state from an existing save
   is unaffected.

3. **Raw offset editing is dropped.** The Python tool's advanced "Raw edits"
   tab accepted free-form `offset = value` pairs (u32 or `:u8`) against the
   block. It existed as a development aid while the block layout was still
   being mapped and is not carried over (see §1). Advanced **mode** remains: it
   still unlocks glitch/crash item IDs and values above the in-game caps.

4. **Normal-mode input limits.** Numeric inputs enforce the displayed Normal
   caps on entry and commit. Advanced mode permits values above gameplay caps;
   Rust still validates data-type bounds and rejects invalid writes.

5. **A mode change does not reload species stats.** The Python editor re-reads
   the stored EXP when Advanced is toggled (via `_load_species_stats`), which
   shows a lower value than Normal's lifted one. DW4orge keeps the draft across
   a toggle — mode affects validation and the cap hints only — because
   re-projecting would discard unsaved edits. Switching *species* still reloads
   that species' stored block, through `species_stats`. Unsaved level, EXP,
   technique or power-up edits require an explicit discard confirmation before
   a species switch; cancelling retains the current draft.

6. **Techniques are labelled with the `codes::TECHNIQUES` names** (`blunt`,
   `slash`, …), as the Python editor labels them. The names are not in any IPC
   payload, so the frontend mirrors the nine strings.

7. **Story edits copy down to the lower difficulties.** The Python `apply`
   mirrors an edit only into the difficulty selected in its dropdown (or the
   detected one for `auto`) and leaves the other two columns untouched, so a
   save edited on Very Hard still reads as unfinished on Normal and Hard. The
   Python Story tab also always displays the *live* flags, so changing its
   difficulty dropdown does not change the checkboxes. DW4orge instead mirrors
   each edit into the chosen difficulty **and every lower one**, and the Story
   tab shows the selected difficulty's stored column. The tab also keeps a
   separate in-flight draft per difficulty, so changing the selector never
   discards unsaved edits. See §4.5.

8. **Difficulty detection breaks ties toward the hardest difficulty.** The
   Python `_detect_difficulty` keeps the first maximum, so a tie reads as
   Normal. Because DW4orge copies story edits down (item 7), a save it edits can
   have all three columns equally live, which the Python tie-break could never
   name. DW4orge keeps the **last** maximum instead, so such a save reads as
   Very Hard; a wholly unmirrored save still reads as Normal because no column
   scores above zero.

### Story presets

The canonical presets follow the game's debug progression checkpoints
(`entry_0603/chunks_0071–0075`). They include tutorial and briefing prerequisites,
not just a chapter selector and boss bits. A new record also retains init flag
960. Presets contain active flags; mirror bytes are derived for the selected
and lower difficulties.

| Preset | Folders set | Chapter/completion state |
| --- | --- | --- |
| Fresh (tutorial) | — | tutorial 1 |
| After World 1 | 0–3 | post-W1/key briefing 3 |
| After World 2 | 0–5 | World 3 briefing 5 |
| After World 3 | 0–8 | World 4 briefing 6 |
| All worlds + keys | 0–10 | World 4, keys collected; final clear off |
| Story cleared (post-game) | 0–10 | final clear 86; chapter 1–6 off |

`flags::STORY_PRESETS` is the complete flag list. The Story panel replaces every
preset-governed milestone, including required bits without visible checkboxes,
so applying Fresh after a progressed preset removes its prerequisites. Unrelated
flags remain untouched.

The game determines which difficulties are available from its completion state.
The editor's difficulty selection chooses which story columns are written; it
does not bypass the game's difficulty-selection menu. See
[automated testing](testing.md) for the native editor and retail test runners.

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
}

pub enum Mode { Normal, Advanced }
pub enum Difficulty { Normal, Hard, VeryHard }

/// One of the 16 `MODEL_NAME` species. `from_model_stem` maps a `DIGIMONNAME`
/// value to its index, falling back to `Dorumon`.
pub enum Species { Agumon, Veemon, Girumon, Dorumon, WereGarurumon,
                   HerculesKabuterimon, WarGreymon, AngelRimon, Beelzemon,
                   Alphamon, BlackWarGreymon, ImperialdramonFm,
                   ImperialdramonPm, MetalGarurumon, DukeCrimson, Susanoomon }

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
- `Ps2Memcard` — a **native** PS2 memory-card filesystem implementation. No
  third-party crate.

Path selection reproduces the Python semantics: an existing `.ps2` is written in
place; a non-existent `.ps2` is created by copying the source card image and
swapping in the save data; anything else is written as a raw file.

#### Why native

`ps2-memcard` 0.2.2 was evaluated first and **rejected**. Against the real
`Mcd001.ps2` it returns one save with **zero files**:

```text
saves: 1
  dir "BASLUS-20836savedata" (0) files=0
```

A card's save is a directory, and the crate bounds a directory's entry count by
the `.` entry's `length` field. On this card the save directory's `.` entry
holds `0` while the parent directory entry correctly holds `5`. The working
Python `ps2mc` reference does the opposite — `find_sub_entries` bounds by
`parent_entry.length` — which is why it succeeds. The crate also has no
in-place write: its `CardBuilder` rebuilds a whole card and is documented as
not byte-preserving.

The crate's ECC implementation *was* verified correct and was used to establish
the algorithm below: across all 16,384 pages of `Mcd001.ps2`, computed ECC
matches **351 of the 352** in-use pages. The single mismatch is page 1, inside
the pre-allocated superblock region, which is never written.

#### Card geometry

| Unit | Size | Notes |
| --- | --- | --- |
| Page | 512 B | 528 B in a hardware dump; the extra 16 B are the spare area |
| Cluster | `pages_per_cluster` × page (2 × 512 = 1024 B) | what the allocation table counts |
| Spare area | `(page_size / 128) * 4` = 16 B | first 12 B are ECC (4 chunks × 3 B), last 4 B zero on write |

Both image forms are supported and are distinguished by length: 8,388,608 B
(data only) and 8,650,752 B (hardware dump, 528-byte pages).

#### Superblock (offsets from the start of page 0)

| Offset | Size | Field | Real `Mcd001.ps2` |
| --- | --- | --- | --- |
| `0x00` | 28 B | magic `"Sony PS2 Memory Card Format "` | ✓ |
| `0x1C` | 12 B | version string | `"1.2.0.0"` |
| `0x28` | u16 | `page_size` | 512 |
| `0x2A` | u16 | `pages_per_cluster` | 2 |
| `0x2C` | u16 | `pages_per_block` | 16 |
| `0x30` | u32 | `clusters_per_card` | 8192 |
| `0x34` | u32 | `alloc_offset` | 41 |
| `0x38` | u32 | `alloc_end` | 8135 |
| `0x3C` | u32 | `rootdir_cluster` | 0 |
| `0x40` | u32 | `backup_block1` | |
| `0x44` | u32 | `backup_block2` | |
| `0x50` | 32×u32 | `ifc_list` (indirect FAT clusters) | `[8, 0, …]` |
| `0x150` | u8 | `card_type` | |
| `0x151` | u8 | `card_flags` | |

Validation: `alloc_offset + alloc_end <= clusters_per_card`, and
`page_size != 0 && pages_per_cluster != 0`.

#### Directory entry (512 B)

```text
0x00  u16  mode          0x8000 EXISTS, 0x0020 DIRECTORY, 0x0010 FILE
0x02  u16  (ignored)
0x04  u32  length        file: bytes. directory: entry count.
0x08   8B  created
0x10  u32  cluster       first cluster of this entry's contents
0x14  u32  dir_entry     (ignored)
0x18   8B  modified
0x20  u32  attr          (ignored)
0x24  28B  (ignored)
0x40  32B  name          NUL-terminated
0x60 416B  (ignored)
```

**Directory walk:** the entry count comes from the **parent** entry's `length`,
not the directory's own `.` entry — this is the trap that broke `ps2-memcard`.
For the root directory, which has no parent, the `.` entry's `length` is used.
Entries without `EXISTS` are skipped, as are `.` and `..`.

#### Cluster addressing

Two different conventions are in play. Mixing them up reads from the wrong
place, and does so silently — the bytes still parse, they are just the wrong
bytes.

- **System structures — the FAT — use absolute cluster numbers.** `ifc_list`
  names indirect-FAT clusters directly, and the cluster numbers stored inside
  the indirect FAT name FAT clusters directly. Neither has `alloc_offset` added.
- **The data area — directories and files — uses relative cluster numbers.** A
  directory entry's `cluster` and a FAT chain value are both relative to the
  start of the data area.

```text
absolute cluster of a data-area cluster = relative + alloc_offset
byte offset of absolute cluster c       = c * pages_per_cluster * raw_page_size
```

The byte offset is **not** `(alloc_offset + c) * cluster_size`. A cluster's data
is `pages_per_cluster` page-sized runs, and in a hardware dump consecutive pages
are `raw_page_size` (528) bytes apart, not `page_size` (512). On `Mcd001.ps2`
that puts cluster 0 at byte `41 * 2 * 528 = 43296`, not `41 * 1024 = 41984`.

Within a cluster the same interleaving applies: entry *n* of a directory is not
at `n * 512` bytes into the cluster, because the second page starts a full 528
bytes after the first.

Verified against the real card: reading the save through these rules reproduces
the Python editor's extraction byte for byte.

#### Allocation table

Two levels of indirection: `ifc_list` names the indirect-FAT clusters; each of
those holds FAT cluster numbers; each FAT cluster holds the entries.

```text
fat_per_cluster = cluster_size / 4
fat_matrix      = [ entries of each FAT cluster named by the flattened ifc list ]

fat_entry(n)    = fat_matrix[(n / fat_per_cluster) % fat_per_cluster][n % fat_per_cluster]
next(n)         = if fat_entry(n) & 0x8000_0000 { fat_entry(n) & 0x7FFF_FFFF } else { END }
```

`0x7FFF_FFFF` means end-of-chain. A cluster not marked `0x8000_0000` is not part
of a chain. Chain walking must detect loops and refusing to run past
`alloc_end`.

#### ECC

Per 128-byte chunk of page data, three bytes: column parity and two line
parities.

```text
seeds: cp = 0x77, lp0 = 0x7F, lp1 = 0x7F
parity(b) = ((b ^ b>>1 ^ b>>2 ^ b>>4) & 1)
column masks = [0x55, 0x33, 0x0F, 0x00, 0xAA, 0xCC, 0xF0]
column_mask(b) = OR over i of parity(b & mask[i]) << i

for i, b in data[..128]:
    cp  ^= column_mask(b)
    if parity(b): lp0 ^= !i ; lp1 ^= i

ecc = [cp, lp0 & 0x7F, lp1 & 0x7F]
```

A page's 16-byte spare area is four 3-byte codes followed by four zero bytes.
An **erased** page's spare area is all `0xFF`. In-use pages on a real card have
zero in the trailing four bytes, so an in-place write of unchanged content
reproduces the original image exactly.

#### In-place write

`write_save` never rebuilds the card:

1. Walk the root directory to the save directory, then to the target file.
2. Follow the FAT chain from the file's starting cluster.
3. Write the new bytes cluster by cluster along that chain, recomputing each
   page's 12 ECC bytes and zeroing the 4 trailing spare bytes.
4. Leave every other byte of the image untouched.

A file whose size does not divide evenly into clusters writes only as many
bytes as the file declares. The chain must already be long enough; growing the
save is not supported and returns an error rather than silently corrupting the
card.

#### Verified behaviour on `Mcd001.ps2`

| Fact | Value |
| --- | --- |
| Image size / form | 8,650,752 B, 528-byte pages (with spare) |
| Save directory | `BASLUS-20836savedata`, cluster 2, 5 entries |
| Save file | `BASLUS-20836savedata`, cluster 38, 81,920 B |
| Other files | `icon.sys`, `icon1.ico` (34,156 B) |
| In-use pages / erased pages | 352 / 16,032 |
| ECC agreement | 351/352 in-use pages |

#### Creating a card from nothing

A `.ps2` that does not exist and is saved to with no source card is synthesised
as a standard 8 MB card: the committed page-0 template (byte-identical across
every card measured), the indirect FAT at absolute cluster 8, the FAT at
clusters 9–40, then the data area at relative clusters 0–119:

| Clusters | Contents |
| --- | --- |
| 0–1 | root directory: `.`, `..`, the save-directory entry |
| 2, 3, 118 | save directory: `.`, `..`, `icon1.ico`, the save, `icon.sys` |
| 4–37 | `icon1.ico` |
| 38–117 | the save, 80 clusters |
| 119 | `icon.sys` |

The allocation mirrors the reference card. Two entry fields are required and
were found only by running the result in an emulator: each entry's **created and
modified timestamps** must carry the reference card's values (an invented value
greys the card out or reports "not inserted"), and the save directory's `.`
entry stores its own first cluster in byte **`0x14`** (`0` reports "no save
data"). Neither is derivable from the format; both are tabulated in
`crates/dw4core/src/memcard/format.rs`.

Every written page carries ECC; page 1 is left erased, and the backup block is
not written. Emulator testing by subtraction showed neither is required. The
FAT's on-disk encoding is: free `0x7FFFFFFF`, allocated pointer
`0x80000000 | next`, chain end `0xFFFFFFFF`. A donor card passed as the source
is still copied verbatim, so its icons and any other files survive.
The layout and the measurements behind it are in
`docs/card-creation-design.md`.

### 6.2 Builder

`builder.rs` is a pure function of a `SaveSpec` producing one 0xA000 block
duplicated with both checksums, byte-identical to `dw4build.build_save` for the
same spec apart from the deliberate corrections in §5. Defaults: version 4,
`ISUSE` 1, `UNIQUE = 0x6096F82C` (a fixed placeholder — `UNIQUE` is really a
per-save id, but a synthesised save has no history to derive one from), all
species level 1 / EXP 0 / 9 techniques at 1 / power-ups 0, zero padding, disks
initialised to `0x4000 + i`.

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
- Left sidebar: Character, Items, Equipment, Disks, Story, Bank.
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
| Disks | the 12 named `DISKFOLDER` counts (HP/MP Disk α–γ, Cure, Raise, Gate, Recovery, B. Pack, Key Chain), 0–9 in Normal mode (0–65535 in Advanced) |
| Story | difficulty selector with detected badge, preset dropdown, grouped flag/folder checkboxes (intro, chapters, bosses, quests, lobby, folders) and a live mirror preview; one in-flight draft per difficulty |
| Bank | balance plus 96 slots in 8 pages of 12, matching the in-game bank, reusing the Items row widget |

New Save is a dialog (species, name, story preset, difficulty). Destructive and
unsaved-changes actions use confirm dialogs, including closing the window;
validation failures surface both inline on the offending control and in the
status bar.

**Implemented.** The app runs in a plain browser against a fixture-backed mock,
and in the Tauri shell against the real commands. The concrete
architecture — the backend seam, store, validation flow and the game knowledge
mirrored in `src/lib/` — is recorded in
[`docs/frontend-design.md`](frontend-design.md).

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

- **`ci.yml`** (push and PR): a Rust job (`cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`), a frontend job (`tsc --noEmit`, ESLint,
  `vitest run`, `vite build`, the palette contrast check), and a job that runs
  `cargo check` on the excluded `src-tauri` crate on ubuntu-22.04 with the
  webkit dependencies installed.
- **`release.yml`** (tag `v*`): `tauri-apps/tauri-action` matrix over
  `ubuntu-22.04`, `windows-latest`, `macos-latest`, producing
  `.deb` + `.AppImage`, `.msi`, and a universal `.dmg`, attached to a draft
  GitHub release. The bundles are **unsigned**, so Gatekeeper and SmartScreen
  will warn on first run. It runs the build through
  `src-tauri/scripts/tauri-build.sh`, which patches the AppImage for Wayland
  hosts (see the README's Linux section).

Bundle identity: product name `DW4orge`, identifier
`io.github.crashbashash.dw4orge`, version `0.2.0`. The icon set lives in
`src-tauri/icons/` and is generated from `src-tauri/icons/source.png` by
`tools/gen_icons.py`.

---

## 10. Licensing and attribution

DW4orge is GPL-3.0 (matching the existing `LICENSE`). `dw4core` takes no
third-party format or filesystem dependency; its external crates are `serde`,
`serde_json` and `thiserror` only.

Implementation references: the PS2 memory-card filesystem details in §6.1 were
derived from the `ps2mc` Python package (the working reference used by the
original editor) and verified against the real `Mcd001.ps2` dump. The
`ps2-memcard` Rust crate was evaluated and rejected (§6.1) but its ECC routine
was used as a cross-check.

A `NOTICE` file credits the reverse-engineering findings and their provenance,
and records that `crates/dw4core/data/*.json` is vendored verbatim from
the original Python editor's item database.

---

## 11. Risks

| Risk | Mitigation |
| --- | --- |
| Writing an edited save to a real memory card is destructive if we get the FAT/ECC wrong | Native implementation validated against the real card at every layer: ECC against all 16,384 pages, directory walk against the known 5-entry save directory, and a byte-identical read→write→read round-trip. Plus atomic write, `.bak`, and post-write verification |
| The native card writer only overwrites an existing chain, so a save that grows could not be written | The DW4 save is a fixed 81,920 bytes and never grows; the writer returns an error rather than corrupting the card if the chain is too short |
| `react-aria-components` API churn | Confined to four primitive components behind our own wrappers |
| Species 4–15 identification is best-effort (only the four starters are pinned) | Identical limitation to the Python editor; documented; defaults to Dorumon as before |
| Rarity seed→colour mapping is a threshold model with one known anomaly | Six verified colours only, exactly as the Python editor; arbitrary colours remain unavailable |

---

## 12. Build order

The sequence the project was built in. Each step produces working, tested
software on its own.

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
10. **Frontend sections** — Character, Items, Equipment, Disks, Story, Bank;
    New Save dialog; confirm dialogs.
11. **Polish** — keyboard shortcuts, inline validation, empty/error states,
    accessibility pass.
12. **Release** — icons, README, NOTICE, `release.yml`, first tagged build for
    Windows, macOS and Linux.

A `.ps2` is created when `save_as` targets one that does not exist: the save is
written into a freshly synthesised standard card
(`dw4core::memcard::format_card`). Passing a donor card
(`dw4cli new --card <template.ps2>`) instead copies that card and overwrites its
save, which preserves its icons and extra files.


## Regional PS2 support (0.2.0)

All three releases use the same 81,920-byte record, version 4, mirrored 0xA000
blocks, additive checksum and field offsets. The European and Japanese record
writer/restore routines match the USA routines after accounting for address
relocation; the checksum routine is byte-identical. Generated cards were accepted
by the regional retail games with matching BIOSes, overwritten through the Save
Keeper, and loaded into gameplay again after a cold boot. This validates that
path, not every item, advanced value, name glyph or campaign preset.

`Region` owns the identifiers: USA `BASLUS-20836savedata`, Europe
`BESLES-53405savedata`, Japan `BISLPS-25453savedata`. Card discovery checks all three
and rejects ambiguity until a caller selects one. `Document` retains the chosen
region through Save and Save As, so writes touch only its file chain. A destination
or donor must contain the matching save. Existing icons and unrelated files are
preserved. Raw saves have no inherent region; an export region is contextual and
is not inserted into the raw game data.

New cards use the selected identifier. Japan's `icon.sys` title is Shift-JIS
デジモンワールドＸセーブデータ with split value 18; USA and Europe retain the
existing fullwidth English title and split value 26. The animated icon and card
geometry use the existing templates accepted by all three games.

IPC adds `NewSaveRequest.region`, `OpenResult.region`, optional region arguments
for open/save-as, and `region_choice` errors carrying the available regions. The
UI asks for a region on mixed-card open and unknown-raw-to-card export. The CLI
accepts `--region usa|europe|japan` for explicit selection and new-card creation.
