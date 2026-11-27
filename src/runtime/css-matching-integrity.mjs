// Replaying owned selector facts must preserve the ambient dependency checks
// and callouts of a fresh match. Refuse reuse if a reachable intrinsic changes.
const ownKeys = Reflect.ownKeys;
const prototype = Object.getPrototypeOf;
const descriptor = Object.getOwnPropertyDescriptor;
const descriptors = Object.getOwnPropertyDescriptors;
const own = Object.hasOwn;
const same = Object.is;
const global = globalThis;

function capture(object) {
  return { object, prototype: prototype(object), keys: ownKeys(object), fields: descriptors(object) };
}
const arrayIteratorPrototype = prototype([][Symbol.iterator]());
const prototypes = [
  Object.prototype, Function.prototype, Array.prototype, String.prototype,
  Number.prototype, Boolean.prototype, RegExp.prototype, Map.prototype,
  Set.prototype, WeakMap.prototype, WeakSet.prototype, Symbol.prototype,
  arrayIteratorPrototype, prototype(''[Symbol.iterator]()),
  prototype(new Map().entries()), prototype(new Set().values()),
];
const intrinsicObjects = [
  ...prototypes,
  prototype(arrayIteratorPrototype),
  Object, Array, String, Number, Symbol, RegExp, Reflect, Math,
];
const surfaces = intrinsicObjects.map(capture);
const globalNames = ['Object', 'Array', 'String', 'Number', 'Symbol', 'RegExp', 'Reflect', 'Math', 'Map', 'Set', 'WeakMap', 'WeakSet'];
const bindings = globalNames.map(name => [name, descriptor(global, name)]);

function sameDescriptor(a, b) {
  if (!a || a.configurable !== b.configurable || a.enumerable !== b.enumerable) return false;
  if (own(b, 'value')) return own(a, 'value') && a.writable === b.writable && same(a.value, b.value);
  return !own(a, 'value') && a.get === b.get && a.set === b.set;
}
function unchangedSurface(surface) {
  const { object, keys, fields } = surface;
  if (prototype(object) !== surface.prototype || ownKeys(object).length !== keys.length) return false;
  for (let index = 0; index < keys.length; index++) {
    const key = keys[index];
    if (!sameDescriptor(descriptor(object, key), fields[key])) return false;
  }
  return true;
}
export function matchingIntrinsicsIntact() {
  for (let index = 0; index < bindings.length; index++) {
    const binding = bindings[index];
    if (!sameDescriptor(descriptor(global, binding[0]), binding[1])) return false;
  }
  for (let index = 0; index < surfaces.length; index++)
    if (!unchangedSurface(surfaces[index])) return false;
  return true;
}
