(() => {
  class X extends HTMLElement {
    constructor() {
      super();
      this.i = this.attachInternals();
    }
  }
  customElements.define("x-setlike-internals", X);
  const host = new X(),
    state = host.i.states,
    proto = CustomStateSet.prototype,
    result = {};
  const shape = (d) => [
    d.enumerable,
    d.configurable,
    d.writable ?? null,
    d.value?.name ?? d.get?.name ?? null,
    d.value?.length ?? d.get?.length ?? null,
  ];
  result.descriptors = Object.getOwnPropertyNames(proto).map((name) => [
    name,
    shape(Object.getOwnPropertyDescriptor(proto, name)),
  ]);
  result.iteratorAliases = [proto[Symbol.iterator] === proto.values, proto.keys === proto.values];
  state.add("one");
  state.add("two");
  const iterator = state.values(),
    p = Object.getPrototypeOf(iterator);
  result.iterator = [
    Object.prototype.toString.call(iterator),
    Object.getOwnPropertyNames(p),
    Object.getOwnPropertySymbols(p).map((x) => String(x)),
    iterator[Symbol.iterator]() === iterator,
    p.next.name,
    p.next.length,
  ];
  result.live = [iterator.next()];
  state.delete("two");
  state.add("three");
  result.live.push(iterator.next(), iterator.next());
  state.add("four");
  result.live.push(iterator.next());
  const calls = [];
  const receiver = {};
  state.forEach(function (value, key, owner) {
    calls.push([value, key, owner === state, this === receiver]);
  }, receiver);
  result.foreach = calls;
  const failures = [];
  for (const name of ["add", "clear", "delete", "entries", "forEach", "has", "keys", "values"]) {
    try {
      proto[name].call({}, () => {});
      failures.push(null);
    } catch (error) {
      failures.push(error.name);
    }
  }
  result.failures = failures;
  return result;
})();
