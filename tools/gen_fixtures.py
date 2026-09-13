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
FLAG_OUT = REPO / "crates/dw4core/tests/fixtures/flags"
BUILDER_OUT = REPO / "crates/dw4core/tests/fixtures/builder"
DOC_OUT = REPO / "crates/dw4core/tests/fixtures/document"

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


def _gui_tables(editor: Path) -> dict:
    """Pull the GUI's mirror and preset tables out of its source.

    `save_editor_gui` imports tkinter at module scope, which is frequently
    absent (it is on this machine), so the module cannot be imported. The tables
    are plain literals, so parse the file and execute only the assignments we
    need. This is deliberately narrow: it extracts named constants, not code.
    """
    import ast  # noqa: PLC0415

    src = (editor / "save_editor_gui.py").read_text()
    tree = ast.parse(src)
    wanted = {
        "_story_preset",
        "STORY_PRESETS",
        "FLAG_MIRRORS",
        "FOLDER_MIRRORS",
        "FLAG_OFF",
        "FOLDER_OFF",
        "BOSS_LABELS",
        "CHAPTER_LABELS",
        "INTRO_LABELS",
        "QUEST_LABELS",
        "LOBBY_LABELS",
        "FOLDER_LABELS",
    }

    def names_of(node):
        if isinstance(node, ast.Assign):
            return {t.id for t in node.targets if isinstance(t, ast.Name)}
        return {getattr(node, "name", "")}

    keep = [n for n in tree.body if names_of(n) & wanted]
    found = {}
    exec(  # noqa: S102
        compile(ast.Module(body=keep, type_ignores=[]), "<gui-subset>", "exec"),
        found,
    )
    return found


def dump_flags(decomp: Path, out: Path) -> None:
    """Both partial Python mirror tables, and both preset dictionaries."""
    # Resolved at runtime from the path `import_reference` pushed, so static
    # tools cannot see it - same as `dw4save` above.
    import dw4build  # noqa: PLC0415  # type: ignore[import-not-found]

    out.mkdir(parents=True, exist_ok=True)
    gui = _gui_tables(decomp / "DW4_Save_Editor")

    mirrors = {
        "gui": {
            name: {str(k): v for k, v in table.items()}
            for name, table in gui["FLAG_MIRRORS"].items()
        },
        "gui_folders": {
            name: {str(k): v for k, v in table.items()}
            for name, table in gui["FOLDER_MIRRORS"].items()
        },
        "builder_normal": {str(k): v for k, v in dw4build.NORMAL_FLAG_MIRRORS.items()},
        "folder_base": {"Normal": 518, "Hard": 530, "Very Hard": 542},
    }
    (out / "mirrors.json").write_text(
        json.dumps(mirrors, indent=1, sort_keys=True) + "\n"
    )

    def preset_map(presets):
        """Normalise both preset shapes to {name: {flags, folders}}.

        The builder stores `active_flags`/`active_folders` lists; the GUI stores
        `{(kind, id): 0|1}`, including every label it knows, switched off.
        """
        normalised = {}
        for name, preset in presets.items():
            if "active_flags" in preset or "active_folders" in preset:
                flags = sorted(preset.get("active_flags", []))
                folders = sorted(preset.get("active_folders", []))
            else:
                flags = sorted(
                    i for (kind, i), on in preset.items() if kind == "flag" and on
                )
                folders = sorted(
                    i for (kind, i), on in preset.items() if kind == "folder" and on
                )
            normalised[name] = {"flags": flags, "folders": folders}
        return normalised

    presets = {
        "gui": preset_map(gui["STORY_PRESETS"]),
        "builder": preset_map(dw4build.STORY_PRESETS),
    }
    (out / "presets.json").write_text(
        json.dumps(presets, indent=1, sort_keys=True) + "\n"
    )

    print(
        f"wrote {out}/mirrors.json ({len(mirrors['builder_normal'])} builder rows, "
        f"{sum(len(v) for v in mirrors['gui'].values())} gui rows) and presets.json"
    )


