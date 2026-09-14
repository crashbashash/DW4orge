import { Button, ComboBox, Input, Label, ListBox, ListBoxItem, Popover } from 'react-aria-components';
import type { Category, Item } from '../bindings';
import { buildItemId, itemLabel, splitItemId } from '../lib/items';

/**
 * A searchable catalogue picker.
 *
 * Picking an item resets the seed and mod count (the base id is what the
 * catalogue names); the row's rarity control sets the seed afterwards.
 */
export function ItemPicker({
  value,
  catalogue,
  categories,
  onChange,
}: {
  value: number;
  catalogue: readonly Item[];
  categories?: readonly Category[];
  onChange: (baseId: number) => void;
}) {
  const { baseId } = splitItemId(value);
  const items = categories
    ? catalogue.filter((item) => matchesCategory(item.category, categories))
    : catalogue;
  const selected = catalogue.find((item) => item.base_id === baseId);

  return (
    <ComboBox
      className="item-picker"
      selectedKey={selected ? selected.base_id : null}
      defaultFilter={(textValue, inputValue) =>
        textValue.toLowerCase().includes(inputValue.toLowerCase())
      }
      onSelectionChange={(key) => {
        if (key === null) return;
        onChange(buildItemId(Number(key), 0, 0));
      }}
    >
      <Label className="label">Item</Label>
      <Input placeholder="Search the catalogue" />
      <Button aria-label="Show items">▾</Button>
      <Popover>
        <ListBox
          items={items}
          renderEmptyState={() => <span className="muted">No matching item</span>}
        >
          {(item) => (
            <ListBoxItem id={item.base_id} textValue={itemLabel(item)}>
              {itemLabel(item)}
            </ListBoxItem>
          )}
        </ListBox>
      </Popover>
    </ComboBox>
  );
}

function matchesCategory(category: Category, wanted: readonly Category[]): boolean {
  return wanted.some((entry) =>
    typeof entry === 'string' ? entry === category : category === entry,
  );
}
