import { Button, Label, ListBox, ListBoxItem, Popover, Select, SelectValue } from 'react-aria-components';

export type SelectOption<T extends string | number> = { value: T; label: string };

/** A single-select built on react-aria-components, styled entirely by our CSS. */
export function SelectField<T extends string | number>({
  label,
  value,
  options,
  onChange,
  disabled,
}: {
  label: string;
  value: T;
  options: readonly SelectOption<T>[];
  onChange: (value: T) => void;
  disabled?: boolean;
}) {
  return (
    <Select
      className="field"
      selectedKey={value}
      isDisabled={disabled}
      onSelectionChange={(key) => {
        if (key !== null) onChange(key as T);
      }}
    >
      <Label className="label">{label}</Label>
      <Button className="select-button">
        <SelectValue />
      </Button>
      <Popover className="popover">
        <ListBox className="listbox">
          {options.map((option) => (
            <ListBoxItem
              key={String(option.value)}
              id={option.value}
              textValue={option.label}
              className="listbox-item"
            >
              {option.label}
            </ListBoxItem>
          ))}
        </ListBox>
      </Popover>
    </Select>
  );
}
