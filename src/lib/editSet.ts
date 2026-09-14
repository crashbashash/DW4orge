import type { EditSet } from '../bindings';
import { EMPTY } from './items';

/**
 * A draft matching a freshly built save (`builder::SaveSpec::default()`):
 * Dorumon named TST at level 1, everything else empty.
 *
 * The generated bindings declare fixed-length tuples, so the arrays are built
 * by length and asserted in `editSet.test.ts`; a silent shape change in Rust
 * shows up there and in the integration test's key comparison.
 */
export function emptyEditSet(): EditSet {
  const device = Array.from({ length: 30 }, () => EMPTY);
  const bankItems = Array.from({ length: 96 }, () => EMPTY);

  return {
    species: 'Dorumon',
    name: 'TST',
    bit: 0,
    xdata: 0,
    junk: 0,
    level: 1,
    exp: 0,
    tech: [1, 1, 1, 1, 1, 1, 1, 1, 1],
    upcnt: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    // SAFETY: the binding declares `device` as a 30-element tuple; the array is
    // built with exactly length 30 and `editSet.test.ts` pins that length.
    device: device as unknown as EditSet['device'],
    weapons: [EMPTY, EMPTY, EMPTY],
    armor: EMPTY,
    sub: EMPTY,
    wmods: [null, null, null, null, null],
    amods: [null, null, null, null, null],
    story: [],
    difficulty: 'auto',
    bank_bit: 0,
    disks: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    bank_items: bankItems,
  };
}
