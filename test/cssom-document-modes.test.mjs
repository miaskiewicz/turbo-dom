import assert from 'node:assert/strict';
import test from 'node:test';
import { createEnvironment } from '../src/runtime/index.mjs';
import { CSSStyleSheet, CSSStyleRule, cssSheetSnapshot } from '../src/runtime/cssom.mjs';
import { ensureCssState, cssStateSnapshot } from '../src/runtime/css-invalidation.mjs';
import { captureCssSources } from '../src/runtime/css-sources.mjs';

test('prearm/public/constructed stores retain original plain identity forever; other documents do not arm them', () => {
  const first = createEnvironment('<style></style>');
  const old = first.document.querySelector('style').sheet;
  old.insertRule('.old{}');
  const rule = old.cssRules[0],
    list = old.cssRules;
  ensureCssState(first.document);
  assert.equal(cssSheetSnapshot(old), null);
  const fresh = createEnvironment('<style></style>');
  assert.equal(cssSheetSnapshot(fresh.document.querySelector('style').sheet), null);
  assert.equal(cssSheetSnapshot(new CSSStyleSheet(first.document.querySelector('style'))), null);
  const constructed = new CSSStyleSheet();
  constructed.insertRule('.standalone{}');
  first.document.adoptedStyleSheets = [constructed];
  assert.equal(cssSheetSnapshot(constructed), null);
  assert.equal(old.cssRules, list);
  assert.equal(old.cssRules[0], rule);
  assert.equal(Object.getPrototypeOf(old), CSSStyleSheet.prototype);
  assert.equal(Object.getPrototypeOf(rule), CSSStyleRule.prototype);
  assert.equal(CSSStyleRule.length, 1);
  assert.equal(CSSStyleSheet.length, 0);
  const late = first.document.createElement('style');
  first.document.head.append(late);
  late.sheet.insertRule('.late{}');
  assert.equal(cssSheetSnapshot(late.sheet).rulesTrusted, true);
});
test('unobserved CSSOM text edits after arm force exact-source cache misses despite unchanged document stamps', () => {
  const env = createEnvironment('<style></style><div id=target></div>');
  const owner = env.document.querySelector('style'),
    sheet = owner.sheet;
  const red = '#target{color:red}',
    blue = '#target{color:blue}';
  sheet.insertRule(red);
  ensureCssState(env.document);
  const beforeStamp = cssStateSnapshot(env.document);
  const before = captureCssSources(env.document);
  assert.deepEqual(captureCssSources(env.document).fingerprint, before.fingerprint);
  sheet.cssRules[0].cssText = blue;
  assert.equal(cssStateSnapshot(env.document), beforeStamp);
  assert.equal(cssSheetSnapshot(sheet), null);
  const after = captureCssSources(env.document);
  assert.notDeepEqual(after.fingerprint, before.fingerprint);
  assert.deepEqual(captureCssSources(env.document).fingerprint, after.fingerprint);
  sheet.disabled = true;
  assert.notDeepEqual(captureCssSources(env.document).fingerprint, after.fingerprint);
  sheet.disabled = false;
  sheet.media.push('print');
  assert.notDeepEqual(captureCssSources(env.document).fingerprint, after.fingerprint);
});
test('sheet mode selection never reads overridden ownerDocument getters; borrowed and proxied plain errors stay native', () => {
  const env = createEnvironment('<style></style>');
  const owner = env.document.querySelector('style');
  let reads = 0;
  Object.defineProperty(owner, 'ownerDocument', {
    configurable: true,
    get() {
      reads++;
      throw Error('unexpected owner getter');
    },
  });
  const sheet = owner.sheet;
  assert.equal(reads, 0);
  assert.equal(cssSheetSnapshot(sheet), null);
  const plain = new CSSStyleSheet();
  const proxy = new Proxy(plain, {});
  assert.throws(() => proxy.insertRule('.a{}'), TypeError);
  assert.equal(plain.cssRules[0].cssText, '.a{}');
  const foreign = { cssRules: [] };
  assert.throws(() => CSSStyleSheet.prototype.insertRule.call(foreign, '.b{}'), TypeError);
  assert.equal(foreign.cssRules[0].cssText, '.b{}');
  const error = {};
  const exotic = new Proxy(owner, {
    getOwnPropertyDescriptor() {
      throw error;
    },
  });
  assert.equal(exotic.sheet, sheet);
  const other = env.document.createElement('style');
  const exoticNew = new Proxy(other, {
    getOwnPropertyDescriptor() {
      throw error;
    },
  });
  assert.throws(
    () => exoticNew.sheet,
    (value) => value === error,
  );
});
