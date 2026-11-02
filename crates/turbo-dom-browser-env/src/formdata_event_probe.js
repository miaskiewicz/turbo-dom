(() => {
  const result = {},
    data = new FormData();
  const attempt = (fn) => {
    try {
      const value = fn();
      return { value: value === undefined ? "undefined" : value, error: null };
    } catch (error) {
      return { error: error.name };
    }
  };
  result.members = Object.getOwnPropertyNames(FormDataEvent.prototype);
  const descriptor = Object.getOwnPropertyDescriptor(FormDataEvent.prototype, "formData");
  result.descriptor = [
    descriptor.enumerable,
    descriptor.configurable,
    descriptor.get.name,
    descriptor.get.length,
    descriptor.set === undefined,
  ];
  result.constructor = [
    FormDataEvent.name,
    FormDataEvent.length,
    Object.getPrototypeOf(FormDataEvent) === Event,
    Object.getPrototypeOf(FormDataEvent.prototype) === Event.prototype,
  ];
  result.failures = [
    attempt(() => new FormDataEvent()),
    attempt(() => new FormDataEvent("x")),
    attempt(() => new FormDataEvent("x", null)),
    attempt(() => new FormDataEvent("x", {})),
    attempt(() => new FormDataEvent("x", { formData: {} })),
    attempt(() => FormDataEvent("x", { formData: data })),
    attempt(() => descriptor.get.call({})),
  ];
  const event = new FormDataEvent("formdata", {
    formData: data,
    bubbles: true,
    cancelable: true,
    composed: true,
  });
  result.event = [
    event instanceof FormDataEvent,
    event instanceof Event,
    Object.prototype.toString.call(event),
    event.formData === data,
    event.bubbles,
    event.cancelable,
    event.composed,
    event.isTrusted,
  ];
  const reads = [];
  const init = {};
  for (const name of ["bubbles", "cancelable", "composed", "formData"])
    Object.defineProperty(init, name, {
      get() {
        reads.push(name);
        return name === "formData" ? data : true;
      },
    });
  new FormDataEvent("x", init);
  result.reads = reads;
  result.brand = attempt(() => descriptor.get.call(event) === data);
  const form = document.createElement("form");
  let delivered;
  form.addEventListener("formdata", (value) => {
    delivered = value;
  });
  const nativeData = new FormData(form);
  result.nativeEvent = [
    delivered.isTrusted,
    delivered.bubbles,
    delivered.cancelable,
    delivered.formData === nativeData,
  ];
  document.body.append(form);
  form.addEventListener("submit", (value) => {
    result.submitBrand = [
      value instanceof SubmitEvent,
      value.constructor === SubmitEvent,
      Object.getPrototypeOf(value) === SubmitEvent.prototype,
      Object.prototype.toString.call(value),
      Object.getOwnPropertyDescriptor(SubmitEvent.prototype, "submitter").get.call(value) === null,
    ];
    value.preventDefault();
  });
  form.requestSubmit();
  return result;
})();
