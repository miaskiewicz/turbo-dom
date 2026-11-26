import { test, expect, vi } from 'vitest';
test('3: window.scrollTo sees vi.stubGlobal', () => {
  const fn = vi.fn();
  vi.stubGlobal('scrollTo', fn);
  window.scrollTo({ top: 0, behavior: 'smooth' });
  expect(fn).toHaveBeenCalledWith({ top: 0, behavior: 'smooth' });
  expect(window).toBe(globalThis);
  vi.unstubAllGlobals();
});

test('A: vi.spyOn(HTMLAnchorElement.prototype, click) works (regression)', () => {
  const clickSpy = vi.fn();
  const spy = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(clickSpy);
  const a = document.createElement('a');
  a.download = 'tpl.csv';
  a.click();
  expect(clickSpy).toHaveBeenCalled();
  expect(a.download).toBe('tpl.csv');
  expect(a instanceof HTMLAnchorElement).toBe(true);
  expect(document.createElement('div') instanceof HTMLAnchorElement).toBe(false);
  spy.mockRestore();
});
