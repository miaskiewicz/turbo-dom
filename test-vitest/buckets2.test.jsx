import { test, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';

test('1: date input user.type sets value + onChange', async () => {
  const onChange = vi.fn();
  render(<input type="date" aria-label="d" onChange={(e) => onChange(e.target.value)} />);
  const el = screen.getByLabelText('d');
  await userEvent.type(el, '2026-04-01');
  expect(el.value).toBe('2026-04-01');
  expect(onChange).toHaveBeenCalledWith('2026-04-01');
});

test('2: FormData keeps File identity + name', () => {
  const fd = new FormData();
  const file = new File(['a,b'], 'roster.csv', { type: 'text/csv' });
  fd.append('file', file);
  const got = fd.get('file');
  expect(got).toBeInstanceOf(File);
  expect(got.name).toBe('roster.csv');
  expect(got).toBe(file);
});

test('4: blur fires when focus moves (user.tab)', async () => {
  const onBlur = vi.fn();
  render(<div><input aria-label="a" onBlur={() => onBlur('a')} /><input aria-label="b" /></div>);
  const a = screen.getByLabelText('a');
  a.focus();
  await userEvent.tab();
  expect(onBlur).toHaveBeenCalledWith('a');
});

test('6: anchor.download reflects', () => {
  const a = document.createElement('a');
  a.download = 'tpl.csv';
  expect(a.download).toBe('tpl.csv');
});

test('7: controlled select.value reads back', () => {
  function F() { const [v] = useState('NO_END_DATE'); return (
    <select aria-label="s" value={v} onChange={() => {}}>
      <option value="NO_END_DATE">no end</option><option value="END_DATE">end</option>
    </select>
  ); }
  render(<F />);
  expect(screen.getByLabelText('s').value).toBe('NO_END_DATE');
});

test('8: fake timers advance setInterval', () => {
  vi.useFakeTimers();
  let n = 0;
  const id = setInterval(() => { n++; }, 1000);
  vi.advanceTimersByTime(3000);
  clearInterval(id);
  vi.useRealTimers();
  expect(n).toBe(3);
});
