(function () {
  "use strict";
  const g = globalThis,
    ns = g[Symbol.for("ts")],
    make = ns.makeNativeFunction,
    bridge = g.__internalsNative;
  const ce = g.__captureCustomElementInternals,
    forms = g.__captureFormInternals;
  delete g.__internalsNative;
  delete g.__captureCustomElementInternals;
  delete g.__captureFormInternals;
  delete ns.internalsLiveCollection;
  const wellFormed = String.prototype.toWellFormed;
  const apply = Reflect.apply,
    define = Object.defineProperty,
    create = Object.create,
    freeze = Object.freeze;
  const Exception = DOMException,
    ErrorType = TypeError,
    StringValue = String,
    MapValue = Map,
    WeakRefValue = WeakRef;
  const internals = "internals",
    states = "states",
    hosts = "attached",
    records = "host",
    tracked = [];
  const deref = WeakRef.prototype.deref;
  const read = (map, key) => bridge("private", key, map),
    store = (map, key, value) => bridge("private", key, map, value);
  const mapSet = Map.prototype.set,
    mapDelete = Map.prototype.delete,
    mapHas = Map.prototype.has,
    mapClear = Map.prototype.clear,
    mapEntries = Map.prototype.entries,
    mapForEach = Map.prototype.forEach;
  const mapSize = Object.getOwnPropertyDescriptor(Map.prototype, "size").get;
  const nativeIterator = apply(mapEntries, new MapValue(), []),
    nativeIteratorProto = Object.getPrototypeOf(nativeIterator),
    nativeNext = nativeIteratorProto.next;
  const string = (value) => {
    if (typeof value === "symbol") throw new ErrorType("Cannot convert a Symbol to a string");
    return StringValue(value);
  };
  const need = (receiver, map) => {
    const record = read(map, receiver);
    if (!record) throw new ErrorType("Illegal invocation");
    return record;
  };
  const method = (proto, name, length, fn) =>
    define(proto, name, {
      value: make(fn, name, length),
      writable: true,
      enumerable: true,
      configurable: true,
    });
  const accessor = (proto, name, fn, setter) =>
    define(proto, name, {
      get: make(fn, "get " + name, 0),
      ...(setter ? { set: make(setter, "set " + name, 1) } : {}),
      enumerable: true,
      configurable: true,
    });
  const flagNames = [
    "badInput",
    "customError",
    "patternMismatch",
    "rangeOverflow",
    "rangeUnderflow",
    "stepMismatch",
    "tooLong",
    "tooShort",
    "typeMismatch",
    "valueMissing",
  ];
  const flags = () => {
    const value = create(null);
    for (const name of flagNames) value[name] = false;
    value.valid = true;
    return value;
  };
  const Internals = make(
    function ElementInternals() {
      throw new ErrorType("Illegal constructor");
    },
    "ElementInternals",
    0,
    true,
  );
  define(Internals, "prototype", { writable: false });
  define(Internals.prototype, Symbol.toStringTag, {
    value: "ElementInternals",
    configurable: true,
  });
  const CustomStates = make(
    function CustomStateSet() {
      throw new ErrorType("Illegal constructor");
    },
    "CustomStateSet",
    0,
    true,
  );
  define(CustomStates, "prototype", { writable: false });
  define(CustomStates.prototype, Symbol.toStringTag, {
    value: "CustomStateSet",
    configurable: true,
  });
  // WebIDL setlike iterators can resume after exhaustion. Generation keys avoid
  // retaining deleted history; the private owner edge preserves native lifetime.
  const iteratorFamily = "iterator",
    iteratorPrototype = create(Object.getPrototypeOf(nativeIteratorProto));
  define(iteratorPrototype, Symbol.toStringTag, {
    value: "CustomStateSet Iterator",
    configurable: true,
  });
  method(iteratorPrototype, "next", 0, function () {
    const state = need(this, iteratorFamily),
      record = state.record;
    if (state.exhausted) {
      if (state.observed === record.generation) return { done: true };
      state.iterator = apply(mapEntries, record.values, []);
      state.exhausted = false;
    }
    while (true) {
      const next = apply(nativeNext, state.iterator, []);
      if (next.done) {
        state.exhausted = true;
        state.observed = record.generation;
        return { done: true };
      }
      const key = next.value[0],
        generation = next.value[1];
      if (generation <= state.cursor) continue;
      state.cursor = generation;
      return { done: false, value: state.entries ? [key, key] : key };
    }
  });
  accessor(CustomStates.prototype, "size", function () {
    return apply(mapSize, need(this, states).values, []);
  });
  method(CustomStates.prototype, "add", 1, function (value) {
    const record = need(this, states);
    if (!arguments.length) throw new ErrorType("One argument required");
    value = string(value);
    bridge("add", record.host, value);
    if (!apply(mapHas, record.values, [value]))
      apply(mapSet, record.values, [value, ++record.generation]);
  });
  method(CustomStates.prototype, "delete", 1, function (value) {
    const record = need(this, states);
    if (!arguments.length) throw new ErrorType("One argument required");
    value = string(value);
    bridge("delete", record.host, value);
    return apply(mapDelete, record.values, [value]);
  });
  method(CustomStates.prototype, "clear", 0, function () {
    const record = need(this, states);
    bridge("clear", record.host);
    apply(mapClear, record.values, []);
  });
  method(CustomStates.prototype, "has", 1, function (value) {
    const record = need(this, states);
    if (!arguments.length) throw new ErrorType("One argument required");
    return apply(mapHas, record.values, [string(value)]);
  });
  for (const name of ["entries", "keys", "values"])
    method(CustomStates.prototype, name, 0, function () {
      const record = need(this, states),
        value = create(iteratorPrototype);
      store(iteratorFamily, value, {
        owner: this,
        record,
        entries: name === "entries",
        cursor: 0n,
        observed: record.generation,
        exhausted: false,
        iterator: apply(mapEntries, record.values, []),
      });
      return value;
    });
  method(CustomStates.prototype, "forEach", 1, function (callback, thisArg) {
    const record = need(this, states),
      self = this;
    if (typeof callback !== "function") throw new ErrorType("Callback must be callable");
    apply(mapForEach, record.values, [
      (_generation, key) => apply(callback, thisArg, [key, key, self]),
    ]);
  });
  define(CustomStates.prototype, Symbol.iterator, {
    value: CustomStates.prototype.values,
    writable: true,
    configurable: true,
  });
  function recordFor(host, definition) {
    let record = read(records, host);
    if (record) return record;
    record = {
      host,
      associated: definition.formAssociated,
      owner: null,
      disabled: false,
      flags: flags(),
      message: "",
      value: null,
      aria: create(null),
      states: null,
    };
    store(records, host, record);
    if (record.associated) {
      record.reset = () => ce.callback(host, "formResetCallback", []);
      forms.register(host, record);
      tracked[tracked.length] = new WeakRefValue(host);
    }
    return record;
  }
  ce.created((host, definition) => {
    if (definition.formAssociated) recordFor(host, definition);
  });
  ce.updates((event) => {
    let count = 0;
    for (let i = 0; i < tracked.length; i++) {
      const host = apply(deref, tracked[i], []);
      if (!host) continue;
      tracked[count++] = tracked[i];
      const record = read(records, host);
      if (!record?.associated || ce.record(host)?.status !== "custom") continue;
      if (event?.type === "attributes") {
        const name = event.attributeName;
        if (name !== "disabled" && name !== "form" && name !== "id") continue;
        if (name === "form" && event.target !== host) continue;
        if (name === "disabled" && event.target !== host && !apply(contains, event.target, [host]))
          continue;
        if (name === "id" && forms.attribute(host, "form") === null) continue;
      }
      if (event?.type === "childList") {
        let relevant = false;
        for (const rows of [event.removedNodes, event.addedNodes])
          for (let at = 0; at < rows.length; at++)
            if (
              apply(contains, rows[at], [host]) ||
              (record.owner && apply(contains, rows[at], [record.owner]))
            )
              relevant = true;
        if (!relevant) continue;
      }
      if (event?.type === "childList" && record.owner) {
        for (let at = 0; at < event.removedNodes.length; at++) {
          const removed = event.removedNodes[at];
          if (apply(contains, removed, [host]) && !apply(contains, removed, [record.owner])) {
            record.owner = null;
            ce.callback(host, "formAssociatedCallback", [null]);
            break;
          }
        }
      }
      const owner = forms.owner(host),
        disabled = forms.disabled(host);
      if (owner !== record.owner) {
        record.owner = owner;
        ce.callback(host, "formAssociatedCallback", [owner]);
      }
      if (disabled !== record.disabled) {
        record.disabled = disabled;
        ce.callback(host, "formDisabledCallback", [disabled]);
      }
    }
    tracked.length = count;
  });
  const requireForm = (record) => {
    if (!record.associated)
      throw new Exception("The element is not form-associated.", "NotSupportedError");
    return record;
  };
  const formRecord = (receiver) => requireForm(need(receiver, internals));
  accessor(Internals.prototype, "form", function () {
    return forms.owner(formRecord(this).host);
  });
  accessor(Internals.prototype, "willValidate", function () {
    return forms.candidate(formRecord(this).host);
  });
  accessor(Internals.prototype, "validity", function () {
    return forms.validity(formRecord(this).host);
  });
  accessor(Internals.prototype, "validationMessage", function () {
    return forms.message(formRecord(this).host);
  });
  accessor(Internals.prototype, "states", function () {
    return need(this, internals).states;
  });
  accessor(Internals.prototype, "shadowRoot", function () {
    return bridge("shadow", need(this, internals).host);
  });
  accessor(Internals.prototype, "labels", function () {
    return forms.labels(formRecord(this).host);
  });
  for (const name of ["checkValidity", "reportValidity"])
    method(Internals.prototype, name, 0, function () {
      return forms.validateControl(formRecord(this).host);
    });
  const snapshot = (record, value) => {
    if (value === null) return null;
    const branded = bridge("value-snapshot", record.host, value);
    if (branded !== null && branded !== undefined) return branded;
    return apply(wellFormed, string(value), []);
  };
  method(Internals.prototype, "setFormValue", 1, function (value, state) {
    const record = formRecord(this);
    if (!arguments.length) throw new ErrorType("One argument required");
    const converted = snapshot(record, value),
      restored = arguments.length > 1 ? snapshot(record, state) : converted;
    record.value = converted;
    record.restore = restored;
  });
  const contains = Node.prototype.contains;
  accessor(HTMLLabelElement.prototype, "control", function () {
    return forms.labelControl(this);
  });
  method(Internals.prototype, "setValidity", 1, function (input, message, anchor) {
    const record = formRecord(this);
    if (!arguments.length) throw new ErrorType("One argument required");
    const next = flags();
    if (input !== null && input !== undefined) {
      for (const key of flagNames) {
        next[key] = !!input[key];
        if (next[key]) next.valid = false;
      }
    }
    const text = message === undefined ? "" : string(message);
    if (anchor !== undefined && anchor !== null) forms.tag(anchor);
    if (!next.valid && text === "") throw new ErrorType("A validation message is required.");
    if (anchor !== undefined && !apply(contains, record.host, [anchor]))
      throw new Exception("The anchor is not a descendant.", "NotFoundError");
    record.flags = next;
    record.message = next.valid ? "" : text;
    record.anchor = anchor || null;
  });
  const ariaNames = [
    "role",
    "ariaAtomic",
    "ariaAutoComplete",
    "ariaBusy",
    "ariaBrailleLabel",
    "ariaBrailleRoleDescription",
    "ariaChecked",
    "ariaColCount",
    "ariaColIndex",
    "ariaColIndexText",
    "ariaColSpan",
    "ariaCurrent",
    "ariaDescription",
    "ariaDisabled",
    "ariaExpanded",
    "ariaHasPopup",
    "ariaHidden",
    "ariaInvalid",
    "ariaKeyShortcuts",
    "ariaLabel",
    "ariaLevel",
    "ariaLive",
    "ariaModal",
    "ariaMultiLine",
    "ariaMultiSelectable",
    "ariaOrientation",
    "ariaPlaceholder",
    "ariaPosInSet",
    "ariaPressed",
    "ariaReadOnly",
    "ariaRelevant",
    "ariaRequired",
    "ariaRoleDescription",
    "ariaRowCount",
    "ariaRowIndex",
    "ariaRowIndexText",
    "ariaRowSpan",
    "ariaSelected",
    "ariaSetSize",
    "ariaSort",
    "ariaValueMax",
    "ariaValueMin",
    "ariaValueNow",
    "ariaValueText",
  ];
  for (const name of ariaNames)
    accessor(
      Internals.prototype,
      name,
      function () {
        return need(this, internals).aria[name] ?? null;
      },
      function (value) {
        need(this, internals).aria[name] = value == null ? null : string(value);
      },
    );
  const references = [
    "ariaActionsElements",
    "ariaControlsElements",
    "ariaDescribedByElements",
    "ariaDetailsElements",
    "ariaErrorMessageElements",
    "ariaFlowToElements",
    "ariaLabelledByElements",
  ];
  for (const name of references)
    accessor(
      Internals.prototype,
      name,
      function () {
        return need(this, internals).aria[name] ?? null;
      },
      function (value) {
        const record = need(this, internals);
        if (value == null) {
          record.aria[name] = null;
          return;
        }
        const rows = [];
        for (const node of value) {
          forms.tag(node);
          rows[rows.length] = node;
        }
        record.aria[name] = freeze(rows);
      },
    );
  accessor(
    Internals.prototype,
    "ariaActiveDescendantElement",
    function () {
      return need(this, internals).aria.ariaActiveDescendantElement ?? null;
    },
    function (value) {
      const record = need(this, internals);
      if (value !== null && value !== undefined) forms.tag(value);
      record.aria.ariaActiveDescendantElement = value ?? null;
    },
  );
  method(HTMLElement.prototype, "attachInternals", 0, function () {
    const definition = ce.record(this)?.definition;
    if (
      !definition ||
      definition.name !== definition.local ||
      definition.disabledFeatures.includes("internals") ||
      read(hosts, this)
    )
      throw new Exception(
        "ElementInternals is not available for this element.",
        "NotSupportedError",
      );
    const value = create(Internals.prototype),
      stateObject = create(CustomStates.prototype),
      record = recordFor(this, definition);
    record.states = stateObject;
    store(internals, value, record);
    store(hosts, this, value);
    store(states, stateObject, { host: this, values: new MapValue(), generation: 0n });
    return value;
  });
  // WebIDL installation order is observable through own-property reflection.
  const reorder = (proto, names, constructor) => {
    const descriptors = Object.getOwnPropertyDescriptors(proto);
    for (const name of Object.getOwnPropertyNames(proto)) delete proto[name];
    for (const name of names)
      define(
        proto,
        name,
        name === "constructor"
          ? { value: constructor, writable: true, configurable: true }
          : descriptors[name],
      );
  };
  reorder(
    Internals.prototype,
    [
      "form",
      "willValidate",
      "validity",
      "validationMessage",
      "labels",
      "states",
      "shadowRoot",
      "role",
      "ariaAtomic",
      "ariaAutoComplete",
      "ariaBusy",
      "ariaBrailleLabel",
      "ariaBrailleRoleDescription",
      "ariaChecked",
      "ariaColCount",
      "ariaColIndex",
      "ariaColSpan",
      "ariaCurrent",
      "ariaDescription",
      "ariaDisabled",
      "ariaExpanded",
      "ariaHasPopup",
      "ariaHidden",
      "ariaInvalid",
      "ariaKeyShortcuts",
      "ariaLabel",
      "ariaLevel",
      "ariaLive",
      "ariaModal",
      "ariaMultiLine",
      "ariaMultiSelectable",
      "ariaOrientation",
      "ariaPlaceholder",
      "ariaPosInSet",
      "ariaPressed",
      "ariaReadOnly",
      "ariaRelevant",
      "ariaRequired",
      "ariaRoleDescription",
      "ariaRowCount",
      "ariaRowIndex",
      "ariaRowSpan",
      "ariaSelected",
      "ariaSetSize",
      "ariaSort",
      "ariaValueMax",
      "ariaValueMin",
      "ariaValueNow",
      "ariaValueText",
      "checkValidity",
      "reportValidity",
      "setFormValue",
      "setValidity",
      "ariaColIndexText",
      "ariaRowIndexText",
      "ariaActionsElements",
      "ariaActiveDescendantElement",
      "ariaControlsElements",
      "ariaDescribedByElements",
      "ariaDetailsElements",
      "ariaErrorMessageElements",
      "ariaFlowToElements",
      "ariaLabelledByElements",
      "constructor",
    ],
    Internals,
  );
  reorder(
    CustomStates.prototype,
    [
      "size",
      "add",
      "clear",
      "delete",
      "entries",
      "forEach",
      "has",
      "keys",
      "values",
      "constructor",
    ],
    CustomStates,
  );
  g.__captureInternalsUpdates = (host, name) =>
    ce.notify({ type: "attributes", target: host, attributeName: name });
  define(g, "ElementInternals", { value: Internals, writable: true, configurable: true });
  define(g, "CustomStateSet", { value: CustomStates, writable: true, configurable: true });
})();
