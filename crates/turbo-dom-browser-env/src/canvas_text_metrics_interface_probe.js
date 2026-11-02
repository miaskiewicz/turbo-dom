(() => {
  // The TextMetrics interface (turbo-dom#237): measureText() returns a TextMetrics whose
  // fields are prototype getters, and the interface object is not constructible.
  const ctx = document.createElement('canvas').getContext('2d');
  const m = ctx.measureText('Wave');
  const proto = TextMetrics.prototype;
  const error = (f) => { try { f(); return null; } catch (e) { return [e.constructor.name, e.message]; } };
  const getter = (key) => Object.getOwnPropertyDescriptor(proto, key);
  const keys = Reflect.ownKeys(proto).map(String);
  return {
    type: typeof TextMetrics,
    globalDescriptor: (({ writable, enumerable, configurable }) => [writable, enumerable, configurable])(
      Object.getOwnPropertyDescriptor(window, 'TextMetrics')),
    name: TextMetrics.name,
    length: TextMetrics.length,
    instance: m instanceof TextMetrics,
    protoOf: Object.getPrototypeOf(m) === proto,
    ownKeys: Reflect.ownKeys(m).length,
    tag: Object.prototype.toString.call(m),
    keys,
    getters: keys.filter((k) => k !== 'constructor' && !k.startsWith('Symbol')).map((k) => {
      const d = getter(k);
      return [k, typeof d.get, d.set === undefined, d.enumerable, d.configurable, d.get.name, d.get.length];
    }),
    widthType: typeof m.width,
    construct: error(() => new TextMetrics()),
    call: error(() => TextMetrics()),
    badReceiver: error(() => getter('width').get.call({})),
    protoRead: error(() => proto.width),
    json: JSON.stringify(m),
    distinct: ctx.measureText('Wave') !== m,
  };
})()
