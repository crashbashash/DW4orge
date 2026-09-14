# Design — creating a memory-card image from scratch

Design reference for a plan that lets DW4orge synthesise a standard 8 MB PS2
memory card containing a save and the game's own icon, instead of requiring a
donor card. `docs/save-format.md` remains the format reference; this document
covers the creation algorithm layered on it.

Approved 2026-09-14. Branch `feat/dw4core-card-creation`, cut from
`feat/dw4cli-tauri` so the CLI test it must change travels with it.

---

## 1. Why

Today `render_container` copies a donor card and overwrites the save's existing
cluster chain; a target `.ps2` with no source card is refused with
`Error::NoSave`. The Python editor behaved the same way. That leaves one
documented limitation: you cannot produce a `.ps2` without an existing DW4
card.

Creating a card is implementable because the format is now fully measured
against ten real images in `Decomp/DW4/`. The goal is that
`dw4cli new -o x.ps2` and the GUI's save-as-to-`.ps2` simply work.

### Decisions confirmed with the user

| # | Decision | Choice |
| --- | --- | --- |
| 1 | Where the work lives | A separate plan, landed **before** plan 6, so the frontend is built against a complete New Save flow. Plan 7 stays packaging |
| 2 | What a fresh card contains | The save **and the game's own icon**, extracted once from a real card and committed as an asset |
| 3 | How creation is triggered | **Automatic**: `save_as` to a non-existent `.ps2` with no source card formats one. `--card` stays as an override |
| 4 | Definition of done | Structural verification plus a cross-check with the vendored `ps2mc` reader. In-game acceptance is explicitly **unverified** |

---

## 2. Facts measured from the reference images

Measured across all ten `.ps2` files under `Decomp/DW4/` (two real `Mcd001`
dumps, four probe dumps, four editor outputs):

| Fact | Value |
| --- | --- |
| Size | 8,650,752 B = 16,384 pages × 528 |
| Geometry (all cards identical) | version `1.2.0.0`, page 512, 2 pages/cluster, 16 pages/block, 8192 clusters, `alloc_offset` 41, `alloc_end` 8135, `rootdir_cluster` 0, `ifc_list [8]`, `card_type` 2, `card_flags` 0x2b |
| **Page 0** | **1 variant across all ten cards**, including its ECC. A safe template |
| Page 1 | **2 variants** (`1e453f1d05f2c4e0` on six, all-`0xFF` on four). Not a constant |
| `icon.sys` | 964 B, **1 variant** |
| `icon1.ico` | 34,156 B, **1 variant** |
| Indirect FAT (absolute cluster 8) | `9,10,…,40` then `0xFFFFFFFF` |
| FAT clusters | absolute 9–40; 32 × 256 = 8192 entries |
| Free FAT entry (raw) | `0x7FFF_FFFF` (bit 31 clear) |
| Allocated FAT pointer (raw) | `0x8000_0000 \| next_relative` |
| Allocated chain end (raw) | `0xFFFF_FFFF` |
| Directory entry mode | dir `0x8427`, file `0x8497`, root `..` `0xa426`, save dir `..` `0x8427` |
| Erased slot / page | all `0xFF` |
| Save directory `.` length | **0**, while its parent entry holds 5 — the quirk that breaks `ps2-memcard` |

There is **no local create/format reference**: `ps2mc` 0.1.1 exposes
`read_page`/`write_page`/`read_cluster`/`write_cluster` but no `format`,
`create_dir`, `create_file` or allocation. The algorithm below is derived from
the measurements, not transcribed from a reference.

---

## 3. Assets

Extracted once from `Decomp/DW4/memcards/Mcd001.ps2` and committed under
`crates/dw4core/data/card/`:

| File | Bytes | Use |
| --- | --- | --- |
| `superblock.bin` | 512 | page 0 data (the 340-byte superblock plus its 172-byte tail), page-0 ECC recomputed at format time |
| `icon.sys` | 964 | written into the save directory |
| `icon1.ico` | 34,156 | written into the save directory |

