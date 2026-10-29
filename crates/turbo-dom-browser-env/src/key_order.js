// Move interface object and prototype keys into Chrome's own-key order (Blink's generated
// bindings: attributes, constants, operations, then `constructor`, with runtime enabled
// members after it). key_order.rs plans each object natively: the longest prefix already in
// Chrome's order stays and the keys after it arrive here in their final order. V8 lists string
// keys in creation order and symbols after them, so a key moves by being deleted and redefined
// with its unchanged descriptor. Web IDL constants are created configurable during install so
// they can move; the constants Chrome has non-configurable are re-added frozen, before any
// guest code runs (changing the attribute in place would leave V8 retaining the realm).
(function () {
  'use strict';
  const g = globalThis;
  const describe = Object.getOwnPropertyDescriptor, remove = Reflect.deleteProperty, defineAll = Object.defineProperties;
  const symbolText = Symbol.prototype.toString, apply = Reflect.apply;
  const label = (key) => typeof key === 'symbol' ? apply(symbolText, key, []) : key;

  // jobs: [object, keys in final order, labels Chrome has non-configurable]
  g.__captureKeyOrder = (jobs) => {
    for (const [object, keys, fixed] of jobs) {
      // Deleting from a prototype V8 already optimized re-optimizes it each time, so the moved
      // keys are deleted last-first and defined back in one call.
      const moved = { __proto__: null }, order = [];
      for (const key of keys) {
        const descriptor = describe(object, key);
        if (!descriptor.configurable) continue;
        if ('value' in descriptor && typeof descriptor.value === 'number' && fixed.indexOf(label(key)) >= 0) descriptor.configurable = false;
        moved[key] = descriptor;
        order.push(key);
      }
      for (let index = order.length - 1; index >= 0; index--) remove(object, order[index]);
      defineAll(object, moved);
    }
  };
})();
