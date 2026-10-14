#!/usr/bin/env python3
"""Cold-boot editor cards and test every difficulty offered by retail DW4.

Inputs are supplied explicitly. Uses the private PCSX2 PINE client, Pillow,
Tesseract with English data, and Python 3.14's zstd ZIP support. All emulator
profiles, writable card copies, captures and reports stay in a new output folder.
"""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import time
import zipfile

MODES = ['Normal', 'Hard', 'Very Hard']


def read_ram(path):
    with zipfile.ZipFile(path) as archive:
        return archive.read('eeMemory.bin')


def properties(ram, name, size=None):
    """Find typed retail property records by their actual string and array size.

    igProperty_FindByName uses the key at +8, payload at +12. Array payloads
    carry count at +8 and buffer at +16. Heap addresses are discovered per state.
    """
    def u32(at):
        return struct.unpack_from('<I', ram, at)[0]
    string = name.encode() + b'\0'
    strings = []
    at = 0
    while (at := ram.find(string, at)) >= 0:
        strings.append(at)
        at += 1
    found = {}
    for pointer in strings:
        needle = struct.pack('<I', pointer)
        at = 0
        while (at := ram.find(needle, at)) >= 0:
            record = at - 8
            at += 4
            if record % 4 or not 0x600000 < record < len(ram) - 20:
                continue
            if not 0x400000 <= u32(record) < 0x580000:
                continue
            value = u32(record + 12)
            if size is None:
                if value in range(3):
                    found[record] = value
            elif 0x600000 < value < len(ram) - 20 and u32(value + 8) == size:
                data = u32(value + 16)
                if 0x600000 < data <= len(ram) - size:
                    found[data] = ram[data:data + size]
    if len(found) != 1:
        raise AssertionError(f'expected one live {name} property, found {len(found)}')
    return next(iter(found.values()))


def player_state(ram):
    # beNDMWMdlPlayerInfoWork, validated using its retail vtable and live-instance
    # header. The generic MDL scratch pointer can refer to an NPC and is unsuitable.
    found = []
    at = 0x600000
    while (at := ram.find(struct.pack('<I', 0x53D4A0), at)) >= 0:
        if at % 4 == 0 and struct.unpack_from('<I', ram, at + 4)[0] & 0x10000000 and ram[at + 0xEC] == 1:
            found.append(at)
        at += 4
    if len(found) != 1:
        raise AssertionError(f'expected one live player controller, found {len(found)}')
    address = found[0]
    return dict(control=bool(ram[address + 0x44]), frozen=bool(ram[address + 0x45]),
                mainLobby=bool(ram[address + 0x125]),
                heading=struct.unpack_from('<f', ram, address + 0x9C)[0])


