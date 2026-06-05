import { test, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';

test('globals are installed', () => {
  expect(typeof window).toBe('object');
  expect(typeof document).toBe('object');
  expect(typeof globalThis.Symbol).toBe('function'); // not clobbered
  const d = document.createElement('div');
  d.dataset.x = '1';
  expect(d.getAttribute('data-x')).toBe('1');
});

function Counter() {
  const [n, setN] = useState(0);
  return <button onClick={() => setN((x) => x + 1)}>count: {n}</button>;
}

test('React + RTL + user-event through turbo-dom', async () => {
  render(<Counter />);
  const btn = screen.getByRole('button', { name: /count: 0/ });
  await userEvent.click(btn);
  expect(await screen.findByText('count: 1')).toBeTruthy();
});
