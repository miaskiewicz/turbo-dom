import { expect, test } from 'vitest';

test('focus and selection follow DOM changes through the Vitest globals', () => {
  const panel = document.createElement('section');
  const field = document.createElement('textarea');
  panel.append(field);
  document.body.append(panel);
  panel.inert = true;
  field.focus();
  expect(document.activeElement).toBe(document.body);
  expect(panel.hasAttribute('inert')).toBe(true);
  panel.inert = false;
  field.value = 'hello';
  field.focus();
  expect(document.activeElement).toBe(field);
  expect([field.selectionStart, field.selectionEnd]).toEqual([5, 5]);
  panel.remove();
  expect(document.activeElement).toBe(document.body);
});

test('installed geometry and history globals agree with window', () => {
  const rect = document.body.getBoundingClientRect();
  expect(rect).toBeInstanceOf(DOMRect);
  expect(rect).toBeInstanceOf(window.DOMRectReadOnly);
  location.hash = 'first';
  expect(window.location.hash).toBe('#first');
  history.pushState({}, '', '/next#second');
  expect(document.URL).toBe('http://localhost/next#second');
  history.replaceState(null, '', '/');
  expect(location.href).toBe('http://localhost/');
});
