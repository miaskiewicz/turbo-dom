(function () {
  "use strict";
  const g = globalThis,
    ns = g[Symbol.for("ts")],
    native = g.__shadowHtmlNative;
  delete g.__shadowHtmlNative;
  const make = ns.makeNativeFunction,
    apply = Reflect.apply,
    define = Object.defineProperty,
    describe = Object.getOwnPropertyDescriptor,
    prototypeOf = Object.getPrototypeOf;
  const TypeErrorValue = TypeError,
    WeakMapValue = WeakMap,
    iteratorKey = Symbol.iterator;
  const mapGet = WeakMapValue.prototype.get,
    mapSet = WeakMapValue.prototype.set;
  const shadowCheck = ns.domAccessor("__isShadowRoot", false, "Node");
  const elementBrand = ns.domAccessor("nodeType", false, "Element");
  const ownerDocument = ns.domAccessor("ownerDocument", false, "Node");
  const querySelectorAll = DocumentFragment.prototype.querySelectorAll;
  const addListener = EventTarget.prototype.addEventListener,
    removeListener = EventTarget.prototype.removeEventListener,
    preventDefault = Event.prototype.preventDefault;
  const lookup = (proto, name) => {
    for (let cursor = proto; cursor; cursor = prototypeOf(cursor)) {
      const descriptor = describe(cursor, name);
      if (descriptor) return descriptor;
    }
  };
  const localName = ns.domAccessor("localName", false, "Element");
  const styleSheet = lookup(HTMLStyleElement.prototype, "sheet").get,
    linkSheet = lookup(HTMLLinkElement.prototype, "sheet").get;
  const isShadow = (value) => {
    try {
      return apply(shadowCheck, value, []) === true;
    } catch {
      return false;
    }
  };
  const shadow = (value) => {
    if (!isShadow(value)) throw new TypeErrorValue("Illegal invocation");
  };
  const object = (value) => (typeof value === "object" && value !== null) || typeof value === "function";
  // GetHTMLOptions: members are read in IDL dictionary order, then the
  // ShadowRoot sequence is iterated through the value's own iterator method.
  function options(raw, owner) {
    const prefix = `Failed to execute 'getHTML' on '${owner}': `;
    if (raw === undefined || raw === null) return [false, []];
    if (!object(raw)) throw new TypeErrorValue(prefix + "The provided value is not of type 'GetHTMLOptions'.");
    const serializable = !!raw.serializableShadowRoots;
    const list = raw.shadowRoots,
      roots = [];
    if (list === undefined) return [serializable, roots];
    const member = prefix + "Failed to read the 'shadowRoots' property from 'GetHTMLOptions': ";
    const method = object(list) ? list[iteratorKey] : undefined;
    if (typeof method !== "function") throw new TypeErrorValue(member + "The provided value cannot be converted to a sequence.");
    const iterator = apply(method, list, []);
    if (!object(iterator)) throw new TypeErrorValue(member + "The provided value cannot be converted to a sequence.");
    const next = iterator.next;
    for (;;) {
      const step = apply(next, iterator, []);
      if (!object(step)) throw new TypeErrorValue("Iterator result is not an object");
      if (step.done) break;
      const root = step.value;
      if (!isShadow(root)) throw new TypeErrorValue(member + "Failed to convert value to 'ShadowRoot'.");
      roots[roots.length] = root;
    }
    return [serializable, roots];
  }
  const method = (proto, name, length, body) =>
    define(proto, name, { value: make(body, name, length), writable: true, enumerable: true, configurable: true });
  const getter = (name, body, setter) =>
    define(ShadowRoot.prototype, name, {
      get: make(body, "get " + name, 0),
      set: setter && make(setter, "set " + name, 1),
      enumerable: true,
      configurable: true,
    });
  const serialize = (owner, check) =>
    function getHTML(raw) {
      check(this);
      const [serializable, roots] = options(raw, owner);
      return native("getHTML", this, serializable, roots);
    };
  const unsafe = (owner, check) =>
    function setHTMLUnsafe(html, options) {
      check(this);
      if (arguments.length < 1)
        throw new TypeErrorValue(`Failed to execute 'setHTMLUnsafe' on '${owner}': 1 argument required, but only 0 present.`);
      // (TrustedHTML or DOMString), SetHTMLUnsafeOptions and the Trusted Types
      // check (sanitizer.js); the native step receives converted markup.
      const converted = ns.sanitizerUnsafeArguments(this, html, options, owner, "setHTMLUnsafe");
      native("setHTMLUnsafe", this, converted.html, converted.config);
    };
  const element = (value) => apply(elementBrand, value, []);
  method(Element.prototype, "getHTML", 0, serialize("Element", element));
  method(ShadowRoot.prototype, "getHTML", 0, serialize("ShadowRoot", shadow));
  method(Element.prototype, "setHTMLUnsafe", 1, unsafe("Element", element));
  method(ShadowRoot.prototype, "setHTMLUnsafe", 1, unsafe("ShadowRoot", shadow));
  for (const name of ["clonable", "serializable", "delegatesFocus"])
    getter(name, function () {
      return native(name, this);
    });
  getter("activeElement", function () {
    return native("activeElement", this);
  });
  // The document's fullscreen element is the only fullscreen state; retarget it.
  getter(
    "fullscreenElement",
    function () {
      shadow(this);
      const document = apply(ownerDocument, this, []);
      const current = document && ns.fullscreenElementOf ? ns.fullscreenElementOf(document) : null;
      return object(current) ? native("retarget", this, current) : null;
    },
    function () {
      shadow(this);
    },
  );
  // The native pointer lock element retargeted into this shadow tree. This engine has
  // no picture-in-picture entry point, so the document never has such an element.
  getter("pointerLockElement", function () {
    return native("pointerLockElement", this);
  });
  getter("pictureInPictureElement", function () {
    shadow(this);
    return null;
  });
  const sheets = new WeakMapValue();
  getter("styleSheets", function () {
    shadow(this);
    let list = apply(mapGet, sheets, [this]);
    if (list) return list;
    const root = this;
    // Sheets exist only for connected owner nodes; nested roots own theirs.
    list = ns.styleSheetList(() => {
      const nodes = apply(querySelectorAll, root, ["style,link"]),
        result = [];
      for (let index = 0; index < nodes.length; index++) {
        const node = nodes[index];
        const sheet = apply(apply(localName, node, []) === "link" ? linkSheet : styleSheet, node, []);
        if (sheet) result[result.length] = sheet;
      }
      return result;
    });
    apply(mapSet, sheets, [this, list]);
    return list;
  });
  const handlers = new WeakMapValue();
  getter(
    "onslotchange",
    function () {
      shadow(this);
      const entry = apply(mapGet, handlers, [this]);
      return entry ? entry.value : null;
    },
    function (value) {
      shadow(this);
      value = object(value) ? value : null;
      let entry = apply(mapGet, handlers, [this]);
      if (!entry) {
        entry = {
          value: null,
          callback: make(
            function (event) {
              const current = apply(mapGet, handlers, [this]).value;
              if (typeof current === "function" && apply(current, this, [event]) === false)
                apply(preventDefault, event, []);
            },
            "onslotchange",
            1,
          ),
        };
        apply(mapSet, handlers, [this, entry]);
      }
      if (value && !entry.value) apply(addListener, this, ["slotchange", entry.callback]);
      if (!value && entry.value) apply(removeListener, this, ["slotchange", entry.callback]);
      entry.value = value;
    },
  );
})();
