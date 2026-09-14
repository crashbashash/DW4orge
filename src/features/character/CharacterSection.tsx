import type { AppInfo, EditSet, Mode, Species } from '../../bindings';
import { NumberField } from '../../components/NumberField';
import { SectionCard } from '../../components/SectionCard';
import { SelectField } from '../../components/SelectField';
import { TextField } from '../../components/TextField';
import { useEditor } from '../../app/EditorProvider';
import { errorsByPath } from '../../app/selectors';
import { capFor, powerupCap } from '../../lib/caps';
import { JUNK_TIERS, junkThreshold, junkTierFromCounter } from '../../lib/junk';
import { levelThreshold } from '../../lib/level';
import { formatNumber } from '../../lib/num';

const SPECIES: readonly Species[] = [
  'Agumon',
  'Veemon',
  'Girumon',
  'Dorumon',
  'WereGarurumon',
  'HerculesKabuterimon',
  'WarGreymon',
  'AngelRimon',
  'Beelzemon',
  'Alphamon',
  'BlackWarGreymon',
  'ImperialdramonFm',
  'ImperialdramonPm',
  'MetalGarurumon',
  'DukeCrimson',
  'Susanoomon',
];

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

function normalHint(appInfo: AppInfo, field: string, mode: Mode): string | undefined {
  if (mode !== 'normal') return undefined;
  const cap = capFor(appInfo.ui.caps, field);
  return cap ? `max ${formatNumber(cap.cap.normal_max)}` : undefined;
}

export function CharacterSection() {
  const { state, setField, speciesStats } = useEditor();
  const { draft, appInfo, mode } = state;
  if (!draft || !appInfo) return null;

  const errors = errorsByPath(state);

  const onSpecies = async (species: Species) => {
    // Reproduce `_load_species_stats`: show the block the new species stores.
    const stats = await speciesStats(species, mode);
    setField(
      {
        species,
        level: stats.level,
        exp: stats.exp,
        tech: stats.tech,
        upcnt: stats.upcnt,
      },
      'species',
    );
  };

  const onLevel = (level: number) =>
    setField({ level, exp: Math.min(levelThreshold(level), 0xffffffff) }, 'level');

  return (
    <SectionCard title="Character">
      <div className="grid">
        <SelectField
          label="Species"
          value={draft.species}
          options={SPECIES.map((species) => ({ value: species, label: species }))}
          onChange={(species) => void onSpecies(species)}
        />
        <TextField
          label="Player name"
          value={draft.name}
          maxLength={3}
          onChange={(name) => setField({ name }, 'name')}
          error={errors.get('name')}
        />
        <NumberField
          label="BIT"
          value={draft.bit}
          onChange={(bit) => setField({ bit }, 'bit')}
          error={errors.get('bit')}
          hint={normalHint(appInfo, 'bit', mode)}
        />
        <NumberField
          label="X-Data"
          value={draft.xdata}
          onChange={(xdata) => setField({ xdata }, 'xdata')}
          error={errors.get('xdata')}
          hint={normalHint(appInfo, 'xdata', mode)}
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
          hint={normalHint(appInfo, 'level', mode)}
        />
        <NumberField
          label="EXP"
          value={draft.exp}
          onChange={(exp) => setField({ exp }, 'exp')}
          error={errors.get('exp')}
          hint={normalHint(appInfo, 'exp', mode)}
        />
      </div>

      <h3>Techniques</h3>
      <div className="grid">
        {draft.tech.map((value, index) => (
          <NumberField
            key={index}
            label={`Technique ${index + 1}`}
            value={value}
            onChange={(next) => setField({ tech: setAt(draft.tech, index, next) }, `tech[${index}]`)}
            error={errors.get(`tech[${index}]`)}
          />
        ))}
      </div>

      <h3>Power-ups</h3>
      <div className="grid">
        {draft.upcnt.map((value, index) => (
          <NumberField
            key={index}
            label={appInfo.ui.powerups[index]?.stat ?? `Power-up ${index + 1}`}
            value={value}
            onChange={(next) => setField({ upcnt: setUpcnt(draft.upcnt, index, next) }, `upcnt[${index}]`)}
            error={errors.get(`upcnt[${index}]`)}
            hint={mode === 'normal' ? `max ${formatNumber(powerupCap(appInfo.ui.powerups, index))}` : undefined}
          />
        ))}
      </div>
    </SectionCard>
  );
}
