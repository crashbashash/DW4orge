import type { Category, Item } from '../bindings';
import { EMPTY } from './items';

export type BucketId = 'weapon' | 'armor' | 'sub' | 'mod';

export type ItemBucket = { id: BucketId; label: string; categories: readonly Category[] };

/**
 * The four item buckets, mirroring the Python editor's `ITEM_TYPE_BUCKETS` and
 * `_bucket_for_cat`: weapon covers both graded and styled weapons.
 */
export const ITEM_BUCKETS: readonly ItemBucket[] = [
  { id: 'weapon', label: 'Weapon', categories: ['weapon', 'styled'] },
  { id: 'armor', label: 'Armor', categories: ['core'] },
  { id: 'sub', label: 'Sub slot', categories: ['board'] },
  { id: 'mod', label: 'Mod chip', categories: ['mod'] },
];

/** The bucket a catalogue category belongs to, or null for an unknown one. */
export function bucketForCategory(category: Category): ItemBucket | null {
  return (
    ITEM_BUCKETS.find((bucket) => bucket.categories.some((wanted) => wanted === category)) ?? null
  );
}

/** The bucket a stored item id belongs to, or null when empty or unknown. */
export function bucketForId(value: number, catalogue: readonly Item[]): ItemBucket | null {
  if (value === EMPTY) return null;
  const baseId = value & 0xffff;
  const item = catalogue.find((entry) => entry.base_id === baseId);
  return item ? bucketForCategory(item.category) : null;
}

const EMPTY_ITEMS: readonly Item[] = [];

// Filtering the 539-entry catalogue is cheap once but not per row: every row of
// the Items and Bank pages asks for the same bucket, and a RAC collection is
// built from whatever array it is handed. Cache by catalogue identity.
const cache = new WeakMap<readonly Item[], Map<string, readonly Item[]>>();

/** The catalogue entries in any of `categories`, shared per catalogue. */
export function itemsForCategories(
  categories: readonly Category[],
  catalogue: readonly Item[],
): readonly Item[] {
  if (categories.length === 0) return EMPTY_ITEMS;
  const key = categories.map(String).join(',');
  let byKey = cache.get(catalogue);
  if (!byKey) {
    byKey = new Map();
    cache.set(catalogue, byKey);
  }
  const hit = byKey.get(key);
  if (hit) return hit;
  const filtered = catalogue.filter((item) =>
    categories.some((category) => category === item.category),
  );
  byKey.set(key, filtered);
  return filtered;
}

/**
 * The entries in a bucket, or none at all.
 *
 * A null bucket deliberately yields an empty list: the picker is disabled then,
 * and handing it the whole catalogue made each empty row build hundreds of
 * collection nodes for nothing.
 */
export function itemsForBucket(
  bucket: ItemBucket | null,
  catalogue: readonly Item[],
): readonly Item[] {
  return bucket ? itemsForCategories(bucket.categories, catalogue) : EMPTY_ITEMS;
}
