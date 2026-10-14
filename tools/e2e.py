#!/usr/bin/env python3
"""Exercise the actual editor UI and verify its saved files through the CLI.

Requires an agent-control editor session and the dw4cli binary. Output is written
only to a new directory. No emulator, game files or credentials are bundled.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import time
import urllib.request
from agent import evaluate


class Editor:
    def __init__(self, session):
        self.session = session

    def eval(self, script):
        return evaluate(self.session, script)

    def command(self, *args):
        return json.loads(subprocess.check_output(
            [sys.executable, str(Path(__file__).with_name('agent.py')), '--session', self.session, *args], text=True))

    def wait(self, predicate, timeout=15):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if predicate():
                return
            time.sleep(.05)
        raise AssertionError('editor did not reach the expected state: ' + self.eval('document.body.innerText'))

    def view(self):
        return self.eval("window.__TAURI_INTERNALS__.invoke('get_view', {mode:'advanced'})")

    def saved(self):
        self.wait(lambda: self.eval("!document.querySelector('[aria-label=\"Unsaved changes\"]') && !document.querySelector('[role=alert]')"))

    def create(self, name, preset, difficulty, species=None):
        if self.eval("!!document.querySelector('[aria-label=\"Unsaved changes\"]')"):
            self.command('click', 'New')
            self.command('click', 'Discard')
        else:
            self.command('click', 'New')
        if species:
            self.command('select', 'Species', species)
        self.command('fill', 'Player name', name)
        self.command('select', 'Story preset', preset or 'None (new game)')
        self.command('select', 'Difficulty', difficulty)
        self.command('click', 'Create')
        self.wait(lambda: self.eval("!document.querySelector('[role=dialog]') && !!document.querySelector('[aria-label=\"Unsaved changes\"]')"))

    def save_as(self, path, first=False):
        self.command('dialog', 'save', str(path))
        self.command('click', 'Save' if first else 'Save As')
        self.saved()
        assert path.is_file(), path

    def open(self, path):
        self.command('dialog', 'open', str(path))
        self.command('click', 'Open')
        self.saved()
        self.wait(lambda: str(path) in self.eval('document.body.innerText'))

    def pick_first_item(self, label='Item', slot=None):
        root = f"document.querySelector('[data-testid=item-row-{slot}]')" if slot is not None else 'document'
        script = """(async () => {
          const root = ROOT;
          const buttons = [...root.querySelectorAll('button')].filter(e=>e.getAttribute('aria-label') === LABEL);
          if (buttons.length !== 1) throw Error('expected one item picker');
          buttons[0].focus(); buttons[0].click();
          for (let n=0;n<120;n++) {
            await new Promise(requestAnimationFrame);
            const e=document.querySelector('[role=option]');
            if(e) {const text=e.textContent;e.click();await new Promise(requestAnimationFrame);return text;}
          }
          throw Error('item picker did not open');
        })()""".replace('ROOT', root).replace('LABEL', json.dumps('Show '+label+' options'))
        return self.eval(script)

    def select_first_nonempty(self, label):
        self.eval("""(() => {
          const e=[...document.querySelectorAll('button')].find(e=>(e.getAttribute('aria-labelledby')||'').split(' ').some(id=>document.getElementById(id)?.textContent===LABEL));
          if (!e) throw Error('select missing');e.click();return true;
        })()""".replace('LABEL',json.dumps(label)))
        self.wait(lambda: self.eval("document.querySelectorAll('[role=option]').length > 1"))
        return self.eval("(() => {const e=document.querySelectorAll('[role=option]')[1];const text=e.textContent;e.click();return text;})()")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--session', required=True)
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    editor = Editor(args.session)
    info = editor.eval("window.__TAURI_INTERNALS__.invoke('app_info')")
    checks = []
    cases = []

    def record(name):
        checks.append(name)
        print('PASS: ' + name, flush=True)

    session = json.loads(Path(args.session).read_text())
    request = urllib.request.Request(f"http://127.0.0.1:{session['port']}/eval",
        data=json.dumps({'script': 'window.__DW4_UNAUTHORIZED_TEST__ = true'}).encode(),
        headers={'Content-Type': 'application/json'})
    with urllib.request.urlopen(request, timeout=15) as response:
        unauthorized = json.load(response)
    assert unauthorized['ok'] is False and unauthorized['error'] == 'unauthorized'
    assert editor.eval('window.__DW4_UNAUTHORIZED_TEST__ ?? null') is None
    record('unauthenticated requests cannot execute webview scripts')

    def dump(path):
        return json.loads(subprocess.check_output([str(args.cli.resolve()), '--json', 'dump', str(path)], text=True))

    def verify(path):
        subprocess.run([str(args.cli.resolve()), 'verify', str(path)], check=True, stdout=subprocess.DEVNULL)

    for index, preset in enumerate([None] + [p['name'] for p in info['ui']['story_presets']]):
        for difficulty in ['Normal', 'Hard', 'Very Hard']:
            stem = f'p{index}-{difficulty.lower().replace(" ", "-")}'
            editor.create(f'E2E{index}', preset, difficulty)
            card = args.output / (stem + '.ps2')
            raw = args.output / (stem + '.raw')
            editor.save_as(card, first=True)
            native = editor.view()
            verify(card)
            assert dump(card) == native, stem
            editor.save_as(raw)
            verify(raw)
            assert raw.stat().st_size == 81920 and dump(raw) == native
            assert card.stat().st_size == 8650752
            editor.open(card)
            assert editor.view() == native
            cases.append({'file': card.name, 'preset': preset, 'builtDifficulty': difficulty,
                          'flags': [i for i,v in enumerate(native['story_flags']) if v],
                          'folders': [i for i,v in enumerate(native['story_folders']) if v]})
            record(f'create, card save, raw save, reopen: {preset or "new game"} / {difficulty}')

    (args.output / 'cases.json').write_text(json.dumps(cases, indent=2) + '\n')
    card = args.output / 'editing.ps2'
    editor.save_as(card)

    # Exercise edits through every section, then reopen the real native output.
    editor.command('click', 'Character')
    for label, value in [('Player name', 'EDITTEST'), ('BIT', '123456'), ('X-Data', '1234'), ('Level', '42'), ('BLUNT', '17'), ('HP MAX', '99')]:
        editor.command('fill', label, value)
    editor.command('click', 'Disks')
    for label in ['HP Disk α', 'Cure Disk', 'Key Chain']:
        editor.command('fill', label, '9')
    editor.command('click', 'Bank')
    editor.command('fill', 'Bank balance', '54321')
    editor.command('click', 'Save')
    editor.saved()
    path = card
    edited = editor.view()
    assert edited['name'] == 'EDITTEST' and edited['bit'] == 123456 and edited['level'] == 42
    assert edited['bank_bit'] == 54321 and edited['disks'][0] == 9 and edited['disks'][6] == 9 and edited['disks'][11] == 9
    assert edited['tech'][0] == 17 and edited['upcnt'][0] == 99
    editor.open(path)
    assert editor.view() == edited
    verify(path)
    record('character, techniques, power-ups, disks and bank edits survive a card reopen')

    editor.command('click', 'Items')
    editor.command('select', 'Type 1', 'Weapon')
    weapon = editor.pick_first_item(slot=0)
    editor.command('click', 'Equipment')
    equipped = editor.select_first_nonempty('Weapon 1')
    editor.pick_first_item('Weapon mod 1')
    editor.command('click', 'Bank')
    editor.command('click', 'Page 8')
    editor.command('select', 'Type 96', 'Armor')
    bank_item = editor.pick_first_item(slot=95)
    editor.command('click', 'Save')
    editor.saved()
    items = editor.view()
    assert items['device'][0] != 4294967295 and items['weapons'][0] == 0
    assert items['wmods'][0] != 4294967295 and items['bank_items'][95] != 4294967295
    editor.open(path)
    assert editor.view() == items
    record('normal item picker, equipment, automatic mod inventory and last bank page survive reopen')

    # A story preset must clear earlier prerequisites and set undisplayed ones too.
    editor.command('click', 'Story')
    editor.command('select', 'Preset', 'After World 1')
    editor.command('click', 'Save')
    editor.saved()
    first = editor.view()
    assert first['story_flags'][3] == 1 and first['story_flags'][320] == 1 and first['story_flags'][383] == 1
    editor.command('select', 'Preset', 'Fresh (tutorial)')
    editor.command('click', 'Save')
    editor.saved()
    fresh = editor.view()
    assert fresh['story_flags'][1] == 1 and fresh['story_flags'][3] == 0 and fresh['story_flags'][383] == 0
    editor.open(path)
    assert editor.view() == fresh
    record('story presets replace all governed milestones, including undisplayed prerequisites')

    # Species selection reads each form's own saved progression.
    editor.command('click', 'Character')
    editor.command('select', 'Species', 'Agumon')
    editor.wait(lambda: editor.eval("[...document.querySelectorAll('input')].find(e=>[...e.labels].some(l=>l.textContent==='Level')).value==='1'"))
    editor.command('fill', 'Level', '7')
    editor.command('click', 'Save')
    editor.saved()
    editor.command('select', 'Species', 'Dorumon')
    editor.wait(lambda: editor.eval("[...document.querySelectorAll('input')].find(e=>[...e.labels].some(l=>l.textContent==='Level')).value==='42'"))
    editor.command('click', 'Save')
    editor.saved()
    editor.open(path)
    assert editor.view()['species'] == 'Dorumon' and editor.view()['level'] == 42
    record('species switches read and preserve independently saved progression')

    # Save and reopen every form offered by the real New Save dialog.
    editor.command('click', 'New')
    species = editor.eval("[...document.querySelector('[role=dialog] select').options].map(o=>o.textContent.trim()).filter(Boolean)")
    editor.command('click', 'Cancel')
    for index, form in enumerate(species):
        editor.create(f'FORM{index}', 'Fresh (tutorial)', 'Normal', species=form)
        target = args.output / f'form-{index}.raw'
        editor.save_as(target, first=True)
        result = dump(target)
        assert result['species'] == form and result['name'] == f'FORM{index}'
        verify(target)
    record(f'all {len(species)} species create valid, correctly identified saves')

    # A species switch must not silently discard edits, and saved edits must
    # remain available when that form is selected again.
    editor.command('click', 'Character')
    editor.command('fill', 'Level', '17')
    editor.command('select', 'Species', 'Agumon')
    editor.wait(lambda: editor.eval("!!document.querySelector('[role=dialog]')"))
    editor.command('click', 'Cancel')
    assert editor.eval("[...document.querySelectorAll('input')].find(e=>[...e.labels].some(l=>l.textContent==='Level')).value") == '17'
    editor.command('click', 'Save')
    editor.saved()
    editor.command('select', 'Species', 'Agumon')
    editor.command('select', 'Species', species[-1])
    editor.wait(lambda: editor.eval("[...document.querySelectorAll('input')].find(e=>[...e.labels].some(l=>l.textContent==='Level')).value==='17'"))
    editor.command('click', 'Save')
    editor.saved()
    record('cancelling a species switch retains unsaved progression; saving makes it persistent')

    editor.command('fill', 'BIT', '4321')
    editor.eval("window.dispatchEvent(new KeyboardEvent('keydown',{key:'z',ctrlKey:true,bubbles:true}))")
    editor.wait(lambda: editor.eval("[...document.querySelectorAll('input')].find(e=>[...e.labels].some(l=>l.textContent==='BIT')).value==='0'"))
    editor.eval("window.dispatchEvent(new KeyboardEvent('keydown',{key:'z',ctrlKey:true,shiftKey:true,bubbles:true}))")
    editor.wait(lambda: editor.eval("[...document.querySelectorAll('input')].find(e=>[...e.labels].some(l=>l.textContent==='BIT')).value==='4321'"))
    original = editor.view()
    editor.eval("window.dispatchEvent(new KeyboardEvent('keydown',{key:'o',ctrlKey:true,bubbles:true}))")
    editor.wait(lambda: editor.eval("!!document.querySelector('[role=dialog]')"))
    editor.command('click', 'Cancel')
    assert editor.eval("[...document.querySelectorAll('input')].find(e=>[...e.labels].some(l=>l.textContent==='BIT')).value") == '4321'
    record('Ctrl+O uses the same discard guard as the toolbar')
    editor.command('dialog', 'save')
    editor.command('click', 'Save As')
    assert editor.eval("!!document.querySelector('[aria-label=\"Unsaved changes\"]')")
    assert editor.view() == original
    editor.command('click', 'Save')
    editor.saved()
    record('undo/redo and cancelled Save As preserve the draft and the original file')

    editor.command('click', 'Mode: Normal')
    editor.command('click', 'Character')
    editor.command('fill', 'BIT', '12000000')
    editor.command('fill', 'X-Data', '12345')
    editor.command('fill', 'Level', '1000')
    editor.command('fill', 'blunt', '10000')
    editor.command('fill', 'HP max', '100000')
    editor.command('click', 'Items')
    editor.command('fill', 'Raw id 1', '1293')
    advanced = args.output / 'advanced.raw'
    editor.save_as(advanced)
    data = editor.view()
    assert data['bit'] == 12000000 and data['xdata'] == 12345 and data['level'] == 1000
    assert data['tech'][0] == 10000 and data['upcnt'][0] == 100000 and data['device'][0] == 1293
    verify(advanced)
    editor.open(advanced)
    assert editor.view() == data
    record('Advanced values above gameplay caps and raw item IDs survive save/reopen')
    editor.command('click', 'Story')
    editor.command('check', 'Apocalymon (W1)', 'on')
    editor.command('click', 'Save')
    editor.saved()
    assert editor.view()['story_flags'][67] == 1 and editor.view()['story_flags'][708] == 1
    editor.command('check', 'Apocalymon (W1)', 'off')
    editor.command('click', 'Save')
    editor.saved()
    assert editor.view()['story_flags'][67] == 0 and editor.view()['story_flags'][708] == 0
    editor.command('click', 'Character')
    record('manual story checkbox sets and clears its corrected retail mirror')

    invalid = args.output / 'invalid.raw'
    invalid.write_bytes(b'not a save')
    valid = editor.view()
    editor.command('dialog', 'open', str(invalid))
    editor.command('click', 'Open')
    editor.wait(lambda: editor.eval("!!document.querySelector('[role=alert]')"))
    assert editor.view() == valid
    editor.command('click', 'Dismiss')
    editor.command('dialog', 'save', str(args.output / 'missing-folder' / 'save.raw'))
    editor.command('click', 'Save As')
    editor.wait(lambda: editor.eval("!!document.querySelector('[role=alert]')"))
    assert editor.view() == valid
    editor.command('click', 'Dismiss')
    record('invalid input and failed writes report errors without replacing the current document')

    editor.command('fill', 'BIT', '123')
    editor.command('close')
    editor.wait(lambda: editor.eval("!!document.querySelector('[role=dialog]')"))
    editor.command('click', 'Cancel')
    assert editor.eval("[...document.querySelectorAll('input')].find(e=>[...e.labels].some(l=>l.textContent==='BIT')).value") == '123'
    editor.command('click', 'Save')
    editor.saved()
    record('native window-close guard cancels closing and preserves unsaved work')

    report = {'passed': checks, 'editorSchema': info['schema_version']}
    (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(f'Report: {args.output / "report.json"}', flush=True)


if __name__ == '__main__':
    main()
