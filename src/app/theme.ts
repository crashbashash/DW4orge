import { isTauri } from '@tauri-apps/api/core';

export type Theme = 'light' | 'dark';

const KEY = 'dw4orge.theme';

/** A stored choice wins; otherwise follow the OS. A junk value is ignored. */
export function resolveTheme(stored: string | null, prefersDark: boolean): Theme {
  if (stored === 'light' || stored === 'dark') return stored;
  return prefersDark ? 'dark' : 'light';
}

export function readStoredTheme(): string | null {
  try {
    return window.localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

export function storeTheme(theme: Theme): void {
  try {
    window.localStorage.setItem(KEY, theme);
  } catch {
    /* private mode: the choice simply does not persist */
  }
}

/**
 * Point the native window at the same theme.
 *
 * On Linux, Tauri/tao maps this to `gtk-application-prefer-dark-theme`, which is
 * what tints the GTK client-side title bar it draws on Wayland. Without it a
 * dark app keeps the light default GTK header bar — the "white title bar on
 * KDE Wayland" report. It needs `core:window:allow-set-theme`.
 *
 * A no-op outside the Tauri shell, and best-effort: a title-bar colour is
 * cosmetic, so a refused or absent IPC must never break the editor.
 */
function applyNativeTheme(theme: Theme): void {
  if (!isTauri()) return;
  void import('@tauri-apps/api/window')
    .then(({ getCurrentWindow }) => getCurrentWindow().setTheme(theme))
    .catch(() => undefined);
}

export function applyTheme(theme: Theme, root: HTMLElement): void {
  root.dataset.theme = theme;
  root.style.colorScheme = theme;
  applyNativeTheme(theme);
}

/**
 * The theme to use now: a stored choice, otherwise the OS preference.
 *
 * `matchMedia` is absent in jsdom, so this is safe to call in tests.
 */
export function detectTheme(): Theme {
  const prefersDark =
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-color-scheme: dark)').matches;
  return resolveTheme(readStoredTheme(), prefersDark);
}