`crates/dw4core/data/PROVENANCE.md` gains a row for each, as the vendored item
JSONs already have. The NOTICE file already covers vendored game data.

Assets are embedded with `include_bytes!`, like the item JSONs, so the crate
still has no runtime data dependency. A test pins `superblock.bin` against the
committed sparse card fixture's page 0, and pins the icon hashes to the values
recorded here (`icon.sys` sha256 `51d741d295…`, `icon1.ico` sha256 `7b2b6eee27…`).

---

## 4. The algorithm

`dw4core::memcard::format_card() -> Vec<u8>` returns a complete 8,650,752-byte
image. It writes only these regions; everything else stays erased (`0xFF`).

### 4.1 Reserved and allocation-table region

- **Page 0**: the 512-byte `superblock.bin` template, then its 16-byte spare
  computed by `page_spare` with the trailing four bytes zero.
- **Page 1**: left erased. Some real cards carry eight non-`0xFF` bytes here; we
  deliberately do not copy them, because they are not constant and may be
  card identity. Our reader ignores page 1 (it is the documented ECC exception)
  and four produced cards in the reference set have it erased.
- **Absolute cluster 8** (indirect FAT): `9..=40` as u32, then `0xFFFFFFFF`
  for the unused entries.
- **Absolute clusters 9–40** (FAT): 8192 u32 entries. Allocated entries use the
  raw encodings in §2; every other entry is `0x7FFF_FFFF`.

### 4.2 Data area (relative clusters; absolute = relative + 41)

| Relative clusters | Contents | Length in the parent entry |
| --- | --- | --- |
| 0 → 1 | root directory: `.`, `..`, the save-directory entry | 3 |
| 2 → 3 → 4 | save directory: `.`, `..`, `icon1.ico`, the save file, `icon.sys` | 5 |
| 5 … 38 | `icon1.ico`, 34 clusters | 34,156 bytes |
| 39 … 118 | the save file, 80 clusters | 81,920 bytes |
| 119 | `icon.sys`, 1 cluster | 964 bytes |

Directory entries are built with a small helper (`entry_slot(mode, length,
cluster, name)`), not parsed: `Entry` has no constructor today. Fields the
crate does not interpret are written zero, matching the reference except for
timestamps, which are cosmetic. The exact entries are:

| Where | name | mode | length | cluster |
| --- | --- | --- | --- | --- |
| root | `.` | `0x8427` | 3 | 0 |
| root | `..` | `0xa426` | 0 | 0 |
| root | `BASLUS-20836savedata` | `0x8427` | 5 | 2 |
| save dir | `.` | `0x8427` | 0 | 0 |
| save dir | `..` | `0x8427` | 0 | 0 |
| save dir | `icon1.ico` | `0x8497` | 34,156 | 5 |
| save dir | `BASLUS-20836savedata` | `0x8497` | 81,920 | 39 |
| save dir | `icon.sys` | `0x8497` | 964 | 119 |

The save directory's `.` length of **0** mirrors the reference card; our reader
uses the parent entry's length, so it does not depend on it, and a test asserts
that explicitly. The `.` and `..` cluster fields are 0 on the reference and are
written 0 here. Unused entry slots in a directory cluster are `0xFF`, not zero.

Every written page gets ECC via `page_spare` plus four zero spare bytes. The
save data itself comes from `builder::build_save`, so its checksums are already
fixed.

### 4.3 Allocated FAT entries

- root: `0 → 1`, `1` chain end
- save directory: `2 → 3`, `3 → 4`, `4` chain end
- `icon1.ico`: `5 → 6 → … → 38`, `38` chain end
- save: `39 → … → 118`, `118` chain end
- `icon.sys`: `119` chain end

---

## 5. Integration

`memcard::render_container` gains one branch: when `path` does not exist, ends
in `.ps2`, and `source` is `None`, it returns `format_card()` with the save
written in, instead of `Error::NoSave`. When `source` is `Some`, the existing
donor-copy path is unchanged.

