// The JS runtime against the Chrome 156 control for miaskiewicz/turbo-dom#29
// (test/fixtures/dom_adoption_reactions_{probe.js,cases.json,chrome156.json}, recorded with
// crates/turbo-dom-browser-env/scripts/select-element-chrome-control.mjs):
//   1. createElementNS in any namespace (null / unknown namespaces, prefixes, name checks),
//   2. insertion across documents adopts the node (old document no longer reports it),
//   3. template contents live in an inert document: no upgrades until inserted into the
//      document; template content / innerHTML / cloning,
//   4. custom element reactions are queued per element and run when the DOM call returns.
// The probe runs unchanged; the whole result (including the reaction log) must match.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createEnvironment } from '../src/runtime/index.mjs';

const dir = new URL('./fixtures/', import.meta.url);
const read = (name) => readFileSync(new URL(name, dir));
const cases = JSON.parse(read('dom_adoption_reactions_cases.json'));
const control = JSON.parse(read('dom_adoption_reactions_chrome156.json'));
const probe = read('dom_adoption_reactions_probe.js').toString('utf8');

// The one deviation: the JS runtime has no per-interface element classes — `HTMLElement` is
// `Element` — so `el instanceof HTMLElement` is true for every element (index 5 of a view).
function dropInstanceofHTMLElement(expected, actual) {
  const views = (r) => [...Object.values(r.ns), r.nsClone, r.nsImport, r.nsTwoColons];
  for (const view of views(actual)) { assert.equal(view[5], true); view[5] = null; }
  for (const view of views(expected)) view[5] = null;
}

export function runProbe() {
  const { window } = createEnvironment(cases[0].html);
  const globals = {
    document: window.document,
    customElements: window.customElements,
    HTMLElement: window.HTMLElement,
    Element: window.Element,
    DOMParser: window.DOMParser,
  };
  const run = new Function(...Object.keys(globals), `return ${probe.trim().replace(/;$/, '')}`);
  return run(...Object.values(globals));
}

let memo;
function results() {
  if (memo) return memo;
  assert.ok(control.browser.startsWith('Chrome/156.'));
  assert.deepEqual(control.errors, []);
  assert.equal(control.probeSha256, createHash('sha256').update(probe).digest('hex'));
  assert.equal(control.casesSha256, createHash('sha256').update(read('dom_adoption_reactions_cases.json')).digest('hex'));
  assert.equal(cases.length, 1);
  const expected = structuredClone(control.result[0].result);
  const actual = runProbe();
  dropInstanceofHTMLElement(expected, actual);
  return (memo = { expected, actual });
}

// Each item compares its own keys of the result (and its probe section ran without throwing).
const ITEMS = {
  1: { sections: ['ns'], keys: ['ns', 'nsAttrs', 'nsClone', 'nsImport', 'nsSerialized', 'nsInner', 'nsErrors', 'nsTwoColons', 'nsByTag'] },
  2: { sections: ['adopt'], keys: ['adopt', 'adoptFragment', 'moverInOther', 'mover', 'otherCreate'] },
  3: { sections: ['template'], keys: ['inert', 'contentClone', 'upgradeInert', 'detached', 'connected', 'imported', 'templateInner', 'intoContent', 'templateClone', 'moveContent'] },
  4: { sections: ['reactions', 'crossDocument'], keys: ['cloneDeep', 'log'] },
  // pre-existing bugs found on the way: turbo-dom#33 (void serialization), #34 (insert before
  // itself), #35 (doctype clones), #37 (<meta>.content), #38 (shadow roots adopt); #36 (the
  // template element model) is item 3's `inert` / `templateInner` / `templateClone`
  extra: { sections: ['extra'], keys: ['foreignVoid', 'insertSelf', 'doctypeClone', 'contentProperty', 'shadowAdopt'] },
};
function check(item) {
  const { expected, actual } = results();
  const { sections, keys } = ITEMS[item];
  for (const name of sections) assert.equal(actual.errors[name], undefined, `section ${name} threw`);
  for (const key of keys) assert.deepEqual(actual[key], expected[key], key);
}

test('turbo-dom#29 item 1: createElementNS in any namespace matches Chrome 156', () => check(1));
test('turbo-dom#29 item 2: insertion across documents adopts like Chrome 156', () => check(2));
test('turbo-dom#29 item 3: template contents are inert until inserted, like Chrome 156', () => check(3));
test('turbo-dom#33-#38: serialization, insert before itself, doctype clones, meta.content, shadow adoption match Chrome 156', () => check('extra'));
