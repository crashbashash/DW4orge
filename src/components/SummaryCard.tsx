import type { Difficulty } from '../bindings';
import { useEditor } from '../app/EditorProvider';
import { formatNumber } from '../lib/num';
import { SelectField } from './SelectField';

const DIFFICULTIES: Difficulty[] = ['Normal', 'Hard', 'VeryHard'];

const DIFFICULTY_LABELS: Record<Difficulty, string> = {
  Normal: 'Normal',
  Hard: 'Hard',
  VeryHard: 'Very Hard',
};

const DIFFICULTY_OPTIONS = [
  { value: 'auto', label: 'Auto (detected)' },
  ...DIFFICULTIES.map((difficulty) => ({
    value: difficulty as string,
    label: DIFFICULTY_LABELS[difficulty],
  })),
];

export function SummaryCard() {
  const { state, setDifficulty } = useEditor();
  const { session } = state;
  if (!session) return null;
  const { view } = session;

  const choice = state.difficulty === 'auto' ? 'auto' : state.difficulty.fixed;

  return (
    <section className="summary" aria-label="Save summary">
      <Field label="Species" value={view.species} />
      <Field label="Name" value={view.name} />
      <Field label="Level" value={String(view.level)} />
      <Field label="BIT" value={formatNumber(view.bit)} />
      <Field label="X-Data" value={formatNumber(view.xdata)} />
      <Field label="Junk tier" value={String(view.junk_tier)} />
      <Field
        label="Checksum"
        value={view.checksum_ok ? 'OK' : 'Mismatch'}
        tone={view.checksum_ok ? 'ok' : 'danger'}
      />
      <SelectField
        label={`Difficulty (detected: ${DIFFICULTY_LABELS[view.difficulty]})`}
        value={choice}
        options={DIFFICULTY_OPTIONS}
        onChange={(value) => {
          setDifficulty(value === 'auto' ? 'auto' : { fixed: value as Difficulty });
        }}
      />
    </section>
  );
}

function Field({ label, value, tone }: { label: string; value: string; tone?: 'ok' | 'danger' }) {
  return (
    <div className="field">
      <span className="label">{label}</span>
      <span className={tone ? `value ${tone}` : 'value'}>{value}</span>
    </div>
  );
}
