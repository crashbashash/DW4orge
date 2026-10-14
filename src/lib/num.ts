/**
 * Parse an integer the way the Python editor's `_parse_int` accepts one:
 * decimal, `0x` hex, `_` separators and surrounding whitespace. Anything with
 * trailing junk is rejected rather than silently truncated.
 */
export function parseIntLoose(text: string): number | null {
  const cleaned = text.trim().replaceAll('_', '');
  if (/^-?0x[0-9a-f]+$/i.test(cleaned)) return Number.parseInt(cleaned, 16);
  if (/^-?\d+$/.test(cleaned)) return Number.parseInt(cleaned, 10);
  return null;
}

const GROUPED = new Intl.NumberFormat('en-US');

/** Group thousands, as the Python editor's `{:,}` does. */
export function formatNumber(value: number): string {
  return GROUPED.format(value);
}
