# Design — the DW4orge frontend

The architecture of the React app in `src/`: how it talks to Rust, how a draft
is held and undone, how validation works, and which game knowledge is mirrored
on the TypeScript side. The format itself is `docs/save-format.md`; the IPC
surface it consumes is `docs/dw4ipc-cli-tauri-design.md`.

Implemented in plan 6 (2026-09-14) on `feat/dw4frontend`.

---

## 1. The backend seam

Everything the UI can ask of Rust goes through one interface,
`src/ipc/backend.ts`:

```ts
export interface Backend {
  appInfo(): Promise<AppInfo>;
  openSave(path: string): Promise<OpenResult>;
  newSave(request: NewSaveRequest): Promise<OpenResult>;
  getView(mode: Mode): Promise<SaveView>;
  validateEdits(edits: EditSet, mode: Mode): Promise<ValidationReport>;
  save(edits: EditSet, mode: Mode): Promise<OpenResult>;
  saveAs(path: string, edits: EditSet, mode: Mode): Promise<OpenResult>;
  speciesStats(species: Species, mode: Mode): Promise<SpeciesStats>;
  pickOpenPath(): Promise<string | null>;
  pickSavePath(defaultName: string): Promise<string | null>;
  openSample?(): Promise<OpenResult>; // browser mock only
}
```

Two implementations:

- **`src/ipc/tauri.ts`** — `invoke` from `@tauri-apps/api/core` plus the dialog
  plugin. Tauri passes command arguments as **camelCase** in JavaScript, so a
  Rust `save_as(path, edits, mode)` is `invoke('save_as', { path, edits, mode })`.
- **`src/ipc/mock.ts`** — an in-memory session over the JSON fixtures under
  `src/ipc/fixtures/`. It exists so the whole UI runs in a plain browser
  (`npm run dev` with no webkit build) and so component tests need no Tauri.

`src/main.tsx` picks the real adapter when `__TAURI_INTERNALS__` is present and
the mock otherwise. Tests inject the mock through `BackendProvider`.

**Validation is a value, not an exception.** `validate_edits` rejects with
`IpcError::Validation`, but being invalid is a normal state, so the adapter
catches that one case and returns `ValidationReport { errors, warnings }`.
Only a genuine failure rejects, and `describeIpcError` turns it into a status
line. No component ever branches on prose.

---

## 2. The draft store

Rust owns the file; TypeScript owns the draft (spec §3.3). One `useReducer`
store, `src/app/store.ts`, holds:

| Field | Meaning |
| --- | --- |
| `session` | the loaded `SaveView`, its path/source, and the **baseline** draft and story it started from |
| `draft` | the wire `EditSet` being edited |
| `story` | two boolean arrays (1024 flags, 12 folders) |
| `difficulty` | `auto` or a fixed `Difficulty` |
| `mode` | `normal` / `advanced` |
| `validation` | the last `ValidationReport` |
| `history` | undo/redo stacks |
| `theme`, `section`, `lastWrite`, `error` | UI state |

Two conversions are the whole contract with Rust:

- **`viewToEditSet(view)`** mirrors `SaveView::to_edit_set`. It copies the
  scalars and tuples, maps the mod sockets from device *indices* to chip *base
  ids* (`socketBaseId`), copies `junk_counter` into `EditSet::junk`, and sets
  `difficulty = { fixed: view.difficulty }`.
- **`buildEditSet(draft, story, baselineStory, difficulty)`** emits the wire
  payload, with `story` containing **only the bits that differ** from the
  baseline (`diffStory`). A bit the user never touched is never written.

`EditorProvider` (`src/app/EditorProvider.tsx`) owns the reducer and the async
orchestration — open/new/save/saveAs/speciesStats — and exposes it as
`useEditor()`. Components never call `invoke` directly.

### Undo/redo

`src/app/history.ts` is pure: a `Snapshot` is `{ draft, story, difficulty }`,
and an edit records the *previous* snapshot together with a **tag**. When
consecutive edits carry the same tag (typing in one number field), they
coalesce into one undo step; a discrete action passes `null` and always pushes.
Snapshots are small (an `EditSet` is a few hundred bytes), so full copies are
simpler and safer than patches.

### Dirty state

`dirty(state)` compares the current draft and story against the session
baseline. Both `EditSet`s are produced by the same function at load, so their
key order matches and a JSON comparison is a faithful deep-equal.

---

## 3. Validation flow

