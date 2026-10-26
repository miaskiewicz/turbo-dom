(async () => {
  const rows = [],
    events = [];
  const capture = (fn) => {
    try {
      return { value: fn() };
    } catch (error) {
      return { error: error.name };
    }
  };
  const label = (node) => (node.nodeType === 3 ? "#text:" + node.data : node.id || node.localName);
  const nodes = (slot, flatten = false) => slot.assignedNodes({ flatten }).map(label);
  const snapshot = (name, slots, children) =>
    rows.push({
      name,
      nodes: slots.map((slot) => nodes(slot)),
      flat: slots.map((slot) => nodes(slot, true)),
      elements: slots.map((slot) => slot.assignedElements().map(label)),
      assigned: children.map((child) => child.assignedSlot?.id || null),
    });
  const host = document.createElement("div");
  document.body.append(host);
  const a = document.createElement("b");
  a.id = "a";
  a.slot = "x";
  const b = document.createElement("i");
  b.id = "b";
  const text = document.createTextNode("light");
  host.append(a, b, text, document.createComment("ignored"));
  const root = host.attachShadow({ mode: "open" });
  root.innerHTML =
    '<slot id="x1" name="x"><span id="fallback-x">x</span></slot><slot id="x2" name="x"></slot><slot id="default"><span id="fallback-default">d</span></slot>';
  const x1 = root.querySelector("#x1"),
    x2 = root.querySelector("#x2"),
    def = root.querySelector("#default");
  for (const slot of [x1, x2, def])
    slot.addEventListener("slotchange", (event) =>
      events.push({
        slot: slot.id,
        target: event.target === slot,
        bubbles: event.bubbles,
        composed: event.composed,
        cancelable: event.cancelable,
        trusted: event.isTrusted,
      }),
    );
  new MutationObserver(() => events.push({ observer: true })).observe(host, {
    childList: true,
    subtree: true,
    attributes: true,
  });
  snapshot("named-initial", [x1, x2, def], [a, b, text]);
  await Promise.resolve();
  events.length = 0;
  a.slot = "";
  b.slot = "x";
  snapshot("slot-attribute", [x1, x2, def], [a, b, text]);
  Promise.resolve().then(() => events.push({ promise: true }));
  await Promise.resolve();
  const mutationOrder = events.splice(0);
  x1.name = "y";
  snapshot("slot-name", [x1, x2, def], [a, b, text]);
  x2.remove();
  snapshot("slot-remove", [x1, x2, def], [a, b, text]);
  a.remove();
  b.remove();
  text.remove();
  snapshot("fallback", [x1, x2, def], [a, b, text]);
  const nested = document.createElement("slot");
  nested.id = "nested";
  nested.append(document.createTextNode("nested fallback"));
  x1.replaceChildren(nested);
  snapshot("nested-fallback", [x1, nested], [a]);
  const outside = document.createElement("slot");
  outside.id = "outside";
  outside.append(document.createTextNode("outside fallback"));
  rows.push({ name: "outside", nodes: nodes(outside), flat: nodes(outside, true) });
  const lightSlotHost = document.createElement("div"),
    lightSlot = document.createElement("slot");
  lightSlot.id = "light-slot";
  lightSlot.append(document.createTextNode("must not flatten"));
  lightSlotHost.append(lightSlot);
  const lightRoot = lightSlotHost.attachShadow({ mode: "open" }),
    lightReceiver = document.createElement("slot");
  lightReceiver.id = "light-receiver";
  lightRoot.append(lightReceiver);
  snapshot("light-slot-kept", [lightReceiver, lightSlot], [lightSlot]);
  const closedHost = document.createElement("div"),
    closedChild = document.createElement("b");
  closedChild.id = "closed-child";
  closedHost.append(closedChild);
  const closed = closedHost.attachShadow({ mode: "closed" }),
    closedSlot = document.createElement("slot");
  closedSlot.id = "closed-slot";
  closed.append(closedSlot);
  snapshot("closed", [closedSlot], [closedChild]);
  const manualHost = document.createElement("div"),
    manualRoot = manualHost.attachShadow({ mode: "open", slotAssignment: "manual" });
  const first = document.createElement("slot"),
    second = document.createElement("slot");
  first.id = "manual1";
  second.id = "manual2";
  manualRoot.append(first, second);
  const m1 = document.createElement("i"),
    m2 = document.createElement("b"),
    mt = document.createTextNode("manual");
  m1.id = "m1";
  m2.id = "m2";
  manualHost.append(m1, m2, mt);
  snapshot("manual-initial", [first, second], [m1, m2, mt]);
  first.assign(m2, mt, m1, m2);
  snapshot("manual-order", [first, second], [m1, m2, mt]);
  second.assign(m1);
  snapshot("manual-steal", [first, second], [m1, m2, mt]);
  const badAssign = capture(() => first.assign(m1, document.createComment("bad")));
  snapshot("manual-failed", [first, second], [m1, m2, mt]);
  const foreign = document.implementation.createHTMLDocument().createElement("i");
  foreign.id = "foreign";
  second.assign(foreign);
  snapshot("manual-foreign-before", [second], [foreign]);
  manualHost.append(foreign);
  snapshot("manual-foreign-after", [second], [foreign]);
  first.assign();
  snapshot("manual-clear", [first, second], [m1, m2, mt, foreign]);
  const foreignDocument = document.implementation.createHTMLDocument(),
    foreignHost = foreignDocument.createElement("div"),
    foreignRoot = foreignHost.attachShadow({ mode: "open", slotAssignment: "manual" }),
    foreignSlot = foreignDocument.createElement("slot"),
    foreignChild = foreignDocument.createElement("b");
  foreignSlot.id = "foreign-slot";
  foreignChild.id = "foreign-child";
  foreignRoot.append(foreignSlot);
  foreignHost.append(foreignChild);
  foreignSlot.assign(foreignChild);
  snapshot("foreign-source-assigned", [foreignSlot], [foreignChild]);
  second.assign(foreignChild);
  snapshot("foreign-steal-before-adoption", [foreignSlot, second], [foreignChild]);
  manualHost.append(foreignChild);
  snapshot("foreign-steal-after-adoption", [foreignSlot, second], [foreignChild]);
  const invalidAssignment = capture(() =>
    document.createElement("div").attachShadow({ mode: "open", slotAssignment: "invalid" }),
  );
  const assignNamed = capture(() => x1.assign(b));
  snapshot("named-assign", [x1], [b]);
  const method = HTMLSlotElement.prototype.assignedNodes,
    getAssigned = Object.getOwnPropertyDescriptor(Element.prototype, "assignedSlot").get;
  const brands = [
    {},
    document.createElement("div"),
    document.createElementNS(null, "slot"),
    document.createElementNS("http://www.w3.org/1999/xhtml", "SLOT"),
    document.createElementNS("http://www.w3.org/1999/xhtml", "p:slot"),
  ].map((receiver) => capture(() => method.call(receiver).map(label)));
  const dictionaries = [
    null,
    undefined,
    true,
    7,
    "value",
    { flatten: true },
    {
      get flatten() {
        throw new RangeError("test");
      },
    },
  ].map((value) => capture(() => x1.assignedNodes(value).map(label)));
  const descriptor = Object.getOwnPropertyDescriptor(ShadowRoot.prototype, "slotAssignment");
  const shapes = {
    root: root.slotAssignment,
    manual: manualRoot.slotAssignment,
    descriptor: {
      enumerable: descriptor.enumerable,
      configurable: descriptor.configurable,
      name: descriptor.get.name,
      length: descriptor.get.length,
    },
    name: x1.name,
    methods: ["assign", "assignedNodes", "assignedElements"].map((name) => ({
      name,
      length: HTMLSlotElement.prototype[name].length,
      native: Function.prototype.toString
        .call(HTMLSlotElement.prototype[name])
        .includes("[native code]"),
    })),
    elementBrand: capture(() => getAssigned.call({})),
    textBrand: capture(() =>
      Object.getOwnPropertyDescriptor(Text.prototype, "assignedSlot").get.call(
        document.createComment("bad"),
      ),
    ),
  };
  await Promise.resolve();
  host.remove();
  const reentrantOrder = [],
    reentrantHost = document.createElement("div"),
    reentrantRoot = reentrantHost.attachShadow({ mode: "open" }),
    reentrantSlot = document.createElement("slot"),
    reentrantChild = document.createElement("b");
  reentrantRoot.append(reentrantSlot);
  reentrantSlot.addEventListener("slotchange", () =>
    reentrantOrder.push("slot:" + reentrantSlot.assignedNodes().length),
  );
  new MutationObserver(() => {
    reentrantOrder.push("observer");
    if (reentrantChild.slot !== "away") reentrantChild.slot = "away";
  }).observe(reentrantHost, { childList: true, subtree: true, attributes: true });
  reentrantHost.append(reentrantChild);
  Promise.resolve().then(() => reentrantOrder.push("promise"));
  await Promise.resolve();
  await Promise.resolve();
  return {
    reentrantOrder,
    rows,
    mutationOrder,
    badAssign,
    invalidAssignment,
    assignNamed,
    brands,
    dictionaries,
    shapes,
  };
})();
