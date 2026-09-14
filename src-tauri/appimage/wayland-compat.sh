# --- DW4orge Wayland compatibility -------------------------------------------
#
# Appended to the generated apprun hook by scripts/patch-appimage.sh, because
# Tauri runs linuxdeploy last (it writes this hook and then packs the AppImage),
# so neither `bundle.files` nor a separate hook file would take effect.
#
# Why this exists:
#
# 1. The AppImage bundles the *build runner's* Wayland client libraries
#    (linuxdeploy-plugin-gtk pulls in libwayland-client/cursor/egl/server). On a
#    host whose Wayland is newer — Arch/CachyOS, for instance — the bundled copy
#    wins on the binary's RUNPATH, cannot talk to the running compositor, and
#    WebKitGTK's EGL init aborts with:
#
#        Could not create default EGL display: EGL_BAD_PARAMETER. Aborting...
#
#    Preloading the host's own copy makes the dynamic loader satisfy every
#    later request for that SONAME from it instead. Distro paths are tried in
#    turn; on a host with none of them this does nothing.
#
# 2. The GTK hook above pins GDK_BACKEND=x11, sending every Wayland session
#    through XWayland. Once the client library is the host's, GTK can use
#    Wayland directly, so drop that pin when the session offers Wayland.
#    DW4ORGE_GDK_BACKEND overrides the choice either way, e.g.
#    `DW4ORGE_GDK_BACKEND=x11 ./DW4orge_*.AppImage`.
#
# Guarded throughout: AppRun sources this with `set -e`, so nothing here may
# return non-zero.

export DESKTOPINTEGRATION=1

if [ -z "${DW4ORGE_WAYLAND_PRELOAD_DONE:-}" ]; then
  for lib in \
    /usr/lib/libwayland-client.so.0 \
    /usr/lib64/libwayland-client.so.0 \
    /usr/lib/x86_64-linux-gnu/libwayland-client.so.0 \
    /usr/lib/aarch64-linux-gnu/libwayland-client.so.0 \
    /usr/lib/arm-linux-gnueabihf/libwayland-client.so.0
  do
    if [ -f "$lib" ]; then
      if [ -n "${LD_PRELOAD:-}" ]; then
        export LD_PRELOAD="$lib:$LD_PRELOAD"
      else
        export LD_PRELOAD="$lib"
      fi
      export DW4ORGE_WAYLAND_PRELOAD_DONE=1
      break
    fi
  done
fi

if [ -n "${DW4ORGE_GDK_BACKEND:-}" ]; then
  export GDK_BACKEND="$DW4ORGE_GDK_BACKEND"
elif [ -n "${WAYLAND_DISPLAY:-}" ] && [ "${GDK_BACKEND:-}" = "x11" ]; then
  unset GDK_BACKEND
fi
