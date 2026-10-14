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
import type { Item } from '../bindings';
import { buildItemId, itemLabel, splitItemId } from '../lib/items';

/**
 * A searchable catalogue picker.
 *
 * A combo box: the field itself is editable and the list filters as you type
 * (like Vuetify's autocomplete), with the caret of the other dropdowns on the
 * right. `items` is the already-filtered list to offer — callers pass a bucket
 * or category slice, so this never builds a collection for the whole
 * catalogue. Picking an item resets the seed and mod count; the row's rarity
 * control sets the seed afterwards.
 */
export function ItemPicker({
  value,
  catalogue,
  items,
  label = 'Item',
  placeholder = 'Search or pick',
  disabled,
  onChange,
}: {
  value: number;
  catalogue: readonly Item[];
  items: readonly Item[];
  label?: string;
  placeholder?: string;
  disabled?: boolean;
  onChange: (baseId: number) => void;
}) {
  const { baseId } = splitItemId(value);
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
