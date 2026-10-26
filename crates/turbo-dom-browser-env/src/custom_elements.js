(function () {
  "use strict";
  const g = globalThis,
    ns = g[Symbol.for("ts")],
    make = ns.makeNativeFunction;
  const nativeConstructor = g.__customElementConstructor,
    children = g.__customElementChildren,
    nativeUpgrade = g.__customElementUpgrade,
    nativePrivate = g.__customElementPrivate,
    nativeReport = g.__customElementReport,
    nativeDefining = g.__customElementDefining,
    nativeReact = g.__customElementReact,
    nativeActive = g.__customElementActive;
  delete g.__customElementConstructor;
  delete g.__customElementChildren;
  delete g.__customElementUpgrade;
  delete g.__customElementPrivate;
  delete g.__customElementReport;
  delete g.__customElementDefining;
  delete g.__customElementReact;
  delete g.__customElementActive;
  const apply = Reflect.apply,
    construct = Reflect.construct,
    ProxyValue = Proxy,
    define = Object.defineProperty;
  const objectCreate = Object.create,
    arrayFrom = Array.from,
    arrayIncludes = Array.prototype.includes,
    ArrayValue = Array,
    WeakRefValue = WeakRef,
    weakDeref = WeakRef.prototype.deref,
    setHas = Set.prototype.has;
  const getPrototype = Object.getPrototypeOf,
    setPrototype = Object.setPrototypeOf,
    keys = Reflect.ownKeys,
    descriptor = Object.getOwnPropertyDescriptor;
  const WeakMapValue = WeakMap,
    MapValue = Map,
    PromiseValue = Promise,
    promiseResolve = Promise.resolve,
    promiseReject = Promise.reject,
    TypeErrorValue = TypeError,
    Exception = DOMException,
    StringValue = String;
  const weakGet = WeakMap.prototype.get,
    weakSet = WeakMap.prototype.set,
    weakHas = WeakMap.prototype.has;
  const mapGet = Map.prototype.get,
    mapSet = Map.prototype.set,
    mapHas = Map.prototype.has,
    mapDelete = Map.prototype.delete;
  const registryStates = {},
    constructors = new WeakMapValue(),
    elementStates = {},
    associations = {};
  const family = (map) =>
    map === registryStates
      ? "registry"
      : map === elementStates
        ? "element"
        : map === associations
          ? "association"
          : null;
  const wget = (map, key) =>
    family(map) ? nativePrivate(key, undefined, family(map)) : apply(weakGet, map, [key]);
  const wset = (map, key, value) =>
    family(map) ? nativePrivate(key, value, family(map)) : apply(weakSet, map, [key, value]);
  const whas = (map, key) =>
    family(map) ? wget(map, key) !== undefined : apply(weakHas, map, [key]);
  const mget = (map, key) => apply(mapGet, map, [key]),
    mset = (map, key, value) => apply(mapSet, map, [key, value]),
    mhas = (map, key) => apply(mapHas, map, [key]),
    mdel = (map, key) => apply(mapDelete, map, [key]);
  const string = (value) => {
    if (typeof value === "symbol") throw new TypeErrorValue("Cannot convert a Symbol to a string");
    return StringValue(value);
  };
  const state = (receiver) => {
    const value = wget(registryStates, receiver);
    if (!value) throw new TypeErrorValue("Illegal invocation");
    return value;
  };
  const constructible = (value) => {
    try {
      construct(
        new ProxyValue(value, {
          construct() {
            return {};
          },
        }),
        [],
      );
      return true;
    } catch {
      return false;
    }
  };
  const forbiddenNames = new Set([
    "annotation-xml",
    "color-profile",
    "font-face",
    "font-face-src",
    "font-face-uri",
    "font-face-format",
    "font-face-name",
    "missing-glyph",
  ]);
  const codeUnit = String.prototype.charCodeAt;
  const valid = (name) => {
    if (!name.length) return false;
    const first = apply(codeUnit, name, [0]);
    if (first < 97 || first > 122 || apply(setHas, forbiddenNames, [name])) return false;
    let dash = false;
    for (let i = 0; i < name.length; i++) {
      const c = apply(codeUnit, name, [i]);
      if (c === 45) dash = true;
      if (
        (c >= 65 && c <= 90) ||
        c === 0 ||
        c === 9 ||
        c === 10 ||
        c === 12 ||
        c === 13 ||
        c === 32 ||
        c === 47 ||
        c === 62
      )
        return false;
    }
    return dash;
  };
  const includes = (list, value) => apply(arrayIncludes, list, [value]);
  const push = (list, value) =>
    define(list, list.length, {
      value,
      writable: true,
      enumerable: true,
      configurable: true,
    });
  const isShadowRoot = ns.domAccessor("__isShadowRoot", false, "Node");
  const parentNode = descriptor(Node.prototype, "parentNode").get,
    childNodes = descriptor(Node.prototype, "childNodes").get,
    shadowHost = descriptor(ShadowRoot.prototype, "host").get;
  const nodeType = descriptor(Node.prototype, "nodeType").get,
    owner = descriptor(Node.prototype, "ownerDocument").get,
    connected = descriptor(Node.prototype, "isConnected").get;
  const local = descriptor(Element.prototype, "localName").get,
    namespace = descriptor(Element.prototype, "namespaceURI").get;
  const attrs = descriptor(Element.prototype, "attributes").get,
    getAttribute = Element.prototype.getAttribute,
    getAttributeNS = Element.prototype.getAttributeNS;
  const create = Document.prototype.createElement,
    createNS = Document.prototype.createElementNS,
    htmlPrototype = HTMLElement.prototype,
    unknownPrototype = HTMLUnknownElement.prototype,
    contextDocument = g.document;
  const htmlNamespace = "http://www.w3.org/1999/xhtml";
  const read = (fn, node) => apply(fn, node, []),
    kind = (node) => read(nodeType, node),
    documentOf = (node) => (kind(node) === 9 ? node : read(owner, node));
  const ErrorEventValue = ErrorEvent;
  const report = (error) => nativeReport(error);
  let updateInternals, initializeInternals;
  const callback = (node, record, name, args) => {
    const definition = record.definition, fn = definition.callbacks[name];
    if (!fn || !nativeActive(definition.registry) || !nativeActive(definition.constructor) || !nativeActive(fn)) return;
    if (nativeReact(node, fn, args, definition.registry, definition.constructor)) return;
    if (fn)
      try {
        apply(fn, node, args);
      } catch (error) {
        report(error);
      }
  };
  function walk(root, visit) {
    const pending = objectCreate(null);
    let count = 1;
    pending[0] = root;
    while (count) {
      const node = pending[--count];
      delete pending[count];
      visit(node);
      const list = children(node);
      for (let index = list.length - 1; index >= 0; index--) pending[count++] = list[index];
    }
  }
  function associate(node, selected, defaultGlobal = false) {
    // Global associations follow the owner document without retaining its old realm.
    wset(
      associations,
      node,
      kind(node) !== 9 && ((selected && state(selected).global) || defaultGlobal)
        ? false
        : selected,
    );
  }
  function associationFor(node) {
    let cursor = node;
    while (cursor) {
      const associated = wget(associations, cursor);
      if (associated !== undefined) {
        return kind(cursor) === 9 && associated && state(associated).global ? false : associated;
      }
      cursor =
        read(parentNode, cursor) ||
        (kind(cursor) === 11 && read(isShadowRoot, cursor) ? read(shadowHost, cursor) : null);
    }
    const associated = wget(associations, documentOf(node));
    return associated && state(associated).global ? false : (associated ?? false);
  }
  function registryFor(node) {
    const associated = associationFor(node);
    return associated === false ? (wget(associations, documentOf(node)) ?? null) : associated;
  }
  function definitionFor(node, selected) {
    if (kind(node) !== 1 || read(namespace, node) !== htmlNamespace || !selected || !nativeActive(selected)) return;
    const record = state(selected),
      tag = read(local, node),
      is = nativePrivate(node) ?? apply(getAttribute, node, ["is"]);
    const definition = mget(record.definitions, valid(tag) ? tag : is);
    return definition && definition.local === tag && nativeActive(definition.constructor) ? definition : undefined;
  }
  function upgraded(node, definition) {
    let record = wget(elementStates, node);
    if (record && record.status !== "undefined") return;
    record = {
      definition,
      status: "constructing",
      owner: documentOf(node),
      connected: read(connected, node),
    };
    wset(elementStates, node, record);
    if (initializeInternals) initializeInternals(node, definition);
    const initial = [],
      attributes = read(attrs, node);
    for (let i = 0; i < attributes.length; i++) {
      const attribute = attributes[i];
      if (includes(definition.observed, attribute.localName))
        push(initial, [attribute.localName, null, attribute.value, attribute.namespaceURI]);
    }
    try {
      nativeUpgrade(definition.constructor, node);
      record.status = "custom";
    } catch (error) {
      record.status = "failed";
      report(error);
      return;
    }
    for (let i = 0; i < initial.length; i++)
      callback(node, record, "attributeChangedCallback", initial[i]);
    if (record.connected) callback(node, record, "connectedCallback", []);
    if (updateInternals) updateInternals();
  }
  function visit(node, selected, connect) {
    if (kind(node) !== 1) return;
    let record = wget(elementStates, node);
    if (!record) {
      const definition = definitionFor(node, selected);
      if (definition) {
        upgraded(node, definition);
        return;
      }
    }
    if (!record || record.status !== "custom") return;
    const next = documentOf(node);
    if (record.owner !== next) {
      const old = record.owner;
      record.owner = next;
      callback(node, record, "adoptedCallback", [old, next]);
    }
    if (connect) {
      const nextConnected = read(connected, node);
      if (nextConnected !== record.connected) {
        record.connected = nextConnected;
        callback(node, record, nextConnected ? "connectedCallback" : "disconnectedCallback", []);
      }
    }
  }
  function mutation(event) {
    if (event.type === "move") {
      walk(event.target, (node) => {
        const record = wget(elementStates, node);
        if (record && record.status === "custom" && record.connected && read(connected, node)) {
          nativeReact(node, undefined, []);
          if (record.definition.callbacks.connectedMoveCallback) {
            record.moving = true;
            callback(node, record, "connectedMoveCallback", []);
          }
        }
      });
      return;
    }
    if (event.type === "adoption") {
      walk(event.target, (node) => visit(node, null, false));
      return;
    }
    if (event.type === "attributes") {
      const node = event.target,
        record = wget(elementStates, node);
      if (
        record &&
        record.status === "custom" &&
        includes(record.definition.observed, event.attributeName)
      )
        callback(node, record, "attributeChangedCallback", [
          event.attributeName,
          event.oldValue,
          apply(getAttributeNS, node, [event.attributeNamespace, event.attributeName]),
          event.attributeNamespace,
        ]);
      return;
    }
    if (event.type !== "childList") return;
    for (let i = 0; i < event.removedNodes.length; i++)
      walk(event.removedNodes[i], (node) => {
        const record = wget(elementStates, node);
        if (record && record.status === "custom" && record.connected && !record.moving) {
          record.connected = false;
          callback(node, record, "disconnectedCallback", []);
        }
      });
    for (let i = 0; i < event.addedNodes.length; i++)
      walk(event.addedNodes[i], (node) => {
        const record = wget(elementStates, node);
        if (record?.moving) {
          record.moving = false;
          return;
        }
        if (kind(node) === 1 && wget(associations, node) === undefined)
          wset(associations, node, associationFor(node));
        visit(node, registryFor(node), true);
      });
  }
  const Registry = make(
    function CustomElementRegistry() {
      if (!new.target) throw new TypeErrorValue("Please use the new operator");
      wset(registryStates, this, {
        global: false,
        definitions: new MapValue(),
        pending: new MapValue(),
        constructors: new WeakMapValue(),
        roots: [],
      });
    },
    "CustomElementRegistry",
    0,
    true,
    true,
  );
  const prototype = Registry.prototype;
  delete prototype.constructor;
  function method(name, length, implementation) {
    define(prototype, name, {
      value: make(implementation, name, length),
      enumerable: true,
      configurable: true,
      writable: true,
    });
  }
  method("define", 2, function (name, constructor, options) {
    const record = state(this);
    if (arguments.length < 2) throw new TypeErrorValue("Two arguments required");
    name = string(name);
    if (!constructible(constructor)) throw new TypeErrorValue("The value is not a constructor");
    let extension;
    if (options != null) {
      if (typeof options !== "object" && typeof options !== "function")
        throw new TypeErrorValue("Invalid options");
      const value = options.extends;
      if (value !== undefined) extension = string(value);
    }
    if (!valid(name)) throw new Exception("Invalid custom element name", "SyntaxError");
    if (
      mhas(record.definitions, name) ||
      whas(record.constructors, constructor) ||
      nativeDefining(this)
    )
      throw new Exception("Duplicate custom element definition", "NotSupportedError");
    const localName = extension === undefined ? name : extension;
    if (extension !== undefined && (valid(extension) || !apply(setHas, knownTags, [extension])))
      throw new Exception("Invalid extension", "NotSupportedError");
    const definition = nativeDefining(this, () => {
      const object = constructor.prototype;
      if (object === null || (typeof object !== "object" && typeof object !== "function"))
        throw new TypeErrorValue("Invalid constructor prototype");
      const callbacks = objectCreate(null);
      const callbackNames = [
        "connectedCallback",
        "disconnectedCallback",
        "connectedMoveCallback",
        "adoptedCallback",
        "attributeChangedCallback",
      ];
      for (let index = 0; index < callbackNames.length; index++) {
        const name = callbackNames[index],
          value = object[name];
        if (value !== undefined) {
          if (typeof value !== "function") throw new TypeErrorValue("Invalid lifecycle callback");
          callbacks[name] = value;
        }
      }
      let observed = [];
      if (callbacks.attributeChangedCallback) {
        const value = constructor.observedAttributes;
        if (value !== undefined) {
          if (value === null || (typeof value !== "object" && typeof value !== "function"))
            throw new TypeErrorValue("Invalid observedAttributes sequence");
          const iterator = value[Symbol.iterator];
          if (typeof iterator !== "function")
            throw new TypeErrorValue("Invalid observedAttributes sequence");
          const iterable = {
            [Symbol.iterator]() {
              return apply(iterator, value, []);
            },
          };
          observed = apply(arrayFrom, ArrayValue, [iterable, string]);
        }
      }
      const disabledValue = constructor.disabledFeatures;
      let disabledFeatures = [];
      if (disabledValue !== undefined)
        disabledFeatures = apply(arrayFrom, ArrayValue, [disabledValue, string]);
      const formAssociated = !!constructor.formAssociated;
      if (formAssociated) {
        for (const name of [
          "formAssociatedCallback",
          "formResetCallback",
          "formDisabledCallback",
          "formStateRestoreCallback",
        ]) {
          const value = object[name];
          if (value !== undefined) {
            if (typeof value !== "function") throw new TypeErrorValue("Invalid lifecycle callback");
            callbacks[name] = value;
          }
        }
      }
      return {
        disabledFeatures,
        formAssociated,
        name,
        local: localName,
        constructor,
        prototype: object,
        callbacks,
        observed,
        registry: this,
      };
    });
    mset(record.definitions, name, definition);
    wset(record.constructors, constructor, definition);
    if (this === registry) wset(constructors, constructor, definition);
    if (this === registry)
      walk(contextDocument, (node) => {
        if (registryFor(node) === registry && definitionFor(node, registry) === definition)
          upgraded(node, definition);
      });
    else
      for (let i = 0; i < record.roots.length; i++) {
        const root = apply(weakDeref, record.roots[i], []);
        if (root)
          walk(root, (node) => {
            if (registryFor(node) === this && definitionFor(node, this) === definition)
              upgraded(node, definition);
          });
      }
    const pending = mget(record.pending, name);
    if (pending) {
      pending.resolve(constructor);
      mdel(record.pending, name);
    }
  });
  method("get", 1, function (name) {
    const record = state(this);
    if (!arguments.length) throw new TypeErrorValue("One argument required");
    return mget(record.definitions, string(name))?.constructor;
  });
  method("getName", 1, function (constructor) {
    const record = state(this);
    if (!arguments.length || typeof constructor !== "function")
      throw new TypeErrorValue("Invalid constructor");
    const definition = wget(record.constructors, constructor);
    return definition ? definition.name : null;
  });
  method("initialize", 1, function (root) {
    const record = state(this);
    if (!arguments.length) throw new TypeErrorValue("One argument required");
    kind(root);
    if (wget(associations, root) === null || wget(associations, root) === undefined) {
      wset(associations, root, this);
      push(record.roots, new WeakRefValue(root));
    }
    walk(root, (node) => visit(node, this, false));
  });
  method("upgrade", 1, function (root) {
    state(this);
    if (!arguments.length) throw new TypeErrorValue("One argument required");
    kind(root);
    walk(root, (node) => visit(node, this, false));
  });
  method("whenDefined", 1, function (name) {
    try {
      const record = state(this);
      if (!arguments.length) throw new TypeErrorValue("One argument required");
      name = string(name);
      if (!valid(name)) throw new Exception("Invalid custom element name", "SyntaxError");
      if (mhas(record.definitions, name))
        return apply(promiseResolve, PromiseValue, [mget(record.definitions, name).constructor]);
      let pending = mget(record.pending, name);
      if (!pending) {
        let resolve;
        const promise = new PromiseValue((done) => (resolve = done));
        pending = { promise, resolve };
        mset(record.pending, name, pending);
      }
      return pending.promise;
    } catch (error) {
      return apply(promiseReject, PromiseValue, [error]);
    }
  });
  define(prototype, "constructor", {
    value: Registry,
    writable: true,
    configurable: true,
  });
  define(prototype, Symbol.toStringTag, {
    value: "CustomElementRegistry",
    configurable: true,
  });
  define(Registry, "prototype", { writable: false });
  define(g, "CustomElementRegistry", {
    value: Registry,
    writable: true,
    configurable: true,
  });
  const registry = new Registry();
  state(registry).global = true;
  wset(associations, contextDocument, registry);
  walk(contextDocument, (node) => {
    if (kind(node) === 1) associate(node, registry);
  });
  define(g, "customElements", {
    get: make(
      function () {
        if (this !== g) throw new TypeErrorValue("Illegal invocation");
        return registry;
      },
      "get customElements",
      0,
    ),
    enumerable: true,
    configurable: true,
  });
  const knownTags = new Set(
    "a abbr address area article aside audio b base bdi bdo blockquote body br button canvas caption cite code col colgroup data datalist dd del details dfn dialog div dl dt em embed fieldset figcaption figure footer form h1 h2 h3 h4 h5 h6 head header hgroup hr html i iframe img input ins kbd label legend li link main map mark menu meta meter nav noscript object ol optgroup option output p picture pre progress q rp rt ruby s samp script search section select slot small source span strong style sub summary sup table tbody td template textarea tfoot th thead time title tr track u ul var video wbr".split(
      " ",
    ),
  );
  const replacements = new MapValue(),
    originals = [];
  for (const name of keys(g)) {
    if (typeof name !== "string" || (name !== "HTMLElement" && !/^HTML.*Element$/.test(name)))
      continue;
    const old = g[name];
    if (typeof old !== "function" || !old.prototype || name === "HTMLUnknownElement") continue;
    originals.push([name, old, getPrototype(old)]);
  }
  for (const [name, old] of originals) {
    const expected = old.prototype;
    const replacement = nativeConstructor((target, prototype, existing) => {
      const definition =
        existing === undefined
          ? wget(constructors, target)
          : wget(elementStates, existing)?.definition;
      if (!definition || definition.constructor !== target)
        throw new TypeErrorValue("Illegal constructor");
      const required =
        definition.local === definition.name
          ? htmlPrototype
          : getPrototype(apply(create, contextDocument, [definition.local]));
      if (required !== expected) throw new TypeErrorValue("Illegal constructor");
      const node =
        existing === undefined ? apply(create, contextDocument, [definition.local]) : existing;
      setPrototype(
        node,
        prototype && (typeof prototype === "object" || typeof prototype === "function")
          ? prototype
          : expected,
      );
      if (existing === undefined) {
        if (definition.local !== definition.name) nativePrivate(node, definition.name);
        wset(elementStates, node, {
          definition,
          status: "custom",
          owner: contextDocument,
          connected: false,
        });
        if (initializeInternals) initializeInternals(node, definition);
      }
      return node;
    }, name);
    define(replacement, "prototype", { value: expected, writable: false });
    define(expected, "constructor", {
      value: replacement,
      writable: true,
      configurable: true,
    });
    mset(replacements, old, replacement);
    define(g, name, { value: replacement, writable: true, configurable: true });
  }
  for (const [name, , parent] of originals)
    setPrototype(g[name], mget(replacements, parent) || parent);
  function options(raw) {
    let selected, is;
    if (raw !== null && (typeof raw === "object" || typeof raw === "function")) {
      selected = raw.customElementRegistry;
      if (selected !== undefined && selected !== null) state(selected);
      const value = raw.is;
      if (value !== undefined) is = string(value);
    } else if (raw !== undefined && raw !== null) string(raw);
    return { selected, is };
  }
  function failedCreation(node, definition, error) {
    const fallback = apply(create, documentOf(node), [definition.local]);
    setPrototype(fallback, unknownPrototype);
    wset(elementStates, fallback, { definition, status: "failed" });
    report(error);
    return fallback;
  }
  function creation(node, options) {
    let selected = options.selected === undefined ? registryFor(node) : options.selected,
      is = options.is;
    if (selected && state(selected).global && selected !== wget(associations, documentOf(node)))
      throw new Exception(
        "A foreign global registry cannot create elements in this document",
        "NotSupportedError",
      );
    if (is !== undefined) nativePrivate(node, is);
    associate(node, selected, options.selected === undefined && associationFor(node) === false);
    const definition = definitionFor(node, selected);
    if (!definition) return node;
    if (definition.name !== definition.local) {
      upgraded(node, definition);
      return node;
    }
    let custom;
    try {
      if (selected === registry) custom = construct(definition.constructor, []);
      else {
        wset(elementStates, node, {
          definition,
          status: "constructing",
          owner: documentOf(node),
          connected: false,
        });
        custom = nativeUpgrade(definition.constructor, node, false);
        wget(elementStates, node).status = "custom";
      }
    } catch (error) {
      return failedCreation(node, definition, error);
    }
    let valid = false;
    try {
      valid =
        kind(custom) === 1 &&
        read(namespace, custom) === htmlNamespace &&
        read(local, custom) === definition.local &&
        documentOf(custom) === documentOf(node) &&
        read(parentNode, custom) === null &&
        read(childNodes, custom).length === 0 &&
        read(attrs, custom).length === 0;
    } catch {}
    if (!valid)
      return failedCreation(
        node,
        definition,
        new Exception(
          "The custom element constructor produced an invalid element",
          "NotSupportedError",
        ),
      );
    associate(custom, selected);
    return custom;
  }
  define(Document.prototype, "createElement", {
    value: make(
      function (name, raw) {
        if (kind(this) !== 9) throw new TypeErrorValue("Illegal invocation");
        if (!arguments.length) throw new TypeErrorValue("One argument required");
        name = string(name);
        const init = options(raw);
        const node = apply(create, this, [name]);
        return creation(node, init);
      },
      "createElement",
      1,
    ),
    enumerable: true,
    configurable: true,
    writable: true,
  });
  define(Document.prototype, "createElementNS", {
    value: make(
      function (namespace, name, raw) {
        if (kind(this) !== 9) throw new TypeErrorValue("Illegal invocation");
        if (arguments.length < 2) throw new TypeErrorValue("Two arguments required");
        namespace = namespace == null ? null : string(namespace);
        name = string(name);
        const init = options(raw);
        const node = apply(createNS, this, [namespace, name]);
        return creation(node, init);
      },
      "createElementNS",
      2,
    ),
    enumerable: true,
    configurable: true,
    writable: true,
  });
  for (const proto of [Element.prototype, Document.prototype, ShadowRoot.prototype]) {
    const element = proto === Element.prototype,
      document = proto === Document.prototype;
    define(proto, "customElementRegistry", {
      get: make(
        function () {
          const type = kind(this);
          if (element ? type !== 1 : document ? type !== 9 : type !== 11 || !read(shadowHost, this))
            throw new TypeErrorValue("Illegal invocation");
          return document ? (wget(associations, this) ?? null) : registryFor(this);
        },
        "get customElementRegistry",
        0,
      ),
      enumerable: true,
      configurable: true,
    });
  }
  const attach = Element.prototype.attachShadow;
  define(Element.prototype, "attachShadow", {
    value: make(
      function (raw) {
        if (kind(this) !== 1) throw new TypeErrorValue("Illegal invocation");
        if (raw === null || (typeof raw !== "object" && typeof raw !== "function"))
          throw new TypeErrorValue("Invalid options");
        const clonable = !!raw.clonable;
        let selected = raw.customElementRegistry;
        if (selected !== undefined && selected !== null) state(selected);
        const delegatesFocus = !!raw.delegatesFocus,
          rawMode = raw.mode;
        const reading = "Failed to execute 'attachShadow' on 'Element': Failed to read the ";
        if (rawMode === undefined)
          throw new TypeErrorValue(reading + "'mode' property from 'ShadowRootInit': Required member is undefined.");
        const mode = string(rawMode);
        // ShadowRootMode is an enum: an invalid mode throws before later members are read.
        if (mode !== "open" && mode !== "closed")
          throw new TypeErrorValue(
            reading + `'mode' property from 'ShadowRootInit': The provided value '${mode}' is not a valid enum value of type ShadowRootMode.`,
          );
        const rawTarget = raw.referenceTarget;
        if (typeof rawTarget === "symbol")
          throw new TypeErrorValue(
            reading + "'referenceTarget' property from 'ShadowRootInit': Cannot convert a Symbol value to a string",
          );
        const referenceTarget =
            rawTarget === undefined || rawTarget === null ? null : StringValue(rawTarget),
          serializable = !!raw.serializable,
          rawAssignment = raw.slotAssignment,
          slotAssignment = rawAssignment === undefined ? "named" : string(rawAssignment);
        const shadow = apply(attach, this, [
          { clonable, delegatesFocus, mode, referenceTarget, serializable, slotAssignment },
        ]);
        if (selected === undefined) selected = registryFor(this);
        associate(shadow, selected);
        if (selected) push(state(selected).roots, new WeakRefValue(shadow));
        return shadow;
      },
      "attachShadow",
      1,
    ),
    enumerable: true,
    configurable: true,
    writable: true,
  });
  g.__captureCustomElementInternals = {
    record: (node) => wget(elementStates, node),
    updates: (callback) => {
      updateInternals = callback;
    },
    created: (callback) => {
      initializeInternals = callback;
    },
    notify: (event) => {
      if (updateInternals) updateInternals(event);
    },
    callback: (node, name, args) => {
      const record = wget(elementStates, node);
      if (record) callback(node, record, name, args);
    },
  };
  g.__captureCustomElementError = (error, message, filename, lineno, colno) =>
    new ErrorEventValue("error", {
      error,
      message,
      filename: typeof filename === "string" ? filename : "",
      lineno,
      colno,
    });
  g.__captureCustomElementMutation = (event) => {
    mutation(event);
    if (updateInternals) updateInternals(event);
  };
  g.__captureCustomElementParsed = (node, adopting, context) => {
    if (context) {
      // Detached parser results inherit the context's scoped registry. Template
      // fragments use their associated inert document, whose registry is null.
      const selected = documentOf(node) === documentOf(context) ? registryFor(context) : null;
      walk(node, (element) => {
        if (element === node || kind(element) === 1) associate(element, selected);
      });
      return;
    }
    if (adopting) {
      // Adoption rebinds the old document's registry, including an inert null.
      walk(node, (element) => {
        if (
          (kind(element) === 1 || (kind(element) === 11 && read(isShadowRoot, element))) &&
          registryFor(element) === null &&
          (wget(associations, documentOf(element)) ?? null) === null
        )
          wset(associations, element, false);
      });
    } else if (kind(node) === 1 && wget(associations, node) === undefined)
      wset(associations, node, associationFor(node));
  };
  g.__captureCustomElementClone = (source, clone, deep) => {
    const a = objectCreate(null),
      b = objectCreate(null);
    let count = 1;
    a[0] = source;
    b[0] = clone;
    while (count) {
      const first = a[--count],
        second = b[count];
      delete a[count];
      delete b[count];
      const is = nativePrivate(first);
      if (is !== undefined) nativePrivate(second, is);
      const selected = registryFor(second);
      if (kind(second) === 1) {
        const definition = definitionFor(second, selected);
        if (definition) upgraded(second, definition);
      }
      if (deep) {
        const from = children(first),
          to = children(second);
        for (let i = 0; i < from.length && i < to.length; i++) {
          a[count] = from[i];
          b[count++] = to[i];
        }
      }
    }
  };
})();
