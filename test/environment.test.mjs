import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import vitestEnv from '../src/environment/vitest.mjs';

const require = createRequire(import.meta.url);

test('vitest env: name + transformMode', () => {
  assert.equal(vitestEnv.name, 'turbodom');
  assert.equal(vitestEnv.transformMode, 'web');
});

test('vitest env installs document/window/self + lazy globals', () => {
  const g = {};
  const { teardown } = vitestEnv.setup(g, {});
  assert.ok(g.document, 'document defined');
  assert.equal(g.window, g.window.window);
  assert.equal(g.self, g.window);
  // DOM works through the installed global document
  const el = g.document.createElement('div');
  el.dataset.x = '1';
  assert.equal(el.dataset.x, '1');
  // lazy globals materialize on access
  assert.equal(typeof g.getComputedStyle, 'function');
  assert.equal(typeof g.MutationObserver, 'function');
  assert.equal(typeof g.IntersectionObserver, 'function');
  g.localStorage.setItem('k', 'v');
  assert.equal(g.localStorage.getItem('k'), 'v');
  teardown();
});

test('vitest env seeds the document from environmentOptions.turbodom.html', () => {
  const g = {};
  vitestEnv.setup(g, { turbodom: { html: '<!doctype html><body><main id=x>hi</main></body>' } });
  assert.equal(g.document.getElementById('x').textContent, 'hi');
});

test('lazy: render-only env touches zero lazy globals', () => {
  const g = {};
  vitestEnv.setup(g, {});
  g.document.querySelectorAll('div'); // pure DOM
  assert.deepEqual(g.__turbodom.touched(), []);
});

test('@testing-library/dom works through globals installed on globalThis', () => {
  const had = Object.getOwnPropertyDescriptor(globalThis, 'document');
  try {
    vitestEnv.setup(globalThis, { turbodom: { html: '<!doctype html><body><button>Click me</button></body>' } });
    const { screen } = require('@testing-library/dom'); // screen binds to global document lazily
    assert.ok(screen.getByText('Click me'));
    assert.equal(screen.queryByText('nope'), null);
  } finally {
    if (had) Object.defineProperty(globalThis, 'document', had);
    else delete globalThis.document;
  }
});
