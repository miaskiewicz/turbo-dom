// Live collections. Each reads its backing array on every access, so they
// reflect mutations immediately — the exact place happy-dom bleeds liveness bugs.

function makeLive(getArray, extra = {}) {
  const target = function () {};
  return new Proxy(target, {
    get(_t, key) {
      if (typeof key === 'string') {
        // hot path: indexed access coll[i]. A property key starting with a digit
        // is a numeric index — detect via charCode, no regex, no string-compare
        // chain. getArray() is called only when actually needed.
        const c = key.charCodeAt(0);
        if (c >= 48 && c <= 57) return getArray()[+key];
        if (key === 'length') return getArray().length;
        if (key === 'item') return (i) => getArray()[i] ?? null;
        if (key === 'forEach') return (cb, thisArg) => getArray().forEach(cb, thisArg);
        if (key === 'entries') return () => getArray().entries();
        if (key === 'keys') return () => getArray().keys();
        if (key === 'values') return () => getArray()[Symbol.iterator]();
        if (key === 'toString') return () => '[object NodeList]';
        if (key in extra) return extra[key](getArray());
        return undefined;
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
