(async () => {
  const result = {},
    attempt = (fn) => {
      try {
        const value = fn();
        return value === undefined ? "undefined" : value;
      } catch (error) {
        return error.name;
      }
    };
  result.borrowWhen = await CustomElementRegistry.prototype.whenDefined.call({}, "x-borrow").then(
    () => "resolved",
    (e) => e.name,
  );
  result.callConstructor = attempt(() => CustomElementRegistry());
  result.names = [];
  for (const name of ["x-\u00d7", "x-\u037e", "x-\ufffe", "x-\u{f0000}", "x-\u{effff}", "a:b-x"])
    result.names.push([
      name,
      attempt(() => customElements.define(name, class extends HTMLElement {})),
    ]);
  result.options = [];
  for (const option of [
    true,
    1,
    "button",
    null,
    {},
    { extends: "unknown" },
    { extends: "DIV" },
    { extends: "div" },
    { extends: "x-tag" },
  ]) {
    const name = "x-opt-" + result.options.length;
    result.options.push(
      attempt(() => customElements.define(name, class extends HTMLElement {}, option)),
    );
  }
  const order = [];
  const ctor = new Proxy(class extends HTMLElement {}, {
    get(target, name, receiver) {
      order.push(String(name));
      return Reflect.get(target, name, receiver);
    },
  });
  result.conversion = attempt(() =>
    customElements.define("invalid", ctor, {
      get extends() {
        order.push("extends");
        return "div";
      },
    }),
  );
  result.conversionOrder = order;
  class Reused extends HTMLElement {}
  customElements.define("x-reused", Reused);
  const other = new CustomElementRegistry();
  result.sameConstructorOtherRegistry = attempt(() => other.define("x-reused-other", Reused));
  result.globalRegistry = [
    Object.getOwnPropertyDescriptor(Document.prototype, "customElementRegistry") !== undefined,
    document.customElementRegistry === customElements,
  ];
  const events = [];
  class Button extends HTMLButtonElement {
    constructor() {
      super();
      events.push(["ctor", this.localName, this.getAttribute("is")]);
    }
    connectedCallback() {
      events.push(["connected", this.isConnected]);
    }
  }
  customElements.define("x-button", Button, { extends: "button" });
  const direct = new Button(),
    created = document.createElement("button", { is: "x-button" }),
    legacy = document.createElement("button", "x-button");
  result.builtin = [
    direct.localName,
    direct.getAttribute("is"),
    direct instanceof Button,
    created instanceof Button,
    created.getAttribute("is"),
    created.outerHTML,
    legacy instanceof Button,
    legacy.getAttribute("is"),
  ];
  class Wrong extends HTMLDivElement {}
  customElements.define("x-wrong", Wrong);
  result.wrongSuperclass = attempt(() => new Wrong());
  const failures = [];
  const errors = [];
  addEventListener("error", (event) => {
    errors.push([event.error?.name, event.isTrusted]);
    event.preventDefault();
  });
  class AddsAttribute extends HTMLElement {
    constructor() {
      super();
      this.setAttribute("bad", "yes");
    }
  }
  customElements.define("x-add-attribute", AddsAttribute);
  const invalid = document.createElement("x-add-attribute");
  failures.push([
    invalid instanceof AddsAttribute,
    invalid.localName,
    invalid.getAttribute("bad"),
    Object.getPrototypeOf(invalid) === HTMLUnknownElement.prototype,
  ]);
  class ReturnsOther extends HTMLElement {
    constructor() {
      super();
      return document.createElement("i");
    }
  }
  customElements.define("x-other-result", ReturnsOther);
  const invalidResult = document.createElement("x-other-result");
  failures.push([invalidResult instanceof ReturnsOther, invalidResult.localName]);
  class Throws extends HTMLElement {
    constructor() {
      super();
      throw new RangeError("expected");
    }
  }
  customElements.define("x-throws", Throws);
  const invalidThrows = document.createElement("x-throws");
  failures.push([invalidThrows instanceof Throws, invalidThrows.localName]);
  result.failures = failures;
  result.errors = errors;
  const scope = new CustomElementRegistry(),
    host = document.createElement("div"),
    shadow = host.attachShadow({ mode: "open", customElementRegistry: scope });
  document.body.append(host);
  let localCount = 0;
  class Local extends HTMLElement {
    constructor() {
      super();
      localCount++;
    }
  }
  scope.define("x-local", Local);
  const scoped = document.createElement("x-local", {
    customElementRegistry: scope,
  });
  shadow.append(scoped);
  shadow.innerHTML += "<x-local></x-local>";
  result.scoped = [
    shadow.customElementRegistry === scope,
    scoped instanceof Local,
    localCount,
    shadow.lastChild instanceof Local,
    attempt(() => customElements.get("x-local")),
    attempt(() => new Local()) instanceof Local,
  ];
  const emptyHost = document.createElement("div"),
    empty = emptyHost.attachShadow({
      mode: "open",
      customElementRegistry: null,
    });
  result.initialize = [
    empty.customElementRegistry === null,
    attempt(() => scope.initialize(empty)),
    empty.customElementRegistry === scope,
    attempt(() => customElements.initialize(empty)),
    attempt(() => scope.initialize(document.createElement("div"))),
    attempt(() => scope.initialize(document.createDocumentFragment())),
  ];
  result.events = events;
  return result;
})();
