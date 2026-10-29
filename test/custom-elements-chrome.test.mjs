// The JS runtime's custom element state (`:defined`; src/runtime/custom-elements.mjs) against
// the Chrome 156 control that also gates the Rust engine + V8 binding
// (crates/turbo-dom-browser-env/src/custom_elements_defined_{probe.js,cases.json,chrome156.json};
// miaskiewicz/turbo-dom#24 item 3, #26 item 2). The probe runs unchanged over the case HTML:
// creation-time states, upgrades on define() / insertion / customElements.upgrade(), the
// constructor's view while it runs, constructed / cloned / innerHTML-parsed elements, template
// content, a throwing constructor, seen by matches(), querySelectorAll() and getComputedStyle().
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createEnvironment } from '../src/runtime/index.mjs';

const dir = new URL('../crates/turbo-dom-browser-env/src/', import.meta.url);
const read = (name) => readFileSync(new URL(name, dir));
const cases = JSON.parse(read('custom_elements_defined_cases.json'));
const control = JSON.parse(read('custom_elements_defined_chrome156.json'));
const probe = read('custom_elements_defined_probe.js').toString('utf8');

test(':defined follows the custom element state like the Chrome 156 control', () => {
  assert.ok(control.browser.startsWith('Chrome/156.'));
  assert.deepEqual(control.errors, []);
  assert.equal(control.probeSha256, createHash('sha256').update(probe).digest('hex'));
  assert.equal(cases.length, 1);
  const { window } = createEnvironment(cases[0].html);
  const globals = {
    document: window.document,
    customElements: window.customElements,
    HTMLElement: window.HTMLElement,
    HTMLButtonElement: window.HTMLButtonElement,
    getComputedStyle: (el) => window.getComputedStyle(el),
  };
  const run = new Function(...Object.keys(globals), `return ${probe.trim().replace(/;$/, '')}`);
  const actual = run(...Object.values(globals));
  const expected = { ...control.result[0].result };
  // The one deviation: the JS runtime has no per-interface element classes, so the element
  // a throwing constructor leaves behind (HTMLUnknownElement in Chrome) is a plain element.
  // It is still an element in the "failed" state (`fresh.failedAfter`), not an exception.
  assert.equal(expected.failedAfterKind, '[object HTMLUnknownElement]');
  assert.match(actual.failedAfterKind, /^\[object /);
  delete expected.failedAfterKind;
  delete actual.failedAfterKind;
  assert.deepEqual(actual, expected);
});
