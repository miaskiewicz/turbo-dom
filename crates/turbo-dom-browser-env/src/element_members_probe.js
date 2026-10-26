(() => {
  const E = Element.prototype;
  const failure = (callback) => {
    try {
      const value = callback();
      return { value: value === undefined ? "undefined" : value };
    } catch (error) {
      return { name: error.name, message: error.message, dom: error instanceof DOMException };
    }
  };
  const shape = (owner, name) => {
    const descriptor = Object.getOwnPropertyDescriptor(owner, name);
    if (!descriptor) return null;
    const record = { enumerable: descriptor.enumerable, configurable: descriptor.configurable };
    for (const kind of ["get", "set", "value"])
      if (typeof descriptor[kind] === "function")
        record[kind] = [
          descriptor[kind].name,
          descriptor[kind].length,
          Object.hasOwn(descriptor[kind], "prototype"),
        ];
    if ("writable" in descriptor) record.writable = descriptor.writable;
    return record;
  };
  const svgNs = "http://www.w3.org/2000/svg",
    mathNs = "http://www.w3.org/1998/Math/MathML";
  const body = document.body;
  body.innerHTML =
    '<section id="outer" class="box"><p id="inner" data-k="v">x</p></section><svg><g id="shape"></g></svg>';
  const inner = document.getElementById("inner"),
    shapeElement = document.getElementById("shape");
  const matchesSelector = {
    descriptor: shape(E, "webkitMatchesSelector"),
    distinct: E.webkitMatchesSelector !== E.matches,
    results: [
      "p",
      "#outer > p",
      ".box p[data-k=v]",
      "section",
      "p:not(.x)",
      "div p",
      "P",
    ].map((selector) => inner.webkitMatchesSelector(selector)),
    svg: [shapeElement.webkitMatchesSelector("svg > g"), shapeElement.webkitMatchesSelector("g#shape")],
    coerced: [inner.webkitMatchesSelector({ toString: () => "p" }), failure(() => inner.webkitMatchesSelector(null))],
    grammar: [
      ":scope",
      "::before",
      "p::after",
      "p:hover",
      ":scope > p",
      ":is(p, q)",
      ":where(p)",
      ":not(p)",
      ":not(section) > p",
      ":first-child",
      ":nth-child(1 of p)",
      ":popover-open",
      ":defined",
      "*|p",
      "p,",
      "p >",
      "> p",
      "p {",
      "p }",
      "p { color: red } p",
      "[data-k='{']",
      "#1a",
      ".1a",
      "p:nth-child(foo)",
      ":unknown-pseudo",
      "p::unknown",
      "a\\:b",
      "p !",
      "  p  ",
      " ",
    ].map((selector) => [selector, failure(() => inner.webkitMatchesSelector(selector))]),
    invalid: failure(() => inner.webkitMatchesSelector("[")),
    empty: failure(() => inner.webkitMatchesSelector("")),
    missing: failure(() => inner.webkitMatchesSelector()),
    symbol: failure(() => inner.webkitMatchesSelector(Symbol("s"))),
    brand: [
      failure(() => E.webkitMatchesSelector.call(document, "p")),
      failure(() => E.webkitMatchesSelector.call({}, "p")),
      failure(() => E.webkitMatchesSelector.call(document.createTextNode("t"), "p")),
    ],
  };
  const names = [
    "onbeforecopy",
    "onbeforecut",
    "onbeforepaste",
    "onsearch",
    "onfullscreenchange",
    "onfullscreenerror",
    "onwebkitfullscreenchange",
    "onwebkitfullscreenerror",
  ];
  const owners = {
    Element: E,
    HTMLElement: HTMLElement.prototype,
    SVGElement: SVGElement.prototype,
    Node: Node.prototype,
  };
  const describe = (value) =>
    typeof value === "function" ? [value.name, value.length, Function.prototype.toString.call(value)] : value;
  const handlers = names.map((name) => {
    const type = name.slice(2),
      descriptor = Object.getOwnPropertyDescriptor(E, name);
    const elements = {
      html: document.createElement("div"),
      svg: document.createElementNS(svgNs, "rect"),
      math: document.createElementNS(mathNs, "mi"),
      foreign: document.createElementNS("urn:probe", "x"),
      none: document.createElementNS(null, "y"),
    };
    const attribute = {};
    for (const [kind, element] of Object.entries(elements)) {
      const before = element[name];
      element.setAttribute(name, "return [this.localName, arguments[0].type, arguments.length]");
      const compiled = element[name];
      const ran =
        typeof compiled === "function"
          ? compiled.call(element, new Event(type))
          : null;
      element.removeAttribute(name);
      attribute[kind] = { before, compiled: describe(compiled), ran, removed: element[name] };
    }
    const target = document.createElement("span");
    body.append(target);
    const order = [];
    target.addEventListener(type, () => order.push("before"));
    target[name] = function (event) {
      order.push(["first", this === target, event.type, arguments.length]);
      return false;
    };
    target.addEventListener(type, () => order.push("after"));
    const event = new Event(type, { cancelable: true, bubbles: true });
    const dispatched = target.dispatchEvent(event);
    const firstOrder = order.splice(0);
    target[name] = function () {
      order.push("replacement");
    };
    target.dispatchEvent(new Event(type));
    const replacedOrder = order.splice(0);
    target.setAttribute(name, "this.dataset.fired = event.type");
    target.dispatchEvent(new Event(type));
    const afterAttribute = { order: order.splice(0), fired: target.dataset.fired ?? null };
    target.removeAttribute(name);
    target.dispatchEvent(new Event(type));
    const afterRemove = { order: order.splice(0), value: describe(target[name]) };
    target[name] = null;
    target.dispatchEvent(new Event(type));
    const cleared = order.splice(0);
    const bubbling = [];
    body[name] = (event) => bubbling.push([event.eventPhase, event.target === target]);
    target.dispatchEvent(new Event(type, { bubbles: true }));
    target.dispatchEvent(new Event(type));
    body[name] = null;
    target.remove();
    const svgTarget = document.createElementNS(svgNs, "circle");
    let svgCalls = 0;
    svgTarget[name] = () => svgCalls++;
    svgTarget.dispatchEvent(new Event(type));
    const values = {};
    const probe = document.createElement("i");
    const object = {};
    for (const [label, value] of [
      ["object", object],
      ["string", "return 1"],
      ["number", 7],
      ["undefined", undefined],
      ["true", true],
    ]) {
      probe[name] = function () {};
      probe[name] = value;
      const read = probe[name];
      values[label] = read === object ? "same-object" : describe(read);
    }
    return {
      name,
      owners: Object.keys(owners).filter((owner) => Object.hasOwn(owners[owner], name)),
      descriptor: shape(E, name),
      own: Object.hasOwn(target, name),
      initial: document.createElement("b")[name],
      attribute,
      dispatched,
      prevented: event.defaultPrevented,
      firstOrder,
      replacedOrder,
      afterAttribute,
      afterRemove,
      cleared,
      bubbling,
      svgCalls,
      values,
      brand: [
        failure(() => descriptor.get.call(document)),
        failure(() => descriptor.get.call({})),
        failure(() => descriptor.set.call(document.createTextNode("t"), null)),
        failure(() => descriptor.set.call(undefined, null)),
      ],
    };
  });
  const order = (() => {
    const element = document.createElement("div");
    const seen = [];
    element.onsearch = () => seen.push("idl");
    element.setAttribute("onsearch", "seen.push('attribute')");
    element.dispatchEvent(new Event("search"));
    element.setAttribute("onbeforecopy", "return 1");
    const compiled = element.onbeforecopy;
    element.onbeforecopy = null;
    const nulled = element.onbeforecopy;
    element.setAttribute("onbeforecopy", "return 2");
    return { seen, compiled: typeof compiled, nulled, recompiled: element.onbeforecopy.call(element) };
  })();
  body.textContent = "";
  return JSON.stringify({ matchesSelector, handlers, order });
})();
