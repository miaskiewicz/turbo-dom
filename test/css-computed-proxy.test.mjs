import assert from 'node:assert/strict';
import test from 'node:test';
import { createComputedStyleProxy } from '../src/runtime/css-computed-proxy.mjs';
const units = (text) => Array.from({ length: text.length }, (_, index) => text.charCodeAt(index));
const row = () => ({
  values: { color: units('rgb(1, 2, 3)'), 'font-size': units('16px'), 'margin-left': units('0px') },
  custom: { '--Case': units('UP'), '--case': units('down') },
});
test('retains published proxy surface without performing value policy', () => {
  const proxy = createComputedStyleProxy(row(), 42);
  assert.equal(proxy.__v, 42);
  assert.match(proxy.__honest, /owned serialized Rust/);
  assert.equal(proxy.color, 'rgb(1, 2, 3)');
  assert.equal(proxy.fontSize, '16px');
  assert.equal(proxy['margin-left'], '0px');
  assert.equal(proxy.getPropertyValue('COLOR'), 'rgb(1, 2, 3)');
  assert.equal(proxy.getPropertyValue('missing'), '');
  assert.equal(proxy.unknown, '');
  assert.equal(proxy.length, 5);
  assert.deepEqual([...proxy], ['color', 'font-size', 'margin-left', '--Case', '--case']);
  assert.equal(proxy.item(0), 'color');
  assert.equal(proxy.item('1'), 'font-size');
  assert.equal(proxy.item(-1), '');
  assert.equal(proxy.item(100), '');
  assert.equal(proxy.item('length'), 5); // Preserve legacy array-index coercion.
  assert.equal(proxy.getPropertyPriority('color'), '');
  assert.equal(proxy.getPropertyPriority(Symbol()), '');
  assert.equal(proxy[Symbol.toStringTag], undefined);
  assert.equal(proxy[Symbol('unknown')], undefined);
  assert.equal(Object.prototype.toString.call(proxy), '[object Object]');
  assert.deepEqual(Object.keys(proxy), []);
  const error = {};
  assert.throws(
    () =>
      proxy.getPropertyValue({
        toString() {
          throw error;
        },
      }),
    (value) => value === error,
  );
  assert.throws(
    () =>
      proxy.item({
        toString() {
          throw error;
        },
      }),
    (value) => value === error,
  );
});
test('custom names retain case while final values are copied verbatim', () => {
  const proxy = createComputedStyleProxy(row(), 0);
  assert.equal(proxy.getPropertyValue('--Case'), 'UP');
  assert.equal(proxy.getPropertyValue('--case'), 'down');
  assert.equal(proxy.getPropertyValue('--CASE'), '');
  assert.equal(proxy['--Case'], 'UP');
  assert.equal(proxy['--case'], 'down');
  const raw = createComputedStyleProxy(
    { values: { 'font-family': units('a,b'), 'margin-left': units('0'), color: units('red') } },
    0,
  );
  assert.equal(raw.fontFamily, 'a,b');
  assert.equal(raw.marginLeft, '0');
  assert.equal(raw.color, 'red');
});
test('exact UTF16 retains lone units, marker lookalikes and long strings without native getters', () => {
  const text = '\ud800a\udfff\u0000actual-marker-\\uD800' + 'x'.repeat(20000);
  const result = { values: { content: units(text) }, custom: { '--typed': new Uint16Array([0xd800, 65, 0xdfff]) } };
  const proxy = createComputedStyleProxy(result, 1);
  assert.equal(proxy.content, text);
  assert.equal(proxy.getPropertyValue('--typed'), '\ud800A\udfff');
});
test('snapshot owns copied primitives after input/model disposal and performs zero callbacks on reads', () => {
  let callbacks = 0;
  const result = row();
  Object.defineProperty(result.values, 'width', {
    enumerable: true,
    get() {
      callbacks++;
      return units('20px');
    },
  });
  const proxy = createComputedStyleProxy(result, 1);
  assert.equal(callbacks, 1);
  result.values.color.fill(0);
  result.custom['--Case'].fill(0);
  result.values = {};
  result.custom = {};
  Object.freeze(result);
  for (let index = 0; index < 10; index++) {
    assert.equal(proxy.color, 'rgb(1, 2, 3)');
    assert.equal(proxy.width, '20px');
    assert.equal(proxy.getPropertyValue('--Case'), 'UP');
    [...proxy];
    proxy.item(1);
    proxy.length;
  }
  assert.equal(callbacks, 1);
});
test('invalid transport is rejected before returning a snapshot without coercion', () => {
  for (const value of ['red', null, {}, [-1], [65536], [1.5], [NaN], new Array(1)]) {
    assert.throws(() => createComputedStyleProxy({ values: { color: value } }, 0), TypeError);
  }
  const error = {};
  assert.throws(
    () =>
      createComputedStyleProxy(
        {
          get values() {
            throw error;
          },
        },
        0,
      ),
    (value) => value === error,
  );
  const empty = createComputedStyleProxy({}, 0);
  assert.equal(empty.length, 0);
  assert.deepEqual([...empty], []);
});
