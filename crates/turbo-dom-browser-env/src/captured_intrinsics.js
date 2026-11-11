// Bootstrap's last step, before any page script runs: capture the engine's own EventTarget
// methods and `on*` IDL attribute setters, so the engine never reaches a page-replaced
// builtin or accessor on its insertion / handler paths (miaskiewicz/turbo-dom#172).
(function () {
  'use strict';
  const g = globalThis, ns = __tsNs, apply = Reflect.apply, gopd = Object.getOwnPropertyDescriptor;
  const getProto = Object.getPrototypeOf, ownKeys = Reflect.ownKeys, mapGet = Map.prototype.get, mapSet = Map.prototype.set;
  // turbo-dom#437: wrap each live read + capture in a helper that names the
  // intrinsic. Previously a page-installed thrower getter (e.g. a replaced
  // `Promise[Symbol.species]`) surfaced as its own opaque error, and a frozen
  // `__tsNs` silently dropped every write because the IIFE was non-strict.
  // Strict mode turns the frozen-write into a TypeError; the helper rewraps
  // either failure with a readable message.
  // R1 L5: preserve the original error's subclass identity (so
  // `instanceof TypeError` etc. still works downstream) and attach it as
  // `cause` so programmatic triage can inspect the real failure. Fall back
  // to plain Error if the original's constructor is unusable.
  const rewrap = function (name, phase, inner) {
    const message = (inner && inner.message) || String(inner);
    const prefix = 'captured_intrinsics: failed to ' + phase + ' ' + name + ': ' + message;
    let Ctor = Error;
    try {
      const C = inner && inner.constructor;
      if (typeof C === 'function') Ctor = C;
    } catch (_) { /* a hostile getter on `.constructor` falls back to Error */ }
    let outer;
    try { outer = new Ctor(prefix); }
    catch (_) { outer = new Error(prefix); }
    try { outer.cause = inner; } catch (_) { /* non-configurable `cause` on a subclass */ }
    return outer;
  };
  const capture = function (name, read) {
    let value;
    try { value = read(); }
    catch (inner) { throw rewrap(name, 'read', inner); }
    try { ns[name] = value; }
    catch (inner) { throw rewrap(name, 'assign ' + 'to __tsNs (frozen?):', inner); }
    return value;
  };
  const target = g.EventTarget.prototype;
  // Absent only when the namespace was frozen before bootstrap (install degrades).
  if (typeof ns.completeNodeIntrinsics === 'function') ns.completeNodeIntrinsics();
  capture('handlerAdd', function () { return target.addEventListener; });
  capture('handlerRemove', function () { return target.removeEventListener; });
  // Every well-known `@@species` getter V8 may consult on a derived-class protocol
  // (miaskiewicz/turbo-dom#449 / turbo-surf#245): `Array.prototype.slice|map|filter|concat|splice`,
  // `Promise.prototype.then|catch|finally`, `Map.prototype.*` / `Set.prototype.*`'s
  // subclass protocol, `RegExp[@@species]` which `String.prototype.split` /
  // `replaceAll` reach through, and the typed-array / buffer `slice` family.
  // (`WeakMap`/`WeakSet` have no `@@species`, so none is captured.)
  // A page that replaces one of these with a thrower before the engine runs its
  // iframe install (or any later engine path) poisons the live read; the captured
  // reference lets the engine (and surf's own install, which reads them off
  // `__tsNs`) dispatch to the real constructor without ever touching live
  // `Promise`/`Array` etc. The capture runs before the first page script, so a
  // patch that lands after bootstrap never reaches these bindings.
  capture('ArraySpecies', function () { return Array[Symbol.species]; });
  capture('PromiseSpecies', function () { return Promise[Symbol.species]; });
  capture('MapSpecies', function () { return Map[Symbol.species]; });
  capture('SetSpecies', function () { return Set[Symbol.species]; });
  capture('RegExpSpecies', function () { return RegExp[Symbol.species]; });
  capture('ArrayBufferSpecies', function () { return ArrayBuffer[Symbol.species]; });
  // Every interface prototype's own `on*` setters, keyed by prototype.
  const tables = new Map(), names = ownKeys(g);
  // One prototype's own `on*` setters, or null when it has none.
  const handlerSetters = function (proto) {
    const keys = ownKeys(proto);
    let table = null;
    for (let k = 0; k < keys.length; k++) {
      const key = keys[k];
      if (typeof key !== 'string' || key.length < 3 || key[0] !== 'o' || key[1] !== 'n') continue;
      const descriptor = gopd(proto, key);
      if (!descriptor || typeof descriptor.set !== 'function') continue;
      if (table === null) table = { __proto__: null };
      table[key] = descriptor.set;
    }
    return table;
  };
  const collectHandlerTable = function (name) {
    if (typeof name !== 'string') return;
    const global = gopd(g, name), ctor = global && global.value;
    if (typeof ctor !== 'function') return;
    const prototype = gopd(ctor, 'prototype');
    if (!prototype || prototype.value === null || typeof prototype.value !== 'object') return;
    const table = handlerSetters(prototype.value);
    if (table !== null) apply(mapSet, tables, [prototype.value, table]);
  };
  for (let i = 0; i < names.length; i++) collectHandlerTable(names[i]);
  g.__setInlineHandler = function (element, name, value) {
    for (let p = getProto(element); p !== null; p = getProto(p)) {
      const table = apply(mapGet, tables, [p]);
      if (table !== undefined && table[name] !== undefined) return apply(table[name], element, [value]);
    }
    element[name] = value;
  };
})();
