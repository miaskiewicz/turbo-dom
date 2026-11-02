(() => {
  const result = {};
  const describe = (error) => (error ? error.name : null);
  const attempt = (callback) => {
    try {
      const value = callback();
      return { value: value === undefined ? "undefined" : value, error: null };
    } catch (error) {
      return { error: describe(error) };
    }
  };
  const validityDescriptor = Object.getOwnPropertyDescriptor(ValidityState, "prototype");
  const validity = document.createElement("input").validity;
  const validGetter = Object.getOwnPropertyDescriptor(ValidityState.prototype, "valid").get;
  result.validityState = {
    prototypeDescriptor: [validityDescriptor.writable, validityDescriptor.enumerable, validityDescriptor.configurable],
    brand: [validity instanceof ValidityState, Object.getPrototypeOf(validity) === ValidityState.prototype, Object.prototype.toString.call(validity)],
    valid: attempt(() => validGetter.call(validity)),
    illegalReceiver: attempt(() => validGetter.call({})),
    illegalConstructor: attempt(() => new ValidityState()),
  };
  function fixture(content = '<input id="field"><button id="send">Send</button>') {
    const host = document.createElement("div");
    host.innerHTML = "<form>" + content + "</form>";
    document.body.append(host);
    return {
      host,
      form: host.querySelector("form"),
      input: host.querySelector("input"),
      button: host.querySelector("button"),
    };
  }
  for (const [owner, method] of [
    ["HTMLFormElement", "requestSubmit"],
    ["HTMLFormElement", "submit"],
    ["HTMLFormElement", "checkValidity"],
    ["HTMLFormElement", "reportValidity"],
    ["HTMLInputElement", "setCustomValidity"],
  ]) {
    const descriptor = Object.getOwnPropertyDescriptor(globalThis[owner].prototype, method);
    result[owner + "." + method] = descriptor
      ? {
          length: descriptor.value.length,
          name: descriptor.value.name,
          enumerable: descriptor.enumerable,
          writable: descriptor.writable,
          configurable: descriptor.configurable,
        }
      : null;
  }
  for (const tag of ["input", "textarea", "select", "button", "fieldset", "object", "output"]) {
    const control = document.createElement(tag);
    result[tag] = [
      control.willValidate,
      control.validationMessage,
      control.validity?.valid,
      control.checkValidity(),
      control.reportValidity(),
    ];
  }
  {
    const { form, input, host } = fixture("<input required><button>Send</button>");
    const events = [];
    input.addEventListener("invalid", (event) => {
      events.push([
        event.type,
        event.isTrusted,
        event.bubbles,
        event.cancelable,
        event.target === input,
      ]);
      event.preventDefault();
    });
    form.addEventListener("invalid", () => events.push("bubbled"));
    result.required = [
      input.willValidate,
      input.validity.valueMissing,
      input.validity.valid,
      input.checkValidity(),
      form.checkValidity(),
      events,
    ];
    input.value = "filled";
    result.filled = [input.validity.valueMissing, input.validity.valid, form.checkValidity()];
    input.setCustomValidity("own message");
    result.custom = [
      input.validationMessage,
      input.validity.customError,
      input.validity.valid,
      input.checkValidity(),
    ];
    input.setCustomValidity("");
    result.cleared = [input.validationMessage, input.validity.customError, input.checkValidity()];
    result.customSymbol = attempt(() => input.setCustomValidity(Symbol("x")));
    result.customMissing = attempt(() => input.setCustomValidity());
    host.remove();
  }
  for (const attribute of ["disabled", "readonly", 'type="hidden"']) {
    const { input, form, host } = fixture(
      "<input required " + attribute + "><button>Send</button>",
    );
    result[attribute] = [
      input.willValidate,
      input.validity.valueMissing,
      input.validity.valid,
      input.checkValidity(),
      form.checkValidity(),
    ];
    host.remove();
  }
  for (const kind of [
    "omitted",
    "undefined",
    "null",
    "button",
    "input",
    "foreign",
    "non-submit",
    "required",
    "novalidate",
    "formnovalidate",
  ]) {
    const f = fixture(
      kind === "input"
        ? '<input type="submit" id="send">'
        : '<input required><button id="send">Send</button>',
    );
    const log = [];
    f.form.addEventListener("submit", (event) => {
      log.push({
        type: event.type,
        constructor: event.constructor.name,
        trusted: event.isTrusted,
        bubbles: event.bubbles,
        cancelable: event.cancelable,
        composed: event.composed,
        submitter: event.submitter?.id || null,
      });
      event.preventDefault();
    });
    if (f.input)
      f.input.addEventListener("invalid", (event) => {
        log.push({ type: "invalid", trusted: event.isTrusted });
        event.preventDefault();
      });
    if (f.input && kind !== "required" && kind !== "novalidate" && kind !== "formnovalidate")
      f.input.value = "ok";
    if (kind === "novalidate") f.form.noValidate = true;
    if (kind === "formnovalidate") f.button.formNoValidate = true;
    let foreign;
    if (kind === "foreign") {
      foreign = fixture();
      foreign.button.id = "foreign";
    }
    if (kind === "non-submit") f.button.type = "button";
    result["request-" + kind] = [
      attempt(() => {
        if (kind === "omitted" || kind === "required" || kind === "novalidate")
          return f.form.requestSubmit();
        if (kind === "undefined") return f.form.requestSubmit(undefined);
        if (kind === "null") return f.form.requestSubmit(null);
        return f.form.requestSubmit(
          kind === "input" ? f.input : kind === "foreign" ? foreign.button : f.button,
        );
      }),
      log,
    ];
    f.host.remove();
    foreign?.host.remove();
  }
  const validityKeys = [
    "valueMissing",
    "typeMismatch",
    "patternMismatch",
    "tooLong",
    "tooShort",
    "rangeUnderflow",
    "rangeOverflow",
    "stepMismatch",
    "badInput",
    "customError",
    "valid",
  ];
  for (const [id, markup, value] of [
    ["email-missing-at", '<input type="email">', "bad"],
    ["email-empty-user", '<input type="email">', "@x.test"],
    ["email-empty-host", '<input type="email">', "x@"],
    ["email-invalid-host", '<input type="email">', "x@bad..host"],
    ["email-multiple", '<input type="email" multiple>', "first@a.test, second@b.test"],
    ["url-valid", '<input type="url">', "https://example.test/a"],
    ["url-invalid", '<input type="url">', "relative/path"],
    ["pattern", '<input pattern="[A-Z]{2}">', "ab"],
    ["pattern-invalid", '<input pattern="[">', "abc"],
    ["number-min", '<input type="number" min="5">', "2"],
    ["number-max", '<input type="number" max="5">', "8"],
    ["number-step", '<input type="number" step="2">', "3"],
    ["checkbox-required", '<input type="checkbox" required>', ""],
    ["textarea-required", "<textarea required></textarea>", ""],
    [
      "select-required",
      '<select required><option value="">Choose</option><option value="a">A</option></select>',
      "",
    ],
  ]) {
    const host = document.createElement("div");
    host.innerHTML = markup;
    document.body.append(host);
    const control = host.firstElementChild;
    control.value = value;
    const flags = {};
    for (const key of validityKeys) flags[key] = control.validity[key];
    result["flags-" + id] = [flags, control.validationMessage];
    host.remove();
  }
  return result;
})();
