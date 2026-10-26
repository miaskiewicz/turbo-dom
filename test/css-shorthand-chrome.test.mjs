// The npm runtime's `element.style` shorthands (src/runtime/css-shorthand.mjs, the lockstep
// port of turbo-dom's rtdom::css_shorthand) against the Chrome 156 control that gates the Rust
// module and the V8 binding: crates/turbo-dom/tests/fixtures/css_shorthand_chrome156.json
// (miaskiewicz/turbo-dom#90). The committed probe runs against a createEnvironment() document
// of the control's cases page; every scenario (cssText, longhands, priorities, shorthand
// values, return values, re-assembly from longhands) must equal Chrome's.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createEnvironment } from '../src/runtime/index.mjs';

const read = (name) => readFileSync(new URL(`../crates/turbo-dom/tests/fixtures/${name}`, import.meta.url), 'utf8');
const probe = read('css-shorthand-probe.js');
const casesText = read('css-shorthand-cases.json');
const control = JSON.parse(read('css_shorthand_chrome156.json'));
const sha = (text) => createHash('sha256').update(text).digest('hex');

// Scenarios whose npm result differs from Chrome for a documented reason, by id.
const KNOWN_GAPS = new Set([]);

test('element.style shorthands expand and serialize like Chrome 156 (turbo-dom#90)', () => {
  assert.equal(control.browser, 'Chrome/156.0.8078.4');
  assert.deepEqual(control.errors, []);
  assert.equal(control.probeSha256, sha(probe));
  assert.equal(control.casesSha256, sha(casesText));
  const { window, document } = createEnvironment(JSON.parse(casesText)[0].html);
  const actual = new Function('document', 'CSSStyleSheet', `return ${probe.trim().replace(/;$/, '')}`)(document, window.CSSStyleSheet);
  const expected = control.result[0].result;
  const failures = [];
  for (const key of ['table', 'aliases']) {
    try { assert.deepEqual(actual[key], expected[key]); } catch { failures.push(key); }
  }
  // The npm CSSOM is minimal by design (CLAUDE.md): a CSSStyleRule keeps its authored text and
  // has no `style` declaration, so the probe's rule rows record that instead of Chrome's.
  assert.deepEqual(actual.rules, expected.rules.map((row) => [row[0], 'TypeError']));
  assert.equal(actual.results.length, expected.results.length);
  const failed = [];
  expected.results.forEach((row, index) => {
    const ours = actual.results[index];
    assert.equal(ours.id, row.id);
    try { assert.deepEqual(ours, row); } catch {
      failed.push(row.id);
      if (!KNOWN_GAPS.has(row.id)) failures.push(`${row.id}\n  npm:    ${JSON.stringify(ours)}\n  chrome: ${JSON.stringify(row)}`);
    }
  });
  const stale = [...KNOWN_GAPS].filter((id) => !failed.includes(id));
  assert.deepEqual(stale, [], 'stale KNOWN_GAPS');
  assert.deepEqual(failures, [], `${failures.length} of ${expected.results.length} scenarios differ from Chrome`);
});
