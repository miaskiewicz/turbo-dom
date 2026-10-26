(function () {
  "use strict";
  const g = globalThis,
    ns = g[Symbol.for("ts")],
    make = ns.makeNativeFunction;
  const factory = ns.formCollectionFactory,
    forms = g.__captureFormCollections;
  delete ns.formCollectionFactory;
  delete g.__captureFormCollections;
  const apply = Reflect.apply,
    define = Object.defineProperty,
    setPrototype = Object.setPrototypeOf;
  const StringValue = String,
    ErrorType = TypeError,
    MapValue = Map,
    WeakRefValue = WeakRef;
  const mapGet = Map.prototype.get,
    mapSet = Map.prototype.set,
    mapEach = Map.prototype.forEach,
    mapDelete = Map.prototype.delete,
    deref = WeakRef.prototype.deref;
  const checked = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "checked");
  const string = (value) => {
    if (typeof value === "symbol") throw new ErrorType("Cannot convert a Symbol to a string");
    return StringValue(value);
  };
  const local = (element) => apply(forms.localNameGetter, element, []);
  const html = (element) => !!forms.bridge("html-element", element);
  const labelListPrototype = NodeList.prototype;
  function labelsList(element) {
    let list = forms.privateGet(element, "control-labels");
    if (!list) {
      list = factory.live(() => forms.bridge("labels-rows", element), false, {
        prototype: labelListPrototype,
        kind: "NodeList",
        readonlyIndices: true,
      });
      forms.privateSet(element, "control-labels", list);
    }
    return list;
  }
  forms.bridge("install-labels", document, labelsList);
  for (const [name, tag] of [
    ["HTMLInputElement", "input"],
    ["HTMLButtonElement", "button"],
    ["HTMLTextAreaElement", "textarea"],
    ["HTMLSelectElement", "select"],
    ["HTMLOutputElement", "output"],
    ["HTMLMeterElement", "meter"],
    ["HTMLProgressElement", "progress"],
  ]) {
    const constructor = g[name];
    if (!constructor) continue;
    define(constructor.prototype, "labels", {
      get: make(
        function () {
          if (!html(this) || local(this) !== tag) throw new ErrorType("Illegal invocation");
          if (tag === "input" && forms.bridge("labels-hidden", this)) return null;
          return forms.bridge("labels-list", this);
        },
        "get labels",
        0,
      ),
      enumerable: true,
      configurable: true,
    });
  }
  g.__captureFormInternals.labels = (element) => forms.bridge("labels-list", element);
  g.__captureFormInternals.labelControl = (element) => {
    if (!html(element) || local(element) !== "label") throw new ErrorType("Illegal invocation");
    return forms.bridge("label-control", element);
  };
  const radio = (element) =>
    html(element) && local(element) === "input" && forms.type(element) === "radio";
  const value = (element) => forms.attr(element, "value") ?? "on";
  function setChecked(element, state) {
    if (!html(element) || local(element) !== "input") throw new ErrorType("Illegal invocation");
    apply(checked.set, element, [state]);
  }
  const row = (array, item) =>
    define(array, array.length, {
      __proto__: null,
      value: item,
      writable: true,
      enumerable: true,
      configurable: true,
    });
  function listed(form) {
    const rows = forms.formControls(form),
      result = [];
    for (let index = 0; index < rows.length; index++) {
      const element = rows[index];
      const kind = local(element);
      if (
        forms.associatedGet(element) ||
        (html(element) &&
          (kind === "input" ||
            kind === "button" ||
            kind === "textarea" ||
            kind === "select" ||
            kind === "fieldset" ||
            kind === "object" ||
            kind === "output"))
      ) {
        if (local(element) !== "input" || forms.type(element) !== "image") row(result, element);
      }
    }
    return result;
  }
  function namedRows(form, name) {
    const rows = listed(form),
      result = [];
    if (name !== "")
      for (let index = 0; index < rows.length; index++)
        if (forms.attr(rows[index], "id") === name || forms.attr(rows[index], "name") === name)
          row(result, rows[index]);
    return result;
  }
  function constructor(name, parent) {
    const result = make(
      function () {
        throw new ErrorType("Illegal constructor");
      },
      name,
      0,
      true,
    );
    delete result.prototype.constructor;
    setPrototype(result, parent);
    setPrototype(result.prototype, parent.prototype);
    return result;
  }
  const Controls = constructor("HTMLFormControlsCollection", HTMLCollection),
    Radios = constructor("RadioNodeList", NodeList);
  const record = (receiver, kind) => {
    const value = factory.getRecord(receiver);
    if (!value || value.kind !== kind) throw new ErrorType("Illegal invocation");
    return value;
  };
  function cache(form) {
    let result = forms.privateGet(form, "form-collection-cache");
    if (!result) {
      result = { __proto__: null, controls: null, names: new MapValue() };
      forms.privateSet(form, "form-collection-cache", result);
    }
    return result;
  }
  function namedItem(form, name) {
    const rows = namedRows(form, name);
    if (!rows.length) return null;
    if (rows.length === 1) return rows[0];
    const state = cache(form),
      reference = apply(mapGet, state.names, [name]);
    let list = reference ? apply(deref, reference, []) : null;
    if (!list) {
      list = factory.live(() => namedRows(form, name), false, {
        __proto__: null,
        prototype: Radios.prototype,
        kind: "RadioNodeList",
        readonlyIndices: true,
        named: null,
        namedEnumerable: false,
      });
      apply(mapEach, state.names, [
        (reference, key) => {
          if (!apply(deref, reference, [])) apply(mapDelete, state.names, [key]);
        },
      ]);
      apply(mapSet, state.names, [name, new WeakRefValue(list)]);
    }
    return list;
  }
  define(Controls.prototype, "namedItem", {
    value: make(
      function (name) {
        const state = record(this, "HTMLFormControlsCollection");
        if (!arguments.length) throw new ErrorType("namedItem requires 1 argument");
        return state.named(string(name));
      },
      "namedItem",
      1,
    ),
    writable: true,
    enumerable: true,
    configurable: true,
  });
  define(Radios.prototype, "value", {
    get: make(
      function () {
        const rows = record(this, "RadioNodeList").read();
        for (let index = 0; index < rows.length; index++)
          if (radio(rows[index]) && forms.checkedValue(rows[index])) return value(rows[index]);
        return "";
      },
      "get value",
      0,
    ),
    set: make(
      function (next) {
        const state = record(this, "RadioNodeList");
        next = string(next);
        const rows = state.read();
        for (let index = 0; index < rows.length; index++)
          if (radio(rows[index]) && value(rows[index]) === next) {
            setChecked(rows[index], true);
            return;
          }
      },
      "set value",
      1,
    ),
    enumerable: true,
    configurable: true,
  });
  for (const [name, type, parent] of [
    ["HTMLFormControlsCollection", Controls, HTMLCollection],
    ["RadioNodeList", Radios, NodeList],
  ]) {
    define(type.prototype, "constructor", { value: type, writable: true, configurable: true });
    define(type.prototype, Symbol.toStringTag, { value: name, configurable: true });
    define(type.prototype, Symbol.iterator, {
      value: parent.prototype[Symbol.iterator],
      writable: true,
      configurable: true,
    });
    define(type, "prototype", { writable: false });
    define(g, name, { value: type, writable: true, enumerable: false, configurable: true });
  }
  function elements(form) {
    if (!html(form) || local(form) !== "form") throw new ErrorType("Illegal invocation");
    forms.checkForm(form);
    const state = cache(form);
    if (!state.controls) {
      const options = {
        __proto__: null,
        prototype: Controls.prototype,
        kind: "HTMLFormControlsCollection",
        readonlyIndices: true,
        hiddenNames: true,
        named: (_rows, name) => namedItem(form, name),
        namedEnumerable: true,
      };
      state.controls = factory.live(() => listed(form), true, options);
      const metadata = factory.getRecord(state.controls);
      define(metadata, "named", { __proto__: null, value: (name) => namedItem(form, name) });
    }
    return state.controls;
  }
  for (const [name, read] of [
    ["elements", elements],
    ["length", (form) => elements(form).length],
  ])
    define(HTMLFormElement.prototype, name, {
      get: make(
        function () {
          return read(this);
        },
        "get " + name,
        0,
      ),
      enumerable: true,
      configurable: true,
    });
  factory.setIterator((receiver) =>
    html(receiver) && local(receiver) === "form" ? elements(receiver) : null,
  );
  define(HTMLFormElement.prototype, Symbol.iterator, {
    value: HTMLCollection.prototype[Symbol.iterator],
    writable: true,
    configurable: true,
  });
})();
