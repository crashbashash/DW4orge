# Design — `dw4ipc`, `dw4cli`, `src-tauri`, and `ts-rs` bindings

Design reference for plan 5 (CLI, IPC and bindings). Approved 2026-09-14 before
implementation. The format itself is `docs/save-format.md`; this document covers
the public API layered on top of it.

---

## 1. Context

`dw4core` is feature-complete (plans 1–4): 227 tests passing, fmt and clippy
clean on the verified baseline `30d5de8`. Plan 5 is the first plan that fixes a
**public API**: the IPC surface the React frontend (plan 6) consumes and the
CLI's output contract. That is why it was brainstormed and confirmed rather
than executed autonomously.

Spec references: `docs/save-format.md` §3.1 (layout), §3.2 (boundaries),
§3.3 (data flow), §3.4 (IPC surface), §3.5 (write safety), §12 steps 7–8.

### Decisions confirmed with the user

| # | Decision | Choice |
| --- | --- | --- |
| 1 | Tauri shell cannot compile here (no `webkit2gtk-4.1`) | Put commands + payloads in a pure `dw4ipc` crate; `src-tauri` is a thin excluded shell proven by `cargo check` on a host |
| 2 | IPC surface | Confirm the spec's seven commands; add `OpenResult` (file identity) and `IpcError` wrappers |
| 3 | CLI output | Five verbs, human-readable by default, global `--json` sharing the IPC payloads; exit 0/1/2 |
| 4 | `ts-rs` derives | Feature-gated in `dw4core` (`ts`), generation test owned by `dw4ipc` |
| 5 | Static UI data the frontend needs (catalogue, caps, mirrors, labels, presets) | `app_info()` returns it once in `AppInfo.ui`; `OpenResult` stays lean |

### Facts verified from source, not memory

- `SaveView` (`document.rs`) has **no** path/source field, so `open_save` needs
  a wrapper for file identity.
- `Warning` is a **type alias** of `FieldError` (`document.rs:82`).
- `Document::save` already does atomic write + `.bak` + post-write verification
  for every container type; plan 5 must not reimplement any of it.
- `render_container` (`memcard/mod.rs`) raises `NoSave` when a new `.ps2` is
  requested with no source card. A brand-new card image cannot be synthesised.
- `Document::validate` / `apply` return `Result<Vec<Warning>, Vec<FieldError>>`.
- Versions on crates.io at design time: `ts-rs 12.0.1`, `tauri 2.11.5`,
  `tauri-build 2.6.3`, `clap 4.6.6`.
