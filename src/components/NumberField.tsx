import { useId, useState } from 'react';
import type { FieldError } from '../bindings';
import { parseIntLoose } from '../lib/num';
import { FieldMessage } from './FieldMessage';

/**
 * An integer input that edits locally and commits on blur/Enter.
 *
 * The displayed text is the local draft while the user is typing, and the
 * store's value otherwise — so undo and a species switch update it without an
 * effect (and a half-typed number never reaches the store).
 */
export function NumberField({
  label,
  value,
  onChange,
  error,
  hint,
  max,
  disabled,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
  error?: FieldError[];
  hint?: string;
  /** Upper bound applied on commit. Normal-mode caps pass one; Advanced does not. */
  max?: number;
  disabled?: boolean;
}) {
  const id = useId();
  const [draft, setDraft] = useState<string | null>(null);
  const text = draft ?? String(value);

  const commit = () => {
    setDraft(null);
    const parsed = parseIntLoose(text);
    if (parsed === null) return;
    const next = max === undefined ? parsed : Math.min(parsed, max);
    if (next === value) return;
    onChange(next);
  };

  return (
    <div className="field">
      <label className="label" htmlFor={id}>
        {label}
      </label>
      <input
        id={id}
        type="text"
        inputMode="numeric"
        value={text}
        disabled={disabled}
        aria-invalid={error && error.length > 0 ? true : undefined}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === 'Enter') commit();
        }}
      />
      {hint ? <span className="hint">{hint}</span> : null}
      <FieldMessage errors={error} />
    </div>
  );
}
