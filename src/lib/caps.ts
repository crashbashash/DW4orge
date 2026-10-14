import type { Mode, NamedCap, PowerupLimit } from '../bindings';
import { formatNumber } from './num';

/** The cap for a validator path (`"bit"`, `"upcnt"`, …), if the table has one. */
export function capFor(caps: readonly NamedCap[], field: string): NamedCap | undefined {
  return caps.find((entry) => entry.field === field);
}

/**
 * The Normal-mode cap for a power-up slot.
 *
 * `dw4ipc::ui_data` emits all eleven slots; the fallback only exists so a short
 * table cannot produce `undefined` in an input's `max`.
 */
export function powerupCap(powerups: readonly PowerupLimit[], slot: number): number {
  return powerups.find((entry) => entry.slot === slot)?.normal_max ?? 9_999;
}

/**
 * The Normal-mode cap for a validator path, or `undefined` in Advanced mode.
 *
 * Sections pass this to `NumberField`'s `max`, so Normal mode cannot accept a
 * value the validator would reject, while Advanced mode leaves the box uncapped
 * and lets the data-type range (checked in Rust) be the only limit.
 */
export function normalMax(
  caps: readonly NamedCap[],
  field: string,
  mode: Mode,
): number | undefined {
  return mode === 'normal' ? capFor(caps, field)?.cap.normal_max : undefined;
}

/** The Normal-mode cap for a power-up slot, or `undefined` in Advanced mode. */
export function normalPowerupMax(
  powerups: readonly PowerupLimit[],
  slot: number,
  mode: Mode,
): number | undefined {
  return mode === 'normal' ? powerupCap(powerups, slot) : undefined;
}

/** The hint shown beside a capped field (`max 9,999`), or nothing if uncapped. */
export function capHint(max: number | undefined): string | undefined {
  return max === undefined ? undefined : `max ${formatNumber(max)}`;
}