def dump_builder(out: Path) -> None:
    """Saves synthesised by the Python builder, for byte-identity tests."""
    # Resolved at runtime from the path `import_reference` pushed.
    import dw4build  # noqa: PLC0415  # type: ignore[import-not-found]

    out.mkdir(parents=True, exist_ok=True)

    def write(name: str, spec: dict) -> None:
        (out / name).write_bytes(dw4build.build_save(spec))

    # `dw4build.fresh_spec()` leaves `"flags"` as None, and `build_block` does
    # `bytearray(spec.get("flags", ...))` - because the key *exists* with value
    # None, the default is not used and `bytearray(None)` raises TypeError. So a
    # storyless fresh save has to be handed its 1024 zero bytes explicitly,
    # which is exactly what `SaveSpec::default()` produces on the Rust side.
    plain = dw4build.fresh_spec()
    plain["flags"] = b"\x00" * 1024
    write("fresh_plain.raw", plain)
    write(
        "fresh_story.raw",
        dw4build.spec_with_story("Fresh (tutorial)", species=3, player_name="TST"),
    )
    write("maxed.raw", dw4build.spec_maxed("fresh", species=3, name="TST"))

    # story_{i}.raw must line up with STORY_PRESETS[i] on the Rust side, so
    # print the mapping and let the byte-identity test catch a mismatch.
    for index, name in enumerate(dw4build.STORY_PRESETS):
        write(
            f"story_{index}.raw",
            dw4build.spec_with_story(name, species=3, player_name="TST"),
        )
        print(f"  story_{index}.raw <- {name}")

    print(
        f"wrote {out}/ with fresh_plain.raw, fresh_story.raw, maxed.raw and "
        f"{len(dw4build.STORY_PRESETS)} story saves"
    )


def _story_bytes(gui, raw, story, difficulty):
    """Reproduce App.apply's story write, including the mirror copies.

    Transcribed from `save_editor_gui.py:1234`. The GUI mirrors through its
    *partial* tables (10 flags per difficulty), so the scripted edits below are
    chosen from that intersection - the deliberate full-table divergence is
    asserted separately on the Rust side.
    """
    fl = bytearray(raw[gui["FLAG_OFF"] : gui["FLAG_OFF"] + 1024])
    fo = bytearray(raw[gui["FOLDER_OFF"] : gui["FOLDER_OFF"] + 12])
    for (kind, i), val in story.items():
        (fo if kind == "folder" else fl)[i] = val
    for (kind, i), val in story.items():
        if kind == "folder" and 0 <= i <= 9:
            fl[gui["FOLDER_MIRRORS"][difficulty][i]] = val
        elif kind == "flag" and i in gui["FLAG_MIRRORS"][difficulty]:
            fl[gui["FLAG_MIRRORS"][difficulty][i]] = val
    return fl, fo


