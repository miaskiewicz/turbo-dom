import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';
import { cssFocusTarget, cssStateSnapshot, ensureCssState } from '../src/runtime/css-invalidation.mjs';

function environment() {
  return createEnvironment('<body><input id=first><button id=second>next</button></body>');
}

test('actual focus before tracking is distinct from activeElement body fallback and does not arm tracking', () => {
  const { document } = environment();
  const input = document.querySelector('#first');
  assert.equal(document.activeElement, document.body);
  assert.equal(cssFocusTarget(document), null);
  assert.equal(cssStateSnapshot(document), null);
  input.focus();
  assert.equal(cssFocusTarget(document), input);
  assert.equal(document.activeElement, input);
  assert.equal(cssStateSnapshot(document), null);
  ensureCssState(document);
  assert.equal(cssFocusTarget(document), input);
  assert.notEqual(cssStateSnapshot(document), null);
  assert.equal(document.querySelector(':focus'), null);
});

test('blur keeps public body fallback while body.focus records an explicit body target', () => {
  const { document } = environment();
  const input = document.querySelector('#first'),
    body = document.body;
  input.focus();
  ensureCssState(document);
  const focused = cssStateSnapshot(document),
    version = document.__version;
  input.blur();
  assert.equal(document.activeElement, body);
  assert.equal(cssFocusTarget(document), null);
  const blurred = cssStateSnapshot(document);
  assert.notEqual(blurred, focused);
  let focusEvents = 0;
  body.addEventListener('focus', () => focusEvents++);
  body.focus();
  assert.equal(cssFocusTarget(document), body);
  assert.equal(document.activeElement, body);
  assert.notEqual(cssStateSnapshot(document), blurred);
  assert.equal(focusEvents, 0); // Preserve the pre-existing equal-active early-return event policy.
  const explicit = cssStateSnapshot(document);
  body.focus();
  assert.equal(cssStateSnapshot(document), explicit);
  body.blur();
  assert.equal(cssFocusTarget(document), null);
  assert.equal(document.activeElement, body);
  assert.equal(document.__version, version);
});

test('focus transitions expose no target during blur and the new target during focus callbacks', () => {
  const { document } = environment();
  const first = document.querySelector('#first'),
    second = document.querySelector('#second');
  first.focus();
  ensureCssState(document);
  const old = cssStateSnapshot(document);
  let duringBlur, duringFocus;
  first.addEventListener('blur', () => {
    duringBlur = cssFocusTarget(document);
    assert.notEqual(cssStateSnapshot(document), old);
  });
  second.addEventListener('focus', () => {
    duringFocus = cssFocusTarget(document);
  });
  second.focus();
  assert.equal(duringBlur, null);
  assert.equal(duringFocus, second);
  assert.equal(cssFocusTarget(document), second);
  const token = cssStateSnapshot(document);
  second.focus();
  assert.equal(cssStateSnapshot(document), token);
});

test('reset clears actual focus while keeping lazy tracking and document identity semantics', () => {
  const env = environment();
  const { document } = env;
  document.querySelector('#first').focus();
  env.reset();
  assert.equal(cssFocusTarget(document), null);
  assert.equal(cssStateSnapshot(document), null);
  ensureCssState(document);
  document.body.focus();
  const previous = cssStateSnapshot(document);
  env.reset();
  assert.equal(env.document, document);
  assert.equal(cssFocusTarget(document), null);
  assert.notEqual(cssStateSnapshot(document), previous);
});

test('foreign/proxy owners and replaced active-field getters yield Unknown without executing guest getters', () => {
  const { document } = environment();
  const first = document.querySelector('#first');
  for (const unknown of [{}, null, 3, () => {}]) assert.equal(cssFocusTarget(unknown), undefined);
  const proxy = new Proxy(document, {
    get() {
      assert.fail('document proxy getter');
    },
    getOwnPropertyDescriptor() {
      assert.fail('document proxy descriptor');
    },
  });
  assert.equal(cssFocusTarget(proxy), undefined);
  first.focus();
  Object.defineProperty(document, '__active', {
    configurable: true,
    get() {
      assert.fail('active getter');
    },
  });
  assert.equal(cssFocusTarget(document), undefined);
  Object.defineProperty(document, '__active', { configurable: true, writable: true, value: 17 });
  assert.equal(cssFocusTarget(document), undefined);
  Object.defineProperty(document, '__active', { configurable: true, writable: true, value: null });
  assert.equal(cssFocusTarget(document), null);
});
