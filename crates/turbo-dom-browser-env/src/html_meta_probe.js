(() => {
  const meta = document.createElement("meta"),
    result = [];
  for (const [name, attribute] of [
    ["name", "name"],
    ["httpEquiv", "http-equiv"],
    ["content", "content"],
    ["media", "media"],
    ["scheme", "scheme"],
  ]) {
    const descriptor = Object.getOwnPropertyDescriptor(HTMLMetaElement.prototype, name);
    const record = {
      name,
      empty: meta[name],
      own: Object.hasOwn(meta, name),
      enumerable: descriptor.enumerable,
      configurable: descriptor.configurable,
      getter: descriptor.get.name,
      setter: descriptor.set.name,
      getLength: descriptor.get.length,
      setLength: descriptor.set.length,
      getPrototype: Object.hasOwn(descriptor.get, "prototype"),
      setPrototype: Object.hasOwn(descriptor.set, "prototype"),
    };
    meta[name] = "MiXeD";
    record.attribute = meta.getAttribute(attribute);
    meta.setAttribute(attribute, "replacement");
    record.property = meta[name];
    descriptor.set.call(meta, null);
    record.nullValue = meta.getAttribute(attribute);
    descriptor.set.call(meta, undefined);
    record.undefinedValue = meta.getAttribute(attribute);
    try {
      descriptor.set.call(meta, Symbol("invalid"));
    } catch (error) {
      record.symbol = error.name;
    }
    const errors = [];
    for (const receiver of [
      null,
      {},
      document.createElement("div"),
      document.createElementNS("http://www.w3.org/2000/svg", "meta"),
      new Proxy(meta, {}),
    ]) {
      for (const operation of [
        () => descriptor.get.call(receiver),
        () => descriptor.set.call(receiver, "bad"),
      ]) {
        try {
          operation();
          errors.push("accepted");
        } catch (error) {
          errors.push(error.name);
        }
      }
    }
    record.errors = errors;
    result.push(record);
  }
  let reads = 0;
  Object.defineProperty(meta, "getAttribute", {
    get() {
      reads++;
      throw Error("guest getter");
    },
  });
  Object.defineProperty(meta, "setAttribute", {
    get() {
      reads++;
      throw Error("guest setter");
    },
  });
  meta.httpEquiv = "Content-Security-Policy";
  meta.content = "base-uri 'none'";
  const privateReflection = { httpEquiv: meta.httpEquiv, content: meta.content, reads };
  return { result, privateReflection };
})();
