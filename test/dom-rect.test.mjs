import assert from 'node:assert/strict';
import { test } from 'node:test';
import { JSDOM } from 'jsdom';
import { installGlobals } from '../src/environment/install.mjs';

function exercise({ DOMRect, DOMRectReadOnly }) {
  const rect = new DOMRect('5', 6, -2, -3);
  assert.ok(rect instanceof DOMRectReadOnly);
  assert.deepEqual(rect.toJSON(), { x: 5, y: 6, width: -2, height: -3, top: 3, right: 5, bottom: 6, left: 3 });
  const snapshot = rect.toJSON();
  Object.assign(rect, { x: 9, y: 8, width: '4', height: 2 });
  assert.deepEqual([rect.top, rect.right, rect.bottom, rect.left], [8, 13, 10, 9]);
  assert.equal(snapshot.x, 5);
  const read = DOMRectReadOnly.fromRect(rect);
  assert.equal(read instanceof DOMRect, false);
  assert.throws(() => { read.x = 11; }, TypeError);
  assert.deepEqual(DOMRect.fromRect(read).toJSON(), rect.toJSON());
  const empty = new DOMRectReadOnly();
  assert.deepEqual(DOMRect.fromRect(null).toJSON(), empty.toJSON());
  assert.deepEqual(DOMRect.fromRect().toJSON(), empty.toJSON());
  assert.deepEqual(DOMRect.fromRect({ x: undefined }).toJSON(), empty.toJSON());
  assert.equal(Object.prototype.toString.call(rect), '[object DOMRect]');
  assert.equal(Object.prototype.toString.call(read), '[object DOMRectReadOnly]');
  assert.throws(() => DOMRect.fromRect(5), TypeError);
  assert.throws(() => new DOMRect(4n), TypeError);
  assert.throws(() => Object.getOwnPropertyDescriptor(DOMRect.prototype, 'x').set.call(read, 1), TypeError);
  assert.throws(() => DOMRectReadOnly.prototype.toJSON.call({}), TypeError);
  assert.ok(Number.isNaN(new DOMRect(3, 0, NaN, 1).right));
  const order = [];
  const dict = new Proxy(() => {}, {
    get(_target, key) {
      order.push(key);
      return { valueOf() { order.push('convert'); return 1; } };
    },
  });
  DOMRect.fromRect(dict);
  assert.deepEqual(order, ['height', 'convert', 'width', 'convert', 'x', 'convert', 'y', 'convert']);
}
test('installed rectangle constructors and dictionary conversions agree with jsdom', () => {
  const globals = {};
  const { teardown } = installGlobals(globals);
  const oracle = new JSDOM('').window;
  try {
    exercise(globals);
    exercise(oracle);
  } finally {
    oracle.close();
    teardown();
  }
});

test('element and range rectangles have fresh mutable DOMRect identity', () => {
  const globals = {};
  const { teardown } = installGlobals(globals);
  try {
    const el = globals.document.createElement('div');
    const r = el.getBoundingClientRect();
    assert.equal(r.constructor.name, 'DOMRect');
    assert.equal(Object.getPrototypeOf(r), globals.DOMRect.prototype);
    const width = r.width;
    r.width = -50;
    assert.equal(el.getBoundingClientRect().width, width);
    assert.ok(el.getClientRects()[0] instanceof globals.DOMRect);
    assert.ok(globals.document.createRange().getBoundingClientRect() instanceof globals.DOMRect);
  } finally { teardown(); }
});
