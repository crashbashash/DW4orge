#!/usr/bin/env python3
"""Regenerate the Tauri icon set from the source artwork.

The generated files are committed, so this is only needed when the artwork
changes. It reads ``src-tauri/icons/source.png`` (square, 1024x1024, RGBA) and
writes the sizes the bundler looks for:

    src-tauri/icons/  32x32.png, 128x128.png, 128x128@2x.png, icon.png,
                      icon.ico, icon.icns
    public/           favicon.png  (the browser tab, and the dev server)

Pillow is the only dependency; keep it out of the project's own environment:

    python3 -m venv /tmp/iconvenv
    /tmp/iconvenv/bin/pip install Pillow
    /tmp/iconvenv/bin/python tools/gen_icons.py

Run it by hand when the art changes; commit the result.
"""

from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image  # type: ignore[import-not-found]

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "src-tauri" / "icons"
SOURCE = ICONS / "source.png"
PUBLIC = ROOT / "public"

# Flattened PNGs: file name -> square size.
PNG_SIZES = {
    "32x32.png": 32,
    "128x128.png": 128,
    "128x128@2x.png": 256,
    "icon.png": 512,
}

# The favicon Vite copies into the built app; 128 covers HiDPI tabs.
FAVICON_SIZE = 128

# A Windows .ico carries several sizes; a macOS .icns carries all of them.
ICO_SIZES = (16, 24, 32, 48, 64, 128, 256)
ICNS_SIZES = (16, 32, 64, 128, 256, 512, 1024)


def main() -> int:
    if not SOURCE.is_file():
        print(f"missing source artwork: {SOURCE}", file=sys.stderr)
        return 1

    source = Image.open(SOURCE).convert("RGBA")
    if source.width != source.height:
        print(f"source must be square, got {source.size}", file=sys.stderr)
        return 1
    if source.width < 1024:
        print(f"source should be at least 1024px, got {source.width}", file=sys.stderr)
        return 1

    for name, size in PNG_SIZES.items():
        source.resize((size, size), Image.LANCZOS).save(ICONS / name)
        print(f"wrote {name} ({size}x{size})")

    source.save(ICONS / "icon.ico", format="ICO", sizes=[(s, s) for s in ICO_SIZES])
    print(f"wrote icon.ico ({', '.join(str(s) for s in ICO_SIZES)})")

    frames = [source.resize((s, s), Image.LANCZOS) for s in ICNS_SIZES]
    frames[0].save(ICONS / "icon.icns", format="ICNS", append_images=frames[1:])
    print(f"wrote icon.icns ({', '.join(str(s) for s in ICNS_SIZES)})")

    PUBLIC.mkdir(exist_ok=True)
    source.resize((FAVICON_SIZE, FAVICON_SIZE), Image.LANCZOS).save(PUBLIC / "favicon.png")
    print(f"wrote public/favicon.png ({FAVICON_SIZE}x{FAVICON_SIZE})")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
