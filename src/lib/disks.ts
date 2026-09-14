/**
 * The twelve `DISKFOLDER` types, in UI order (`0x4000..0x400b`).
 *
 * Mirrors `save_editor_gui.DISK_LABELS` and `FLAG_MAP.md` §11. The names are
 * not in any IPC payload — the Rust validator only labels them `Disk {i}` — so
 * they are mirrored here and pinned by the Disks section's test.
 *
 * Slot 11 is `B. Pack` as the game displays it; the Python editor expands it to
 * `Battery Pack`. The game's text wins so the two screens agree.
 */
export const DISK_LABELS: readonly string[] = [
  'HP Disk α',
  'HP Disk β',
  'HP Disk γ',
  'MP Disk α',
  'MP Disk β',
  'MP Disk γ',
  'Cure Disk',
  'Raise Disk',
  'Gate Disk',
  'Recovery',
  'B. Pack',
  'Key Chain',
];
