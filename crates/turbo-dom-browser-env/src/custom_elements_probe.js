(async () => {
  const result = {};
  const attempt = (fn) => {
    try {
      const value = fn();
      return value === undefined ? "undefined" : value;
    } catch (error) {
      return error.name;
    }
  };
  const descriptor = (object, name) => {
    const d = Object.getOwnPropertyDescriptor(object, name);
    return (
      d && {
        enumerable: d.enumerable,
        configurable: d.configurable,
        writable: d.writable,
        get: !!d.get,
        set: !!d.set,
      }
    );
  };
  result.shape = {
    registry: typeof CustomElementRegistry,
    tag: Object.prototype.toString.call(customElements),
    parent: Object.getPrototypeOf(CustomElementRegistry.prototype) === Object.prototype,
    instance: customElements instanceof CustomElementRegistry,
    own: Reflect.ownKeys(CustomElementRegistry.prototype).map((key) =>
      typeof key === "symbol" ? key.toString() : key,
    ),
    global: descriptor(globalThis, "customElements"),
    construct: attempt(() => Object.prototype.toString.call(new CustomElementRegistry())),
    html: attempt(() => new HTMLElement()),
  };
  result.methods = {};
  for (const name of ["define", "get", "getName", "upgrade", "whenDefined"]) {
    const fn = CustomElementRegistry.prototype[name];
    result.methods[name] = {
      length: fn.length,
      name: fn.name,
      native: Function.prototype.toString.call(fn).includes("[native code]"),
      descriptor: descriptor(CustomElementRegistry.prototype, name),
      borrow: attempt(() => fn.call({}, "x-no")),
    };
  }
  result.names = [];
  for (const name of [
    "plain",
    "X-upper",
    "x-UPPER",
    "annotation-xml",
    "font-face",
    "x-",
    "x-a",
    "x-\u{1f600}",
    "x-\ud800",
  ]) {
    class Definition extends HTMLElement {}
    result.names.push([name, attempt(() => customElements.define(name, Definition))]);
  }
  class Unique extends HTMLElement {}
  customElements.define("x-unique", Unique);
  result.duplicates = [
    attempt(() => customElements.define("x-unique", class extends HTMLElement {})),
    attempt(() => customElements.define("x-unique-again", Unique)),
  ];
  result.get = [
    customElements.get("x-unique") === Unique,
    customElements.get("x-missing") === undefined,
    customElements.getName(Unique),
    customElements.getName(class extends HTMLElement {}),
    attempt(() => customElements.get(Symbol())),
    attempt(() => customElements.getName({})),
    attempt(() => customElements.define("x-arrow", () => {})),
  ];
  const pending = customElements.whenDefined("x-wait");
  result.when = {
    samePending: pending === customElements.whenDefined("x-wait"),
    invalidSync: "returned",
  };
  let invalid;
  try {
    invalid = customElements.whenDefined("plain");
  } catch (error) {
    result.when.invalidSync = error.name;
  }
  result.when.invalid = invalid
    ? await invalid.then(
        () => "resolved",
        (error) => error.name,
      )
    : null;
  class Waiting extends HTMLElement {}
  customElements.define("x-wait", Waiting);
  result.when.value = (await pending) === Waiting;
  result.when.fulfilled = (await customElements.whenDefined("x-unique")) === Unique;
  const reads = [];
  class Observed extends HTMLElement {}
  for (const name of [
    "connectedCallback",
    "disconnectedCallback",
    "adoptedCallback",
    "attributeChangedCallback",
    "connectedMoveCallback",
  ]) {
    Object.defineProperty(Observed.prototype, name, {
      get() {
        reads.push(name);
        return function () {};
      },
      configurable: true,
    });
  }
  Object.defineProperty(Observed, "observedAttributes", {
    get() {
      reads.push("observedAttributes");
      return ["data-v"];
    },
  });
  customElements.define("x-observed", Observed);
  result.reads = reads;
  const events = [];
  let original;
  const existing = document.createElement("x-late");
  existing.setAttribute("data-v", "before");
  document.body.append(existing);
  class Late extends HTMLElement {
    static observedAttributes = ["data-v"];
    constructor() {
      super();
      original = this;
      events.push(["construct", this.localName, this.ownerDocument === document]);
    }
    connectedCallback() {
      events.push(["connected", this === existing]);
    }
    disconnectedCallback() {
      events.push(["disconnected", this === existing]);
    }
    attributeChangedCallback(name, old, value, namespace) {
      events.push(["attribute", name, old, value, namespace]);
    }
    adoptedCallback(old, owner) {
      events.push(["adopted", old === document, owner === destination]);
    }
  }
  let destination;
  customElements.define("x-late", Late);
  result.upgrade = {
    same: original === existing,
    prototype: Object.getPrototypeOf(existing) === Late.prototype,
    constructor: existing.constructor === Late,
    instance: existing instanceof Late,
  };
  existing.setAttribute("data-v", "after");
  existing.remove();
  document.body.append(existing);
  destination = document.implementation.createHTMLDocument();
  destination.adoptNode(existing);
  destination.body.append(existing);
  result.lifecycle = events;
  const direct = document.createElement("x-unique"),
    created = new Unique();
  result.constructed = [
    direct instanceof Unique,
    created instanceof Unique,
    direct.localName,
    created.localName,
    created.ownerDocument === document,
    created.isConnected,
    created.attributes.length,
    created.childNodes.length,
  ];
  const detached = document.createElement("div");
  detached.innerHTML = "<x-detached></x-detached>";
  const child = detached.firstChild;
  let upgraded = 0;
  class Detached extends HTMLElement {
    constructor() {
      super();
      upgraded++;
    }
  }
  customElements.define("x-detached", Detached);
  result.detached = {
    before: upgraded,
    instanceBefore: child instanceof Detached,
  };
  customElements.upgrade(detached);
  result.detached.after = upgraded;
  result.detached.instanceAfter = child instanceof Detached;
  const host = document.createElement("div"),
    shadow = host.attachShadow({ mode: "closed" });
  shadow.innerHTML = "<x-shadow></x-shadow>";
  document.body.append(host);
  let shadowCount = 0;
  class Shadow extends HTMLElement {
    constructor() {
      super();
      shadowCount++;
    }
  }
  customElements.define("x-shadow", Shadow);
  result.shadow = [shadowCount, shadow.firstChild instanceof Shadow];
  return result;
})();
