(() => {
  const target = globalThis.__windowIndexTarget;
  const caught = fn => {
    try { return { ok: true, value: fn() }; }
    catch (error) { return { ok: false, name: error.name, accessingRealm: error instanceof DOMException }; }
  };
  return {
    length: target.length,
    properties: ['0', '1', '2', '00', '01', '-1', '4294967294', '4294967295', 'NamedChild0', 'missing'].map(key => ({
      key,
      read: caught(() => target[key] === target.frames[key]),
      has: caught(() => key in target),
      descriptor: caught(() => {
        const d = Object.getOwnPropertyDescriptor(target, key);
        if (d === undefined) return null;
        return {
          writable: d.writable,
          enumerable: d.enumerable,
          configurable: d.configurable,
          getType: typeof d.get,
          setType: typeof d.set,
          valueMatchesIndex: d.value === target[key],
          accessingRealm: Object.getPrototypeOf(d) === Object.prototype,
        };
      }),
    })),
    ownKeys: caught(() => Reflect.ownKeys(target).map(key => typeof key === 'symbol' ? String(key) : key)),
    objectKeys: caught(() => Object.keys(target)),
  };
})()
