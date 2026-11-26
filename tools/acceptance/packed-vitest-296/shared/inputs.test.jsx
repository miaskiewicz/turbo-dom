import { test, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';

test('text input: type fires change + reflects value', async () => {
  const onChange = vi.fn();
  function F() { const [v, setV] = useState(''); return <input aria-label="t" value={v} onChange={(e) => { setV(e.target.value); onChange(e.target.value); }} />; }
  render(<F />);
  const el = screen.getByLabelText('t');
  await userEvent.type(el, 'hi');
  expect(el).toHaveValue('hi');
  expect(onChange).toHaveBeenCalled();
});

test('textarea: type fires change + reflects value', async () => {
  function F() { const [v, setV] = useState(''); return <textarea aria-label="ta" value={v} onChange={(e) => setV(e.target.value)} />; }
  render(<F />);
  const el = screen.getByLabelText('ta');
  await userEvent.type(el, 'abc');
  expect(el).toHaveValue('abc');
});

test('checkbox: click toggles + fires change', async () => {
  const onChange = vi.fn();
  render(<input type="checkbox" aria-label="cb" onChange={onChange} />);
  const el = screen.getByLabelText('cb');
  expect(el).not.toBeChecked();
  await userEvent.click(el);
  expect(el).toBeChecked();
  expect(onChange).toHaveBeenCalled();
});

test('radio: click selects within group', async () => {
  function F() { const [v, setV] = useState('a'); return (
    <div>
      <label><input type="radio" name="g" value="a" checked={v==='a'} onChange={(e)=>setV(e.target.value)} />A</label>
      <label><input type="radio" name="g" value="b" checked={v==='b'} onChange={(e)=>setV(e.target.value)} />B</label>
    </div>
  ); }
  render(<F />);
  const b = screen.getByDisplayValue ? screen.getAllByRole('radio')[1] : screen.getAllByRole('radio')[1];
  await userEvent.click(b);
  expect(b).toBeChecked();
});

test('select: selectOptions fires change + reflects value', async () => {
  const onChange = vi.fn();
  function F() { const [v, setV] = useState('hours'); return (
    <select aria-label="u" value={v} onChange={(e) => { setV(e.target.value); onChange(e.target.value); }}>
      <option value="hours">hours</option><option value="days">days</option>
    </select>
  ); }
  render(<F />);
  const el = screen.getByLabelText('u');
  expect(el).toHaveValue('hours');
  await userEvent.selectOptions(el, 'days');
  expect(el).toHaveValue('days');
  expect(onChange).toHaveBeenCalledWith('days');
});

test('range input: fireEvent.change reflects value', () => {
  const onChange = vi.fn();
  render(<input type="range" aria-label="r" min="0" max="100" onChange={(e)=>onChange(e.target.value)} />);
  const el = screen.getByLabelText('r');
  fireEvent.change(el, { target: { value: '42' } });
  expect(el).toHaveValue('42'); expect(el.valueAsNumber).toBe(42);
  expect(onChange).toHaveBeenCalledWith('42');
});

test('getByRole textbox / checkbox / radio / combobox roles resolve', () => {
  render(<div>
    <input aria-label="x" />
    <textarea aria-label="y" />
    <input type="checkbox" aria-label="z" />
    <select aria-label="s"><option>a</option></select>
  </div>);
  expect(screen.getByRole('textbox', { name: 'x' })).toBeTruthy();
  expect(screen.getByRole('checkbox', { name: 'z' })).toBeTruthy();
  expect(screen.getByRole('combobox', { name: 's' })).toBeTruthy();
});
