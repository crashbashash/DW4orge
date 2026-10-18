# Desktop UI and native dialogs

DW4orge uses the same neutral dark surfaces, 7px control corners, 13–14px
control/text scale and 12px minimum labels as DigiLab and DigiAtlas, with amber
accents. The light theme remains available. The welcome view offers both opening
an existing save and creating a new one. Narrow windows move the section navigation
above the editor, and let the toolbar and item fields wrap.

On Linux, both native file pickers explicitly receive their parent window and
use the desktop portal. The configured portal supplies the picker UI; on a KDE
portal setup this is the KDE picker. File filters, suggested names and source-format
restrictions are still supplied by the existing frontend backend adapter. Cancelling
returns no path and does not write a save.

The Linux main window hides GTK decorations while GDK reports a tiled state.
When GDK reports an untiled state, decorations are enabled. This follows the
compositor's state notifications rather than identifying individual desktops.
Windows and macOS retain their normal native window controls.

Validation covered the production frontend build, frontend tests, lint, WCAG AA
palette checks, the native shell build, Clippy and agent protocol tests. Live
Wayland checks confirmed that Open and Save As use floating portal dialogs, that
cancelling preserves the editor, and that the GTK title bar disappears while tiled.
Dark and light editor views and a 560px-wide item view were inspected; the narrow
view had no horizontal overflow.

The optional agent build normally substitutes queued file-picker results. For a
manual portal test, use a normal build, or set `window.__DW4_AGENT_CONTROL__ = false`
in the agent-controlled webview before clicking Open or Save As.
