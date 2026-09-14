import type { Item } from '../bindings';

/** The "no item / no slot" sentinel. Never `0`. */
export const EMPTY = 0xffffffff;

/** Bits of the seed that carry the additive stat bonus; bit 11 is masked off. */
export const SEED_BONUS_MASK = 0x7ff;

/** Grade letters, indexed by the catalogue's `grade`. */
const GRADE_LETTER = ['\u03b1', '\u03b2', '\u03b3', '\u03b4', '\u03b5'];

export type Rarity = 'white' | 'blue' | 'green' | 'yellow' | 'orange' | 'pink';

/** Rarity colours, ascending. */
export const RARITIES: readonly Rarity[] = ['white', 'blue', 'green', 'yellow', 'orange', 'pink'];

/** The inclusive `+N` bonus range each colour spans. */
export const RARITY_RANGES: Record<Rarity, readonly [number, number]> = {
  white: [0x000, 0x000],
  blue: [0x001, 0x00f],
  green: [0x010, 0x07f],
  yellow: [0x080, 0x1ff],
  orange: [0x200, 0x3ff],
  pink: [0x400, 0x7ff],
};

/** The rarity colour a seed falls into. Bit 11 never counts. */
export function colorForSeed(seed: number): Rarity {
  const effective = seed & SEED_BONUS_MASK;
  for (let i = RARITIES.length - 1; i >= 0; i -= 1) {
    const rarity = RARITIES[i] as Rarity;
    if (effective >= RARITY_RANGES[rarity][0]) return rarity;
  }
  return 'white';
}

/** Pull a `+N` bonus into `rarity`'s inclusive range. */
export function clampBonusToRarity(bonus: number, rarity: Rarity): number {
  const [lo, hi] = RARITY_RANGES[rarity];
  return Math.min(Math.max(bonus, lo), hi);
}

/** A human label for a seed: the colour plus the raw `+N` bonus. */
export function rarityName(seed: number): string {
  return `${colorForSeed(seed)} (+${seed & SEED_BONUS_MASK})`;
}

/**
 * Pack a device-folder u32: `base_id | ((seed << 4 | mods) << 16)`.
 *
 * Seed is masked to 12 bits and mods to 4, matching `item::build_item_id`.
 */
export function buildItemId(baseId: number, seed: number, mods: number): number {
  const instance = ((seed & 0x0fff) << 4) | (mods & 0x0f);
  return (baseId | (instance << 16)) >>> 0;
}

/** Unpack a stored u32 into its base id, 12-bit seed and 4-bit mod count. */
export function splitItemId(full: number): { baseId: number; seed: number; mods: number } {
  return {
    baseId: full & 0xffff,
    seed: (full >>> 20) & 0x0fff,
    mods: (full >>> 16) & 0x0f,
  };
}

/** The mod-chip base id a socket index points at, or null if empty/out of range. */
export function socketBaseId(device: readonly number[], index: number): number | null {
  if (index === EMPTY || index < 0 || index >= device.length) return null;
  const slot = device[index];
  if (slot === undefined || slot === EMPTY) return null;
  return slot & 0xffff;
}

/** The grade letter for a catalogue grade, or the empty string. */
export function gradeLetter(grade: number | null): string {
  if (grade === null) return '';
  return GRADE_LETTER[grade] ?? '';
}

/** A catalogue item's display name, with its grade letter where it has one. */
export function itemLabel(item: Item): string {
  const letter = gradeLetter(item.grade);
  return letter ? `${item.name} ${letter}` : item.name;
}

/** The Rust `catalogue::category_label` for a category byte. */
export function categoryLabel(byte: number): string {
  switch (byte) {
    case 0x00:
      return 'Weapon';
    case 0x05:
      return 'Weapon (styled)';
    case 0x10:
      return 'Core (armor)';
    case 0x20:
      return 'Board (sub)';
    case 0x30:
      return 'Mod (chip)';
    default:
      return `cat 0x${byte.toString(16).padStart(2, '0')}`;
  }
}

/** Render a stored item id for display, valid or not (`catalogue::describe_item_id`). */
export function describeItemId(full: number, catalogue: readonly Item[]): string {
  if (full === EMPTY) return '(empty)';

  const { baseId, seed, mods } = splitItemId(full);
  const suffix =
    seed === 0 && mods === 0 ? '' : ` (${colorForSeed(seed)} (+${seed & SEED_BONUS_MASK}), ${mods} mods)`;

  const item = catalogue.find((entry) => entry.base_id === baseId);
  if (item) return `${itemLabel(item)}${suffix}`;

  const category = (baseId >>> 8) & 0xff;
  return `[invalid] ${categoryLabel(category)} 0x${full.toString(16).padStart(8, '0')}${suffix}`;
}
