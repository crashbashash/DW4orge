import type { FieldError, IpcError } from '../bindings';

const KINDS = ['validation', 'core', 'no_open_document', 'unsupported'] as const;

export function isIpcError(value: unknown): value is IpcError {
  if (typeof value !== 'object' || value === null || !('kind' in value)) return false;
  return (KINDS as readonly string[]).includes((value as { kind: string }).kind);
}

/** The per-field errors of a validation rejection, or null for any other kind. */
export function fieldErrors(value: unknown): FieldError[] | null {
  if (!isIpcError(value) || value.kind !== 'validation') return null;
  return value.fields;
}

/** A status-bar-ready message. Never throws. */
export function describeIpcError(value: unknown): string {
  if (isIpcError(value)) {
    switch (value.kind) {
      case 'validation':
        return value.fields[0]?.message ?? 'the edit was rejected';
      case 'core':
        return value.message;
      case 'no_open_document':
        return 'no save is open';
      case 'unsupported':
        return value.message;
    }
  }
  if (value instanceof Error) return value.message;
  return String(value);
}
