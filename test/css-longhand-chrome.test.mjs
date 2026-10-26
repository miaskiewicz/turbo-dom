// The npm runtime's element.style longhand spelling (src/runtime/css-shorthand.mjs
// `longhandValue`, the lockstep port of rtdom::css_shorthand) against the Chrome 156 control
// crates/turbo-dom/tests/fixtures/css_longhand_chrome156.json (miaskiewicz/turbo-dom#97).
// The npm runtime has no property grammar beyond css-shorthand.mjs: a value Chrome rejects that
// no grammar there knows is kept, in the context-free spelling.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createEnvironment } from '../src/runtime/index.mjs';

const read = (name) => readFileSync(new URL(`../crates/turbo-dom/tests/fixtures/${name}`, import.meta.url), 'utf8');
const probe = read('css-longhand-probe.js');
const casesText = read('css-longhand-cases.json');
const control = JSON.parse(read('css_longhand_chrome156.json'));
const sha = (text) => createHash('sha256').update(text).digest('hex');

// Pairs whose npm result differs from Chrome for a documented reason, by `name|value`.
const KNOWN_GAPS = new Set([]);
const KEPT = ['display|bogus'];

test('element.style longhands spell values like Chrome 156 (turbo-dom#97)', () => {
  assert.equal(control.browser, 'Chrome/156.0.8078.4');
  assert.deepEqual(control.errors, []);
  assert.equal(control.probeSha256, sha(probe));
  assert.equal(control.casesSha256, sha(casesText));
  const { document } = createEnvironment(JSON.parse(casesText)[0].html);
  const actual = new Function('document', `return ${probe.trim().replace(/;$/, '')}`)(document);
  const expected = control.result[0].result;
  assert.equal(actual.length, expected.length);
  const failures = [], failed = [], kept = [];
  expected.forEach((row, index) => {
    const ours = actual[index];
    const id = `${row[0]}|${row[1]}`;
    // A value Chrome rejects stays as written here (no longhand validation in this runtime).
    if (row[2] === '' && ours[2] !== '') {
      kept.push(id);
      assert.equal(ours[3], `${row[0] === 'word-wrap' ? 'overflow-wrap' : row[0]}: ${ours[2]};`, id);
      return;
    }
    try { assert.deepEqual(ours, row); } catch {
      failed.push(id);
      if (!KNOWN_GAPS.has(id)) failures.push(`${id}\n  npm:    ${JSON.stringify(ours)}\n  chrome: ${JSON.stringify(row)}`);
    }
  });
  assert.deepEqual([...KNOWN_GAPS].filter((id) => !failed.includes(id)), [], 'stale KNOWN_GAPS');
  assert.deepEqual(failures, [], `${failures.length} of ${expected.length} pairs differ from Chrome`);
  // The values Chrome rejects that this runtime keeps (no grammar for the property here).
  assert.deepEqual(kept, KEPT);
});
