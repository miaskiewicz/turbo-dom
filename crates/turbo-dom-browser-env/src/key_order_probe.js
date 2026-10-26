(() => {
  // Own-key order of every Web IDL interface object and interface prototype object on the
  // global, as Reflect.ownKeys reports it (symbols by description), plus the keys that are
  // non-configurable there. ECMAScript built-ins and legacy factory functions (Image, Option,
  // Audio) are skipped.
  const builtins = ['AggregateError', 'Array', 'ArrayBuffer', 'AsyncDisposableStack', 'BigInt', 'BigInt64Array', 'BigUint64Array', 'Boolean',
    'DataView', 'Date', 'DisposableStack', 'Error', 'EvalError', 'FinalizationRegistry', 'Float16Array', 'Float32Array', 'Float64Array',
    'Function', 'Int16Array', 'Int32Array', 'Int8Array', 'Iterator', 'Map', 'Number', 'Object', 'Promise', 'Proxy', 'RangeError',
    'ReferenceError', 'RegExp', 'Set', 'SharedArrayBuffer', 'String', 'SuppressedError', 'Symbol', 'SyntaxError', 'TypeError', 'URIError',
    'Uint16Array', 'Uint32Array', 'Uint8Array', 'Uint8ClampedArray', 'WeakMap', 'WeakRef', 'WeakSet'];
  const label = (key) => typeof key === 'symbol' ? key.toString() : key;
  const shape = (object) => {
    const keys = Reflect.ownKeys(object);
    const entry = { keys: keys.map(label) };
    const fixed = keys.filter((key) => key !== 'prototype' && !Object.getOwnPropertyDescriptor(object, key).configurable).map(label);
    if (fixed.length) entry.fixed = fixed;
    return entry;
  };
  const result = {};
  for (const name of Object.getOwnPropertyNames(globalThis).sort()) {
    if (builtins.includes(name) || !/^[A-Z]/.test(name)) continue;
    const descriptor = Object.getOwnPropertyDescriptor(globalThis, name);
    const value = descriptor && descriptor.value;
    if (typeof value !== 'function') continue;
    const prototype = value.prototype;
    if (!prototype || (typeof prototype !== 'object' && typeof prototype !== 'function')) continue;
    if (!Object.hasOwn(prototype, 'constructor') || prototype.constructor !== value) continue;
    result[name] = { interface: shape(value), prototype: shape(prototype) };
  }
  return JSON.stringify(result);
})()
