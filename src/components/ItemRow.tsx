import type { Category, FieldError, Item, Mode } from '../bindings';
import {
  EMPTY,
  RARITIES,
  buildItemId,
  clampBonusToRarity,
  colorForSeed,
  describeItemId,
  splitItemId,
  type Rarity,
} from '../lib/items';
import { FieldMessage } from './FieldMessage';
import { ItemPicker } from './ItemPicker';
import { NumberField } from './NumberField';

/**
 * One device-folder or bank slot.
 *
 * The stored id is `base_id | seed<<4|mods << 16`; the picker sets the base id,
 * the rarity control sets the seed (clamped into the colour's band) and the
 * mods field sets the mod count.
 */
export function ItemRow({
  slot,
  value,
  catalogue,
  categories,
  mode,
  onChange,
  error,
}: {
  slot: number;
  value: number;
  catalogue: readonly Item[];
  categories?: readonly Category[];
  mode: Mode;
  onChange: (id: number) => void;
  error?: FieldError[];
}) {
  const { baseId, seed, mods } = splitItemId(value);
  const isEmpty = value === EMPTY;
  const rarity = colorForSeed(seed);
  // The picker shows a known item's name, so its own span is only needed for an
  // id that is not in the catalogue and would otherwise read as "(empty)".
  const unknown = !isEmpty && !catalogue.some((item) => item.base_id === baseId);

  return (
    <div className="item-row" data-testid={`item-row-${slot}`}>
      <span className="slot" aria-hidden="true">
        {slot + 1}
      </span>
      {unknown ? (
        <span className="item-name" title={describeItemId(value, catalogue)}>
          {describeItemId(value, catalogue)}
        </span>
      ) : null}
      <ItemPicker
        value={value}
        catalogue={catalogue}
        categories={categories}
        onChange={(picked) => onChange(buildItemId(picked, 0, 0))}
      />
      <select
        aria-label={`Rarity ${slot + 1}`}
        value={rarity}
        disabled={isEmpty}
        onChange={(event) => {
          const colour = event.target.value as Rarity;
          onChange(buildItemId(baseId, clampBonusToRarity(seed, colour), mods));
        }}
      >
        {RARITIES.map((colour) => (
          <option key={colour} value={colour}>
            {colour}
          </option>
        ))}
      </select>
      <NumberField
        label={`+N ${slot + 1}`}
        value={seed}
        disabled={isEmpty}
        onChange={(bonus) => onChange(buildItemId(baseId, bonus & 0x7ff, mods))}
      />
      <NumberField
        label={`Mods ${slot + 1}`}
        value={mods}
        disabled={isEmpty}
        onChange={(count) => onChange(buildItemId(baseId, seed, count))}
      />
      {mode === 'advanced' ? (
        <NumberField
          label={`Raw id ${slot + 1}`}
          value={value}
          onChange={(raw) => onChange(raw >>> 0)}
        />
      ) : null}
      <button type="button" onClick={() => onChange(EMPTY)} disabled={isEmpty}>
        Clear
      </button>
      <FieldMessage errors={error} />
    </div>
  );
}
