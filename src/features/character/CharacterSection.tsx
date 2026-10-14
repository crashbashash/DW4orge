import { useLayoutEffect, useRef, useState } from 'react';
import type { EditSet, Species, SpeciesStats } from '../../bindings';
import { ConfirmDialog } from '../../components/ConfirmDialog';
import { describeIpcError } from '../../ipc/errors';
import { NumberField } from '../../components/NumberField';
import { SectionCard } from '../../components/SectionCard';
import { SelectField } from '../../components/SelectField';
import { TextField } from '../../components/TextField';
import { useEditor } from '../../app/EditorProvider';
import { errorsByPath } from '../../app/selectors';
import { capHint, normalMax, normalPowerupMax } from '../../lib/caps';
import { JUNK_TIERS, junkThreshold, junkTierFromCounter } from '../../lib/junk';
import { levelThreshold } from '../../lib/level';
import { PLAYER_NAME_MAX } from '../../lib/name';
import { formatNumber } from '../../lib/num';
import { SPECIES } from '../../lib/species';
import { TECHNIQUES } from '../../lib/techniques';

function setAt(tech: readonly number[], index: number, value: number): EditSet['tech'] {
  const next = [...tech];
  next[index] = value;
  // SAFETY: `next` is a clone of the binding's 9-element tuple; one element
  // changes and the length is preserved.
  return next as unknown as EditSet['tech'];
}

function setUpcnt(upcnt: readonly number[], index: number, value: number): EditSet['upcnt'] {
  const next = [...upcnt];
  next[index] = value;
  // SAFETY: `next` is a clone of the binding's 11-element tuple; one element
  // changes and the length is preserved.
  return next as unknown as EditSet['upcnt'];
}

