import { beforeEach, describe, expect, it, vi } from 'vitest';
import { applyTheme, resolveTheme } from './theme';

const h = vi.hoisted(() => ({
  setTheme: vi.fn(),
  state: { tauri: false },
}));

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => h.state.tauri }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ setTheme: h.setTheme }),
}));

beforeEach(() => {
  h.state.tauri = false;
  h.setTheme.mockReset();
});

describe('resolveTheme', () => {
  it('prefers a stored choice', () => {
    expect(resolveTheme('light', true)).toBe('light');
    expect(resolveTheme('dark', false)).toBe('dark');
  });

  it('falls back to the OS preference', () => {
    expect(resolveTheme(null, true)).toBe('dark');
    expect(resolveTheme(null, false)).toBe('light');
  });

  it('ignores a junk stored value', () => {
    expect(resolveTheme('chartreuse', true)).toBe('dark');
  });
});

describe('applyTheme', () => {
  it('sets the DOM theme and colour scheme', () => {
    const root = document.createElement('div');
    applyTheme('dark', root);
    expect(root.dataset.theme).toBe('dark');
    expect(root.style.colorScheme).toBe('dark');
  });

  it('leaves the native window alone outside the Tauri shell', async () => {
    applyTheme('dark', document.createElement('div'));
    await Promise.resolve();
    expect(h.setTheme).not.toHaveBeenCalled();
  });

  it('tells the native window the theme inside the Tauri shell', async () => {
    h.state.tauri = true;
    applyTheme('dark', document.createElement('div'));
    await vi.waitFor(() => expect(h.setTheme).toHaveBeenCalledWith('dark'));
  });
});
