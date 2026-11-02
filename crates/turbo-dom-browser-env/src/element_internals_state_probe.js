(() => {
  const events = [];
  class Bare extends HTMLElement {
    static formAssociated = true;
    formAssociatedCallback(form) {
      events.push(["form", form?.id || null]);
    }
    formDisabledCallback(value) {
      events.push(["disabled", value]);
    }
  }
  customElements.define("x-bare-internals", Bare);
  const form = document.createElement("form");
  form.id = "owner";
  document.body.append(form);
  const bare = new Bare();
  form.append(bare);
  bare.setAttribute("disabled", "");
  bare.removeAttribute("disabled");
  bare.remove();
  class Stateful extends HTMLElement {
    constructor() {
      super();
      this.internals = this.attachInternals();
      this.internals.states.add("ctor");
    }
  }
  customElements.define("x-stateful-internals", Stateful);
  const host = new Stateful();
  document.body.append(host);
  const states = host.internals.states;
  const keys = ["ready", "has space", "a,b", "\ud800", "\udc00", "\u0000"];
  for (const key of keys) states.add(key);
  const selectors = ["ready", "has\\ space", "a\\,b", "\ud800", "\udc00", "\ufffd"];
  const matches = selectors.map((selector) => {
    try {
      return host.matches(":state(" + selector + ")");
    } catch (error) {
      return error.name;
    }
  });
  const clone = host.cloneNode(true),
    other = document.implementation.createHTMLDocument("target");
  const imported = other.importNode(host, true);
  const units = (value) =>
    Array.from({ length: value.length }, (_, index) => value.charCodeAt(index));
  const stateUnits = (state) => [...state].map(units);
  const before = stateUnits(states),
    internals = host.internals;
  other.body.append(other.adoptNode(host));
  const adopted = [
    host.ownerDocument === other,
    host.internals === internals,
    stateUnits(states),
    host.matches(":state(ready)"),
    host.attributes.length,
  ];
  states.delete("ready");
  const removed = host.matches(":state(ready)");
  states.add("later");
  const later = host.matches(":state(later)");
  states.clear();
  return {
    events,
    matches,
    before,
    clone: stateUnits(clone.internals.states),
    imported: imported.internals ? stateUnits(imported.internals.states) : null,
    adopted,
    removed,
    later,
    cleared: host.matches(":state(later)"),
  };
})();
