(function () {
  'use strict';
  const g = globalThis, ns = g[Symbol.for('ts')], native = g.__htmlTextNative;
  delete g.__htmlTextNative;
  const make = ns.makeNativeFunction, DOMError = g.DOMException;
  const string = value => { if (value === null) return ''; if (typeof value === 'symbol') throw new TypeError('Cannot convert a Symbol to a string'); return String(value); };
  for (const [name, operation] of [['innerText', 'inner'], ['outerText', 'outer']]) {
    Object.defineProperty(g.HTMLElement.prototype, name, {
      enumerable: true, configurable: true,
      get: make(function () { return native('read', this); }, 'get ' + name, 0),
      set: make(function (value) { native('check', this); const error = native(operation, this, string(value)); if (typeof error === 'string') throw new DOMError('The element has no parent', error); }, 'set ' + name, 1),
    });
  }
})();