Two layers, one source of truth:

1. **Immediate, local** — range hints and the first error message on a control
   come from `UiData.caps`, `UiData.powerups` and the catalogue. `NumberField`
   edits locally and commits on blur/Enter, so a half-typed number never
   reaches the store.
2. **Authoritative, debounced** — 200 ms after the last change, the provider
   calls `validateEdits(toEditSet(state), mode)` and stores the report.
   `errorsByPath` indexes it by the validator's `path` (`bit`, `device[3].bonus`,
   `equip.armor`, `story.flag[66]`), so each control shows its own message.
   Save re-validates on the Rust side regardless.

A stale response cannot overwrite a newer one: the effect carries a sequence
number and ignores a reply that is not the latest.

Nothing is silently clamped. A value above the Normal cap is kept and flagged,
matching divergence 4 in `docs/save-format.md` §5.

---

## 4. Fixtures

`crates/dw4ipc/examples/gen_ui_fixtures.rs` renders `src/ipc/fixtures/app_info.json`
(the full 539-item `AppInfo`) and `open_result.raw.json` (a real `OpenResult`)
from Rust. `crates/dw4ipc/tests/ui_fixtures.rs` byte-compares them, so the mock
cannot drift from the payloads it pretends to be:

```bash
cargo run -p dw4ipc --example gen_ui_fixtures
```

The `src/bindings/*.ts` types are regenerated the same way with `gen_bindings`;
`src/bindings/index.ts` is a hand-written barrel over them (the generated files
are one type per file and have no index).

---

## 5. Mirrored game knowledge

`UiData` deliberately carries only tables; several things the UI must display or
compute are not in any payload. They are mirrored in `src/lib/` and each is
pinned by a Vitest test whose vectors come from the Rust tests.

| Module | Mirrors | Source |
| --- | --- | --- |
| `items.ts` | item id pack/split, category byte, rarity bands + `color_for_seed`/`rarity_name`, grade letters, `describe_item_id`, EMPTY | `item.rs`, `catalogue.rs`, `codes.rs` |
| `level.ts` | `level_threshold`, `level_from_exp` | `codes.rs` |
| `junk.ts` | `JUNK_TIERS`, `junk_tier_from_counter`, `junk_threshold` | `codes.rs` |
| `story.ts` | group sizes, folder-mirror bases, preset application, mirror preview | `flags.rs`, `document.rs` |
| `mods.ts` | `find_or_add_mod` / `resolve_mods` preview | `document.rs` |
| `techniques.ts` | the nine `TECHNIQUES` names | `codes.rs` |
| `caps.ts`, `format.ts`, `num.ts`, `editSet.ts` | field limits, hex/number formatting, an empty `EditSet` | `document.rs`, `ui.rs` |

`storyGroups` throws if the flat label list is not the known 33 entries, so a
label added in `flags.rs` without the frontend fails loudly instead of
mis-grouping.

---

## 6. Deliberate divergences

Recorded in `docs/save-format.md` §5: no silent clamping, a mode toggle does not
reload species stats, and the technique labels are the `codes::TECHNIQUES`
names. Everything else aims to match the Python editor, including the
`_load_species_stats` reload on a species switch (via `species_stats`) and the
preset dropdown **replacing** the governed flags and folders rather than OR-ing
them.

---

## 7. Testing

- **Vitest + Testing Library + jsdom.** Unit tests for `src/lib/*`, history,
  selectors, the reducer and the mock; one component test per section; a
  `react-aria-components` `Select`/`ComboBox`/`Modal` interaction in each.
- **`src/app/integration.test.tsx`** drives open → edit → save through the store
  against the mock and asserts the full `EditSet` key set against
  `src/bindings/EditSet.ts`, which is the shape contract between the two halves.
- **Gate:** `npm run typecheck && npm run lint && npm test && npm run build`.

## 8. Not verified

There is no browser and no `webkit2gtk` in the development container, so:

- **No visual check.** Layout, colour and focus behaviour are unverified here;
  they are exercised by running `npm run dev` on a machine with a browser.
- **The Tauri shell is not compiled here.** `src-tauri` is excluded from the
  workspace (`docs/dw4ipc-cli-tauri-design.md` §5); its command signatures,
  cap-ability file and dialog plugin are type-checked only by `cargo check` from
  `src-tauri/` on a host with the Linux webkit dependencies.
- **Whether the game accepts an edited save** remains a manual PCSX2 check.
