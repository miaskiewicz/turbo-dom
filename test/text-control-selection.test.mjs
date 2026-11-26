import test from 'node:test';
import assert from 'node:assert/strict';
import { JSDOM } from 'jsdom';
import { installGlobals } from '../src/environment/install.mjs';

const state = el => [el.selectionStart, el.selectionEnd, el.selectionDirection];

test('installed textarea/input selections follow jsdom across value writes, focus, ranges and unsupported types', () => {
  const target = {};
  installGlobals(target, { url: 'http://localhost/' });
  const oracle = new JSDOM('');
  function exercise(document) {
    const result = [];
    for (const tag of ['textarea', 'input']) {
      const el = document.createElement(tag);
      result.push(state(el));
      el.value = 'hello'; document.body.append(el); el.focus();
      result.push(state(el));
      el.setSelectionRange(1, 4, 'backward'); el.value = 'hello';
      result.push(state(el));
      el.selectionStart = 5; result.push(state(el));
      el.selectionEnd = 2; result.push(state(el));
      el.selectionDirection = 'forward'; result.push(state(el));
      el.selectionDirection = 'invalid'; result.push(state(el));
      el.setSelectionRange(4, 1, 'backward'); result.push(state(el));
      el.value = 'x😀'; result.push(state(el));
      el.select(); result.push(state(el));
      el.value = ''; result.push(state(el));
      el.value = null; result.push([el.value, ...state(el)]);
      el.value = undefined; result.push([el.value, ...state(el)]); el.remove();
    }
    const inp = document.createElement('input');
    for (const type of ['search', 'url', 'tel', 'password', 'unknown-type']) {
      inp.type = type; inp.value = type; result.push(state(inp));
    }
    for (const type of ['checkbox', 'number', 'email']) {
      inp.type = type; result.push(state(inp));
      for (const assign of [() => { inp.selectionStart = 1; }, () => { inp.selectionEnd = 1; },
          () => { inp.selectionDirection = 'backward'; }, () => inp.setSelectionRange(0, 1)]) {
        assert.throws(assign, { name: 'InvalidStateError' });
      }
    }
    inp.removeAttribute('type'); result.push(state(inp));
    const ta = document.createElement('textarea');
    ta.value = 'x\r\n😀'; result.push([ta.value, ...state(ta)]);
    ta.value = null; result.push([ta.value,...state(ta)]);
    return result;
  }
  try { assert.deepEqual(exercise(target.document), exercise(oracle.window.document)); }
  finally { oracle.window.close(); }
});

test('selection replacement preserves endpoints or selects the explicit replacement mode', async () => {
  const target = {};
  installGlobals(target);
  const el = target.document.createElement('textarea');
  target.document.body.append(el);
  let selectEvents = 0;
  el.addEventListener('select', () => { selectEvents++; });
  el.value = 'abcdef';
  el.setSelectionRange(0, 5); el.setRangeText('X', 1, 4); assert.deepEqual(state(el), [0, 3, 'none']);
  el.value = 'abcdef';
  el.setSelectionRange(2, 3); el.setRangeText('X', 1, 4); assert.deepEqual(state(el), [1, 2, 'none']);
  for (const [mode, expected] of [['select', [0, 2]], ['start', [0, 0]], ['end', [2, 2]]]) {
    el.setRangeText('xy', 0, 1, mode); assert.deepEqual(state(el).slice(0, 2), expected);
  }
  assert.throws(() => el.setSelectionRange(0), TypeError);
  assert.throws(() => el.setRangeText(), TypeError);
  assert.throws(() => el.setRangeText('x', 1), TypeError);
  assert.throws(() => el.setRangeText('x', 3, 1), {name:'IndexSizeError'});
  assert.throws(() => el.setRangeText('x', 0, 1, 'unknown'), TypeError);
  // This runtime shares the Element prototype. Setting value on another
  // element still performs its existing string conversion without a caret.
  const other = target.document.createElement('div');
  let conversions = 0;
  other.value = { toString() { conversions++; return 'other'; } };
  assert.equal(conversions, 1);
  assert.equal(other.selectionStart, null);
  const input = target.document.createElement('input');
  input.type = 'checkbox'; input.select();
  assert.equal(selectEvents, 0);
  await new Promise(done => setTimeout(done, 5));
  assert.ok(selectEvents > 0);
});
