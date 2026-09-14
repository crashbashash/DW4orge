#!/usr/bin/env bash
#
# Build wrapper for tauri-action.
#
# It runs the requested build and then patches the AppImage so it starts on
# Wayland hosts (see src-tauri/appimage/wayland-compat.sh). release.yml points
# tauri-action at this through the `tauriScript` input, so it runs on every
# platform — only Linux has an AppImage to patch, hence the guard.
#
#     npm run build:appimage            # local equivalent
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

npm exec -- tauri "$@"

if [ "$(uname -s)" = "Linux" ] && [ -d src-tauri/target/release/bundle/appimage ]; then
  bash src-tauri/scripts/patch-appimage.sh
fi
