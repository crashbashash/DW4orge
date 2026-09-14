import {
  Autocomplete,
  Button,
  Input,
  Label,
  ListBox,
  ListBoxItem,
  Popover,
  SearchField,
  Select,
  SelectValue,
  useFilter,
} from 'react-aria-components';
import type { Category, Item } from '../bindings';
import { buildItemId, itemLabel, splitItemId } from '../lib/items';

/**
 * A searchable catalogue picker.
 *
 * A `Select` so it matches the other dropdowns — the box shows the current
 * item and opens on click — with an `Autocomplete` search field inside the
 * popover to filter the 539 entries. Picking an item resets the seed and mod
 * count; the row's rarity control sets the seed afterwards.
 */
export function ItemPicker({
  value,
  catalogue,
  categories,
  label = 'Item',
  onChange,
}: {
  value: number;
  catalogue: readonly Item[];
  categories?: readonly Category[];
  label?: string;
  onChange: (baseId: number) => void;
}) {
  const { baseId } = splitItemId(value);
  const items = categories
    ? catalogue.filter((item) => matchesCategory(item.category, categories))
    : catalogue;
  const selected = catalogue.find((item) => item.base_id === baseId);
  const { contains } = useFilter({ sensitivity: 'base' });

  return (
    <Select
      className="field item-picker"
      selectedKey={selected ? selected.base_id : null}
      placeholder="(empty)"
      onSelectionChange={(key) => {
        if (key !== null) onChange(buildItemId(Number(key), 0, 0));
      }}
    >
      <Label className="label">{label}</Label>
      <Button className="select-button">
        <SelectValue />
        <span aria-hidden="true" className="caret">
          ▾
        </span>
      </Button>
      <Popover className="popover select-popover">
        <Autocomplete filter={contains}>
          <SearchField aria-label={`Search ${label}`} autoFocus className="search-field">
            <Input placeholder="Search the catalogue" />
          </SearchField>
          <ListBox
            className="listbox"
            items={items}
            renderEmptyState={() => <span className="muted">No matching item</span>}
          >
            {(item) => (
              <ListBoxItem id={item.base_id} textValue={itemLabel(item)} className="listbox-item">
                {itemLabel(item)}
              </ListBoxItem>
            )}
          </ListBox>
        </Autocomplete>
      </Popover>
    </Select>
  );
}

function matchesCategory(category: Category, wanted: readonly Category[]): boolean {
  return wanted.some((entry) =>
    typeof entry === 'string' ? entry === category : category === entry,
  );
}
