#!/usr/bin/env python3
"""Regenerate dw4core's golden fixtures from the reverse-engineering tree.

Reads (never writes) Decomp/DW4/DW4_Save_Editor and its memcard dumps, using the
Python editor as the oracle: whatever dw4save.py reads from the real card is what
the Rust core must reproduce.

The editor's venv/bin/python symlink is broken (it points at a nonexistent
/usr/bin/python), so this script does not use it. It puts the venv's
site-packages on sys.path and runs under any Python 3.8+.

    python3 tools/gen_fixtures.py --decomp /workspace/Decomp/DW4

Run it by hand when the format changes; commit the result.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
OUT = REPO / "crates/dw4core/tests/fixtures/mcd001"
CAT_OUT = REPO / "crates/dw4core/tests/fixtures/catalogue"

# The real card, and the save inside it, as expected.json's provenance records.
CARD = "memcards/Mcd001.ps2"


def import_reference(decomp: Path):
    """Import dw4save from the read-only Decomp tree."""
    editor = decomp / "DW4_Save_Editor"
    venvs = sorted((editor / "venv/lib").glob("python3.*/site-packages"))
    if not venvs:
        sys.exit(f"no site-packages under {editor}/venv/lib - is the venv present?")
    for path in (str(venvs[0]), str(editor)):
        if path not in sys.path:
            sys.path.insert(0, path)

    # Resolved at runtime from the path pushed above, so static tools cannot
    # see it. The venv check just before fails loudly if it is genuinely
    # missing, so an unresolved-import warning here is expected.
    import dw4save  # noqa: E402  # type: ignore[import-not-found]

    return dw4save


def dump(d, save) -> dict:
    """Everything the Rust core must reproduce, read through the Python oracle."""
    species = save.detect_species()
    nicknames = {}
    for i in range(d.DEVICE_SAVE_SLOTS):
        fid = save.device(i)
        if fid != d.EMPTY:
            base = fid & 0xFFFF
            item = d.get_catalogue().get(base)
            nicknames[str(base)] = item.name if item else f"[invalid] 0x{base:04x}"
    return {
        "verify": save.verify(),
        "checksum_block": save.get_u32(0, 0),
        "version": save.version,
        "isuse": save.get_u32(0x08, 0),
        "unique": save.get_u32(0x0C, 0),
        "digimon_name": save.digimon_name,
        "player_name": save.player_name,
        "detected_species": species,
        "bit": save.bit,
        "xdata": save.xdata,
        "menu_level": save.menu_level,
        "junk_counter": save.junk_counter,
        "device": [save.device(i) for i in range(d.DEVICE_SAVE_SLOTS)],
        "weapon": [save.weapon(i) for i in range(3)],
        "weapon_mod": [save.wmod(i) for i in range(5)],
        "armor": save.armor(),
        "armor_mod": [save.amod(i) for i in range(5)],
        "sub": save.sub(),
        "disk": [save.disk_count(i) for i in range(12)],
        "bank_bit": save.bank_bit,
        "bank_device": [save.bank_device(i) for i in range(d.BANK_SLOTS)],
        "base_level": [save.level(i) for i in range(16)],
        "base_exp": [save.exp(i) for i in range(16)],
        "base_skill_active": [save.skill(i, species) for i in range(9)],
        "base_upcnt_active": [save.upcnt(i, species) for i in range(11)],
        "nicknames": nicknames,
    }


def dump_catalogue(d, out: Path) -> None:
    """Every catalogue entry, plus descriptions and validity verdicts."""
    out.mkdir(parents=True, exist_ok=True)
    cat = d.get_catalogue()

    items = [
        {
            "base": base,
            "name": item.name,
            "cat": item.cat,
            "grade": item.grade,
        }
        for base, item in sorted(cat.items())
    ]
    (out / "items.json").write_text(json.dumps(items, indent=1) + "\n")

    # Describe every catalogue id, plus ids that exercise the invalid path and
    # the rarity/mod suffixes. The last group walks the rarity band boundaries
    # and the placeholder id from the appraisal items.
    describe_ids = sorted(cat.keys())
    describe_ids += [
        0xFFFFFFFF,  # the empty sentinel
        0x00010000,  # seed 0, 1 mod
        0x00050000,  # 5 mods, no seed
        0x000F0000,  # 15 mods
        0x00100000,  # blue band, low end (seed 1)
        0x00F00000,  # blue band, high end (seed 15)
        0x01000000,  # green band, low end (seed 16)
        0x07F00000,  # green band, high end (seed 127)
        0x08000000,  # yellow band, low end (seed 128)
        0x1FF00000,  # yellow band, high end (seed 511)
        0x20000000,  # orange band, low end (seed 512)
        0x3FF00000,  # orange band, high end (seed 1023)
        0x40000000,  # pink band, low end (seed 1024)
        0x7FF00000,  # pink band, high end (seed 2047)
        0x80000000,  # bit 11 set, which is masked off
        0x00300000,  # seed 3, in the blue band
        0x01234000,  # invalid base 0x4000
        0x00000100,  # invalid base 0x0100, category byte 0x01
        0x00004400,  # invalid base 0x4400
    ]
    describe = {str(i): d.describe_item_id(i) for i in describe_ids}
    (out / "describe.json").write_text(
        json.dumps(describe, indent=1, sort_keys=True) + "\n"
    )

    # Every base id in the ranges of interest, plus the boundaries around each
    # glitch band and a few ids outside any known category.
    bases = list(range(0x0100))  # graded + unique weapons
    bases += list(range(0x0500, 0x0540))  # styled, including the glitch tail
    bases += list(range(0x1000, 0x1040))  # cores
    bases += list(range(0x2000, 0x2040))  # boards
    bases += list(range(0x30B0, 0x30C0))  # mods, including the blank tail
    bases += [0x3400, 0x3401, 0x44FF, 0xFFFF, 0x1234]
    invalid = {str(b): d.invalid_reason(b) for b in sorted(set(bases))}
    (out / "invalid.json").write_text(
        json.dumps(invalid, indent=1, sort_keys=True) + "\n"
    )

    print(
        f"wrote {out}/items.json ({len(items)} entries), "
        f"describe.json ({len(describe)}), invalid.json ({len(invalid)})"
    )


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--decomp", default="/workspace/Decomp/DW4")
    args = ap.parse_args()
    decomp = Path(args.decomp).resolve()

    d = import_reference(decomp)
    card = decomp / CARD
    save = d.load_any(str(card))

    OUT.mkdir(parents=True, exist_ok=True)

    # The raw save, extracted through the oracle, is the fixture itself.
    (OUT / "save.raw").write_bytes(bytes(save.raw))

    expected = dump(d, save)
    (OUT / "expected.json").write_text(json.dumps(expected, indent=2) + "\n")

    (OUT.parent / "PROVENANCE.md").write_text(
        "# Fixture provenance\n\n"
        "Generated by `tools/gen_fixtures.py`. Do not edit by hand.\n\n"
        "| Item | Source |\n| --- | --- |\n"
        f"| `mcd001/save.raw` | `{CARD}` -> `BASLUS-20836savedata/BASLUS-20836savedata`, "
        "extracted with the Python editor's `dw4save.load_any` |\n"
        "| `mcd001/expected.json` | values read by the same module |\n\n"
        "The Python editor in `Decomp/DW4/DW4_Save_Editor` is the only known-good\n"
        "implementation; these fixtures freeze its reading of a real save so the Rust\n"
        "core can be tested without Python.\n"
    )

    n = len(bytes(save.raw))
    print(f"wrote {OUT}/save.raw ({n} bytes), expected.json, and PROVENANCE.md")
    assert n == 81920, f"expected an 81920-byte save, got {n}"
    assert expected["verify"], "the oracle says the checksum does not verify"

    dump_catalogue(d, CAT_OUT)


if __name__ == "__main__":
    main()
