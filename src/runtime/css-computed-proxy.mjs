// Resolved primitive snapshots only: no CSS policy or live backend references.
const fromUnits = String.fromCharCode;
const apply = Reflect.apply;
const NativeMap = Map,
  NativeSet = Set,
  NativeU16 = Uint16Array;
const min = Math.min;
const mapGet = Map.prototype.get,
  mapSet = Map.prototype.set,
  mapHas = Map.prototype.has;
const mapForEach = Map.prototype.forEach;
const mapSize = Object.getOwnPropertyDescriptor(Map.prototype, 'size').get;
const setHas = Set.prototype.has,
  setAdd = Set.prototype.add,
  setDelete = Set.prototype.delete;
const array = Array.isArray,
  valuesIterator = Array.prototype.values;
const define = Object.defineProperty;
const Failure = TypeError;
const NativeProxy = Proxy;
function append(array, value) {
  define(array, array.length, { __proto__: null, value, writable: true, enumerable: true, configurable: true });
}
function names(map) {
  const result = [];
  apply(mapForEach, map, [(value, key) => append(result, key)]);
  return result;
}
const entries = Object.entries;
const isInteger = Number.isInteger;
const kebab = (key) => key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);

function decodeUnits(units) {
  if (!array(units) && !(units instanceof NativeU16)) {
    throw new TypeError('turbo-dom: invalid UTF16 computed style result');
  }
  let text = '';
  for (let start = 0; start < units.length; start += 8192) {
    const chunk = [];
    const end = min(start + 8192, units.length);
    for (let index = start; index < end; index++) {
      const unit = units[index];
      if (!isInteger(unit) || unit < 0 || unit > 65535) {
        throw new TypeError('turbo-dom: invalid UTF16 computed style result');
      }
      append(chunk, unit);
    }
    text += apply(fromUnits, undefined, chunk);
  }
  return text;
}
function copyValues(result) {
  const map = new NativeMap();
  const groups = [result.values, result.custom];
  for (let group = 0; group < groups.length; group++) {
    const pairs = entries(groups[group] ?? {});
    for (let index = 0; index < pairs.length; index++) apply(mapSet, map, [pairs[index][0], decodeUnits(pairs[index][1])]);
  }
  return map;
}
function propertyName(value) {
  const name = String(value);
  return name.startsWith('--') ? name : name.toLowerCase();
}
function pendingProperties(result, map) {
  const metadata = result.usedProperties;
  if (!array(metadata)) throw new Failure('turbo-dom: missing or invalid Rust usedProperties metadata');
  if (metadata.length === 0) return null;
  const pending = new NativeSet();
  for (let index = 0; index < metadata.length; index++) {
    const name = metadata[index];
    if (typeof name !== 'string' || name.length === 0 || !apply(mapHas, map, [name]))
      throw new Failure('turbo-dom: invalid Rust usedProperties metadata');
    apply(setAdd, pending, [name]);
  }
  return pending;
}
function presentation(result, map, callback, context) {
  if (callback === undefined) return (key) => apply(mapGet, map, [key]) ?? '';
  if (typeof callback !== 'function') throw new Failure('turbo-dom: resolveProperty must be a function');
  const pending = pendingProperties(result, map);
  if (pending === null) return (key) => apply(mapGet, map, [key]) ?? '';
  const presented = new NativeMap();
  return (key) => {
    if (apply(mapHas, presented, [key])) return apply(mapGet, presented, [key]);
    const base = apply(mapGet, map, [key]) ?? '';
    if (!apply(setHas, pending, [key])) return base;
    const value = callback(key, base, context);
    if (typeof value !== 'string') throw new Failure('turbo-dom: resolveProperty must return a string');
    apply(mapSet, presented, [key, value]);
    apply(setDelete, pending, [key]);
    return value;
  };
}
function honesty(lazy) {
  return lazy
    ? 'computed style is an owned Rust result with lazy stateless used-property presentation'
    : 'computed style is an owned serialized Rust result; no live layout or backend reads';
}
function memberValue(getValue, key) {
  return getValue(key.startsWith('--') ? key : kebab(key));
}
export function createComputedStyleProxy(result, version, options) {
  const callback = options?.resolveProperty;
  const context = callback === undefined ? undefined : options.propertyContext;
  const map = copyValues(result),
    getValue = presentation(result, map, callback, context);
  return new NativeProxy(
    {},
    {
      get(_target, key) {
        if (key === '__v') return version;
        if (key === '__honest') return honesty(callback !== undefined);
        if (key === 'getPropertyValue') return (name) => getValue(propertyName(name));
        if (key === 'getPropertyPriority') return () => '';
        if (key === 'length') return apply(mapSize, map, []);
        if (key === 'item') return (index) => names(map)[index] ?? '';
        if (key === Symbol.iterator) {
          const keys = names(map);
          return () => apply(valuesIterator, keys, []);
        }
        if (typeof key !== 'string') return undefined;
        return memberValue(getValue, key);
      },
    },
  );
}
