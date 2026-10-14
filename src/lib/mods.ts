import type { EditSet } from '../bindings';
import { EMPTY, buildItemId } from './items';

/** A draft with the mod sockets resolved to device indices (`ResolvedMods`). */
export type ResolvedMods = {
  device: number[];
  wmods: (number | null)[];
  amods: (number | null)[];
  /** Sockets whose chip could not be placed, as `equip.wmod3`-style paths. */
  failed: string[];
};

/**
 * Reuse a slot already holding the chip, else fill the first empty slot, else
 * fail (`document::find_or_add_mod`). Mutates `device`.
 */
export function findOrAddMod(device: number[], baseId: number): number | null {
  for (let i = 0; i < device.length; i += 1) {
    const slot = device[i];
    if (slot !== undefined && slot !== EMPTY && (slot & 0xffff) === baseId) return i;
  }
  for (let i = 0; i < device.length; i += 1) {
    if (device[i] === EMPTY) {
      device[i] = buildItemId(baseId, 0, 0);
      return i;
    }
  }
  return null;
}

/**
 * Resolve every mod socket against a copy of the device folder.
 *
 * A preview of what Rust does at apply time, so the Items tab can show a chip
 * the socket will add and the Equipment tab can flag a socket that cannot fit.
 */
export function resolveMods(draft: EditSet): ResolvedMods {
  const device = [...draft.device];
  const wmods: (number | null)[] = draft.wmods.map(() => null);
  const amods: (number | null)[] = draft.amods.map(() => null);
  const failed: string[] = [];

  draft.wmods.forEach((baseId, i) => {
    if (baseId === null) return;
    const index = findOrAddMod(device, baseId);
    if (index === null) failed.push(`equip.wmod${i}`);
    else wmods[i] = index;
  });

  draft.amods.forEach((baseId, i) => {
    if (baseId === null) return;
    const index = findOrAddMod(device, baseId);
    if (index === null) failed.push(`equip.amod${i}`);
    else amods[i] = index;
  });

  return { device, wmods, amods, failed };
}
