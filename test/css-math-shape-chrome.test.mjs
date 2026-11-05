// The npm runtime's element.style spelling of tree-counting functions in math expressions and
// of shape() (src/runtime/css-shorthand.mjs, the lockstep port of rtdom::css_shorthand) against
// the Chrome 156 control crates/turbo-dom/tests/fixtures/css_math_shape_chrome156.json
// (miaskiewicz/turbo-dom#338). A value Chrome rejects that no grammar here knows is kept, in the
// context-free spelling, as in css-longhand-chrome.test.mjs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createEnvironment } from '../src/runtime/index.mjs';

const read = (name) => readFileSync(new URL(`../crates/turbo-dom/tests/fixtures/${name}`, import.meta.url), 'utf8');
const probe = read('css-longhand-probe.js');
const casesText = read('css-math-shape-cases.json');
const control = JSON.parse(read('css_math_shape_chrome156.json'));
const sha = (text) => createHash('sha256').update(text).digest('hex');

// Values Chrome rejects for properties this runtime has no grammar for, kept as spelled.
const KEPT = [];

test('element.style spells tree-counting math and shape() like Chrome 156 (turbo-dom#338)', () => {
  assert.match(control.browser, /^Chrome\/156\.0\.8078\./);
  assert.deepEqual(control.errors, []);
  assert.equal(control.probeSha256, sha(probe));
  assert.equal(control.casesSha256, sha(casesText));
  const { document } = createEnvironment(JSON.parse(casesText)[0].html);
  const actual = new Function('document', `return ${probe.trim().replace(/;$/, '')}`)(document);
  const expected = control.result[0].result;
  assert.equal(actual.length, expected.length);
  const failures = [], kept = [];
  expected.forEach((row, index) => {
    const ours = actual[index];
    const id = `${row[0]}|${row[1]}`;
    if (row[2] === '' && ours[2] !== '') { kept.push(id); return; }
    try { assert.deepEqual(ours, row); } catch { failures.push(`${id}\n  npm:    ${JSON.stringify(ours)}\n  chrome: ${JSON.stringify(row)}`); }
  });
  assert.deepEqual(failures, [], `${failures.length} of ${expected.length} pairs differ from Chrome`);
  assert.deepEqual(kept, KEPT);
});
