import type { EditSet, Species } from '../../bindings';
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
  if (!draft || !appInfo) return null;

  const errors = errorsByPath(state);

  // Normal mode caps the boxes at what the validator accepts; Advanced leaves
  // them uncapped so a higher value can reach the data-type check in Rust.
  const bitMax = normalMax(appInfo.ui.caps, 'bit', mode);
  const xdataMax = normalMax(appInfo.ui.caps, 'xdata', mode);
  const levelMax = normalMax(appInfo.ui.caps, 'level', mode);
  const expMax = normalMax(appInfo.ui.caps, 'exp', mode);
  const techMax = normalMax(appInfo.ui.caps, 'tech', mode);

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
