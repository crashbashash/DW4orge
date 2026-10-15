import { useState } from 'react';
import { ConfirmDialog } from './ConfirmDialog';
import { dirty } from '../app/selectors';
import type { Difficulty } from '../bindings';
import { useEditor } from '../app/EditorProvider';
import { formatNumber } from '../lib/num';
import { REGION_LABELS } from './RegionDialog';
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
  const [pendingBox, setPendingBox] = useState<string | null>(null);
  const { state, setDifficulty, selectXboxSlot } = useEditor();
  const { session } = state;
  if (!session) return null;
  const { view } = session;

  const choice = state.difficulty === 'auto' ? 'auto' : state.difficulty.fixed;

  return (
    <section className={session.xbox_slots?.length ? "summary summary-xbox" : "summary"} aria-label="Save summary">
      {session.xbox_slots?.length ? <>
        <SelectField className="xbox-choice" label="Character box" value={session.xbox_selection ?? ''}
          options={session.xbox_slots.map(slot => ({ value: slot.id, label: slot.label }))}
          disabled={state.status === 'busy'}
          onChange={selection => {
            if (selection === session.xbox_selection) return;
            if (dirty(state)) setPendingBox(selection);
            else void selectXboxSlot(selection);
          }} />
        <ConfirmDialog isOpen={pendingBox !== null} title="Discard unsaved changes?"
          message="Switching character boxes will discard your unsaved changes." confirmLabel="Discard"
          onCancel={() => setPendingBox(null)} onConfirm={() => {
            const selection = pendingBox; setPendingBox(null);
            if (selection) void selectXboxSlot(selection);
          }} />
      </> : null}
      <Field
        label="Region"
        value={session.region ? REGION_LABELS[session.region] : session.source === 'gamecube' ? 'GameCube GCI' : session.source === 'xbox' || session.source === 'xbox_mu' ? 'Xbox (USA / Japan)' : 'Unknown (raw save)'}
      />
      <Field label="Species" value={view.species} />
      <Field label="Name" value={view.name} />
      <Field label="Level" value={String(view.level)} />
      <Field label="BIT" value={formatNumber(view.bit)} />
      <Field label="X-Data" value={formatNumber(view.xdata)} />
      <Field label="Junk tier" value={String(view.junk_tier)} />
      <Field
        label={session.source === 'xbox' || session.source === 'xbox_mu' ? 'Signature' : 'Checksum'}
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
