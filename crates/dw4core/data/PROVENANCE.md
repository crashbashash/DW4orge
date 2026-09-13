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
