import { describe, expect, it } from 'vitest';
import type { Item } from '../bindings';
import { EMPTY } from './items';
import { ITEM_BUCKETS, bucketForCategory, bucketForId, itemsForBucket, itemsForCategories } from './itemBuckets';

const catalogue: Item[] = [
  { base_id: 0, name: 'Battle Hawk', category: 'weapon', grade: 0, note: null },
  { base_id: 1281, name: 'Omega Blade', category: 'styled', grade: null, note: null },
  { base_id: 4096, name: 'Brave Core', category: 'core', grade: null, note: null },
  { base_id: 8192, name: 'Achilles Board', category: 'board', grade: null, note: null },
  { base_id: 12288, name: 'Wisdom Chip α', category: 'mod', grade: null, note: null },
];

describe('buckets', () => {
  it('mirrors the Python _bucket_for_cat mapping', () => {
    expect(ITEM_BUCKETS.map((bucket) => bucket.label)).toEqual([
      'Weapon',
      'Armor',
      'Sub slot',
      'Mod chip',
    ]);
    // Graded and styled weapons share the Weapon bucket.
    expect(bucketForCategory('weapon')?.id).toBe('weapon');
    expect(bucketForCategory('styled')?.id).toBe('weapon');
    expect(bucketForCategory('core')?.id).toBe('armor');
    expect(bucketForCategory('board')?.id).toBe('sub');
    expect(bucketForCategory('mod')?.id).toBe('mod');
    expect(bucketForCategory('modequipped')).toBeNull();
    expect(bucketForCategory({ unknown: 0x44 })).toBeNull();
  });

  it('resolves a stored id to its bucket', () => {
    expect(bucketForId(0, catalogue)?.id).toBe('weapon');
    expect(bucketForId(1281, catalogue)?.id).toBe('weapon');
    expect(bucketForId(4096, catalogue)?.id).toBe('armor');
    expect(bucketForId(EMPTY, catalogue)).toBeNull();
    expect(bucketForId(0x7777, catalogue)).toBeNull();
  });
});

describe('bucket item lists', () => {
  it('returns nothing for a null bucket', () => {
    // A disabled picker must not be handed the whole catalogue.
    expect(itemsForBucket(null, catalogue)).toEqual([]);
  });

  it('filters to the bucket and reuses one list per catalogue', () => {
    const weapon = itemsForBucket(ITEM_BUCKETS[0], catalogue);
    expect(weapon.map((item) => item.name)).toEqual(['Battle Hawk', 'Omega Blade']);
    // Identity, not just equality: every row shares the same array.
    expect(itemsForBucket(ITEM_BUCKETS[0], catalogue)).toBe(weapon);
  });

  it('returns nothing for an empty category list', () => {
    expect(itemsForCategories([], catalogue)).toEqual([]);
  });
});
