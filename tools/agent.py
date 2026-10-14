#!/usr/bin/env python3
"""Drive the real desktop UI through the opt-in agent-control build."""
import argparse
import json
import os
from pathlib import Path
import secrets
import subprocess
import time
import urllib.error
import urllib.request


def evaluate(session, script):
    info = json.loads(Path(session).read_text())
    request = urllib.request.Request(
        f"http://127.0.0.1:{info['port']}/eval",
        data=json.dumps({"script": script}).encode(),
        headers={"Authorization": f"Bearer {info['token']}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=15) as response:
        result = json.load(response)
    if not result["ok"]:
        raise RuntimeError(result["error"])
    return result.get("value")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--session", required=True, help="private session JSON outside the repository")
    commands = parser.add_subparsers(dest="command", required=True)
    start = commands.add_parser("start")
    start.add_argument("--binary", required=True)
    start.add_argument("--port", type=int, default=17365)
    commands.add_parser("snapshot")
    commands.add_parser("close", help="request native close; respects unsaved-work guard")
    screenshot = commands.add_parser("screenshot")
    screenshot.add_argument("path")
    fill = commands.add_parser("fill")
    fill.add_argument("label")
    fill.add_argument("value")
    check = commands.add_parser("check", help="set a labelled checkbox")
    check.add_argument("label")
    check.add_argument("value", choices=["on", "off"])
    select = commands.add_parser("select")
    select.add_argument("label")
    select.add_argument("option")
    click = commands.add_parser("click")
    click.add_argument("text")
    dialog = commands.add_parser("dialog", help="queue a native file-dialog result")
    dialog.add_argument("kind", choices=["open", "save"])
    dialog.add_argument("path", nargs="?", help="omit to cancel")
    script = commands.add_parser("eval")
    script.add_argument("script")
    args = parser.parse_args()
    if args.command == "start":
        path = Path(args.session)
        resolved = path.resolve()
        if any((parent / '.git').exists() for parent in resolved.parents):
            parser.error("session file must be outside any Git repository")
        # Never overwrite another agent's credentials/session.
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        token = secrets.token_urlsafe(32)
        env = dict(os.environ, DW4_AGENT_TOKEN=token, DW4_AGENT_PORT=str(args.port))
        try:
            log_fd = os.open(str(path) + ".log", os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
            os.chmod(str(path) + ".log", 0o600)
            with os.fdopen(log_fd, "wb") as log:
                child = subprocess.Popen([str(Path(args.binary).resolve())], env=env,
                                         stdout=log, stderr=log, start_new_session=True)
            with os.fdopen(fd, "w") as session:
                json.dump({"port": args.port, "token": token, "pid": child.pid}, session)
            deadline = time.monotonic() + 20
            while True:
                if child.poll() is not None:
                    raise RuntimeError("editor exited before agent controls became ready; inspect session log")
                try:
                    ready = evaluate(path, "typeof window.__TAURI_INTERNALS__?.invoke === 'function'")
                    if ready: break
                except (urllib.error.URLError, RuntimeError, TimeoutError):
                    if time.monotonic() >= deadline:
                        child.terminate()
                        raise RuntimeError("agent startup timed out; inspect session log")
                time.sleep(0.1)
            print(json.dumps({"pid": child.pid, "session": str(path)}))
        except Exception:
            if 'child' in locals() and child.poll() is None:
                child.terminate()
            path.unlink(missing_ok=True)
            raise
        return
    if args.command == "screenshot":
        windows = subprocess.check_output(["xdotool", "search", "--onlyvisible", "--name", "^DW4orge$"], text=True).splitlines()
        if len(windows) != 1:
            raise RuntimeError("expected exactly one visible app window")
        subprocess.run(["import", "-window", windows[0], args.path], check=True)
        print(json.dumps({"screenshot": args.path}))
        return
    if args.command == "check":
        script = """(async () => {
          const root=document.querySelector('[role=dialog]') || document;
          const inputs=[...root.querySelectorAll('input[type=checkbox]')].filter(e=>[...e.labels].some(l=>l.textContent.trim().toLowerCase()===LABEL.toLowerCase()));
          if(inputs.length!==1) throw Error('expected one labelled checkbox');
          if(inputs[0].checked!==CHECKED) inputs[0].click();
          await new Promise(requestAnimationFrame);return true;
        })()""".replace("LABEL",json.dumps(args.label)).replace("CHECKED",json.dumps(args.value=="on"))
    elif args.command == "close":
        script = "window.__TAURI_INTERNALS__.invoke('agent_close')"
    elif args.command == "select":
        evaluate(args.session, """(() => {
          const root = document.querySelector('[role=dialog]') || document;
          const buttons = [...root.querySelectorAll('button')].filter(e => (e.getAttribute('aria-labelledby') || '').split(' ').some(id => document.getElementById(id)?.textContent.trim().toLowerCase() === LABEL.toLowerCase()));
          if (buttons.length !== 1) throw Error('expected one select labelled ' + LABEL);
          buttons[0].click(); return true;
        })()""".replace("LABEL", json.dumps(args.label)))
        script = """(() => {
          const options = [...document.querySelectorAll('[role=option]')].filter(e => e.innerText.trim() === OPTION);
          if (options.length !== 1) throw Error('expected one option ' + OPTION);
          options[0].click(); return true;
        })()""".replace("OPTION", json.dumps(args.option))
    elif args.command == "dialog":
        script = "window.__TAURI_INTERNALS__.invoke('agent_queue_dialog', " + json.dumps({"kind": args.kind, "path": args.path}) + ")"
    elif args.command == "snapshot":
        script = """({text: document.body.innerText, inputs: [...document.querySelectorAll('input')].map(e =>
          ({label: e.getAttribute('aria-label') || [...(e.labels || [])].map(l=>l.textContent).join(' '), type: e.type, value: e.value, checked: e.checked})),
          buttons: [...document.querySelectorAll('button')].map(e => ({text:e.innerText, disabled:e.disabled}))})"""
    elif args.command == "fill":
        script = """(async () => {
          const root = document.querySelector('[role=dialog]') || document;
          const matches = [...root.querySelectorAll('input')].filter(e => (e.getAttribute('aria-label') || '').toLowerCase() === LABEL.toLowerCase() || [...(e.labels || [])].some(l=>l.textContent.trim().toLowerCase() === LABEL.toLowerCase()));
          if (matches.length !== 1) throw Error('expected one matching input');
          const e = matches[0];
          if (!e) throw Error('input missing');
          e.focus();
          Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(e, VALUE);
          e.dispatchEvent(new Event('input', {bubbles:true}));
          e.dispatchEvent(new Event('change', {bubbles:true}));
          await new Promise(requestAnimationFrame);
          e.blur();
          await new Promise(requestAnimationFrame);
          return true;
        })()""".replace("LABEL", json.dumps(args.label)).replace("VALUE", json.dumps(args.value))
    elif args.command == "click":
        script = """(() => {
          const root = document.querySelector('[role=dialog]') || document;
          const matches = [...root.querySelectorAll('button')].filter(e => e.innerText.trim() === TEXT);
          if (matches.length !== 1) throw Error('expected one matching button');
          if (matches[0].disabled) throw Error('button disabled');
          matches[0].click(); return true;
        })()""".replace("TEXT", json.dumps(args.text))
    else:
        script = args.script
    print(json.dumps(evaluate(args.session, script), indent=2))


if __name__ == "__main__":
    main()
