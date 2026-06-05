import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

const fresh = () => createEnvironment('<!doctype html><html><body><div id=root></div></body></html>');

test('dataset: camelCase <-> data-* both directions + delete + enumerate', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  el.dataset.fooBar = 'x';
  assert.equal(el.getAttribute('data-foo-bar'), 'x');
  el.setAttribute('data-baz', 'y');
  assert.equal(el.dataset.baz, 'y');
  assert.deepEqual(Object.keys(el.dataset).sort(), ['baz', 'fooBar']);
  delete el.dataset.baz;
  assert.equal(el.hasAttribute('data-baz'), false);
});

test('<select> selectedness: value, selectedIndex, set value, multiple', () => {
  const { document } = fresh();
  const sel = document.createElement('select');
  sel.innerHTML = '<option value=a>A</option><option value=b selected>B</option><option value=c>C</option>';
  assert.equal(sel.value, 'b');
  assert.equal(sel.selectedIndex, 1);
  sel.value = 'c';
  assert.equal(sel.value, 'c');
  assert.equal(sel.selectedIndex, 2);
  assert.equal(sel.options.length, 3);
  assert.equal(sel.selectedOptions[0].value, 'c');
});

test('<select> default-selects first option when none marked', () => {
  const { document } = fresh();
  const sel = document.createElement('select');
  sel.innerHTML = '<option value=a>A</option><option value=b>B</option>';
  assert.equal(sel.value, 'a');
  assert.equal(sel.selectedIndex, 0);
});

test('MutationObserver fires childList with added/removed nodes', async () => {
  const { document, window } = fresh();
  const root = document.getElementById('root');
  const records = [];
  const mo = new window.MutationObserver((recs) => records.push(...recs));
  mo.observe(root, { childList: true });
  const span = document.createElement('span');
  root.appendChild(span);
  await Promise.resolve(); await Promise.resolve();
  assert.equal(records.length, 1);
  assert.equal(records[0].type, 'childList');
  assert.equal(records[0].addedNodes[0], span);
  records.length = 0;
  root.removeChild(span);
  await Promise.resolve(); await Promise.resolve();
  assert.equal(records[0].removedNodes[0], span);
});

test('MutationObserver attributes + characterData + subtree + oldValue', async () => {
  const { document, window } = fresh();
  const root = document.getElementById('root');
  const child = document.createElement('p');
  child.textContent = 'hi';
  root.appendChild(child);
  const records = [];
  const mo = new window.MutationObserver((recs) => records.push(...recs));
  mo.observe(root, { attributes: true, characterData: true, subtree: true, attributeOldValue: true });
  child.setAttribute('data-x', '1');
  child.setAttribute('data-x', '2');
  child.firstChild.data = 'bye';
  await Promise.resolve(); await Promise.resolve();
  const attrRecs = records.filter((r) => r.type === 'attributes');
  assert.equal(attrRecs.length, 2);
  assert.equal(attrRecs[1].oldValue, '1');
  assert.ok(records.some((r) => r.type === 'characterData'));
});

test('MutationObserver disconnect stops delivery + takeRecords drains', async () => {
  const { document, window } = fresh();
  const root = document.getElementById('root');
  let calls = 0;
  const mo = new window.MutationObserver(() => calls++);
  mo.observe(root, { childList: true });
  root.appendChild(document.createElement('a'));
  const pending = mo.takeRecords();
  assert.equal(pending.length, 1);     // drained synchronously
  mo.disconnect();
  root.appendChild(document.createElement('b'));
  await Promise.resolve(); await Promise.resolve();
  assert.equal(calls, 0);              // callback never ran (drained + disconnected)
});

test('FileReader reads text async', async () => {
  const { window } = fresh();
  const fr = new window.FileReader();
  const done = new Promise((res) => { fr.onload = () => res(fr.result); });
  fr.readAsText(new window.Blob(['hello world']));
  assert.equal(await done, 'hello world');
});

test('customElements define/get/whenDefined', async () => {
  const { window } = fresh();
  class Widget {}
  const p = window.customElements.whenDefined('x-widget');
  window.customElements.define('x-widget', Widget);
  assert.equal(window.customElements.get('x-widget'), Widget);
  assert.equal(await p, Widget);
});

test('canvas.getContext + attachShadow', () => {
  const { document } = fresh();
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d');
  assert.ok(ctx);
  assert.equal(ctx.measureText('abc').width, 18);
  ctx.fillRect(0, 0, 10, 10); // no-op, must not throw

  const host = document.createElement('div');
  const root = host.attachShadow({ mode: 'open' });
  assert.equal(host.shadowRoot, root);
  root.appendChild(document.createElement('span'));
  assert.equal(root.querySelector('span').localName, 'span');
});

test('form submit + reset events; submit button triggers form submit', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<form><input name=q><button type=submit>Go</button></form>';
  const form = root.querySelector('form');
  let submitted = false, reset = false;
  form.addEventListener('submit', (e) => { submitted = true; e.preventDefault(); });
  form.addEventListener('reset', () => { reset = true; });
  root.querySelector('button').click();
  assert.equal(submitted, true);
  form.reset();
  assert.equal(reset, true);
  assert.ok(form.elements.length >= 2);
});

test('focus/blur dispatch focus + focusin/focusout', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<input id=a><input id=b>';
  const a = document.getElementById('a');
  const events = [];
  a.addEventListener('focus', () => events.push('focus'));
  document.getElementById('root').addEventListener('focusin', () => events.push('focusin'));
  a.focus();
  assert.deepEqual(events, ['focus', 'focusin']);
  assert.equal(document.activeElement, a);
});
