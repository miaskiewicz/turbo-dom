(function () {
  'use strict';
  // HTMLHyperlinkElementUtils on HTMLAnchorElement and HTMLAreaElement (hyperlink_utils.rs).
  const g = globalThis, ns = g[Symbol.for('ts')], native = g.__hyperlinkNative;
  delete g.__hyperlinkNative;
  const make = ns.makeNativeFunction, apply = Reflect.apply, define = Object.defineProperty;
  const setAttribute = g.Element.prototype.setAttribute;
  const string = value => { if (typeof value === 'symbol') throw new TypeError('Cannot convert a Symbol to a string'); return String(value); };
  const components = ['origin', 'protocol', 'username', 'password', 'host', 'hostname', 'port', 'pathname', 'search', 'hash'];
  for (const [name, tag] of [['HTMLAnchorElement', 'a'], ['HTMLAreaElement', 'area']]) {
    const proto = g[name] && g[name].prototype;
    if (!proto) continue;
    const brand = ns.domAccessor('nodeType', false, 'HTML:' + tag);
    const href = Object.getOwnPropertyDescriptor(proto, 'href');
    for (const component of components) {
      const descriptor = { enumerable: true, configurable: true, get: make(function () { apply(brand, this, []); return native('get', this, component); }, 'get ' + component, 0) };
      descriptor.set = component === 'origin' ? undefined :  make(function (value) {
        apply(brand, this, []);
        value = string(value);
        const next = native('set', this, component, value);
        if (next !== null) apply(setAttribute, this, ['href', next]);
      }, 'set ' + component, 1);
      define(proto, component, descriptor);
    }
    define(proto, 'toString', { value: make(function toString() { apply(brand, this, []); return apply(href.get, this, []); }, 'toString', 0), writable: true, enumerable: true, configurable: true });
  }
})();
