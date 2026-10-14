#!/usr/bin/env python3
"""Extract story mirrors and checkpoints from corrected retail Lua disassembly.

Supply the disassembly directory explicitly. No game files or local paths are
stored in the output. The load and save scripts must agree on every flag map.
"""
import argparse
import json
from pathlib import Path
import re


def mirrors(text, reverse=False):
    columns = []
    for difficulty in range(3):
        part = text.split(f"be:SYSgetDifficulty() == {difficulty} then")[-1]
        part = re.split(r"\n(?:elseif|else|end)\b", part)[0]
        rows = {}
        loop = None
        for line in part.splitlines():
            match = re.search(r"for local_1 = (\d+), (\d+) do", line)
            if match:
                loop = tuple(map(int, match.groups()))
                continue
            if line.strip() == "end":
                loop = None
            match = re.search(r"FLGset\((\d+)( \+ local_1)?, be:FLGisOn\((\d+)( \+ local_1)?\)\)", line)
            if not match:
                continue
            target, target_loop, source, source_loop = match.groups()
            for i in range(loop[0], loop[1] + 1) if loop else [0]:
                a = int(target) + (i if target_loop else 0)
                b = int(source) + (i if source_loop else 0)
                active, mirror = (b, a) if reverse else (a, b)
                if active in rows:
                    raise ValueError(f"duplicate active flag {active}")
                rows[active] = mirror
        columns.append(rows)
    if not columns[0] or not all(set(c) == set(columns[0]) for c in columns):
        raise ValueError("incomplete difficulty mapping")
    return [dict(active=a, normal=columns[0][a], hard=columns[1][a], very_hard=columns[2][a])
            for a in sorted(columns[0])]


def checkpoint(text):
    flags = {}
    folders = {}
    for match in re.finditer(r"FLGset\((\d+), ([01])\)", text):
        flags[int(match[1])] = int(match[2])
    for match in re.finditer(r'SYSctlFlagFolder\("(SET|CLR)", (\d+)\)', text):
        folders[int(match[2])] = match[1] == "SET"
    return dict(flags=sorted(k for k, v in flags.items() if v),
                folders=sorted(k for k, v in folders.items() if v))


def folder_bands(load):
    bases = []
    count = None
    for difficulty in range(3):
        part = load.split(f"be:SYSgetDifficulty() == {difficulty} then")[1]
        part = re.split(r"\nend\b", part)[0]
        rows = [(int(flag), int(folder)) for flag, folder in re.findall(
            r'if be:FLGisOn\((\d+)\) == 1 then\s+be:SYSctlFlagFolder\("SET", (\d+)\)', part)]
        if not rows or [folder for _, folder in rows] != list(range(len(rows))):
            raise ValueError('incomplete folder restoration mapping')
        base = rows[0][0]
        if any(flag != base + folder for flag, folder in rows):
            raise ValueError('folder mirrors are not a contiguous band')
        if count is not None and count != len(rows):
            raise ValueError('difficulty folder counts disagree')
        count = len(rows)
        bases.append(base)
    return bases, count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lua", type=Path, required=True)
    parser.add_argument("--out", type=Path, default=Path("crates/dw4core/tests/fixtures/flags/retail.json"))
    args = parser.parse_args()
    load = (args.lua / "entry_0806/chunk_0006.lua").read_text()
    save = (args.lua / "entry_0810/chunk_0004.lua").read_text()
    mapping = mirrors(load)
    bases, count = folder_bands(load)
    if mapping != mirrors(save, reverse=True):
        raise ValueError("retail load/save mirror maps disagree")
    presets = {name: checkpoint((args.lua / f"entry_0603/chunk_{chunk:04}.lua").read_text())
               for name, chunk in [("After World 1", 71), ("After World 2", 73),
                                   ("After World 3", 74), ("Story cleared (post-game)", 75)]}
    result = dict(source=["entry_0806/chunk_0006", "entry_0810/chunk_0004", "entry_0603/chunks_0071-0075"],
                  mirrors=mapping, folder_mirror_base=bases, restored_folders=count,
                  checkpoints=presets)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2) + "\n")
    print(f"Wrote {len(mapping)} retail mirror rows and {len(presets)} checkpoints")


if __name__ == "__main__":
    main()
