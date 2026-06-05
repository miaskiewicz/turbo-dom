import { test, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';

test('A: date input change propagates', () => {
  const onChange = vi.fn();
  render(<input type="date" aria-label="d" onChange={(e) => onChange(e.target.value)} />);
  const el = screen.getByLabelText('d');
  fireEvent.change(el, { target: { value: '2024-01-15' } });
  expect(el.value).toBe('2024-01-15');
  expect(onChange).toHaveBeenCalledWith('2024-01-15');
});

test('B: vi.spyOn(window, open) + scrollTo works', () => {
  const openSpy = vi.spyOn(window, 'open').mockReturnValue(null);
  window.open('/x');
  expect(openSpy).toHaveBeenCalledWith('/x');
  const scrollSpy = vi.spyOn(window, 'scrollTo');
  window.scrollTo({ top: 0, behavior: 'smooth' });
  expect(scrollSpy).toHaveBeenCalledWith({ top: 0, behavior: 'smooth' });
});

test('C: File instanceof Blob', () => {
  const f = new File(['x'], 'a.txt', { type: 'text/plain' });
  expect(f).toBeInstanceOf(Blob);
  expect(f).toBeInstanceOf(File);
  expect(f.name).toBe('a.txt');
});

test('D: gradient via React inline style is computed', () => {
  render(<div data-testid="g" style={{ backgroundImage: 'radial-gradient(red, blue)' }} />);
  const el = screen.getByTestId('g');
  expect(window.getComputedStyle(el).backgroundImage).toContain('radial-gradient');
});

test('E: getPropertyPriority returns important', () => {
  const el = document.createElement('div');
  el.style.setProperty('color', 'red', 'important');
  expect(el.style.getPropertyPriority('color')).toBe('important');
  el.setAttribute('style', 'color: blue !important');
  expect(el.style.getPropertyPriority('color')).toBe('important');
});