class Retail:
    def __init__(self, args, output):
        self.args = args
        self.output = output
        self.process = None
        self.profile = output / 'profile' / 'PCSX2'
        (self.profile / 'inis').mkdir(parents=True)
        (self.profile / 'memcards').mkdir()
        shutil.copytree(args.profile_template / 'bios', self.profile / 'bios')
        ini = (args.profile_template / 'inis' / 'PCSX2.ini').read_text()
        ini = re.sub(r'^PINESlot\s*=.*$', f'PINESlot = {args.pine_slot}', ini, flags=re.M)
        ini = re.sub(r'\n\[MemoryCards\][\s\S]*?(?=\n\[|\Z)', '', ini)
        ini += '\n[MemoryCards]\nSlot1_Enable = true\nSlot1_Filename = Mcd001.ps2\nSlot2_Enable = false\n'
        (self.profile / 'inis' / 'PCSX2.ini').write_text(ini)
        self.last_capture = 0

    def stop(self):
        if self.process and self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)

    def start(self, card, directory):
        self.stop()
        shutil.copy2(card, self.profile / 'memcards' / 'Mcd001.ps2')
        with (directory / 'stdout.log').open('wb') as log:
            self.process = subprocess.Popen([
                str(self.args.emulator), '-datapath', str(self.profile.parent),
                '-logfile', str(directory / 'emulator.log'), '-batch', '-nofullscreen',
                '-unlimited', str(self.args.iso),
            ], env=dict(os.environ, QT_QPA_PLATFORM='xcb'), stdout=log, stderr=log)
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise RuntimeError('emulator exited during startup')
            try:
                emu = self.args.client(slot=self.args.pine_slot)
                if emu.emulator_status() == 'running':
                    return emu
                emu.close()
            except OSError:
                pass
            time.sleep(.1)
        raise RuntimeError('emulator did not boot')

    def snapshot(self, emu, directory, name):
        delay = 1.1 - (time.monotonic() - self.last_capture)
        if delay > 0:
            time.sleep(delay)
        snapshots = self.profile / 'snaps'
        previous = set(snapshots.glob('*.png'))
        emu.gs_dump(frames=1)
        self.last_capture = time.monotonic()
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            files = set(snapshots.glob('*.png')) - previous
            if files:
                source = next(iter(files))
                try:
                    from PIL import Image
                    image = Image.open(source)
                    image.load()
                except (OSError, SyntaxError):
                    time.sleep(.05)
                    continue
                target = directory / f'{name}.png'
                shutil.copy2(source, target)
                source.with_suffix('.gs.zst').unlink(missing_ok=True)
                break
            time.sleep(.05)
        else:
            raise RuntimeError('GS screenshot did not finish')
        def ocr(path, mode):
            return subprocess.check_output([
                'tesseract', str(path), 'stdout', '--tessdata-dir', str(self.args.tessdata),
                '--psm', str(mode),
            ], stderr=subprocess.DEVNULL, text=True)
        text = ocr(target, 11)
        crop = image.convert('L').crop((0, 0, int(image.width * .52), int(image.height * .53)))
        crop = crop.resize((int(image.width * 1.04), int(image.height * 1.06)))
        crop = crop.point(lambda value: 255 if value > 160 else 0)
        crop.save(directory / 'ocr.png')
        return text + ocr(directory / 'ocr.png', 6), image

    def state(self, emu, slot):
        directory = self.profile / 'sstates'
        previous = {p: p.stat().st_mtime_ns for p in directory.glob(f'*.{slot:02}.p2s')}
        emu.pause()
        emu.save_state(slot)
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            files = [p for p in directory.glob(f'*.{slot:02}.p2s') if p.stat().st_mtime_ns != previous.get(p)]
            if files:
                try:
                    return read_ram(files[0])
                except (OSError, zipfile.BadZipFile, EOFError):
                    pass
            time.sleep(.05)
        raise RuntimeError('savestate did not finish')

    def run(self, case, mode):
        directory = self.output / (Path(case['file']).stem + '-' + mode.lower().replace(' ', '-'))
        directory.mkdir()
        with self.start(self.args.cards / case['file'], directory) as emu:
            def press(button=None, left=(0, 0), wait=50):
                emu.press([button] if button else [], left=left, frames=6 if left != (0, 0) else 12)
                emu.press(frames=wait)
            def until(name, predicate, skip=False, retry=None):
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    text, image = self.snapshot(emu, directory, name)
                    if predicate(text, image):
                        return text
                    if retry:
                        press(retry)
                    elif skip:
                        press('Cross')
                    else:
                        emu.press(frames=120)
                raise AssertionError(f'{name} timed out: {text!r}')
            until('title', lambda t, i: 'START' in t.upper(), skip=True)
            press('Start')
            until('main-menu', lambda t, i: bool(re.search(r'\bOption\b', t, re.I)), retry='Start')
            press('Cross')
            until('call-menu', lambda t, i: 'Create' in t and 'Digimon' in t)
            press(left=(0, 1), wait=12)
            press('Cross')
            until('card-choice', lambda t, i: 'MEMORY' in t.upper() or 'memory card' in t.lower())
            press('Cross')
            until('digimon-choice', lambda t, i: 'Dorumon' in t and ('NO DATA' in t.upper() or 'call a' in t.lower()))
            press('Cross')
            text = until('difficulty', lambda t, i: 'START' in t.upper() or 'Difficulty' in t)
            available = ['Normal']
            if 'Difficulty' in text:
                if re.search(r'\bHard\b', text, re.I):
                    available.append('Hard')
                if re.search(r'\bVery\b', text, re.I):
                    available.append('Very Hard')
                assert mode in available, (mode, available, text)
                for _ in range(MODES.index(mode)):
                    press(left=(0, 1), wait=12)
                press('Cross')
                until('login', lambda t, i: 'START' in t.upper())
            else:
                assert mode == 'Normal', (mode, text)
            press('Start')
            def lobby(text, image):
                if 'START' in text.upper() or 'NO DATA' in text.upper():
                    return False
                pixels = image.convert('RGB').get_flattened_data()
                blue = sum(b > r * 1.6 and b > g * 1.4 and b > 140 for r, g, b in pixels)
                return blue / (image.width * image.height) > .24
            until('arrival', lobby)
            arrival = self.state(emu, 220)
            assert properties(arrival, 'DIFFICULTY') == MODES.index(mode), 'wrong game difficulty selected'
            actual = properties(arrival, 'FLAG', 1024)
            saved = set(case['flags'])
            expected = {row['active']: int(row[['normal', 'hard', 'very_hard'][MODES.index(mode)]] in saved)
                        for row in self.args.oracle['mirrors']}
            # The lobby starts the tutorial if a difficulty has no chapter yet.
            if not any(expected.get(flag) for flag in [1, 2, 3, 4, 5, 6, 86]):
                expected[1] = 1
            for flag, value in expected.items():
                assert actual[flag] == value, f'{case["file"]} / {mode}: runtime flag {flag}={actual[flag]}, expected {value}'
            folders = properties(arrival, 'FLAGFOLDER', 12)
            base = self.args.oracle['folder_mirror_base'][MODES.index(mode)]
            assert list(folders[:11]) == [int(base + folder in saved) for folder in range(11)], 'folder restoration mismatch'
            emu.resume()
            for _ in range(35):
                press('Cross', wait=30)
            self.snapshot(emu, directory, 'dialogue-finished')
            before = player_state(self.state(emu, 221))
            assert before['control'] and not before['frozen'] and before['mainLobby'], 'player remained stuck in the arrival sequence'
            emu.resume()
            emu.press(left=(1, 0), frames=30)
            self.snapshot(emu, directory, 'movement-right')
            right = player_state(self.state(emu, 222))
            emu.resume()
            emu.press(left=(-1, 0), frames=30)
            self.snapshot(emu, directory, 'movement-left')
            left = player_state(self.state(emu, 223))
            heading_change = abs((right['heading'] - left['heading'] + math.pi) % (2 * math.pi) - math.pi)
            assert right['control'] and left['control'] and heading_change > 2.8, 'player controller did not respond to opposite stick inputs'
            result = dict(file=case['file'], preset=case['preset'], builtDifficulty=case['builtDifficulty'],
                          loadedDifficulty=mode, available=available, restoredFlags=expected,
                          restoredFolders=list(folders), playerBefore=before, playerRight=right, playerLeft=left, headingChange=heading_change)
            (directory / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
            print(f'PASS: {case["file"]} loaded on {mode}; restored story and player control verified', flush=True)
            return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['emulator', 'iso', 'profile-template', 'pine-client', 'tessdata', 'cards', 'output']:
        parser.add_argument('--' + key, type=Path, required=True)
    parser.add_argument('--pine-slot', type=int, default=28014)
    parser.add_argument('--case', action='append', help='specific card filenames; default: all manifest cases')
    args = parser.parse_args()
    for key in ['emulator', 'iso', 'profile_template', 'pine_client', 'tessdata', 'cards', 'output']:
        setattr(args, key, getattr(args, key).resolve())
    socket = Path(os.environ.get('XDG_RUNTIME_DIR', '/tmp')) / f'pcsx2.sock.{args.pine_slot}'
    if socket.exists():
        raise SystemExit('PINE slot is already in use; choose an unused --pine-slot')
    sys.path.insert(0, str(args.pine_client))
    from pcsx2 import PCSX2
    args.client = PCSX2
    args.oracle = json.loads((Path(__file__).resolve().parents[1] / 'crates/dw4core/tests/fixtures/flags/retail.json').read_text())
    cases = json.loads((args.cards / 'cases.json').read_text())
    if args.case:
        cases = [case for case in cases if case['file'] in args.case]
        assert len(cases) == len(args.case), 'unknown --case filename'
    args.output.mkdir(parents=True, exist_ok=False)
    original_hash = hashlib.file_digest(args.iso.open('rb'), 'md5').hexdigest()
    retail = Retail(args, args.output)
    results = []
    try:
        for case in cases:
            normal = retail.run(case, 'Normal')
            results.append(normal)
            for difficulty in normal['available'][1:]:
                results.append(retail.run(case, difficulty))
            (args.output / 'report.json').write_text(json.dumps({'passed': results, 'discMd5': original_hash}, indent=2) + '\n')
    finally:
        retail.stop()
        assert hashlib.file_digest(args.iso.open('rb'), 'md5').hexdigest() == original_hash, 'input disc changed'


if __name__ == '__main__':
    main()
