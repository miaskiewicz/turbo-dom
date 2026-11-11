// Live collections. Each reads its backing array on every access, so they
// reflect mutations immediately — the exact place happy-dom bleeds liveness bugs.

function stringKeyMethod(key, getArray, extra, tag) {
  if (key === 'length') return getArray().length;
  if (key === 'item') return (i) => getArray()[i] ?? null;
  if (key === 'forEach') return (cb, thisArg) => getArray().forEach(cb, thisArg);
  if (key === 'entries') return () => getArray().entries();
  if (key === 'keys') return () => getArray().keys();
  if (key === 'values') return () => getArray()[Symbol.iterator]();
  if (key === 'toString') return () => `[object ${tag}]`;
  if (key in extra) return extra[key](getArray());
  return undefined;
}

function makeLive(getArray, extra = {}, tag = 'NodeList') {
  const target = {};
  return new Proxy(target, {
    get(_t, key) {
      if (typeof key === 'string') {
        const c = key.charCodeAt(0);
        if (c >= 48 && c <= 57) return getArray()[+key];
        return stringKeyMethod(key, getArray, extra, tag);
      }
      if (key === Symbol.iterator) return () => getArray()[Symbol.iterator]();
      return undefined;
    },
    has(_t, key) {
      if (typeof key === 'string') {
        const c = key.charCodeAt(0);
        if (c >= 48 && c <= 57) return +key < getArray().length;
        return key === 'length' || key === 'item' || key === 'forEach' || key in extra;
      }
      return false;
    },
    ownKeys() {
      const arr = getArray();
      return [...arr.keys()].map(String).concat('length');
    },
    getOwnPropertyDescriptor(_t, key) {
      const arr = getArray();
      if (key === 'length') return { configurable: true, enumerable: false, value: arr.length };
      if (typeof key === 'string') {
        const c = key.charCodeAt(0);
        if (c >= 48 && c <= 57 && +key < arr.length) {
          return { configurable: true, enumerable: true, value: arr[+key] };
        }
      }
      return undefined;
    },
  });
}

export function liveNodeList(getArray) {
  return makeLive(getArray);
}

export function liveHTMLCollection(getArray) {
  return makeLive(getArray, {
    namedItem: (arr) => (name) =>
      arr.find((el) => el.getAttribute('id') === name || el.getAttribute('name') === name) ?? null,
  });
}

// Live NamedNodeMap for Element.attributes. length/indexing read getArray() on
// every access, so React 19's `while (attrs.length) removeAttributeNode(attrs[0])`
// loop sees the map shrink and terminates (a snapshot array never would).
export function liveNamedNodeMap(el, getArray) {
  return makeLive(getArray, {
    getNamedItem: (arr) => (name) => arr.find((a) => a.name === name) ?? null,
    getNamedItemNS: (arr) => (_ns, name) => arr.find((a) => a.name === name) ?? null,
    setNamedItem: () => (attr) => el.setAttributeNode(attr),
    setNamedItemNS: () => (attr) => el.setAttributeNode(attr),
    removeNamedItem: () => (name) => el.removeAttributeNode({ name }),
    removeNamedItemNS: () => (_ns, name) => el.removeAttributeNode({ name }),
  }, 'NamedNodeMap');
}
