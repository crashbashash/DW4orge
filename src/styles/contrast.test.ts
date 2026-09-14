// @vitest-environment node
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

/**
 * WCAG contrast guard for the palette.
 *
 * Reads the real tokens so a palette edit cannot silently drop below the
 * thresholds. Text needs 4.5:1; a large or non-text element needs 3:1.
 */
const css = readFileSync(new URL('./tokens.css', import.meta.url), 'utf8');

function varsFor(selectorFragment: string): Record<string, string> {
  for (const match of css.matchAll(/([^{}]+)\{([^}]*)\}/g)) {
    if (!match[1].includes(selectorFragment)) continue;
    const vars: Record<string, string> = {};
    for (const decl of match[2].matchAll(/--([\w-]+):\s*([^;]+);/g)) {
      vars[decl[1]] = decl[2].trim();
    }
    return vars;
  }
  throw new Error(`no rule containing ${selectorFragment}`);
}

function channel(value: number): number {
  const c = value / 255;
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function luminance(hex: string): number {
  const h = hex.replace('#', '');
  const r = Number.parseInt(h.slice(0, 2), 16);
  const g = Number.parseInt(h.slice(2, 4), 16);
  const b = Number.parseInt(h.slice(4, 6), 16);
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const PAIRS: readonly { fg: string; bg: string; min: number }[] = [
  { fg: 'text', bg: 'surface', min: 4.5 },
  { fg: 'text', bg: 'surface-raised', min: 4.5 },
  { fg: 'text-muted', bg: 'surface', min: 4.5 },
  { fg: 'text-muted', bg: 'surface-raised', min: 4.5 },
  // The brand is accent-coloured text on the raised surface.
  { fg: 'accent', bg: 'surface-raised', min: 4.5 },
  // Borders, focus rings and other non-text uses.
  { fg: 'accent', bg: 'surface', min: 3 },
  { fg: 'accent-contrast', bg: 'accent', min: 4.5 },
  { fg: 'ok', bg: 'surface-raised', min: 4.5 },
  { fg: 'danger', bg: 'surface-raised', min: 4.5 },
  { fg: 'warning', bg: 'surface-raised', min: 4.5 },
];

const THEMES: readonly { name: string; selector: string }[] = [
  { name: 'dark', selector: "data-theme='dark'" },
  { name: 'light', selector: "data-theme='light'" },
];

for (const { name, selector } of THEMES) {
  describe(`${name} theme`, () => {
    const vars = varsFor(selector);

    for (const { fg, bg, min } of PAIRS) {
      it(`${fg} on ${bg} is at least ${min}:1`, () => {
        expect(vars[fg], `--${fg} is defined`).toBeDefined();
        expect(vars[bg], `--${bg} is defined`).toBeDefined();
        const ratio = contrast(vars[fg], vars[bg]);
        expect(Number(ratio.toFixed(2))).toBeGreaterThanOrEqual(min);
      });
    }
  });
}