Consequences:

- `Document::save` keeps its atomic write, `.bak` and post-write verification
  for every container type. Only the bytes handed to it change.
- `dw4cli new -o x.ps2` works with no `--card`. `--card` remains an override
  that copies a donor card (preserving its icons and any other files).
- **The IPC surface does not change.** `new_save` then `save_as("new.ps2")`
  simply succeeds where it previously returned `Core NoSave`. No bindings are
  regenerated, no `schema_version` bump.
- The CLI rule that `--card` requires a `.ps2` output stays.

Tests that asserted the old refusal must flip to asserting success:

- `crates/dw4ipc/tests/session.rs::save_as_creates_a_new_ps2_only_from_a_source_card`
- `crates/dw4cli/tests/cli.rs::new_to_ps2_without_a_card_exits_one`

Both are renamed to describe the new behaviour rather than deleted.

---

## 6. Verification

| Claim | How |
| --- | --- |
| The save round-trips | `format_card` → `Ps2Memcard::from_image` → `read_save` equals the input save, byte for byte |
| The card verifies | `verify_path` on a temp file: `checksum_ok`, no problems, ECC matching on every in-use page |
| Page 0 is the standard one | the assembled page 0 equals the sparse fixture's page 0 |
| Assets are the reference bytes | `include_bytes!` lengths and sha256 pinned |
| The FAT is correct | allocated chains walk back exactly; every other entry is `0x7FFF_FFFF`; `from_card` re-reads the same table |
| Directory structure is right | the reader walks root → save directory → the save file, and finds the icons |
| Spare areas are well-formed | trailing four spare bytes zero on every in-use page, as `tests/memcard.rs` already asserts for a real card |
| An independent reader agrees | one-off, documented in execution notes: the vendored `ps2mc` reader opens the synthesised card and returns the save |

**The ceiling, stated plainly:** no PS2 and no emulator exist in this
container, so *in-game and BIOS acceptance of a synthesised card is not
verified by this plan*. The checks above prove internal consistency and
structural agreement with the measured reference, which is the strongest
available evidence short of hardware.

---

## 7. Risks

| Risk | Mitigation |
| --- | --- |
| Page 1 is genuinely required and we leave it erased | Four reference cards have it erased; the field is documented as a known unknown, and `--card` still copies a donor for a byte-exact card |
| `superblock.bin` embeds a tail we do not understand | It is a constant across all ten references, pinned to the fixture, and never derived from the save |
| Synthesised entries omit timestamps/attributes the PS2 wants | Fields are zero only where the crate does not interpret them; committed tests pin every field we do set against the reference card |
| The layout hardcodes cluster numbers | The allocation is a pure function of the fixed save and icon sizes; a test asserts chains, entries and FAT agree |
| Silent behaviour change (`save_as` no longer errors) | Called out in this doc, in `docs/save-format.md`, and in the PR body |

---

## 8. Out of scope

- Card sizes other than the standard 8 MB image.
- Creating a directory or file inside an existing card.
- Editing or synthesising `icon.sys`/`icon1.ico` beyond writing the committed
  assets.
- Any change to `Document::save`'s write-safety policy.
- In-game verification (see §6).

## 9. Deliverable sequence (detail deferred to writing-plans)

1. Extract and commit `data/card/{superblock.bin,icon.sys,icon1.ico}`; extend
   `PROVENANCE.md`; pin lengths and hashes.
2. `memcard::format`: page 0 + ECC, indirect FAT, FAT table.
3. `memcard::format`: directory entries, icon payloads, the save chain, ECC.
4. `format_card()` public API plus the round-trip and structural tests.
5. `render_container` auto-create branch; update the two refusal tests.
6. CLI behaviour and its tests; confirm `--card` still overrides.
7. `docs/save-format.md` §6 creation algorithm and §12 note; update this doc if
   execution corrects it.
