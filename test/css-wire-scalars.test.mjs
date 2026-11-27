import test from 'node:test';
import assert from 'node:assert/strict';
import {encodeCssWire} from '../src/runtime/css-rust-wire.mjs';

test('CSS wire preserves JSON scalar spelling across IEEE values and lone surrogates', () => {
  const values = [null, true, false, -0, Number.MIN_VALUE, Number.MAX_VALUE,
    1e-7, 1e-6, 1e20, 1e21, '\ud800', '\udfff', '\ud800x\udfff', '\u0000"\\'];
  const view = new DataView(new ArrayBuffer(8));
  let seed = 1771;
  for (let index = 0; index < 10000; index++) {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    view.setUint32(0, seed);
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    view.setUint32(4, seed);
    const value = view.getFloat64(0);
    if (Number.isFinite(value)) values.push(value);
  }
  for (const value of values) assert.equal(encodeCssWire(value), JSON.stringify(value));
  assert.equal(encodeCssWire(values), JSON.stringify(values));
});

test('CSS wire scalar conversion ignores late String and number toJSON hooks', () => {
  const originalString = globalThis.String;
  const descriptor = Object.getOwnPropertyDescriptor(Number.prototype, 'toJSON');
  let calls = 0;
  const hook = () => { calls++; throw new Error('author scalar hook'); };
  try {
    globalThis.String = hook;
    Object.defineProperty(Number.prototype, 'toJSON', {configurable: true, value: hook});
    assert.equal(encodeCssWire([0, -0, 1e21, true, false, null]), '[0,0,1e+21,true,false,null]');
    assert.throws(() => encodeCssWire(1n), TypeError);
    assert.equal(calls, 0);
  } finally {
    globalThis.String = originalString;
    if (descriptor) Object.defineProperty(Number.prototype, 'toJSON', descriptor);
    else delete Number.prototype.toJSON;
  }
});
