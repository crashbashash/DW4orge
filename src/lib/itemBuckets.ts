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
