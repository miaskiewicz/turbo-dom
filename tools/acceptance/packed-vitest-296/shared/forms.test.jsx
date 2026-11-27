import { test, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';

test('getByLabelText resolves <label htmlFor>', () => {
  render(
    <div>
      <label htmlFor="amt">amount</label>
      <input id="amt" />
    </div>
  );
  expect(screen.getByLabelText('amount')).toBeTruthy();
});

test('fireEvent.change on select fires onChange with value', () => {
  const onChange = vi.fn();
  render(
    <select aria-label="acct" onChange={(e) => onChange(e.target.value)}>
      <option value="ws-2">A</option>
      <option value="savings">Savings</option>
    </select>
  );
  fireEvent.change(screen.getByLabelText('acct'), { target: { value: 'savings' } });
  expect(onChange).toHaveBeenCalledWith('savings');
});

test('user.selectOptions fires change + reflects value', async () => {
  const onChange = vi.fn();
  function F() {
    const [v, setV] = useState('hours');
    return (
      <select aria-label="unit" value={v} onChange={(e) => { setV(e.target.value); onChange(e.target.value); }}>
        <option value="hours">hours</option>
        <option value="days">days</option>
      </select>
    );
  }
  render(<F />);
  const sel = screen.getByLabelText('unit');
  expect(sel).toHaveValue('hours');
  await userEvent.selectOptions(sel, 'days');
  expect(onChange).toHaveBeenCalledWith('days');
  expect(sel).toHaveValue('days');
});

test('user.type fires input/change on text input', async () => {
  const onChange = vi.fn();
  function F() {
    const [v, setV] = useState('');
    return <input aria-label="name" value={v} onChange={(e) => { setV(e.target.value); onChange(e.target.value); }} />;
  }
  render(<F />);
  const inp = screen.getByLabelText('name');
  await userEvent.type(inp, 'hi');
  expect(inp).toHaveValue('hi');
  expect(onChange).toHaveBeenCalled();
});
