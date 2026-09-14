# Vendored item data

Copied **verbatim** from `Decomp/DW4/DW4_Save_Editor/data/` in the
reverse-engineering tree. Do not edit by hand: these files are the game's item
tables, distilled from `AREA.AFS` and the executable's string table during the
original reverse engineering.

| File | Contents | Entries |
| --- | --- | --- |
| `weapons.json` | Graded weapons: 50 models × 5 grades, plus 6 unique weapons | 256 + 6 unique |
| `styled.json` | Styled weapons | 21 |
| `cores.json` | Cores (armor) | 32 |
| `armor.json` | Boards (sub slot) | 45 |
| `mods.json` | Mod-chip families, grades and stat-variant offsets | 185 ids from 15 families |

To refresh, re-copy from the same directory and run:

```sh
cargo test -p dw4core --test catalogue
```

That test compares every id, name, category and grade against a dump produced by
the Python editor, plus a description for each id and a validity verdict for
every id in the affected ranges.

## Card assets

Extracted by `tools/gen_fixtures.py` (`dump_card_assets`) from the PS2 card
images under `Decomp/DW4/` and embedded with `include_bytes!`.

| File | Contents | Source |
| --- | --- | --- |
| `card/superblock.bin` | page 0 of a standard 8 MB card: the 340-byte superblock plus its 172-byte tail | byte-identical across all 10 cards in the tree |
| `card/icon.sys` | the save's icon descriptor | `BASLUS-20836savedata/icon.sys` |
| `card/icon1.ico` | the animated save icon | `BASLUS-20836savedata/icon1.ico` |

The generator refuses to write them unless every card agrees, so no one card's
identity is baked in. `docs/card-creation-design.md` explains how they are used;
`crates/dw4core/tests/format.rs` pins the bytes they produce.
