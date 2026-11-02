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
  result.names = [];
  for (const name of [
    "x-a:b",
    "x-a!",
    "x-a?",
    'x-a"',
    "x-a'",
    "x-a=",
    "x-a[",
    "x-a]",
    "x-a\\",
    "x-a`",
    "x-a\u0001",
    "x-a\u007f",
    "x-a ",
    "x-a/",
    "x-a>",
  ])
    result.names.push([
      name,
      attempt(() => customElements.define(name, class extends HTMLElement {})),
    ]);
  result.observed = [];
  for (const observed of [null, undefined, false, 0, "xy", {}, ["a", Symbol()]]) {
    class Observed extends HTMLElement {
      attributeChangedCallback() {}
    }
    Object.defineProperty(Observed, "observedAttributes", { value: observed });
    result.observed.push(
      attempt(() => customElements.define("x-observed-" + result.observed.length, Observed)),
    );
  }
  result.methodConversions = [
    attempt(() => customElements.getName(() => {})),
    attempt(() => customElements.getName(Symbol())),
    attempt(() => customElements.upgrade({})),
    attempt(() => customElements.initialize({})),
  ];
  let errors = [];
  addEventListener("error", (event) => {
    errors.push([event.error.name, event.isTrusted]);
    event.preventDefault();
  });
  const phases = [],
    old = document.createElement("x-phases");
  old.setAttribute("data-v", "old");
  document.body.append(old);
  class Phases extends HTMLElement {
    static observedAttributes = ["data-v"];
    constructor() {
      super();
      phases.push(["ctor-before"]);
      this.setAttribute("data-v", "ctor");
      phases.push(["ctor-after"]);
    }
    attributeChangedCallback(name, old, value) {
      phases.push(["attribute", old, value]);
    }
    connectedCallback() {
      phases.push(["connected"]);
    }
  }
  const ready = customElements.whenDefined("x-phases");
  ready.then(() => phases.push(["defined"]));
  customElements.define("x-phases", Phases);
  phases.push(["after-define"]);
  await ready;
  result.phases = phases;
  const failEvents = [],
    failed = document.createElement("x-failed-upgrade");
  failed.setAttribute("data-v", "old");
  document.body.append(failed);
  class Failed extends HTMLElement {
    static observedAttributes = ["data-v"];
    constructor() {
      super();
      this.setAttribute("data-v", "new");
      throw new Error("fail");
    }
    attributeChangedCallback(...args) {
      failEvents.push(["attribute", ...args]);
    }
    connectedCallback() {
      failEvents.push(["connected"]);
    }
  }
  customElements.define("x-failed-upgrade", Failed);
  customElements.upgrade(failed);
  result.failedUpgrade = [failed instanceof Failed, failed.getAttribute("data-v"), failEvents];
  const cloneEvents = [];
  class Clone extends HTMLElement {
    static observedAttributes = ["data-v"];
    constructor() {
      super();
      cloneEvents.push(["ctor", this.localName, this.getAttribute("data-v")]);
    }
    attributeChangedCallback(name, old, value) {
      cloneEvents.push(["attribute", old, value]);
    }
  }
  customElements.define("x-clone", Clone);
  const source = document.createElement("x-clone");
  source.setAttribute("data-v", "v");
  source.append(document.createTextNode("body"));
  cloneEvents.length = 0;
  const clone = source.cloneNode(true);
  const target = document.implementation.createHTMLDocument();
  const imported = target.importNode(source, true);
  result.clones = [
    clone instanceof Clone,
    clone.textContent,
    clone.getAttribute("data-v"),
    imported instanceof Clone,
    imported.ownerDocument === target,
    cloneEvents,
  ];
  const moves = [];
  class Moving extends HTMLElement {
    connectedCallback() {
      moves.push(["connected", this.isConnected]);
    }
    disconnectedCallback() {
      moves.push(["disconnected", this.isConnected]);
    }
    adoptedCallback(a, b) {
      moves.push(["adopted", a === document, b === target]);
    }
  }
  customElements.define("x-moving", Moving);
  const moving = document.createElement("x-moving"),
    box = document.createElement("div");
  document.body.append(box, moving);
  moves.length = 0;
  box.append(moving);
  moving.replaceWith(moving);
  target.adoptNode(box);
  target.body.append(box);
  result.moves = moves;
  const scope = new CustomElementRegistry(),
    host = document.createElement("div"),
    shadow = host.attachShadow({ mode: "open", customElementRegistry: scope });
  document.body.append(host);
  shadow.innerHTML = "<x-bound></x-bound>";
  let global = 0,
    scoped = 0;
  class Global extends HTMLElement {
    constructor() {
      super();
      global++;
    }
  }
  customElements.define("x-bound", Global);
  const before = [global, scoped, shadow.firstChild instanceof Global];
  class Scoped extends HTMLElement {
    constructor() {
      super();
      scoped++;
    }
  }
  scope.define("x-bound", Scoped);
  result.binding = [before, [global, scoped, shadow.firstChild instanceof Scoped]];
  const noHost = document.createElement("div"),
    noRoot = noHost.attachShadow({ mode: "open", customElementRegistry: null });
  noRoot.innerHTML = "<x-bound></x-bound>";
  scope.initialize(noRoot);
  result.initialized = [
    noRoot.customElementRegistry === scope,
    noRoot.firstChild instanceof Scoped,
  ];
  result.errors = errors;
  return result;
})();
