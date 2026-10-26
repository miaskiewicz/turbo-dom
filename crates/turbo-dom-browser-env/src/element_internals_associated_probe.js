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
  class Associated extends HTMLElement {
    static formAssociated = true;
    constructor() {
      super();
      this.internals = this.attachInternals();
    }
    formAssociatedCallback(form) {
      events.push(["form", form?.id || null]);
    }
    formDisabledCallback(disabled) {
      events.push(["disabled", disabled]);
    }
    formResetCallback() {
      events.push(["reset"]);
    }
  }
  customElements.define("x-associated-internals", Associated);
  const form = document.createElement("form");
  form.id = "first";
  const other = document.createElement("form");
  other.id = "second";
  document.body.append(form, other);
  const host = new Associated(),
    internals = host.internals;
  host.id = "host";
  host.setAttribute("name", "custom");
  form.append(host);
  result.associated = [
    internals.form === form,
    internals.willValidate,
    internals.validity.valid,
    internals.validationMessage,
  ];
  internals.setFormValue("saved");
  const invalid = [];
  host.addEventListener("invalid", (event) => {
    invalid.push([event.isTrusted, event.cancelable, event.bubbles]);
    event.preventDefault();
  });
  result.missingMessage = attempt(() => internals.setValidity({ customError: true }));
  internals.setValidity({ customError: true }, "custom issue");
  result.invalid = [
    internals.validity.customError,
    internals.validity.valid,
    internals.validationMessage,
    internals.checkValidity(),
    form.checkValidity(),
    invalid,
  ];
  result.outsideAnchor = attempt(() =>
    internals.setValidity({ customError: true }, "issue", document.createElement("div")),
  );
  internals.setValidity({});
  result.cleared = [
    internals.validity.valid,
    internals.validationMessage,
    internals.checkValidity(),
  ];
  const label = document.createElement("label");
  label.htmlFor = "host";
  label.textContent = "Custom";
  document.body.append(label);
  const labels = internals.labels;
  result.labels = [
    Object.prototype.toString.call(labels),
    labels.length,
    labels[0] === label,
    label.control === host,
  ];
  label.remove();
  result.labels.push(labels.length);
  other.append(host);
  host.setAttribute("disabled", "");
  result.disabledHost = [internals.willValidate];
  host.removeAttribute("disabled");
  other.reset();
  host.setAttribute("form", "first");
  result.externalForm = internals.form === form;
  result.events = events;
  return result;
})();