export function CharacterSection() {
  const { state, setField, speciesStats } = useEditor();
  const { draft, appInfo, mode } = state;
  const statsCache = useRef(new Map<Species, SpeciesStats>());
  const requestSeq = useRef(0);
  const [switching, setSwitching] = useState<number | null>(null);
  const latestDraft = useRef(draft);
  useLayoutEffect(() => { latestDraft.current = draft; }, [draft]);
  useLayoutEffect(() => { requestSeq.current += 1; }, [mode]);
  const [switchError, setSwitchError] = useState<string | null>(null);
  const [pendingSpecies, setPendingSpecies] = useState<{ species: Species; stats: SpeciesStats } | null>(null);
  useLayoutEffect(() => {
    requestSeq.current += 1;
    statsCache.current.clear();
    const view = state.session?.view;
    if (view) statsCache.current.set(view.species, {
      level: view.level, exp: view.exp, tech: view.tech, upcnt: view.upcnt,
    });
  }, [state.session]);
  if (!draft || !appInfo) return null;

  const errors = errorsByPath(state);

  // Normal mode caps the boxes at what the validator accepts; Advanced leaves
  // them uncapped so a higher value can reach the data-type check in Rust.
  const bitMax = normalMax(appInfo.ui.caps, 'bit', mode);
  const xdataMax = normalMax(appInfo.ui.caps, 'xdata', mode);
  const levelMax = normalMax(appInfo.ui.caps, 'level', mode);
  const expMax = normalMax(appInfo.ui.caps, 'exp', mode);
  const techMax = normalMax(appInfo.ui.caps, 'tech', mode);

  const switchTo = (species: Species, stats: SpeciesStats) => {
    statsCache.current.set(species, stats);
    setField({ species, ...stats }, 'species');
  };

  const onSpecies = async (species: Species) => {
    if (species === draft.species) return;
    const sequence = ++requestSeq.current;
    setSwitching(sequence);
    setSwitchError(null);
    try {
      const current = statsCache.current.get(draft.species) ?? await speciesStats(draft.species, mode);
      const next = await speciesStats(species, mode);
      const active = latestDraft.current;
      if (sequence !== requestSeq.current || !active || active.species !== draft.species) return;
      const changed = active.level !== current.level || active.exp !== current.exp ||
        active.tech.some((value, i) => value !== current.tech[i]) ||
        active.upcnt.some((value, i) => value !== current.upcnt[i]);
      if (changed) setPendingSpecies({ species, stats: next });
      else switchTo(species, next);
    } catch (error) {
      if (sequence === requestSeq.current) setSwitchError(describeIpcError(error));
    } finally {
      setSwitching((active) => active === sequence ? null : active);
    }
  };

  const onLevel = (level: number) =>
    setField({ level, exp: Math.min(levelThreshold(level), 0xffffffff) }, 'level');

  return (
    <SectionCard title="Character">
      {switchError ? <p role="alert">{switchError}</p> : null}
      <ConfirmDialog
        isOpen={pendingSpecies !== null}
        title="Discard progression changes?"
        message={`Switching species discards the unsaved level, EXP, technique and power-up changes for ${draft.species}. Save those changes first, or discard them to switch.`}
        confirmLabel="Discard and switch"
        onCancel={() => setPendingSpecies(null)}
        onConfirm={() => {
          if (pendingSpecies) switchTo(pendingSpecies.species, pendingSpecies.stats);
          setPendingSpecies(null);
        }}
      />
      <div className="grid">
        <SelectField
          label="Species"
          disabled={switching !== null}
          value={draft.species}
          options={SPECIES.map((species) => ({ value: species, label: species }))}
          onChange={(species) => void onSpecies(species)}
        />
        <TextField
          label="Player name"
          value={draft.name}
          maxLength={PLAYER_NAME_MAX}
          onChange={(name) => setField({ name }, 'name')}
          error={errors.get('name')}
        />
        <NumberField
          label="BIT"
          value={draft.bit}
          onChange={(bit) => setField({ bit }, 'bit')}
          error={errors.get('bit')}
          max={bitMax}
          hint={capHint(bitMax)}
        />
        <NumberField
          label="X-Data"
          value={draft.xdata}
          onChange={(xdata) => setField({ xdata }, 'xdata')}
          error={errors.get('xdata')}
          max={xdataMax}
          hint={capHint(xdataMax)}
        />
        <SelectField
          label="Junk shop tier"
          value={junkTierFromCounter(draft.junk)}
          options={JUNK_TIERS.map((tier) => ({
            value: tier.tier,
            label: `Tier ${tier.tier} (${formatNumber(tier.threshold)} bits)`,
          }))}
          onChange={(tier) => setField({ junk: junkThreshold(tier) ?? 0 }, 'junk')}
        />
        <NumberField
          label="Level"
          value={draft.level}
          onChange={onLevel}
          error={errors.get('level')}
          max={levelMax}
          hint={capHint(levelMax)}
        />
        <NumberField
          label="EXP"
          value={draft.exp}
          onChange={(exp) => setField({ exp }, 'exp')}
          error={errors.get('exp')}
          max={expMax}
          hint={capHint(expMax)}
        />
      </div>

      <h3>Techniques</h3>
      <div className="grid">
        {draft.tech.map((value, index) => (
          <NumberField
            key={index}
            label={TECHNIQUES[index] ?? `Technique ${index + 1}`}
            value={value}
            onChange={(next) => setField({ tech: setAt(draft.tech, index, next) }, `tech[${index}]`)}
            error={errors.get(`tech[${index}]`)}
            max={techMax}
          />
        ))}
      </div>

      <h3>Power-ups</h3>
      <div className="grid">
        {draft.upcnt.map((value, index) => {
          const slotMax = normalPowerupMax(appInfo.ui.powerups, index, mode);
          return (
            <NumberField
              key={index}
              label={appInfo.ui.powerups[index]?.stat ?? `Power-up ${index + 1}`}
              value={value}
              onChange={(next) =>
                setField({ upcnt: setUpcnt(draft.upcnt, index, next) }, `upcnt[${index}]`)
              }
              error={errors.get(`upcnt[${index}]`)}
              max={slotMax}
              hint={capHint(slotMax)}
            />
          );
        })}
      </div>
    </SectionCard>
  );
}
