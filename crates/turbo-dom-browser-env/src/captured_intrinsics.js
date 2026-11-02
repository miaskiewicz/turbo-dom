// Bootstrap's last step, before any page script runs: capture the engine's own EventTarget
// methods and `on*` IDL attribute setters, so the engine never reaches a page-replaced
// builtin or accessor on its insertion / handler paths (miaskiewicz/turbo-dom#172).
(function () {
  const g = globalThis, ns = __tsNs, apply = Reflect.apply, gopd = Object.getOwnPropertyDescriptor;
  const getProto = Object.getPrototypeOf, ownKeys = Reflect.ownKeys, mapGet = Map.prototype.get, mapSet = Map.prototype.set;
  const target = g.EventTarget.prototype;
  // Absent only when the namespace was frozen before bootstrap (install degrades).
  if (typeof ns.completeNodeIntrinsics === 'function') ns.completeNodeIntrinsics();
  ns.handlerAdd = target.addEventListener;
  ns.handlerRemove = target.removeEventListener;
  // Every interface prototype's own `on*` setters, keyed by prototype.
  const tables = new Map(), names = ownKeys(g);
  for (let i = 0; i < names.length; i++) {
    const name = names[i];
    if (typeof name !== 'string') continue;
    const global = gopd(g, name), ctor = global && global.value;
    if (typeof ctor !== 'function') continue;
    const prototype = gopd(ctor, 'prototype');
    if (!prototype || prototype.value === null || typeof prototype.value !== 'object') continue;
    const keys = ownKeys(prototype.value);
    let table = null;
    for (let k = 0; k < keys.length; k++) {
      const key = keys[k];
      if (typeof key !== 'string' || key.length < 3 || key[0] !== 'o' || key[1] !== 'n') continue;
      const descriptor = gopd(prototype.value, key);
      if (!descriptor || typeof descriptor.set !== 'function') continue;
      if (table === null) table = { __proto__: null };
      table[key] = descriptor.set;
    }
    if (table !== null) apply(mapSet, tables, [prototype.value, table]);
  }
  g.__setInlineHandler = function (element, name, value) {
    for (let p = getProto(element); p !== null; p = getProto(p)) {
      const table = apply(mapGet, tables, [p]);
      if (table !== undefined && table[name] !== undefined) return apply(table[name], element, [value]);
    }
    element[name] = value;
  };
})();
