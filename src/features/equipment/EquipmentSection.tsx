import type { Category, EditSet } from '../../bindings';
import { FieldMessage } from '../../components/FieldMessage';
import { ItemPicker } from '../../components/ItemPicker';
import { SectionCard } from '../../components/SectionCard';
import { SelectField } from '../../components/SelectField';
import { useEditor } from '../../app/EditorProvider';
import { errorsByPath } from '../../app/selectors';
import { EMPTY } from '../../lib/items';
import { resolveMods } from '../../lib/mods';
import { deviceOptions } from './equipmentOptions';

const WEAPON: readonly Category[] = ['weapon', 'styled'];
const ARMOR: readonly Category[] = ['core'];
const BOARD: readonly Category[] = ['board'];
const MOD: readonly Category[] = ['mod'];

function setWeaponIndex(
  weapons: EditSet['weapons'],
  index: number,
  value: number,
): EditSet['weapons'] {
  const next: [number, number, number] = [weapons[0], weapons[1], weapons[2]];
  next[index] = value;
  return next;
}

export function EquipmentSection() {
  const { state, setField } = useEditor();
  const { draft, appInfo } = state;
  if (!draft || !appInfo) return null;

  const errors = errorsByPath(state);
  const catalogue = appInfo.ui.catalogue;

  const setSocket = (which: 'wmods' | 'amods', index: number, baseId: number | null) => {
    const sockets: EditSet['wmods'] = [
      draft.wmods[0],
      draft.wmods[1],
      draft.wmods[2],
      draft.wmods[3],
      draft.wmods[4],
    ];
    sockets[index] = baseId;
    const next: EditSet =
      which === 'wmods'
        ? { ...draft, wmods: sockets }
        : { ...draft, amods: sockets };
    const resolved = resolveMods(next);
    // SAFETY: `resolveMods` clones the 30-slot device tuple, so its length is
    // the binding's; the chips it adds are the ones Rust would add on save.
    const device = resolved.device as unknown as EditSet['device'];
    setField(
      which === 'wmods' ? { wmods: sockets, device } : { amods: sockets, device },
      `socket-${which}-${index}`,
    );
  };

  return (
    <SectionCard title="Equipment">
      <h3>Weapons</h3>
      <div className="grid">
        {draft.weapons.map((index, slot) => (
          <SelectField
            key={slot}
            label={`Weapon ${slot + 1}`}
            value={index}
            options={deviceOptions(draft.device, catalogue, WEAPON)}
            onChange={(value) =>
              setField({ weapons: setWeaponIndex(draft.weapons, slot, value) }, `weapon${slot}`)
            }
          />
        ))}
      </div>

      <h3>Armor and sub</h3>
      <div className="grid">
        <SelectField
          label="Armor"
          value={draft.armor}
          options={deviceOptions(draft.device, catalogue, ARMOR)}
          onChange={(armor) => setField({ armor }, 'armor')}
        />
        <SelectField
          label="Sub (board)"
          value={draft.sub}
          options={deviceOptions(draft.device, catalogue, BOARD)}
          onChange={(sub) => setField({ sub }, 'sub')}
        />
      </div>

      <h3>Weapon mods</h3>
      <div className="grid">
        {draft.wmods.map((baseId, slot) => (
          <ItemPicker
            key={slot}
            label={`Weapon mod ${slot + 1}`}
            value={baseId ?? EMPTY}
            catalogue={catalogue}
            categories={MOD}
            onChange={(picked) => setSocket('wmods', slot, picked)}
          />
        ))}
      </div>

      <h3>Armor mods</h3>
      <div className="grid">
        {draft.amods.map((baseId, slot) => (
          <ItemPicker
            key={slot}
            label={`Armor mod ${slot + 1}`}
            value={baseId ?? EMPTY}
            catalogue={catalogue}
            categories={MOD}
            onChange={(picked) => setSocket('amods', slot, picked)}
          />
        ))}
      </div>

      <FieldMessage errors={errors.get('equip.armor') ?? errors.get('equip.sub')} />
    </SectionCard>
  );
}