def dump_document(decomp: Path, out: Path, save_bytes: bytes) -> None:
    """The Python editor's view, apply and cap table for the document layer.

    `collect()` and `apply()` live in save_editor_gui.py, which imports tkinter
    at module scope and cannot be imported here. So `apply`'s *byte semantics*
    come from the real dw4save.SaveData, driven in the order App.apply uses
    (`save_editor_gui.py:1197`), and its story step is transcribed literally.
    """
    import dw4save as d  # noqa: PLC0415  # type: ignore[import-not-found]

    out.mkdir(parents=True, exist_ok=True)
    gui = _gui_tables(decomp / "DW4_Save_Editor")
    save = d.SaveData(bytearray(save_bytes))

    # The scripted edit set. Chosen to avoid the deliberate divergences: Normal
    # difficulty, and story flags that are inside the GUI's partial mirror
    # table, so Python and the full Rust table agree exactly.
    sp = 3
    edits = {
        "bit": 123_456,
        "xdata": 789,
        "junk": 26_000,
        "level": 42,
        "exp": d.level_threshold(42),
        "tech": [1, 2, 3, 4, 5, 6, 7, 8, 9],
        "upcnt": [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110],
        "name": "TST",
        "species": sp,
        "device": [
            d.build_item_id(0x0001, 5, 2) if i == 0 else d.EMPTY
            for i in range(d.DEVICE_SLOTS)
        ],
        "equip": {
            "weapon0": 0,
            "weapon1": d.EMPTY,
            "weapon2": d.EMPTY,
            "armor": d.EMPTY,
            "sub": d.EMPTY,
            "wmod0": d.EMPTY,
            "wmod1": d.EMPTY,
            "wmod2": d.EMPTY,
            "wmod3": d.EMPTY,
            "wmod4": d.EMPTY,
            "amod0": d.EMPTY,
            "amod1": d.EMPTY,
            "amod2": d.EMPTY,
            "amod3": d.EMPTY,
            "amod4": d.EMPTY,
        },
        "story": {("flag", 0): 1, ("flag", 1): 1, ("folder", 0): 1},
        "bank_bit": 55_555,
        "disks": [i * 3 for i in range(12)],
        "bank_items": [
            d.build_item_id(0x3000, 1, 0) if i == 0 else d.EMPTY
            for i in range(d.BANK_SLOTS)
        ],
    }

    # --- apply, in App.apply order -------------------------------------
    save.digimon_name = "p_" + d.SPECIES_MODEL[sp]
    save.bit = edits["bit"]
    save.xdata = edits["xdata"]
    save.junk_counter = edits["junk"]
    save.set_level(edits["level"], sp)
    save.menu_level = edits["level"]
    save.set_exp(edits["exp"], sp)
    for i, v in enumerate(edits["tech"]):
        save.set_skill(i, v, sp)
    for i, v in enumerate(edits["upcnt"]):
        save.set_upcnt(i, v, sp)
    save.player_name = edits["name"]
    for i, fid in enumerate(edits["device"]):
        save.set_device(i, fid)
    for i in range(len(edits["device"]), d.DEVICE_SAVE_SLOTS):
        save.set_device(i, d.EMPTY)
    for k, key in enumerate(("weapon0", "weapon1", "weapon2")):
        save.set_weapon(k, edits["equip"][key])
    save.set_armor(edits["equip"]["armor"])
    save.set_sub(edits["equip"]["sub"])
    for k in range(5):
        save.set_wmod(k, edits["equip"][f"wmod{k}"])
    for k in range(5):
        save.set_amod(k, edits["equip"][f"amod{k}"])

    fl, fo = _story_bytes(gui, save.raw, edits["story"], "Normal")
    save.set_bytes(gui["FLAG_OFF"], bytes(fl))
    save.set_bytes(gui["FOLDER_OFF"], bytes(fo))

    save.bank_bit = edits["bank_bit"]
    for i, c in enumerate(edits["disks"]):
        save.set_disk_count(i, c)
    for i, fid in enumerate(edits["bank_items"]):
        save.set_bank_device(i, fid)
    save.fix_checksums()

    (out / "applied.raw").write_bytes(bytes(save.raw))

    # --- the real cap table -------------------------------------------
    verdicts = {}
    for field in ("bit", "xdata", "level", "exp", "tech", "upcnt"):
        entry = d.CAPS[field]
        verdicts[field] = {
            "normal_max": entry[0],
            "dtype_max": entry[1],
            "dtype_min": entry[2] if len(entry) > 2 else 0,
        }
    verdicts["upcnt_safe_cap"] = [d.upcnt_safe_cap(i) for i in range(11)]
    (out / "validate.json").write_text(
        json.dumps(verdicts, indent=1, sort_keys=True) + "\n"
    )

    # --- the view, read back through the real getters -------------------
    view = {
        "species": save.detect_species(),
        "name": save.player_name,
        "bit": save.bit,
        "xdata": save.xdata,
        "junk_counter": save.junk_counter,
        "junk_tier": d.junk_tier_from_counter(save.junk_counter),
        "level": save.level(sp),
        "exp": save.exp(sp),
        "tech": [save.skill(i, sp) for i in range(9)],
        "upcnt": [save.upcnt(i, sp) for i in range(11)],
        "device": [save.device(i) for i in range(d.DEVICE_SLOTS)],
        "weapons": [save.weapon(i) for i in range(3)],
        "armor": save.armor(),
        "sub": save.sub(),
        "wmods": [save.wmod(i) for i in range(5)],
        "amods": [save.amod(i) for i in range(5)],
        "bank_bit": save.bank_bit,
        "disks": [save.disk_count(i) for i in range(12)],
        "bank_items": [save.bank_device(i) for i in range(d.BANK_SLOTS)],
    }
    (out / "view.json").write_text(json.dumps(view, indent=1, sort_keys=True) + "\n")

    print(
        f"wrote {out}/ with applied.raw ({len(bytes(save.raw))} bytes), "
        f"view.json and validate.json"
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
        "The Python editor in `Decomp/DW4/DW4_Save_Editor` is the only known-good\n"
        "implementation. These fixtures freeze its behaviour so the Rust core can be\n"
        "tested without Python and without the read-only `Decomp/` tree.\n\n"
        "| Item | Source |\n| --- | --- |\n"
        f"| `mcd001/save.raw` | `{CARD}` -> `BASLUS-20836savedata/BASLUS-20836savedata`, "
        "extracted with the Python editor's `dw4save.load_any` |\n"
        "| `mcd001/expected.json` | values read by the same module |\n"
        "| `catalogue/*.json` | `dw4save.load_catalogue`, `describe_item_id` and "
        "`invalid_reason` |\n"
        "| `flags/mirrors.json` | `dw4build.NORMAL_FLAG_MIRRORS`, plus the GUI's "
        "`FLAG_MIRRORS`/`FOLDER_MIRRORS` extracted by `ast` |\n"
        "| `flags/presets.json` | `dw4build.STORY_PRESETS` and the GUI's, likewise |\n"
        "| `builder/*.raw` | `dw4build.build_save` |\n"
        "| `document/applied.raw`, `document/view.json` | `dw4save.SaveData`, driven in "
        "`App.apply` order |\n"
        "| `document/validate.json` | `dw4save.CAPS` and `dw4save.upcnt_safe_cap` |\n\n"
        "## What is executed and what is transcribed\n\n"
        "`dw4save.py` imports cleanly and is used directly. `save_editor_gui.py`\n"
        "imports `tkinter` at module scope, which is not installed here, so it can\n"
        "never be imported. Two consequences:\n\n"
        "- Its plain-literal tables (`FLAG_MIRRORS`, `STORY_PRESETS`, the labels) are\n"
        "  extracted by parsing the source with `ast` and executing only the\n"
        "  assignments, never the code.\n"
        "- `App.collect()` and `App.apply()` cannot be run at all. `apply`'s byte\n"
        "  semantics are therefore produced by driving the real\n"
        "  `dw4save.SaveData` setters in the order `App.apply` uses\n"
        "  (`save_editor_gui.py:1197`), and its story-mirror step is transcribed\n"
        "  from `save_editor_gui.py:1234`. The ordering is verified by\n"
        "  `tests/document_parity.rs`, which requires byte-identical output; the\n"
        "  scripted edit set is chosen so that the deliberate divergences in\n"
        "  `docs/save-format.md` section 5 do not apply.\n"
    )

    n = len(bytes(save.raw))
    print(f"wrote {OUT}/save.raw ({n} bytes), expected.json, and PROVENANCE.md")
    assert n == 81920, f"expected an 81920-byte save, got {n}"
    assert expected["verify"], "the oracle says the checksum does not verify"

    dump_catalogue(d, CAT_OUT)
    dump_flags(decomp, FLAG_OUT)
    dump_builder(BUILDER_OUT)
    dump_document(decomp, DOC_OUT, bytes(save.raw))


if __name__ == "__main__":
    main()