- `docs/superpowers/` is gitignored scratch; crates.io is reachable from the
  container (the handoff's "no reliable network" note is stale).

---

## 2. Architecture

### 2.1 Crate topology

```text
crates/
  dw4core/     existing — gains only an optional `ts` feature + cfg-gated derives
  dw4ipc/      NEW: payload types, IpcError, EditorSession, service functions
  dw4cli/      NEW: the clap binary over dw4ipc
src-tauri/     NEW: thin #[tauri::command] wrappers; EXCLUDED from root workspace
src/bindings/  NEW: generated TypeScript, checked in
```

`src-tauri` goes in the root workspace's `exclude`, not `members`. Rationale:
`cargo test/clippy --workspace` in this container would otherwise fail on the
missing `webkit2gtk-4.1`, making the gate that protects plans 1–4 unusable.
Excluded, it keeps its own lockfile and is checked by a host/CI job that
installs `libwebkit2gtk-4.1-dev`.

`dw4ipc` is pure Rust. It has no Tauri dependency, so it compiles and is tested
here. The Tauri shell and the CLI are both thin consumers of it, which is what
makes the CLI and frontend share one serializer.

### 2.2 Dependency boundaries

| Crate | Depends on | New dependencies |
| --- | --- | --- |
| `dw4core` | serde, serde_json, thiserror | `ts-rs 12` **optional**, behind `ts` |
| `dw4ipc` | `dw4core` (features = `["ts"]`), serde, serde_json, thiserror | `ts-rs 12` |
| `dw4cli` | `dw4ipc`, `dw4core` | `clap 4` (derive) |
| `src-tauri` | `dw4ipc`, tauri 2 | `tauri 2.11`, `tauri-build 2.6`, dialog plugin |

`dw4core`'s default dependency set is unchanged: `ts-rs` is optional and only
enabled by `dw4ipc`.

---

## 3. `dw4ipc`

### 3.1 Payloads

Re-exported from `dw4core`: `SaveView`, `EditSet`, `Mode`, `FieldError`
(`Warning` is its alias), `Severity`, `StoryEdit`, `StoryKind`,
`DifficultyChoice`, `Species`, `Difficulty`.

Defined in `dw4ipc`:

```rust
pub enum SourceKind { Raw, Memcard }

pub struct NewSaveRequest {
    pub species: Species,
    pub name: String,
    pub story: Option<String>,   // a STORY_PRESETS name; None = storyless
    pub difficulty: DifficultyChoice,
}

pub struct OpenResult {
    pub path: Option<String>,   // None for a newly synthesised save
    pub source: SourceKind,
    pub view: SaveView,
}

pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub core_version: String,
    pub save_size: usize,
    pub block_size: usize,
    pub schema_version: u32,    // bumped when a payload changes shape
    pub ui: UiData,             // static; fetched once at startup
}

/// Static tables the UI needs for pickers and inline validation, so no
/// keystroke costs an IPC round-trip (spec §3.3).
pub struct UiData {
    pub catalogue: Vec<Item>,          // 539 entries
    pub caps: Vec<NamedCap>,           // every CAP_* constant, keyed by field
    pub powerups: Vec<PowerupLimit>,   // 11 slots: name + Normal cap
    pub mirrors: Vec<Mirror>,          // 38 active -> per-difficulty rows
    pub folder_labels: Vec<String>,    // 12
    pub flag_labels: Vec<FlagLabel>,
    pub story_presets: Vec<StoryPreset>,
}

pub struct NamedCap { pub field: String, pub cap: Cap }
pub struct PowerupLimit { pub slot: u8, pub stat: String, pub normal_max: i64 }
```

`Item`, `Category`, `Cap` and `Mirror` are owned or `Copy`, so they gain
`Serialize` + `Deserialize` unconditionally (`serde` is already a `dw4core`
dependency; the derives are additive) and `TS` behind the `ts` feature.
`FlagLabel` and `StoryPreset` hold `&'static str` / `&'static [u32]` and so can
be `Serialize` + `TS` only, never `Deserialize`; that in turn makes `AppInfo`
and `UiData` Serialize-only, since they carry those two types.

**Verified against ts-rs 12.0.1 by probe, not assumed:** borrowed `'static`
fields derive cleanly (`&'static str` → `string`, `&'static [u32]` →
`Array<number>`), and `Category::Unknown(u8)` → `{ "unknown": number }`. No
owned view types are needed.

This is an agreed deviation from §3.3's literal wording ("`open_save` returns
… the item catalogue, the caps/limits table"): those tables are static and ship
once through `app_info` instead of on every open, new, save and save-as. The
§3.3 intent — inline validation with no per-keystroke round-trip — is preserved.

### 3.2 Error model

```rust
#[serde(tag = "kind")]
pub enum IpcError {
    Validation { fields: Vec<FieldError> },  // from validate/apply Err
    Core { variant: String, message: String }, // from dw4core::Error
    NoOpenDocument,
    Unsupported { message: String },
}
```

Internally tagged with named-field variants only (serde cannot internally-tag a
tuple/`Vec` variant). `Core` carries the `dw4core::Error` variant name and its
`Display` text, so no error internals leak into TypeScript and the frontend can
branch on `kind` without parsing prose.

### 3.3 `EditorSession`

```rust
pub struct EditorSession { doc: Option<Document> }
```

The seven operations, as methods:

| Method | Behaviour |
| --- | --- |
| `open(path) -> Result<OpenResult, IpcError>` | `Document::load` (dispatches card vs raw), project `SaveView` |
| `new(req: NewSaveRequest) -> Result<OpenResult, IpcError>` | `builder`; `path` is `None`, `source` is `Raw` |
| `view(mode) -> Result<SaveView, IpcError>` | re-project; used after a mode switch |
| `validate(edits, mode) -> Result<Vec<Warning>, IpcError>` | authoritative; writes nothing |
| `save(edits: EditSet, mode: Mode) -> Result<OpenResult, IpcError>` | re-validates and applies the draft, then writes over the loaded path; writes nothing on rejection (spec §3.3). A pathless (`new`) session returns `Unsupported { "no path; use save_as" }` |
| `save_as(path, edits: EditSet, mode: Mode) -> Result<OpenResult, IpcError>` | the same draft/validate/apply/write to a new path; card-from-raw needs the source card |
| `app_info() -> AppInfo` | static; carries `ui: UiData`; never fails |

Non-IPC service calls, used only by the CLI: `verify(path) -> VerifyReport`
(parse + checksums; card also ECC/chain/directory) and
`catalogue_search(query, category, limit) -> Vec<Item>`.

Why a session: `Document` remembers the source card (needed for `.ps2` save-as)
and the loaded bytes. The frontend must not be able to fabricate that state
from a path, so it lives behind `State<Mutex<EditorSession>>` in the Tauri
shell.

Paths crossing IPC are `String`. A non-UTF-8 path is a `Core`/`Unsupported`
error, not a lossy silent conversion.

---

## 4. `dw4cli`

`clap` derive. Global `--json`. Exit codes: `0` success, `1` validation or core
error, `2` usage (clap default).

| Verb | Output |
| --- | --- |
| `info <path>` | container kind, checksum status, species, name, level, bit, xdata, difficulty, junk tier, occupied slot counts |
| `dump <path>` | the full `SaveView` |
| `new [--species] [--name] [--story] [--difficulty] [--card <t.ps2>] -o <out>` | synthesise via `builder`; default Dorumon/"TST"; raw output unless `--card` |
| `verify <path>` | checksums; on a card also ECC, chain, directory; every failure listed |
| `items [query] [--category] [--limit]` | catalogue listing/search |

`--json` emits exactly the `dw4ipc` payloads, so the CLI and frontend cannot
diverge in shape.

**Card creation rule:** `-o *.ps2` without `--card` exits 1 with the
`render_container` message. This mirrors the Python editor, which always had a
source card open. A brand-new card filesystem is out of scope for the project.
`--card` with an `-o` that is not a `.ps2` is a usage error (exit 2): the
combination has no meaning.

CLI integration tests spawn the built binary via
`env!("CARGO_BIN_EXE_dw4cli")` and `std::process::Command` — no `assert_cmd`
dependency.

---

## 5. `src-tauri`

Thin: `src/lib.rs` builds the app, `src/commands.rs` holds the seven
`#[tauri::command]` functions, each taking `State<Mutex<EditorSession>>` and
delegating straight to `dw4ipc`; `src/state.rs` owns the mutex; `src/error.rs`
is unnecessary because `IpcError` is already `Serialize`. Capabilities grant
only the file-dialog plugin.

Verification reality, recorded rather than papered over: this crate **cannot be
compiled in the design container**, and it is excluded from the root workspace
so it cannot break the workspace gate. It is proven by `cargo check` run from
`src-tauri/` (its own workspace; `cargo check -p dw4orge` does not resolve from
the root) on a machine with the Linux build dependencies:

```text
libwebkit2gtk-4.1-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev librsvg2-dev
```

plus `tauri-cli`. That check is the only place the Tauri command signatures are
type-checked; the command bodies are one-line delegations whose logic is tested
through `dw4ipc`. `bundle.active` is `false` and no icon paths are set until
plan 7 supplies them, because `generate_context!` would otherwise fail on a
missing icon.

---

## 6. `ts-rs` bindings and the drift test

- `dw4core/Cargo.toml`:

  ```toml
  [features]
  ts = ["dep:ts-rs"]
  [dependencies]
  ts-rs = { version = "12", optional = true }
  ```

- Every payload type carries
  `#[cfg_attr(feature = "ts", derive(ts_rs::TS))]`. No `#[ts(export)]`: that
  generates tests that write into the source tree. Generation is explicit.
- `dw4ipc` owns `examples/gen_bindings.rs`, which renders each payload into
  `src/bindings/<Type>.ts`.
- `dw4ipc/tests/bindings.rs` renders each payload and byte-compares it against
  the checked-in file, failing with the file name and the first differing line
  (the same spirit as `first_difference` in `tests/memcard.rs`).
- The ts-rs 12 API, confirmed from the crate source: `TS::export_to_string(&Config)`,
  `TS::export(&Config)`, `Config::default().with_large_int("number")
  .with_out_dir(dir)`. Generation and the drift check both call
  `export_to_string`, so the two cannot render differently.
- `large_int = "number"` is required: `Cap` holds `i64`, serde_json sends
  numbers, and the default `bigint` would not match what arrives.
- `src/bindings/` is checked in; `schema_version` in `AppInfo` lets the runtime
  detect drift too. The drift test was proven to fail on a one-line edit to a
  checked-in file, naming the file and the first differing byte.

---

## 7. Verification strategy

| Layer | How |
| --- | --- |
| `dw4core` | existing 227 tests must stay green; the `ts` feature must not change the default build |
| `dw4ipc` | integration tests over committed fixtures: open→view→edit→validate→save to tempdir→reopen; rejected validation writes nothing; card round-trip; `save_as *.ps2` with no source card errors |
| bindings | drift test above |
| `dw4cli` | spawn the binary for all five verbs, human and `--json` |
| `src-tauri` | `cargo check` on a webkit host/CI only — **not verifiable here**, and not claimed |

Gate, run as its own step (never chained with `;`, which once let a commit run
on failing fmt):

```bash
cargo fmt --all && cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`src-tauri` is excluded, so the gate covers `dw4core`, `dw4ipc`, `dw4cli`.

---

## 8. Deliverable sequence (plan detail deferred to writing-plans)

1. Read the ts-rs 12 render API from docs.rs; pin the signatures. Decide the
   `FlagLabel`/`StoryPreset` borrowed-field question.
2. `dw4core`: `ts` feature; `cfg_attr` derives on the IPC payload types; serde
   (+ `ts`) derives on `Item`, `Category`, `Cap`, `Mirror`, `FlagLabel`,
   `StoryPreset`. Default build unchanged.
3. `dw4ipc`: payloads (`OpenResult`, `NewSaveRequest`, `AppInfo`, `UiData`,
   `NamedCap`, `PowerupLimit`) + `IpcError` + `EditorSession::open/new/view`.
4. `dw4ipc`: `app_info`/`UiData` assembly; `validate`/`save`/`save_as` +
   `verify` + `catalogue_search`.
5. `dw4ipc`: `gen_bindings` example + `src/bindings/*.ts` + drift test.
6. `dw4cli`: skeleton + `info` + `items`.
7. `dw4cli`: `dump` + `new` + `verify`.
8. `src-tauri`: excluded crate, commands, state, capabilities, config; document
   the webkit dependency.
9. `docs/save-format.md`: final §3.4 surface, §12 steps 7–8, the
   source-card rule for a new `.ps2`, and where the static UI tables are served.

---

## 9. Out of scope

- The React frontend (plan 6) and polish/release (plan 7).
- Creating a memory-card image from nothing.
- Exposing RE data the Python editor never surfaced, per spec §1.
- Reimplementing any part of `Document::save`'s atomic-write/`.bak`/verify
  policy; CLI and Tauri call into it.

## 10. Risks

| Risk | Mitigation |
| --- | --- |
| ts-rs 12 API differs from expectation | Task 1 reads docs.rs first; no binding code until the signature is pinned |
| Feature unification silently enables `ts` for all workspace builds | Acceptable: `ts-rs` is a derive-only dep; the default `dw4core` build is checked explicitly |
| `src-tauri` breaks on the host because it is never compiled here | Keep it to one-line delegations; CI job with webkit is the compile gate; logic tested via `dw4ipc` |
| `clap`/`tauri` derive macros trip new clippy lints | Gate runs `-D warnings`; fix inline as plans 1–4 did |
| `Category::Unknown(u8)` or borrowed fields do not derive cleanly for ts-rs | Resolved by probe: both derive. Generation pins `large_int = "number"` because `Cap` holds `i64` |
| Adding serde derives to `dw4core` types changes the public API beyond `ts` | Derives are additive, need no new dependency, and the default `dw4core` build and its 227 tests are re-run unchanged |
