import {
  Button,
  ComboBox,
  Group,
  Input,
  Label,
  ListBox,
  ListBoxItem,
  Popover,
} from 'react-aria-components';
import type { Category, Item } from '../bindings';
import { buildItemId, itemLabel, splitItemId } from '../lib/items';

/**
 * A searchable catalogue picker.
 *
 * A combo box: the field itself is editable and the list filters as you type
 * (like Vuetify's autocomplete), with the caret of the other dropdowns on the
 * right. Picking an item resets the seed and mod count; the row's rarity
 * control sets the seed afterwards.
 */
export function ItemPicker({
  value,
  catalogue,
  categories,
  label = 'Item',
  placeholder = 'Search or pick',
  disabled,
  onChange,
}: {
  value: number;
  catalogue: readonly Item[];
  categories?: readonly Category[];
  label?: string;
  placeholder?: string;
  disabled?: boolean;
  onChange: (baseId: number) => void;
}) {
  const { baseId } = splitItemId(value);
  const items = categories
    ? catalogue.filter((item) => matchesCategory(item.category, categories))
    : catalogue;
  const selected = catalogue.find((item) => item.base_id === baseId);

  return (
    <ComboBox
      className="field item-picker"
      selectedKey={selected ? selected.base_id : null}
      isDisabled={disabled}
      menuTrigger="focus"
      defaultFilter={(textValue, inputValue) =>
        textValue.toLowerCase().includes(inputValue.toLowerCase())
      }
      onSelectionChange={(key) => {
        if (key === null) return;
        onChange(buildItemId(Number(key), 0, 0));
      }}
    >
      <Label className="label">{label}</Label>
      <Group className="combo-group">
        <Input placeholder={placeholder} />
        <Button aria-label={`Show ${label} options`} className="combo-toggle">
          ▾
        </Button>
      </Group>
      <Popover className="popover select-popover">
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
      </Popover>
    </ComboBox>
  );
}

function matchesCategory(category: Category, wanted: readonly Category[]): boolean {
  return wanted.some((entry) =>
    typeof entry === 'string' ? entry === category : category === entry,
  );
}
