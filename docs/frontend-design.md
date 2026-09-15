# Design — the DW4orge frontend

The architecture of the React app in `src/`: how it talks to Rust, how a draft
is held and undone, how validation works, and which game knowledge is mirrored
on the TypeScript side. The format itself is `docs/save-format.md`; the IPC
surface it consumes is `docs/dw4ipc-cli-tauri-design.md`.

---

## 1. The backend seam

Everything the UI can ask of the shell goes through one interface,
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
  onCloseRequested(handler: () => boolean): Promise<() => void>;
  closeWindow(): Promise<void>;
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

**Closing with unsaved work.** `CloseGuard` (`src/app/CloseGuard.tsx`) subscribes
through `onCloseRequested`; the handler returns `true` to keep the window open
when the draft is dirty. The native close is then only performed by
`closeWindow` (`destroy`, which skips the event) after the same discard prompt
Open/New use. The browser mock's guard is inert, so `npm run dev` is unaffected.
Both paths need `core:window:allow-destroy` in
`src-tauri/capabilities/default.json`: Tauri's JS wrapper calls `destroy()`
itself when the handler does not prevent the event, and `destroy` is **not** in
`core:window:default` — without it the close button silently does nothing.

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
| `session` | the loaded `SaveView`, its path/source, the **baseline** draft, and each difficulty's **stored story** (`baselines`) |
| `draft` | the wire `EditSet` being edited |
| `stories` | one `StoryDraft` (1024 flags, 12 folders) per difficulty, all in flight at once |
| `difficulty` | `auto` or a fixed `Difficulty`; selects which of `stories` the Story tab shows. Changing it is a view change, never an edit |
| `mode` | `normal` / `advanced` |
| `validation` | the last `ValidationReport` |
| `history` | undo/redo stacks |
| `theme`, `section`, `lastWrite`, `error` | UI state |

Two conversions are the whole contract with Rust:

- **`viewToEditSet(view)`** mirrors `SaveView::to_edit_set`. It copies the
  scalars and tuples, maps the mod sockets from device *indices* to chip *base
  ids* (`socketBaseId`), and copies `junk_counter` into `EditSet::junk`.
- **`buildEditSet(draft, stories, baselines)`** emits the wire payload, with
  `story` holding **only the bits that differ** from each difficulty's
  baseline (`diffStory`), tagged with the difficulty they were edited on. A
  bit the user never touched is never written.

A save keeps one live flag/folder block plus three per-difficulty mirror
columns, and the title screen restores `active ← mirror` on load, so a
difficulty's mirror column *is* its stored story. `session.baselines` therefore
holds one `StoryDraft` per difficulty, produced by
`storyDraftsFromView(view, mirrors)` (the live bytes with that difficulty's
column overlaid). The Story tab edits one difficulty at a time, and each
difficulty keeps its own in-flight draft, so changing the selector only changes
which draft is on screen: a half-finished preset or a single checkbox survives
a look at another difficulty, and nothing has to be saved in between. Undo/redo
therefore does not record a difficulty switch. Each wire `StoryEdit` names its
own difficulty; Rust mirrors it into that difficulty and every lower one, so
editing Very Hard also completes Normal and Hard, and a save can carry drafts
from several difficulties at once.

`EditorProvider` (`src/app/EditorProvider.tsx`) owns the reducer and the async
orchestration — open/new/save/saveAs/speciesStats — and exposes it as
`useEditor()`. Components never call `invoke` directly.

### Undo/redo

`src/app/history.ts` is pure: a `Snapshot` is `{ draft, stories, difficulty }`,
and an edit records the *previous* snapshot together with a **tag**. When
consecutive edits carry the same tag (typing in one number field), they
coalesce into one undo step; a discrete action passes `null` and always pushes.
Snapshots are small (an `EditSet` is a few hundred bytes), so full copies are
simpler and safer than patches.

### Dirty state

`dirty(state)` compares the current draft, and every difficulty's story draft,
against the session baseline. Both `EditSet`s are produced by the same function
at load, so their key order matches and a JSON comparison is a faithful
deep-equal. A session with no path — the in-memory save `new_save` returns — is
dirty by definition before that comparison: it has never been written.

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
| `disks.ts` | the twelve `DISKFOLDER` names | `save_editor_gui.DISK_LABELS`, `FLAG_MAP.md` §11 |
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
- **`src/a11y.test.tsx`** walks all six sections and asserts every control has an
  accessible name, that the page has one `h1` plus the `main` and `nav`
  landmarks, and that the New Save dialog is named by its heading and takes
  focus.
- **`src/app/errorStates.test.tsx`** covers the failure banner and the
  busy-disabled toolbar.
- **`src/app/CloseGuard.test.tsx`** covers the close guard: a clean request does
  not prompt, a dirty one warns and Cancel keeps the window open, and Discard
  closes it.
- **`tools/check_contrast.mjs`** (`npm run check:contrast`) reads the real tokens
  and asserts the WCAG ratios for both themes: 4.5:1 for text, 3:1 for non-text.
  It is a plain Node script rather than a Vitest test because it reads a file and
  renders nothing.
- **Gate:** `npm run typecheck && npm run lint && npm test && npm run build &&
  npm run check:contrast`.

## 8. Not verified

The following paths are outside the automated gate:

- **No visual check.** Layout, colour and focus behaviour are not covered by the
  test suite; they need `npm run dev` in a browser.
- **The Tauri shell is compiled, packaged and launched — on Linux only, and only
  under a virtual display.** `src-tauri` is excluded from the root workspace and
  has its own gate: `cargo check --all-targets`, `cargo clippy -D warnings` and
  `cargo fmt --check` are clean there, and `ci.yml` repeats the check on every
  push. `npx tauri build --bundles deb` produces `DW4orge_<version>_amd64.deb`, whose
  control metadata, `.desktop` entry and 32/128/256 hicolor icons were inspected.
  The app has also been driven end to end under Xvfb: it renders, opens a card
  through the native file dialog, and saves an edit that `dw4cli verify` reports
  as checksum-ok with no ECC mismatches. **Not covered:** Windows and macOS
  bundles (never built), and any real desktop — so window-manager behaviour, the
  taskbar/window icon and the native dialog on those platforms are unexercised.
- **The native window-close prompt.** `CloseGuard` is exercised in jsdom
  against a mock that fires the handler by hand; the real
  `getCurrentWindow().onCloseRequested` / `destroy()` path needs the Tauri
  shell, so it is compiled (`cargo check`) but not driven by a real window.
- **The native title bar following the app theme** (`theme.ts` calling Tauri's
  `set_theme`, which tao maps to `gtk-application-prefer-dark-theme`) is a
  real-Wayland path. jsdom and Xvfb cannot show the GTK client-side header bar,
  so this is only observable on a compositor that draws it (KDE Plasma on
  Wayland).
- **The workflows have never run.** They are actionlint-clean and every action
  is pinned to a commit, but `ci.yml` and `release.yml` are unexercised and the
  first `v*` tag is cut by hand.
- **Whether the game accepts an edited save** remains a manual PCSX2 check.
