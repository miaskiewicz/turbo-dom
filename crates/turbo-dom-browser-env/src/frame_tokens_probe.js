(() => {
  const rows = [],
    frame = document.createElement("iframe"),
    list = frame.sandbox;
  const capture = (fn) => {
    try {
      return { value: fn() };
    } catch (error) {
      return { error: error.name };
    }
  };
  const snapshot = (label) =>
    rows.push({
      label,
      attribute: frame.getAttribute("sandbox"),
      value: list?.value,
      tokens: list ? Array.from(list) : null,
    });
  snapshot("initial");
  frame.sandbox = "allow-scripts  allow-forms";
  snapshot("put-forwards");
  frame.sandbox.add("allow-popups");
  snapshot("add");
  frame.setAttribute("sandbox", "allow-same-origin allow-scripts allow-scripts");
  snapshot("attribute");
  frame.sandbox.remove("allow-scripts");
  snapshot("remove");
  const supported = {};
  for (const token of [
    "allow-downloads",
    "allow-forms",
    "allow-modals",
    "allow-orientation-lock",
    "allow-pointer-lock",
    "allow-popups",
    "allow-popups-to-escape-sandbox",
    "allow-presentation",
    "allow-same-origin",
    "allow-scripts",
    "allow-storage-access-by-user-activation",
    "allow-top-navigation",
    "allow-top-navigation-by-user-activation",
    "allow-top-navigation-to-custom-protocols",
    "allow-same-site-none-cookies",
    "ALLOW-SCRIPTS",
    "unknown",
    "",
    "a b",
  ])
    supported[token] = capture(() => list.supports(token));
  const descriptor = Object.getOwnPropertyDescriptor(HTMLIFrameElement.prototype, "sandbox");
  const brands = [];
  for (const receiver of [
    {},
    document.createElement("div"),
    document.createElementNS(null, "iframe"),
    document.createElementNS("http://www.w3.org/1999/xhtml", "IFRAME"),
    document.createElementNS("http://www.w3.org/1999/xhtml", "p:iframe"),
  ])
    brands.push(capture(() => descriptor.get.call(receiver).value));
  let forwarded = [];
  Object.defineProperty(list, "value", {
    configurable: true,
    set(value) {
      forwarded.push(value);
    },
  });
  frame.sandbox = "own forwarded";
  delete list.value;
  Object.defineProperty(list, "value", { configurable: true, value: "fixed", writable: false });
  const frozenForward = capture(() => {
    frame.sandbox = "blocked";
    return frame.getAttribute("sandbox");
  });
  delete list.value;
  const badForward = capture(() =>
    descriptor.set.call(document.createElement("div"), "allow-scripts"),
  );
  const symbolSupports = capture(() => list.supports(Symbol("token")));
  let classForwarded = [];
  const element = document.createElement("div");
  Object.defineProperty(element.classList, "value", {
    set(value) {
      classForwarded.push(value);
    },
  });
  element.classList = "class forwarded";
  return {
    rows,
    same: list === descriptor.get.call(frame),
    supported,
    brands,
    forwarded,
    frozenForward,
    badForward,
    symbolSupports,
    classForwarded,
    descriptor: {
      enumerable: descriptor.enumerable,
      configurable: descriptor.configurable,
      getName: descriptor.get.name,
      getLength: descriptor.get.length,
      setName: descriptor.set.name,
      setLength: descriptor.set.length,
      getNative: Function.prototype.toString.call(descriptor.get).includes("[native code]"),
      setNative: Function.prototype.toString.call(descriptor.set).includes("[native code]"),
    },
  };
})();
