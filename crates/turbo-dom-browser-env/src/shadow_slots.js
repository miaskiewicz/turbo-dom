(function () {
  "use strict";
  const g = globalThis,
    ns = g[Symbol.for("ts")],
    native = g.__shadowSlotNative;
  delete g.__shadowSlotNative;
  const make = ns.makeNativeFunction,
    apply = Reflect.apply,
    define = Object.defineProperty;
  const slotBrand = ns.domAccessor("nodeType", false, "HTML:slot");
  const shadowBrand = ns.domAccessor("__isShadowRoot", false, "Node");
  const elementBrand = ns.domAccessor("nodeType", false, "Element");
  const textBrand = ns.domAccessor("nodeType", false, "Text");
  const getAttribute = Element.prototype.getAttribute,
    setAttribute = Element.prototype.setAttribute;
  const StringValue = String,
    TypeErrorValue = TypeError;
  const string = (value) => {
    if (typeof value === "symbol") throw new TypeErrorValue("Cannot convert a Symbol to a string");
    return StringValue(value);
  };
  define(HTMLSlotElement.prototype, "name", {
    get: make(
      function () {
        apply(slotBrand, this, []);
        return apply(getAttribute, this, ["name"]) || "";
      },
      "get name",
      0,
    ),
    set: make(
      function (value) {
        apply(slotBrand, this, []);
        apply(setAttribute, this, ["name", string(value)]);
      },
      "set name",
      1,
    ),
    enumerable: true,
    configurable: true,
  });
  define(ShadowRoot.prototype, "slotAssignment", {
    get: make(
      function () {
        if (!apply(shadowBrand, this, [])) throw new TypeErrorValue("Illegal invocation");
        return native("mode", this);
      },
      "get slotAssignment",
      0,
    ),
    enumerable: true,
    configurable: true,
  });
  for (const [prototype, brand] of [
    [Element.prototype, elementBrand],
    [Text.prototype, textBrand],
  ])
    define(prototype, "assignedSlot", {
      get: make(
        function () {
          apply(brand, this, []);
          return native("assignedSlot", this);
        },
        "get assignedSlot",
        0,
      ),
      enumerable: true,
      configurable: true,
    });
  for (const name of ["assignedNodes", "assignedElements"])
    define(HTMLSlotElement.prototype, name, {
      value: make(
        function (options) {
          apply(slotBrand, this, []);
          if (options != null && typeof options !== "object" && typeof options !== "function")
            throw new TypeErrorValue("Options must be a dictionary");
          const flatten = options != null && !!options.flatten;
          return native(name, this, flatten);
        },
        name,
        0,
      ),
      writable: true,
      enumerable: true,
      configurable: true,
    });
  define(HTMLSlotElement.prototype, "assign", {
    value: make(
      function () {
        apply(slotBrand, this, []);
        native("assign", this, arguments);
      },
      "assign",
      0,
    ),
    writable: true,
    enumerable: true,
    configurable: true,
  });
})();
