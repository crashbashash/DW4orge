import type { SourceKind } from '../bindings';

/** An uppercase, zero-padded hex id, as the Python editor prints them. */
export function formatHex(value: number, width = 4): string {
  return `0x${value.toString(16).toUpperCase().padStart(width, '0')}`;
}

/** A human label for a container kind. */
export function describeSource(source: SourceKind): string {
  if (source === 'memcard') return 'PS2 memory card';
  if (source === 'gamecube') return 'GameCube GCI save';
  if (source === 'gamecube_card') return 'GameCube memory card';
  return 'Raw save';
}
