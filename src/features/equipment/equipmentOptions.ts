import type { Category, Item } from '../../bindings';
import type { SelectOption } from '../../components/SelectField';
import { EMPTY, describeItemId } from '../../lib/items';

/** The device slots holding an item of one of `categories`, plus `(none)`. */
export function deviceOptions(
  device: readonly number[],
  catalogue: readonly Item[],
  categories: readonly Category[],
): SelectOption<number>[] {
  const options: SelectOption<number>[] = [{ value: EMPTY, label: '(none)' }];
  device.forEach((id, index) => {
    if (id === EMPTY) return;
    const item = catalogue.find((entry) => entry.base_id === (id & 0xffff));
    if (!item) return;
    if (!categories.some((category) => category === item.category)) return;
    options.push({ value: index, label: `slot ${index + 1} — ${describeItemId(id, catalogue)}` });
  });
  return options;
}
