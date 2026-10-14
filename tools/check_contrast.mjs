#!/usr/bin/env node
/**
 * WCAG contrast guard for the palette.
 *
 * Reads the real tokens so a palette edit cannot silently drop below the
 * thresholds: text needs 4.5:1, a large or non-text element 3:1.
 *
 *     node tools/check_contrast.mjs
 *
 * A plain script rather than a Vitest test because it reads a file and renders
 * nothing; it runs as part of the frontend gate and in CI.
 */

import { readFileSync } from 'node:fs';

const cssPath = new URL('../src/styles/tokens.css', import.meta.url);
const css = readFileSync(cssPath, 'utf8');

function varsFor(selectorFragment) {
  for (const match of css.matchAll(/([^{}]+)\{([^}]*)\}/g)) {
    if (!match[1].includes(selectorFragment)) continue;
    const vars = {};
    for (const decl of match[2].matchAll(/--([\w-]+):\s*([^;]+);/g)) {
      vars[decl[1]] = decl[2].trim();
    }
    return vars;
  }
  throw new Error(`no rule containing ${selectorFragment} in ${cssPath.pathname}`);
}

function channel(value) {
  const c = value / 255;
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function luminance(hex) {
  const h = hex.replace('#', '');
  const r = Number.parseInt(h.slice(0, 2), 16);
  const g = Number.parseInt(h.slice(2, 4), 16);
  const b = Number.parseInt(h.slice(4, 6), 16);
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrast(a, b) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const PAIRS = [
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

const THEMES = [
  { name: 'dark', selector: "data-theme='dark'" },
  { name: 'light', selector: "data-theme='light'" },
];

let failures = 0;

for (const { name, selector } of THEMES) {
  const vars = varsFor(selector);
  for (const { fg, bg, min } of PAIRS) {
    if (vars[fg] === undefined || vars[bg] === undefined) {
      console.error(`FAIL ${name}: --${fg} or --${bg} is not defined`);
      failures += 1;
      continue;
    }
    const ratio = contrast(vars[fg], vars[bg]);
    const ok = ratio >= min;
    if (!ok) failures += 1;
    console.log(`${ok ? 'ok  ' : 'FAIL'} ${name.padEnd(5)} ${fg} on ${bg}: ${ratio.toFixed(2)} (min ${min})`);
  }
}

if (failures > 0) {
  console.error(`\n${failures} contrast pair(s) below the WCAG threshold`);
  process.exit(1);
}
console.log('\npalette passes WCAG AA for every checked pair');
