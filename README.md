# DW4orge

A desktop save editor for **Digimon World 4** (PS2, USA, `SLUS_208.36`), written
in Rust with a React frontend. It reads and writes raw 81,920-byte saves and
`.ps2` memory-card images, and ships a headless CLI over the same core library.

The format, the card filesystem and the architecture are documented in
[`docs/save-format.md`](docs/save-format.md). The public API layered on top is
[`docs/dw4ipc-cli-tauri-design.md`](docs/dw4ipc-cli-tauri-design.md), and the
frontend is [`docs/frontend-design.md`](docs/frontend-design.md).

## Layout

```text
crates/dw4core   all game logic: format, catalogue, document, memcard
crates/dw4ipc    the payloads and service layer shared by the CLI and the app
crates/dw4cli    the headless binary (info, dump, new, verify, items)
src-tauri        the Tauri 2 shell (excluded from the root workspace)
src              the React 19 + TypeScript frontend
tools            the fixture generator
```

## Rust

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`src-tauri` needs `libwebkit2gtk-4.1-dev`, `libsoup-3.0-dev`,
`libjavascriptcoregtk-4.1-dev` and `librsvg2-dev`, so it is excluded from the
root workspace and built separately from `src-tauri/` on a host that has them.

## Frontend

```bash
npm install
npm run dev        # http://localhost:5173
```

In a plain browser the app runs against a fixture-backed mock backend (generated
from `dw4ipc`), so no Rust build or webkit is needed to develop the UI; use
**Load sample save** to open the bundled save. Inside the Tauri shell the same
code talks to the real commands.

```bash
npm run typecheck
npm run lint
npm test
npm run build
npm run check:contrast   # palette WCAG ratios, reads src/styles/tokens.css
```

The checked-in TypeScript bindings and mock fixtures are generated from Rust;
regenerate them with:

```bash
cargo run -p dw4ipc --example gen_bindings
cargo run -p dw4ipc --example gen_ui_fixtures
```

The app icons in `src-tauri/icons/` and the browser favicon in `public/` are
generated from the committed source artwork (square, 1024×1024, RGBA):

```bash
python3 -m venv /tmp/iconvenv
/tmp/iconvenv/bin/pip install Pillow
/tmp/iconvenv/bin/python tools/gen_icons.py
```

## Release

`ci.yml` gates every push and pull request. `release.yml` builds the desktop
bundles when a `v*` tag is pushed — `.deb` + `.AppImage`, `.msi` and a universal
`.dmg`, attached to a draft GitHub release:

```bash
git tag v0.1.1 && git push origin v0.1.1
```

The tag must match `version` in `src-tauri/tauri.conf.json`, which is what names
the release and its bundle filenames — not the tag you push. So a release is:
bump that version (and the workspace `Cargo.toml`, `src-tauri/Cargo.toml` and
`package.json`), regenerate the frontend fixtures with
`cargo run -p dw4ipc --example gen_ui_fixtures` (they embed the version), refresh
both lockfiles, then tag the matching `v<version>`.

The bundles are **unsigned**, so macOS Gatekeeper and Windows SmartScreen will
warn on first run.

### Linux

The release workflow produces a `.deb` (Debian/Ubuntu) and an `.AppImage` (any
distro). Neither `.deb` nor `.rpm` installs on an Arch-based system (Arch,
EndeavourOS, CachyOS) — use the AppImage there, or build one locally:

```bash
sudo pacman -S --needed webkit2gtk-4.1 gtk3 librsvg
npm install
npm run build:appimage
```

`build:appimage` is `NO_STRIP=true tauri build --bundles appimage` followed by
`src-tauri/scripts/patch-appimage.sh`. Specify the bundle: the default set also
tries `deb` and `rpm`, which need `dpkg-deb`/`rpmbuild` and fail on Arch.

`NO_STRIP=true` is not optional on a rolling-release distro. linuxdeploy bundles
the build host's libraries and strips them with its own 2024-era `strip`, which
cannot read the `.relr.dyn` section in Arch's newer libraries, so it aborts with
`Strip call failed ... unknown type [0x13] section '.relr.dyn'`
([tauri#13113](https://github.com/tauri-apps/tauri/issues/13113)). Skipping the
strip costs a few MB in the local build; release builds still strip, because the
workflow pins `ubuntu-22.04`, whose libraries predate the problem.

AppImages need FUSE to run; without it, append `--appimage-extract-and-run`.

#### Wayland

AppImages built by linuxdeploy bundle the *build machine's* Wayland client
libraries. On a host with a newer Wayland — Arch, for instance — the bundled
`libwayland-client` cannot talk to the compositor and WebKitGTK aborts at
startup with:

```text
Could not create default EGL display: EGL_BAD_PARAMETER. Aborting...
```

The patch step appends a hook to the AppImage that preloads the host's own
`libwayland-client`, and drops linuxdeploy's `GDK_BACKEND=x11` pin so GTK can use
Wayland directly instead of going through XWayland. It searches the usual distro
paths and does nothing if none is found.

To force XWayland anyway (or to test whether a Wayland problem is ours):

```bash
DW4ORGE_GDK_BACKEND=x11 ./DW4orge_*.AppImage
```

The `.deb` and the raw binary need none of this — they use the system's own
libraries.

## Verifying a save in-game

Whether the game accepts an edited save can only be checked in an emulator or on
hardware. Load the written `.ps2` in PCSX2 and confirm the game lists and loads
it; the automated suite cannot see that failure.
