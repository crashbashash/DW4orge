import type { NamedCap, PowerupLimit } from '../bindings';

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
