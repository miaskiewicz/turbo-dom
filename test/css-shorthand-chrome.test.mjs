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
const KNOWN_GAPS = new Set([
]);

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

test("the module's shorthand table (shorthandNames / longhands / canonical) is Chrome 156's", async () => {
  // the same assertion as crates/turbo-dom/tests/css_shorthand_chrome.rs makes of rtdom::css_shorthand
  const { shorthandNames, longhands, canonical } = await import('../src/runtime/css-shorthand.mjs');
  const names = shorthandNames();
  assert.deepEqual(Object.fromEntries(names.map((name) => [name, longhands(name)])), control.result[0].result.table);
  assert.equal(new Set(names).size, names.length);
  assert.equal(canonical('-webkit-flex'), 'flex');
  assert.equal(longhands('color'), null);
});

test('an unterminated url() ending with a lone backslash uses the U+FFFD replacement (CSS Syntax escape fallback)', async () => {
  // CSS Syntax Level 3 § 4.3.7 "consume an escaped code point": when a `\` is immediately
  // followed by EOF, the escape returns U+FFFD. tokReadUrlEscape covers that fallback for
  // the url() token (`url(abc\` with no closing `)`), which Chrome also reads as `url(abc�)`.
  const { longhandValue } = await import('../src/runtime/css-shorthand.mjs');
  assert.equal(longhandValue('background-image', 'url(abc\\').text, 'url("abc�")');
});

test('an overflowing length reads back clamped to the largest float like Chrome 156 (#262)', () => {
  // Chrome 156: `el.style.borderRadius = "1e39px"` (and `1` followed by 400 zeros) reads
  // back `3.40282e+38px`, as the Rust port (crates/turbo-dom/tests/css_shorthand_api.rs).
  const { document } = createEnvironment('<div></div>');
  for (const value of ['1e39px', `1${'0'.repeat(400)}px`]) {
    const el = document.createElement('div');
    el.style.borderRadius = value;
    assert.equal(el.style.borderTopLeftRadius, '3.40282e+38px', value);
    assert.equal(el.style.borderRadius, '3.40282e+38px', value);
  }
});

test('Object.prototype member names are ordinary function names, not special-function handlers', { todo: 'fails until SPECIAL_FUNCTION_HANDLERS has a null prototype' }, () => {
  // `__proto__(1)` / `constructor(x)` / `toString(1)` must take the unknown-function path like
  // any other name (`foo(…)`) instead of reaching an Object.prototype member through the
  // special-function table (turbo-dom#389 review).
  const { document } = createEnvironment('<div></div>');
  const read = (prop, value) => {
    const el = document.createElement('div');
    el.style[prop] = value;
    return el.style.cssText;
  };
  for (const name of ['__proto__', 'constructor', 'toString', 'hasOwnProperty', 'valueOf']) {
    for (const [prop, args] of [['color', '1'], ['willChange', 'x'], ['transitionTimingFunction', '1'], ['width', '1px']]) {
      assert.equal(read(prop, `${name}(${args})`), read(prop, `foo(${args})`).replaceAll('foo', name), `${prop}: ${name}(${args})`);
    }
  }
  const el = document.createElement('div');
  el.style.color = 'red';
  el.style.color = '__proto__(1)';
  assert.equal(el.style.color, 'red');
});
