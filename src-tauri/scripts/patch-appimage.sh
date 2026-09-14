#!/usr/bin/env bash
#
# Append the Wayland compatibility hook to a built AppImage and repack it.
#
# Tauri runs linuxdeploy last, and linuxdeploy both writes the apprun hook and
# packs the AppImage, so the hook has to be edited afterwards and the AppImage
# rebuilt around it. `tauri-build.sh` calls this automatically; run it twice and
# it is a no-op.
#
#     tauri build --bundles appimage
#     bash src-tauri/scripts/patch-appimage.sh
#
# See src-tauri/appimage/wayland-compat.sh for why the hook exists.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
bundle_dir="$repo_root/src-tauri/target/release/bundle/appimage"
snippet="$repo_root/src-tauri/appimage/wayland-compat.sh"
marker="DW4orge Wayland compatibility"

if [ ! -d "$bundle_dir" ]; then
  echo "patch-appimage: no AppImage bundle directory at $bundle_dir" >&2
  echo "patch-appimage: build one first: tauri build --bundles appimage" >&2
  exit 1
fi

shopt -s nullglob
app_dirs=("$bundle_dir"/*.AppDir)
app_images=("$bundle_dir"/*.AppImage)
shopt -u nullglob

if [ "${#app_dirs[@]}" -ne 1 ]; then
  echo "patch-appimage: expected exactly one AppDir, found ${#app_dirs[@]}" >&2
  exit 1
fi
if [ "${#app_images[@]}" -ne 1 ]; then
  echo "patch-appimage: expected exactly one AppImage, found ${#app_images[@]}" >&2
  exit 1
fi

app_dir="${app_dirs[0]}"
app_image="${app_images[0]}"
hook="$app_dir/apprun-hooks/linuxdeploy-plugin-gtk.sh"

if [ ! -f "$hook" ]; then
  echo "patch-appimage: no apprun hook at $hook" >&2
  exit 1
fi

if grep -q "$marker" "$hook"; then
  echo "patch-appimage: $hook is already patched"
  exit 0
fi

cat "$snippet" >> "$hook"
echo "patch-appimage: appended the Wayland hook to ${hook#"$repo_root"/}"

# Repack with the tool tauri cached while bundling.
case "$(uname -m)" in
  x86_64) appimage_arch=x86_64 ;;
  aarch64 | arm64) appimage_arch=aarch64 ;;
  *)
    echo "patch-appimage: unsupported architecture $(uname -m)" >&2
    exit 1
    ;;
esac

plugin="${XDG_CACHE_HOME:-$HOME/.cache}/tauri/linuxdeploy-plugin-appimage.AppImage"
if [ ! -f "$plugin" ]; then
  echo "patch-appimage: missing $plugin (tauri caches it during a build)" >&2
  exit 1
fi

# --appimage-extract-and-run: CI runners and containers have no FUSE.
APPIMAGE_EXTRACT_AND_RUN=1 ARCH="$appimage_arch" OUTPUT="$app_image" \
  "$plugin" --appdir "$app_dir"
echo "patch-appimage: repacked ${app_image#"$repo_root"/}"
