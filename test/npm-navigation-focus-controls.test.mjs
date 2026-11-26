import test from 'node:test';
import assert from 'node:assert/strict';
import { installGlobals } from '../src/environment/install.mjs';

test('traversals deliver destination state before hashchange; writes and out-of-range traversals stay silent', async () => {
  const w = {};
  const env = installGlobals(w);
  try {
    const seen = [];
    w.onpopstate = e => seen.push([e instanceof w.PopStateEvent, e.state]);
    w.addEventListener('hashchange', () => seen.push('hash'));
    w.history.pushState(1, '', '#a');
    w.history.replaceState(2, '', '#a');
    w.history.pushState(3, '', '#b');
    w.history.go(5);
    w.history.back();
    w.history.forward();
    assert.deepEqual(seen, []);
    await new Promise(resolve => setTimeout(resolve, 10));
    assert.deepEqual(seen, [[true, 2], 'hash', [true, 3], 'hash']);
    assert.equal(new w.PopStateEvent('popstate').state, null);
  } finally { env.teardown(); }
});

test('textarea defaults follow child text, dirty value and form reset', () => {
  const env = installGlobals({}, { html: '<form><textarea value="ignored">seed</textarea></form>' });
  try {
    const t = env.window.document.querySelector('textarea');
    assert.equal(t.defaultValue, 'seed');
    t.defaultValue = 'new';
    assert.deepEqual([t.value, t.textContent], ['new', 'new']);
    t.value = 'dirty';
    t.defaultValue = 'next';
    assert.equal(t.value, 'dirty');
    env.window.document.querySelector('form').reset();
    assert.equal(t.value, 'next');
    t.defaultValue = 'after';
    assert.equal(t.value, 'after');
  } finally { env.teardown(); }
});

test('disabled controls preserve focus, including the disabled fieldset legend exception', () => {
  const env = installGlobals({}, { html: '<fieldset disabled><legend><input></legend><input></fieldset><button disabled></button>' });
  try {
    const d = env.window.document, [first, blocked] = d.querySelectorAll('input');
    first.focus();
    blocked.focus();
    d.querySelector('button').focus();
    assert.equal(d.activeElement, first);
  } finally { env.teardown(); }
});

test('each shadow root sees its focused descendant retargeted across nested boundaries', () => {
  const env = installGlobals({});
  try {
    const d = env.window.document, host = d.createElement('div');
    d.body.append(host);
    const outer = host.attachShadow({mode:'open'}), nested = d.createElement('div');
    outer.append(nested);
    const closed = nested.attachShadow({mode:'closed'}), input = d.createElement('input');
    closed.append(input);
    input.focus();
    assert.equal(d.activeElement, host);
    assert.equal(outer.activeElement, nested);
    assert.equal(closed.activeElement, input);
    host.remove();
    assert.equal(closed.activeElement, null);
    assert.equal(d.activeElement, d.body);
  } finally { env.teardown(); }
});
