import type { FieldError } from '../bindings';

/** The first validation message for a control, if any. */
export function FieldMessage({ errors }: { errors?: FieldError[] }) {
  if (!errors || errors.length === 0) return null;
  const error = errors[0];
  if (!error) return null;
  return (
    <span
      className={error.severity === 'error' ? 'field-message danger' : 'field-message warning'}
      role="alert"
    >
      {error.message}
    </span>
  );
}
