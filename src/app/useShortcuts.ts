import { useEffect, useRef } from 'react';

export type ShortcutHandlers = {
  open(): void;
  save(): void;
  saveAs(): void;
  undo(): void;
  redo(): void;
};

/**
 * The spec §7.2 keyboard map: Ctrl/Cmd+O, Ctrl/Cmd+S, Ctrl/Cmd+Shift+S,
 * Ctrl/Cmd+Z and Ctrl/Cmd+Shift+Z (Ctrl+Y also redoes).
 */
export function useShortcuts(handlers: ShortcutHandlers): void {
  // Keep the latest handlers without re-subscribing on every render.
  const latest = useRef(handlers);
  useEffect(() => {
    latest.current = handlers;
  });

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const mod = event.ctrlKey || event.metaKey;
      if (!mod) return;
      const key = event.key.toLowerCase();

      if (key === 'o') {
        event.preventDefault();
        latest.current.open();
      } else if (key === 's' && event.shiftKey) {
        event.preventDefault();
        latest.current.saveAs();
      } else if (key === 's') {
        event.preventDefault();
        latest.current.save();
      } else if (key === 'z' && event.shiftKey) {
        event.preventDefault();
        latest.current.redo();
      } else if (key === 'z') {
        event.preventDefault();
        latest.current.undo();
      } else if (key === 'y') {
        event.preventDefault();
        latest.current.redo();
      }
    }

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);
}
