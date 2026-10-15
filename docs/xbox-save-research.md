# Xbox save research

Research date: 2026-10-15. The desktop editor and CLI support this format in
version 0.4.0. `tools/xbox_save_probe.py` is an experimental
inspector and selected-box name/BIT editing probe, requiring a local retail XBE.

## Evidence and storage

The USA retail game created a new save through the Central Area Save Keeper in
the private xemu build based on `36334d95b86f9c3c6ae34efb9af06785273f8e1a`.
The save directory observed in its FATX E partition was:

```
UDATA/42410002/
  TitleMeta.xbx       52 bytes
  TitleImage.xbx   10240 bytes
  SaveImage.xbx     4096 bytes
  80482ED3557F/
    SaveMeta.xbx      34 bytes
    SaveData.dat   65584 bytes
```

The child directory is an observed save-container identifier, not a constant
to hardcode. `SaveMeta.xbx` contains UTF-16LE `Name=Save Data\r\n` with a BOM.
The game's creation flow initializes an empty four-box container before the
player chooses a character box. Keep title/container metadata when editing an
existing exported folder. FATX access is a separate layer from game-data editing.
The upstream [FATX library](https://github.com/mborgerson/fatx) was used to extract
files from a paused copy of the working HDD, never to write into a running HDD.

Private ROMs, extracted XBEs, disk copies, saves and reports are outside this
repository at `../Decomp/local/xbox-save-research/` relative to `DW4orge/`.
The original xemu profile and its snapshots were preserved. Do not commit those
binary inputs or mount/write a live qcow2 through a second process.

## File layout and integrity

`SaveData.dat` is exactly `0x10030` bytes (65,584). There are two records:

| File offset | Size | Meaning |
| --- | --- | --- |
| `0x00000` | 4 | Wrapper version, little-endian u32 `4` |
| `0x00004` | `0x8000` | Character-box data |
| `0x08004` | 20 | HMAC-SHA1 of bytes `[0x00000, 0x08004)` |
| `0x08018` | 4 | Second wrapper version, u32 `4` |
| `0x0801C` | `0x8000` | Second character-box data |
| `0x1001C` | 20 | HMAC-SHA1 of bytes `[0x08018, 0x1001C)` |

The two records were byte-identical in both the freshly initialized container
and the populated retail sample. Both signatures matched an independent Python
HMAC calculation. There is no PS2-style additive checksum at the record start.
Each HMAC includes its 4-byte wrapper version and the entire box-data area.

The save UI presents four boxes. The probe models the data as four `0x2000`-byte
slots, with slot-relative `ISUSE` at zero. The first populated sample has only
box 1 in use. A subsequent retail memory-unit save placed the character in
box 4 at payload offset `0x6000`, confirming the stride. Native app tests also
selected and edited all four boxes in a controlled multi-box container.
Preserve all slot padding and other boxes, and update both record signatures.

## Signing key and regional identity

Both supplied discs have title ID `42410002` and identical 16-byte certificate
signature-key fields. XBE certificate offsets are relative to the certificate,
not the file header:

1. Read XBE base address at file `0x104` and certificate VA at `0x118`.
2. Certificate file offset = certificate VA minus base address.
3. Read the 16-byte signature-key field at certificate `+0xC0`.
4. Derive the key as `HMAC-SHA1(Xbox certificate key, certificate signature key)[:16]`.
5. Sign each record's first `0x8004` bytes with that derived key using HMAC-SHA1.

The derivation follows the implementation in
[xbox-save-sig](https://github.com/PMStanley/xbox-save-sig/blob/master/xboxsig.py).
It was also checked against the USA game's live `XboxSignatureKey` pointer:
the independently derived 16 bytes matched exactly. Both retail file signatures
verified using either the USA or Japanese XBE. The save writer calls
`XCalculateSignatureBegin(0)`; the observed signatures use the title key alone,
with no console EEPROM/HDD-key input.

| Input | SHA-256 |
| --- | --- |
| USA `Default.xbe` | `59d67f186feffc15bd5062a3bb099716aa111d5d7e1c48001673d6af7ff21a25` |
| Japan `Default.xbe` | `9e5b8f53870ab5de09c2931139a06c299f222ebf8baf19fe8f2d2b4ade09b02f` |

USA disassembly evidence (virtual addresses):

- `0x1B7810`: new-container write, two iterations of data followed by 20-byte signature.
- `0x1B7BA0`: load path; file length check at `0x1B7C73` requires `0x10030`;
  checks wrapper version and compares each calculated 20-byte signature.
- `0x1B7ED0`: existing-container write with the same signing sequence.
- `0x17CB1`, `0x17CC5`, `0x17D13`: signature begin/update/end wrappers.
- `0x17B14`: begin obtains the title's 16-byte kernel signing key.
- `0x179C6`, `0x17A2D`: HMAC inner/outer processing.

The Japanese load path also checks `0x10030` at `0x1B7E73`. Identical title/key
does not establish complete regional gameplay compatibility; Japanese runtime
load/save verification also passed, as recorded below.

## Character field mapping

The character payload starts with `ISUSE`, not checksum/version. For the first
box, fields are 4 bytes earlier in the Xbox signed record than in the existing
PS2 block map. Relative to a box itself they are 8 bytes earlier:

`Xbox slot offset = PS2 block field offset - 8` (for fields from `ISUSE` onward).

| Field | Xbox slot offset | First record file offset |
| --- | --- | --- |
| ISUSE | `0x0000` | `0x0004` |
| UNIQUE | `0x0004` | `0x0008` |
| Species/model name | `0x0008` | `0x000C` |
| Player name | `0x0028` | `0x002C` |
| Menu level | `0x0048` | `0x004C` |
| HP / MHP / MP / MMP | `0x004C`–`0x0058` | `0x0050`–`0x005C` |
| X-Data | `0x005C` | `0x0060` |
| BIT | `0x0060` | `0x0064` |
| Inventory | `0x0064` | `0x0068` |

The retail Dorumon sample has level 1, HP/MHP 170, MP/MMP 60, BIT/X-Data zero,
and inventory IDs `0x050D`, `0x0513`, `0x0512`. Player name `a` is represented
by the `0xFFFF` marker followed by UTF-16LE fullwidth `ａ` (`0xFF41`), as on PS2.
The Xbox serializer at USA VA `0x225160` and its inverse confirm the remaining
shared map: bank, equipment, disks, flags/folders/counters and per-species
levels, EXP, techniques and power-ups use the PS2 offsets minus eight.
Controlled retail load/save results are recorded below.

## Reproduction

### Retail acceptance check

The populated USA save was copied and edited to name `XBEDIT` and BIT `123456`.
Only those fields in box 1 and the two HMACs changed. All other boxes and
metadata remained byte-identical. A separate xemu profile cold-booted the USA
disc with a copied EEPROM and a disposable HDD containing the edited file.
Call Digimon showed `XBEDIT`; after loading into the Terminal Area, the character
menu showed name `XBEDIT` and BIT `123456`. This establishes actual retail
acceptance of the title-key signatures and those two field edits, rather than
just a successful offline hash calculation.

| File | SHA-256 |
| --- | --- |
| Populated retail `SaveData.dat` | `b6403138b1d804e3a96c6e49075ccc5ecd8a5645e7345d38163d59830426c7aa` |
| Edited `SaveData.dat` | `c6b9e61501402d309a1be27c58e7c15ad87bead681ecf5a3930be2194145b19a` |

Private screenshots and a machine-readable validation report are in
`../Decomp/local/xbox-save-research/evidence/`. The main xemu working HDD was
never patched by the probe; it retains the original retail character save.

### Probe commands

Inspect a local extracted file and verify both signatures:

```sh
python3 tools/xbox_save_probe.py /path/to/SaveData.dat --xbe /path/to/Default.xbe
```

Create a separate edited probe file (output must not already exist):

```sh
python3 tools/xbox_save_probe.py /path/to/SaveData.dat --xbe /path/to/Default.xbe \
  --box 1 --set-name XBEDIT --set-bit 123456 --out /path/to/edited-SaveData.dat
python3 -m unittest discover -s tools -p 'test_xbox_save_probe.py' -v
```

The tool refuses malformed lengths/versions, bad signatures, empty selected
boxes, invalid names and out-of-range BIT. Synthetic tests check selected-box
isolation, both signatures/mirrors, and corrupted-input refusal. It does not
write FATX, create saves from scratch, or integrate with the desktop editor.

## Integrated implementation and remaining scope

`crates/dw4core/src/xbox.rs` owns signed records and FATX parsing. It validates
both signatures before editing, adapts the selected box to shared fields, then
patches only known character-field ranges back into the original container.
All other boxes, slot padding, title/save metadata, file allocations and unrelated
files survive. Writes use existing cluster chains; they never allocate or resize
files. The parser rejects out-of-range/looping chains, shared allocations,
duplicate filenames, invalid geometry and truncated files.

The app detects raw save and memory-unit formats by content. Its character-box
selector lists occupied boxes, including boxes in multiple save containers on a
memory unit, and guards unsaved changes. Save/Save As retain the selection and
verify the entire output. Signature failures are explicit errors, not silently
repaired inputs. The CLI's `--xbox-slot` takes the IDs printed by `info`.

Expansion slot A/B is controller hardware placement, not a save-format variant:
each of four controllers has two slots, making eight possible units. xemu maps
controller 1 A/B to USB ports `1.3.2` / `1.3.3`; it attaches each raw image as a
separate USB storage device. See the [xemu memory-unit documentation](https://xemu.app/docs/xmus/)
and local `ui/xemu-input.c`. The live game recognizes units 1A and 1B separately.

The observed 8 MiB unit is FATX at offset zero, logical sector size 4096,
4 sectors/cluster (16 KiB), a 4096-byte superblock and a rounded 4096-byte
FAT16 table. Data clusters begin at `0x2000`; cluster numbers begin at one.
Directories use 64-byte entries with filename length/attributes, a 42-byte name,
first cluster at `+44` and size at `+48`. On the memory unit the title directory
is at the filesystem root, **without** the HDD's `UDATA` parent:

```
42410002/80482ED3557F/SaveData.dat
```

The game-created memory-unit file has exactly the HDD file's record layout and
both HMACs verify using the same title key. The UI's free-space units are
16 KiB blocks; the formatted 8 MiB test unit reports 510 total, and 506 free
before a title save is created. A/B are interchangeable mounting locations.
Detach a unit or close xemu before editing/replacing its image.

This release supports existing `.dat` save files and raw FATX `.img` memory
units, not direct HDD/qcow2 image editing, physical memory-unit USB access,
spare-area hardware dumps or standalone Xbox container creation. Version 0.9.0
adds empty-box initialization inside an existing occupied container.
Memory-unit images are bounded to 128 MiB with validated geometry; retail
acceptance was tested using standard 8 MiB images. Complete campaign behavior
and cross-title/nonstandard saves are not claimed.


### Retail field round-trip boundary

The USA game cold-loaded an app-written slot-A image, displaying name `MUAEDIT`,
level 17, BIT 234567 and X-Data 4321. Saving it back to the HDD retained EXP 43136,
junk counter 1000, all nine techniques, all eleven power-ups, inventory,
equipment/mod sockets, all twelve disk counts, bank balance 654321 and a bank
item. The bank loader at `0x225FF0` exits at the **first EMPTY** (`0x225FF5`), so
an isolated item at slot 96 after an empty slot is not restored. This is not a
95-slot capacity limit: the loader can consume a contiguous 96-item array.
Normal-mode Xbox edits therefore compact bank items while preserving order;
Advanced mode retains raw placement. A regression test pins both behaviors.

A probe of an arbitrary base flag 500 was cleared on load; flags 908/912 changed
during the menu/game flow. Those are runtime behavior, not a serializer-offset
mismatch. Persistent progression edits must use the documented difficulty mirrors,
as the shared story editor does. Raw/transient flags are not guaranteed to
survive loading. The subsequent probe uses documented milestone 20.


### Final integrated retail validation

The Japanese game cold-booted and loaded the final app-backend output through
controller 1 expansion slot **B**. It displayed `V04TEST`, level 17, BIT 234567
and X-Data 4321, then saved the loaded character to its HDD. Comparing the
result with the editor output matched name, level, EXP, BIT, X-Data, junk
counter, all techniques/power-ups, inventory, all equipment/mod slots, all disk
counts, bank balance, **both bank items** after packing, and story folders.
Documented milestone flag 20 survived loading and saving. Only runtime/menu
flags 908 and 912 differed from the requested story bytes. Both signatures
were valid and the game-written mirrors matched.

Combined with the USA slot-A test and native app tests, this covers both
expansion sockets, both supplied regional builds, HDD saves, retail box-4
placement, all four app box choices, and all editable field groups. It does
not claim a complete campaign playthrough. Private final captures and JSON
comparisons are under `../Decomp/local/xbox-save-research/phase2/evidence/`.

The final desktop summary uses content-sized character selection and a wrapping
row of fields. Native measurement changed its trigger width from about 124 px
for `Box 1 · BOX1` to 152 px for `Box 4 · MUAEDIT`; screenshots verify the final
summary spacing. The native app test also verified the discard guard and that
editing box 2 leaves boxes 1, 3 and 4 unchanged.


## Empty-box creation (0.9.0)

The editor and CLI now initialize empty boxes in existing signed dat containers
and FATX memory-unit saves. Both records must mark the target empty; valid
signatures are required before creation. The new full slot receives fresh
stats, story/difficulty, starter equipment and a distinct nonzero identity.
Only the target slot and signatures change. On multi-save units, the selected
occupied character identifies the destination container; creation reloads that
same container and rechecks the target. No new FATX allocation is needed.
Standalone container creation and entirely blank memory units remain outside
this flow. See [creation format](save-format.md#xbox-character-creation-in-an-existing-save)
and [validation](testing.md#creating-an-empty-xbox-character-box-090--schema-7).

Private 0.9.0 evidence is in `../Decomp/local/xbox-box-creation/`. Native app
checks cover dat creation through all four occupied boxes, MU creation/export,
discard guards and independent preservation/signature checks. USA retail xemu
accepted all four dat boxes, loaded new MU box 2 (`XBMU2`, Veemon), completed
the introduction and saved to HDD box 2. All 22 compared view fields and the
new identity survived, both XBE-derived signatures verified, and the other
three HDD boxes stayed byte-identical. The intro/lobby changed flags
24, 501, 508, 931, 935 and 936. The copied HDD, EEPROM and MU were isolated;
this is a focused USA creation round-trip, not a full campaign validation.
