(() => {
  const frame = document.createElement("iframe");
  const descriptor = Object.getOwnPropertyDescriptor(HTMLIFrameElement.prototype, "credentialless");
  const result = {
    initial: frame.credentialless,
    own: Object.hasOwn(frame, "credentialless"),
    descriptor: {
      enumerable: descriptor.enumerable,
      configurable: descriptor.configurable,
      getName: descriptor.get.name,
      setName: descriptor.set.name,
      getLength: descriptor.get.length,
      setLength: descriptor.set.length,
      getPrototype: Object.hasOwn(descriptor.get, "prototype"),
      setPrototype: Object.hasOwn(descriptor.set, "prototype"),
    },
    values: [],
  };
  for (const value of [
    false,
    0,
    NaN,
    null,
    undefined,
    "",
    true,
    1,
    "false",
    {},
    Symbol("value"),
    0n,
    1n,
  ]) {
    descriptor.set.call(frame, value);
    result.values.push([frame.credentialless, frame.getAttribute("credentialless")]);
  }
  frame.setAttribute("credentialless", "false");
  result.fromAttribute = frame.credentialless;
  frame.removeAttribute("credentialless");
  result.fromRemoval = frame.credentialless;
  result.errors = [];
  for (const receiver of [
    null,
    {},
    document.createElement("div"),
    document.createElementNS("http://www.w3.org/2000/svg", "iframe"),
    new Proxy(frame, {}),
  ]) {
    const row = [];
    for (const member of [descriptor.get, descriptor.set])
      try {
        member.call(receiver, true);
        row.push("accepted");
      } catch (error) {
        row.push(error.name);
      }
    result.errors.push(row);
  }
  let calls = 0;
  const poison = () => {
    calls++;
    throw new Error("public reflection queried");
  };
  Object.defineProperties(frame, {
    getAttribute: { value: poison, configurable: true },
    setAttribute: { value: poison, configurable: true },
    removeAttribute: { value: poison, configurable: true },
  });
  descriptor.set.call(frame, true);
  result.privateTrue = descriptor.get.call(frame);
  descriptor.set.call(frame, false);
  result.privateFalse = descriptor.get.call(frame);
  result.publicCalls = calls;
  let coercions = 0;
  descriptor.set.call(frame, {
    valueOf() {
      coercions++;
      return false;
    },
    toString() {
      coercions++;
      return "";
    },
  });
  result.objectBoolean = frame.credentialless;
  result.coercions = coercions;
  return result;
})();
