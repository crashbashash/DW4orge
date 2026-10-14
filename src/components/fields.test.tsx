import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { FieldMessage } from './FieldMessage';
import { NumberField } from './NumberField';
import { SelectField } from './SelectField';

describe('NumberField', () => {
  it('commits a parsed value on Enter', async () => {
    const onChange = vi.fn();
    render(<NumberField label="BIT" value={0} onChange={onChange} />);
    const input = screen.getByLabelText('BIT');
    await userEvent.clear(input);
    await userEvent.type(input, '1234{Enter}');
    expect(onChange).toHaveBeenCalledWith(1234);
  });

  it('reverts junk without calling onChange', async () => {
    const onChange = vi.fn();
    render(<NumberField label="BIT" value={5} onChange={onChange} />);
    const input = screen.getByLabelText('BIT') as HTMLInputElement;
    await userEvent.clear(input);
    await userEvent.type(input, 'abc{Enter}');
    expect(onChange).not.toHaveBeenCalled();
    expect(input.value).toBe('5');
  });

  it('clamps a value above max on commit', async () => {
    const onChange = vi.fn();
    // An all-nines bound (level's 999) is already enforced by the typed-length
    // cap, so use a bound whose digits exceed it to exercise the clamp itself.
    render(<NumberField label="EXP" value={0} onChange={onChange} max={150} />);
    const input = screen.getByLabelText('EXP');
    await userEvent.clear(input);
    await userEvent.type(input, '999{Enter}');
    expect(onChange).toHaveBeenCalledWith(150);
  });

  it('leaves a value above max alone when no max is given', async () => {
    const onChange = vi.fn();
    render(<NumberField label="BIT" value={0} onChange={onChange} />);
    const input = screen.getByLabelText('BIT');
    await userEvent.clear(input);
    await userEvent.type(input, '10000{Enter}');
    expect(onChange).toHaveBeenCalledWith(10000);
  });

  it('caps the typed length at the digits of max', async () => {
    const onChange = vi.fn();
    render(<NumberField label="Level" value={0} onChange={onChange} max={999} />);
    const input = screen.getByLabelText('Level') as HTMLInputElement;
    await userEvent.clear(input);
    await userEvent.type(input, '1234');
    expect(input.value).toBe('123');
  });

  it('shows the first error', () => {
    render(
      <NumberField
        label="BIT"
        value={0}
        onChange={() => {}}
        error={[{ path: 'bit', message: 'too big', severity: 'error' }]}
      />,
    );
    expect(screen.getByText('too big')).toBeTruthy();
  });
});

describe('SelectField', () => {
  it('selects an option', async () => {
    const onChange = vi.fn();
    render(
      <SelectField
        label="Species"
        value="Agumon"
        options={[
          { value: 'Agumon', label: 'Agumon' },
          { value: 'Dorumon', label: 'Dorumon' },
        ]}
        onChange={onChange}
      />,
    );
    await userEvent.click(screen.getByRole('button', { name: /species/i }));
    await userEvent.click(await screen.findByRole('option', { name: 'Dorumon' }));
    expect(onChange).toHaveBeenCalledWith('Dorumon');
  });
});

describe('FieldMessage', () => {
  it('renders nothing without errors', () => {
    const { container } = render(<FieldMessage />);
    expect(container.firstChild).toBeNull();
  });
});
