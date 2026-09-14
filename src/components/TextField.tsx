import { useId } from 'react';
import type { FieldError } from '../bindings';
import { FieldMessage } from './FieldMessage';

export function TextField({
  label,
  value,
  onChange,
  maxLength,
  error,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  maxLength?: number;
  error?: FieldError[];
}) {
  const id = useId();
  return (
    <div className="field">
      <label className="label" htmlFor={id}>
        {label}
      </label>
      <input
        id={id}
        type="text"
        value={value}
        maxLength={maxLength}
        aria-invalid={error && error.length > 0 ? true : undefined}
        onChange={(event) => onChange(event.target.value)}
      />
      <FieldMessage errors={error} />
    </div>
  );
}
