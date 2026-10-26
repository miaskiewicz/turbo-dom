(() => {
  const result = {},
    events = [];
  const attempt = (fn) => {
    try {
      const value = fn();
      return { value: value === undefined ? "undefined" : value, error: null };
    } catch (error) {
      return { error: error.name };
    }
  };
  result.members = Object.getOwnPropertyNames(ElementInternals.prototype);
  result.statesMembers = Object.getOwnPropertyNames(CustomStateSet.prototype);
  result.div = attempt(() => document.createElement("div").attachInternals());
  result.undefined = attempt(() => document.createElement("x-undefined").attachInternals());
  class Plain extends HTMLElement {
    constructor() {
      super();
      this.internals = this.attachInternals();
    }
  }
  customElements.define("x-plain-internals", Plain);
  const plain = new Plain(),
    i = plain.internals;
  result.identity = [
    i instanceof ElementInternals,
    Object.prototype.toString.call(i),
    i.states === i.states,
  ];
  result.again = attempt(() => plain.attachInternals());
  result.formless = {};
  for (const name of ["form", "labels", "willValidate", "validity", "validationMessage"])
    result.formless[name] = attempt(() => {
      const value = i[name];
      return value === null
        ? null
        : typeof value === "object"
          ? Object.prototype.toString.call(value)
          : value;
    });
  result.formless.setFormValue = attempt(() => i.setFormValue("x"));
  result.formless.setValidity = attempt(() => i.setValidity({}));
  result.formless.checkValidity = attempt(() => i.checkValidity());
  result.formless.reportValidity = attempt(() => i.reportValidity());
  result.shadow = [i.shadowRoot];
  const shadow = plain.attachShadow({ mode: "closed" });
  result.shadow.push(i.shadowRoot === shadow, plain.shadowRoot === null);
  i.role = "button";
  i.ariaLabel = "private";
  result.aria = [i.role, i.ariaLabel, plain.getAttribute("role"), plain.getAttribute("aria-label")];
  plain.setAttribute("role", "link");
  result.aria.push(i.role);
  i.role = null;
  i.ariaLabel = undefined;
  result.aria.push(i.role, i.ariaLabel);
  const states = i.states;
  result.states = [];
  for (const value of ["foo", "--bar", "has space", ""])
    result.states.push([value, attempt(() => states.add(value))]);
  result.states.push([...states], states.size, states.has("foo"), states.delete("foo"), [
    ...states.entries(),
  ]);
  result.states.push(attempt(() => states.add(Symbol("x"))));
  states.clear();
  result.states.push(states.size);
  class Disabled extends HTMLElement {
    static disabledFeatures = ["internals"];
  }
  customElements.define("x-disabled-internals", Disabled);
  result.disabled = attempt(() => new Disabled().attachInternals());
  class Builtin extends HTMLButtonElement {}
  customElements.define("x-builtin-internals", Builtin, { extends: "button" });
  result.builtin = attempt(() => new Builtin().attachInternals());
  return result;
})();
