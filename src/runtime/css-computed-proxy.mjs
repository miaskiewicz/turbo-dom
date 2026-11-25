// Resolved primitive snapshots only: no CSS policy or live backend references.
const fromUnits = String.fromCharCode;
const entries = Object.entries;
const isInteger = Number.isInteger;
const kebab = (key) => key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);

function decodeUnits(units) {
  if (!Array.isArray(units) && !(units instanceof Uint16Array)) {
    throw new TypeError('turbo-dom: invalid UTF16 computed style result');
  }
  let text = '';
  for (let start = 0; start < units.length; start += 8192) {
    const chunk = [];
    const end = Math.min(start + 8192, units.length);
    for (let index = start; index < end; index++) {
      const unit = units[index];
      if (!isInteger(unit) || unit < 0 || unit > 65535) {
        throw new TypeError('turbo-dom: invalid UTF16 computed style result');
      }
      chunk.push(unit);
    }
    text += fromUnits(...chunk);
  }
  return text;
}
function copyValues(result) {
  const map = new Map();
  for (const values of [result.values, result.custom]) {
    for (const [name, units] of entries(values ?? {})) map.set(name, decodeUnits(units));
  }
  return map;
}
function propertyName(value) {
  const name = String(value);
  return name.startsWith('--') ? name : name.toLowerCase();
}
function propertyValue(map, key) {
  return map.get(key.startsWith('--') ? key : kebab(key)) ?? '';
}
export function createComputedStyleProxy(result, version) {
  const map = copyValues(result);
  return new Proxy(
    {},
    {
      get(_target, key) {
        if (key === '__v') return version;
        if (key === '__honest')
          return 'computed style is an owned serialized Rust result; no live layout or backend reads';
        if (key === 'getPropertyValue') return (name) => map.get(propertyName(name)) ?? '';
        if (key === 'getPropertyPriority') return () => '';
        if (key === 'length') return map.size;
        if (key === 'item') return (index) => [...map.keys()][index] ?? '';
        if (key === Symbol.iterator) {
          const names = [...map.keys()];
          return names[Symbol.iterator].bind(names);
        }
        if (typeof key !== 'string') return undefined;
        return propertyValue(map, key);
      },
    },
  );
}
