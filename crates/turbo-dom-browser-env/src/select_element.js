(function () {
  "use strict";
  // HTMLSelectElement's options-collection members (length/item/namedItem/add/remove, indexed
  // getter/setter), size/type, and Chrome's selectedness bookkeeping around those mutations.
  // Measured against Chrome 156 in select_element_chrome156.json.
  const g = globalThis,
    ns = g[Symbol.for("ts")],
    make = ns.makeNativeFunction,
    accessor = ns.domAccessor;
  const selection = ns.selectSelection;
  delete ns.selectSelection;
  const liveCollection = ns.internalsLiveCollection;
  const selectionOf = selection.selectionOf,
    optionStates = selection.optionStates,
    emptySelects = selection.emptySelects,
    isSelected = selection.isSelected,
    optionsOf = selection.optionsOf,
    ownerSelect = selection.ownerSelect;
  const P = g.HTMLSelectElement.prototype;
  const apply = Reflect.apply,
    define = Object.defineProperty,
    setPrototypeOf = Object.setPrototypeOf,
    ErrorType = TypeError,
    StringValue = String,
    exec = RegExp.prototype.exec,
    html = "http://www.w3.org/1999/xhtml";
  const selectBrand = accessor("nodeType", false, "HTML:select"),
    insertableBrand = accessor("nodeType", false, "HTML:option|optgroup"),
    optionBrand = accessor("nodeType", false, "HTML:option"),
    htmlBrand = accessor("nodeType", false, "HTMLElement"),
    parentOf = accessor("parentNode", false, "Node"),
    documentOf = accessor("ownerDocument", false, "Node");
  const insertBefore = g.Node.prototype.insertBefore,
    removeChild = g.Node.prototype.removeChild,
    removeSelf = g.Element.prototype.remove,
    getAttribute = g.Element.prototype.getAttribute,
    setAttribute = g.Element.prototype.setAttribute,
    hasAttribute = g.Element.prototype.hasAttribute,
    createElement = g.Document.prototype.createElement;
  const mapSet = WeakMap.prototype.set,
    setDelete = WeakSet.prototype.delete,
    indexOf = Array.prototype.indexOf;
  // Chrome refuses to grow the option list past this many entries (it only logs a console error).
  const maxListItems = 100000;
  const branded = (brand, value) => {
    if (value === null || (typeof value !== "object" && typeof value !== "function")) return false;
    try {
      apply(brand, value, []);
      return true;
    } catch {
      return false;
    }
  };
  const check = (receiver) => apply(selectBrand, receiver, []);
  // Web IDL numeric conversions: unary plus rejects Symbol and BigInt like ToNumber.
  const unsigned = (value) => +value >>> 0;
  const long = (value) => +value | 0;
  const string = (value) => {
    if (typeof value === "symbol") throw new ErrorType("Cannot convert a Symbol to a string");
    return StringValue(value);
  };
  const required = (count, name) => {
    if (count < 1) throw new ErrorType(name + " requires 1 argument");
  };
  const single = (select) => !apply(hasAttribute, select, ["multiple"]);
  const state = (option, value) => apply(mapSet, optionStates, [option, value]);
  const deselectOthers = (select, keep) => {
    const options = optionsOf(select);
    for (let index = 0; index < options.length; index++)
      if (options[index] !== keep && isSelected(options[index])) state(options[index], false);
  };
  // An option's current selectedness, which Chrome keeps on the option when it moves.
  const selectedness = (option) => {
    const owner = ownerSelect(option);
    return owner ? apply(indexOf, selectionOf(owner), [option]) >= 0 : isSelected(option);
  };
  // Option elements an inserted node brings along: the node itself or its option descendants.
  function descendantOptions(node) {
    const result = [];
    (function walk(element) {
      if (element.namespaceURI === html && element.localName === "option") result[result.length] = element;
      for (let child = element.firstChild; child; child = child.nextSibling) if (child.nodeType === 1) walk(child);
    })(node);
    return result;
  }
  // Chrome stores selectedness eagerly: inserting options keeps the current selection unless an
  // inserted option is itself selected, which then takes over a single select.
  function insert(select, container, node, reference) {
    const before = selectionOf(select),
      moved = descendantOptions(node),
      bits = [];
    for (let index = 0; index < moved.length; index++) bits[index] = selectedness(moved[index]);
    apply(insertBefore, container, [node, reference]);
    apply(setDelete, emptySelects, [select]);
    let claimed = null;
    for (let index = 0; index < moved.length; index++)
      if (bits[index] && ownerSelect(moved[index]) === select) claimed = moved[index];
    if (claimed && single(select)) deselectOthers(select, claimed);
    if (claimed) state(claimed, true);
    else if (before.length && single(select) && ownerSelect(before[0]) === select) state(before[0], true);
  }
  // Removing the selected option resets a single select to its first enabled option; the removed
  // option keeps its selectedness, as Chrome's per-option bit does.
  function remove(select, option) {
    if (selectionOf(select)[0] === option && single(select)) {
      state(option, true);
      deselectOthers(select, option);
    }
    apply(removeChild, apply(parentOf, option, []), [option]);
    apply(setDelete, emptySelects, [select]);
  }
  // Batched form of insert/remove above: the list can grow to 100000 options, so the selection is
  // settled once instead of per option. Appended blanks are never selected themselves.
  function setLength(select, length) {
    const options = optionsOf(select);
    if (length > options.length && length > maxListItems) return;
    const chosen = selectionOf(select)[0],
      keep = single(select) && chosen;
    if (keep && apply(indexOf, options, [chosen]) >= length) deselectOthers(select, chosen);
    if (keep) state(chosen, true);
    const owner = apply(documentOf, select, []);
    for (let count = options.length; count < length; count++)
      apply(insertBefore, select, [apply(createElement, owner, ["option"]), null]);
    for (let index = length; index < options.length; index++) {
      const parent = apply(parentOf, options[index], []);
      if (parent) apply(removeChild, parent, [options[index]]);
    }
    apply(setDelete, emptySelects, [select]);
  }
  // `before` is null, an HTMLElement (which must be a child of the select) or a converted long,
  // which inserts into the indexed option's own parent.
  function add(select, element, before) {
    let container = select,
      reference = before;
    if (typeof before === "number") {
      const option = before >= 0 ? optionsOf(select)[before] : undefined;
      reference = option || null;
      if (option) container = apply(parentOf, option, []);
    }
    insert(select, container, element, reference);
  }
  // Chrome's SetOption: pad with blank options, or remove the replaced option and insert before
  // its successor (which throws NotFoundError when that successor sits inside an optgroup).
  function setOption(select, index, option) {
    const options = optionsOf(select);
    if (option === null) {
      if (index < options.length) remove(select, options[index]);
      return;
    }
    if (index > maxListItems) return;
    let before = null;
    if (index > options.length) setLength(select, index);
    else if (index < options.length) {
      before = options[index + 1] || null;
      remove(select, options[index]);
    }
    add(select, option, before);
  }
  const method = (name, length, implementation) =>
    define(P, name, {
      value: make(implementation, name, length),
      writable: true,
      enumerable: true,
      configurable: true,
    });
  const property = (name, read, write) => {
    const descriptor = { get: make(read, "get " + name, 0), enumerable: true, configurable: true };
    if (write) descriptor.set = make(write, "set " + name, 1);
    define(P, name, descriptor);
  };
  property(
    "length",
    function () {
      check(this);
      return optionsOf(this).length;
    },
    function (value) {
      check(this);
      setLength(this, unsigned(value));
    },
  );
  property(
    "size",
    function () {
      check(this);
      // Reflected unsigned long: non-negative integer parsing, default 0, capped at 2^31 - 1.
      const match = apply(exec, /^[\t\n\f\r ]*([-+]?)([0-9]+)/, [apply(getAttribute, this, ["size"]) ?? ""]);
      const value = match && match[1] !== "-" ? +match[2] : 0;
      return value <= 2147483647 ? value : 0;
    },
    function (value) {
      check(this);
      value = unsigned(value);
      apply(setAttribute, this, ["size", StringValue(value <= 2147483647 ? value : 0)]);
    },
  );
  property("type", function () {
    check(this);
    return single(this) ? "select-one" : "select-multiple";
  });
  method("item", 1, function (index) {
    check(this);
    required(arguments.length, "item");
    return optionsOf(this)[unsigned(index)] || null;
  });
  method("namedItem", 1, function (name) {
    check(this);
    required(arguments.length, "namedItem");
    name = string(name);
    if (name === "") return null;
    const options = optionsOf(this);
    // Chrome resolves ids before names, so a later id match beats an earlier name match.
    for (let pass = 0; pass < 2; pass++)
      for (let index = 0; index < options.length; index++)
        if (apply(getAttribute, options[index], [pass ? "name" : "id"]) === name) return options[index];
    return null;
  });
  method("add", 1, function (element, before) {
    check(this);
    required(arguments.length, "add");
    if (!branded(insertableBrand, element))
      throw new ErrorType("The provided value is not of type '(HTMLOptGroupElement or HTMLOptionElement)'.");
    if (before === undefined || before === null) before = null;
    else if (!branded(htmlBrand, before)) before = long(before);
    add(this, element, before);
  });
  // remove() is ChildNode.remove; any argument selects the remove(long index) overload.
  method("remove", 0, function (index) {
    check(this);
    if (!arguments.length) return apply(removeSelf, this, []);
    index = long(index);
    const options = optionsOf(this);
    if (index >= 0 && index < options.length) remove(this, options[index]);
  });
  define(P, Symbol.iterator, {
    value: Array.prototype.values,
    writable: true,
    enumerable: false,
    configurable: true,
  });
  // <option> text/label/value/index. text strips and collapses ASCII whitespace over the option's
  // Text descendants, skipping script subtrees (HTML and SVG).
  const OP = g.HTMLOptionElement.prototype,
    textContent = Object.getOwnPropertyDescriptor(g.Node.prototype, "textContent"),
    nodeType = accessor("nodeType", false, "Node"),
    nodeData = accessor("nodeValue", false, "Node"),
    localNameOf = accessor("localName", false, "Element"),
    namespaceOf = accessor("namespaceURI", false, "Element"),
    firstChildOf = accessor("firstChild", false, "Node"),
    nextSiblingOf = accessor("nextSibling", false, "Node"),
    replace = String.prototype.replace;
  const checkOption = (receiver) => apply(optionBrand, receiver, []);
  function optionText(option) {
    let text = "";
    (function walk(node) {
      for (let child = apply(firstChildOf, node, []); child; child = apply(nextSiblingOf, child, [])) {
        const kind = apply(nodeType, child, []);
        if (kind === 3 || kind === 4) text += apply(nodeData, child, []);
        else if (kind === 1) {
          const namespace = apply(namespaceOf, child, []);
          if (apply(localNameOf, child, []) === "script" && (namespace === html || namespace === "http://www.w3.org/2000/svg")) continue;
          walk(child);
        }
      }
    })(option);
    return apply(replace, apply(replace, text, [/[\t\n\f\r ]+/g, " "]), [/^ | $/g, ""]);
  }
  const optionAccessor = (name, read, write) => {
    const descriptor = { get: make(function () { checkOption(this); return read(this); }, "get " + name, 0), enumerable: true, configurable: true };
    if (write) descriptor.set = make(function (value) { checkOption(this); write(this, value); }, "set " + name, 1);
    define(OP, name, descriptor);
  };
  optionAccessor("label", (option) => apply(getAttribute, option, ["label"]) ?? optionText(option), (option, value) => apply(setAttribute, option, ["label", string(value)]));
  optionAccessor("value", (option) => apply(getAttribute, option, ["value"]) ?? optionText(option), (option, value) => apply(setAttribute, option, ["value", string(value)]));
  optionAccessor("text", optionText, (option, value) => apply(textContent.set, option, [string(value)]));
  optionAccessor("index", (option) => {
    const select = ownerSelect(option);
    const index = select ? apply(indexOf, optionsOf(select), [option]) : -1;
    return index < 0 ? 0 : index;
  });
  // HTMLOptionsCollection: select.options, a live HTMLCollection over the select's list of
  // options whose mutators are the select's own algorithms above ([SameObject] per select).
  const collectionCtor = make(
    function () {
      throw new ErrorType("Illegal constructor");
    },
    "HTMLOptionsCollection",
    0,
    true,
  );
  const CP = collectionCtor.prototype;
  setPrototypeOf(CP, g.HTMLCollection.prototype);
  setPrototypeOf(collectionCtor, g.HTMLCollection);
  define(collectionCtor, "prototype", { writable: false });
  define(g, "HTMLOptionsCollection", { value: collectionCtor, writable: true, configurable: true, enumerable: false });
  const collectionSelects = new WeakMap(),
    selectCollections = new WeakMap(),
    weakGet = WeakMap.prototype.get,
    weakSet = WeakMap.prototype.set;
  const owning = (receiver) => {
    const select = receiver !== null && typeof receiver === "object" ? apply(weakGet, collectionSelects, [receiver]) : undefined;
    if (!select) throw new ErrorType("Illegal invocation");
    return select;
  };
  const selectedIndex = Object.getOwnPropertyDescriptor(P, "selectedIndex");
  const collectionProperty = (name, read, write) => {
    const descriptor = { get: make(read, "get " + name, 0), enumerable: true, configurable: true };
    if (write) descriptor.set = make(write, "set " + name, 1);
    define(CP, name, descriptor);
  };
  collectionProperty(
    "length",
    function () {
      return optionsOf(owning(this)).length;
    },
    function (value) {
      setLength(owning(this), unsigned(value));
    },
  );
  collectionProperty(
    "selectedIndex",
    function () {
      return apply(selectedIndex.get, owning(this), []);
    },
    function (value) {
      apply(selectedIndex.set, owning(this), [long(value)]);
    },
  );
  define(CP, "add", {
    value: make(
      function (element, before) {
        const select = owning(this);
        required(arguments.length, "add");
        if (!branded(insertableBrand, element))
          throw new ErrorType("The provided value is not of type '(HTMLOptGroupElement or HTMLOptionElement)'.");
        if (before === undefined || before === null) before = null;
        else if (!branded(htmlBrand, before)) before = long(before);
        add(select, element, before);
      },
      "add",
      1,
    ),
    writable: true,
    enumerable: true,
    configurable: true,
  });
  define(CP, "remove", {
    value: make(
      function (index) {
        const select = owning(this);
        required(arguments.length, "remove");
        index = long(index);
        const options = optionsOf(select);
        if (index >= 0 && index < options.length) remove(select, options[index]);
      },
      "remove",
      1,
    ),
    writable: true,
    enumerable: true,
    configurable: true,
  });
  define(CP, "constructor", { value: collectionCtor, writable: true, enumerable: false, configurable: true });
  define(CP, Symbol.toStringTag, { value: "HTMLOptionsCollection", configurable: true });
  define(CP, Symbol.iterator, { value: Array.prototype.values, writable: true, enumerable: false, configurable: true });
  const indexedSet = (select, index, value) => {
    if (value !== null && value !== undefined && !branded(optionBrand, value))
      throw new ErrorType(
        "Failed to set an indexed property [" + index + "] on 'HTMLOptionsCollection': parameter 2 is not of type 'HTMLOptionElement'.",
      );
    setOption(select, index, value ?? null);
    return true;
  };
  function optionsCollection(select) {
    let collection = apply(weakGet, selectCollections, [select]);
    if (collection) return collection;
    collection = liveCollection(() => optionsOf(select), true, {
      prototype: CP,
      kind: "HTMLOptionsCollection",
      // Unlike HTMLCollection, the options collection's named properties are enumerable.
      namedEnumerable: true,
      setIndex(index, value, defining) {
        if (defining) {
          if ("get" in value || "set" in value) throw new ErrorType("Accessor properties are not allowed.");
          value = value.value;
        }
        return indexedSet(select, index, value);
      },
    });
    apply(weakSet, selectCollections, [select, collection]);
    apply(weakSet, collectionSelects, [collection, select]);
    return collection;
  }
  property("options", function () {
    check(this);
    return optionsCollection(this);
  });
  const selectedCollections = new WeakMap();
  property("selectedOptions", function () {
    check(this);
    let collection = apply(weakGet, selectedCollections, [this]);
    if (!collection) {
      const select = this;
      collection = liveCollection(() => selectionOf(select), true);
      apply(weakSet, selectedCollections, [select, collection]);
    }
    return collection;
  });
  // Indexed exotic behaviour on the select itself, dispatched by the native element interceptor
  // (select_element.rs). undefined means "not intercepted", so ordinary lookup continues.
  g.__captureSelectIndexed = function (operation, element, index, value) {
    if (!branded(selectBrand, element)) return undefined;
    const options = optionsOf(element);
    switch (operation) {
      case "get":
        return index < options.length ? options[index] : undefined;
      case "descriptor":
        return index < options.length
          ? { value: options[index], writable: true, enumerable: true, configurable: true }
          : undefined;
      case "indices": {
        const indices = [];
        for (let at = 0; at < options.length; at++) indices[at] = at;
        return indices;
      }
      case "delete":
        return index < options.length ? false : undefined;
      case "set":
        if (value !== null && value !== undefined && !branded(optionBrand, value))
          throw new ErrorType(
            "Failed to set an indexed property [" +
              index +
              "] on 'HTMLSelectElement': parameter 2 is not of type 'HTMLOptionElement'.",
          );
        setOption(element, index, value ?? null);
        return true;
    }
    return undefined;
  };
})();
