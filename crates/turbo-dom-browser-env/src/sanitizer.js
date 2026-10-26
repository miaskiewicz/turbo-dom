(function () {
  "use strict";
  // HTML Sanitizer API as Chrome 156 ships it: the Sanitizer interface, the
  // safe Element/ShadowRoot.setHTML, Document.parseHTML and the sanitizer
  // option of setHTMLUnsafe/parseHTMLUnsafe. Configuration records are
  // null-prototype data kept in a V8 private (valid in every realm).
  const g = globalThis,
    ns = g[Symbol.for("ts")],
    native = g.__sanitizerNative,
    defaultData = g.__sanitizerDefault,
    baselineData = g.__sanitizerBaseline;
  delete g.__sanitizerNative;
  delete g.__sanitizerDefault;
  delete g.__sanitizerBaseline;
  const make = ns.makeNativeFunction,
    apply = Reflect.apply,
    define = Object.defineProperty,
    create = Object.create,
    parse = JSON.parse,
    TypeErrorValue = TypeError,
    StringValue = String,
    iteratorKey = Symbol.iterator,
    startsWith = String.prototype.startsWith;
  const HTML = "http://www.w3.org/1999/xhtml",
    SVG = "http://www.w3.org/2000/svg",
    MATHML = "http://www.w3.org/1998/Math/MathML";
  const object = (value) => (typeof value === "object" && value !== null) || typeof value === "function";
  const nameRecord = (name, namespace) => {
    const result = create(null);
    result.name = name;
    result.namespace = namespace;
    return result;
  };
  const targetRecord = (target) => {
    const result = create(null);
    result.target = target;
    return result;
  };
  const push = (list, item) => {
    list[list.length] = item;
  };

  // ---------------------------------------------------------------- Web IDL
  const fail = (prefix, detail) => {
    throw new TypeErrorValue(prefix + detail);
  };
  const domString = (value, prefix) => {
    if (typeof value === "symbol") fail(prefix, "Cannot convert a Symbol value to a string");
    return StringValue(value);
  };
  // sequence<T>: the value's own iterator is driven to completion, converting
  // each item before the next step.
  function sequence(value, prefix, convert) {
    if (!object(value)) fail(prefix, "The provided value cannot be converted to a sequence.");
    const method = value[iteratorKey];
    if (method === undefined || method === null) fail(prefix, "The object must have a callable @@iterator property.");
    if (typeof method !== "function") fail(prefix, "@@iterator must be a callable.");
    const iterator = apply(method, value, []);
    if (!object(iterator)) fail(prefix, "Iterator object must be an object.");
    const next = iterator.next,
      result = [];
    for (;;) {
      const step = apply(next, iterator, []);
      if (!object(step)) fail(prefix, "Expected iterator.next() to return an Object.");
      if (step.done) break;
      push(result, convert(step.value, prefix));
    }
    return result;
  }
  const member = (prefix, name, dictionary) => prefix + `Failed to read the '${name}' property from '${dictionary}': `;
  // (DOMString or SanitizerElementNamespace[WithAttributes]) and
  // (DOMString or SanitizerAttributeNamespace), canonicalized on conversion.
  function canonicalName(value, prefix, dictionary, defaultNamespace, withAttributes) {
    if (value === undefined || value === null) fail(prefix, `The provided value is not of type '${dictionary}'.`);
    if (!object(value)) {
      const result = nameRecord(domString(value, prefix), defaultNamespace);
      if (withAttributes) result.removeAttributes = [];
      return result;
    }
    const base = defaultNamespace === null ? "SanitizerAttributeNamespace" : "SanitizerElementNamespace";
    const rawName = value.name;
    if (rawName === undefined) fail(member(prefix, "name", base), "Required member is undefined.");
    const name = domString(rawName, member(prefix, "name", base));
    const rawNamespace = value.namespace;
    let namespace = defaultNamespace;
    if (rawNamespace === null) namespace = null;
    else if (rawNamespace !== undefined) namespace = domString(rawNamespace, member(prefix, "namespace", base));
    if (namespace === "") namespace = null;
    const result = nameRecord(name, namespace);
    if (!withAttributes) return result;
    const allowed = value.attributes;
    if (allowed !== undefined) result.attributes = sequence(allowed, member(prefix, "attributes", dictionary), attribute);
    const removed = value.removeAttributes;
    if (removed !== undefined)
      result.removeAttributes = sequence(removed, member(prefix, "removeAttributes", dictionary), attribute);
    if (result.attributes === undefined && result.removeAttributes === undefined) result.removeAttributes = [];
    return result;
  }
  const element = (value, prefix) => canonicalName(value, prefix, "SanitizerElementNamespaceWithAttributes", HTML, true);
  const plainElement = (value, prefix) => canonicalName(value, prefix, "SanitizerElementNamespace", HTML, false);
  const attribute = (value, prefix) => canonicalName(value, prefix, "SanitizerAttributeNamespace", null, false);
  function instruction(value, prefix) {
    if (value === undefined || value === null) fail(prefix, "The provided value is not of type 'SanitizerProcessingInstruction'.");
    if (!object(value)) return targetRecord(domString(value, prefix));
    const target = value.target;
    const targetPrefix = member(prefix, "target", "SanitizerProcessingInstruction");
    if (target === undefined) fail(targetPrefix, "Required member is undefined.");
    return targetRecord(domString(target, targetPrefix));
  }
  // SanitizerConfig, members read in IDL (lexicographic) order.
  const configMembers = [
    ["attributes", attribute],
    ["comments", null],
    ["dataAttributes", null],
    ["elements", element],
    ["processingInstructions", instruction],
    ["removeAttributes", attribute],
    ["removeElements", plainElement],
    ["removeProcessingInstructions", instruction],
    ["replaceWithChildrenElements", plainElement],
  ];
  function configDictionary(value, prefix) {
    const result = create(null);
    if (value === undefined || value === null) return result;
    for (let index = 0; index < configMembers.length; index++) {
      const key = configMembers[index][0],
        convert = configMembers[index][1];
      const raw = value[key];
      if (raw === undefined) continue;
      result[key] = convert ? sequence(raw, member(prefix, key, "SanitizerConfig"), convert) : !!raw;
    }
    return result;
  }
  const preset = (value, prefix) => {
    const text = domString(value, prefix);
    if (text !== "default") fail(prefix, `The provided value '${text}' is not a valid enum value of type SanitizerPresets.`);
    return "default";
  };

  // ------------------------------------------------------- configuration lists
  const same = (a, b) => (a.target !== undefined ? a.target === b.target : a.name === b.name && a.namespace === b.namespace);
  const indexOf = (list, item) => {
    for (let index = 0; index < list.length; index++) if (same(list[index], item)) return index;
    return -1;
  };
  const contains = (list, item) => list !== undefined && indexOf(list, item) >= 0;
  // Remove every matching item; report whether any was removed.
  const remove = (list, item) => {
    if (list === undefined) return false;
    let removed = false;
    for (let index = list.length - 1; index >= 0; index--)
      if (same(list[index], item)) {
        for (let cursor = index; cursor < list.length - 1; cursor++) list[cursor] = list[cursor + 1];
        list.length--;
        removed = true;
      }
    return removed;
  };
  const filter = (list, keep) => {
    const result = [];
    for (let index = 0; index < list.length; index++) if (keep(list[index])) push(result, list[index]);
    return result;
  };
  function unique(list) {
    const result = [];
    for (let index = 0; index < list.length; index++) if (!contains(result, list[index])) push(result, list[index]);
    return result;
  }
  const duplicates = (list) => list !== undefined && unique(list).length !== list.length;
  const disjoint = (a, b) => a === undefined || b === undefined || filter(a, (item) => contains(b, item)).length === 0;
  const isData = (attr) => apply(startsWith, attr.name, ["data-"]);
  const nonReplaceable = [nameRecord("html", HTML), nameRecord("svg", SVG), nameRecord("math", MATHML)];
  const copyName = (item) => (item.target !== undefined ? targetRecord(item.target) : nameRecord(item.name, item.namespace));
  const copyList = (list) => {
    const result = [];
    for (let index = 0; index < list.length; index++) push(result, copyName(list[index]));
    return result;
  };
  function copyElement(item) {
    const result = nameRecord(item.name, item.namespace);
    if (item.attributes !== undefined) result.attributes = copyList(item.attributes);
    if (item.removeAttributes !== undefined) result.removeAttributes = copyList(item.removeAttributes);
    return result;
  }
  function copyConfig(config) {
    const result = create(null);
    for (let index = 0; index < configMembers.length; index++) {
      const key = configMembers[index][0],
        value = config[key];
      if (value === undefined) continue;
      if (typeof value === "boolean") result[key] = value;
      else if (key === "elements") {
        const elements = [];
        for (let index = 0; index < value.length; index++) push(elements, copyElement(value[index]));
        result[key] = elements;
      } else result[key] = copyList(value);
    }
    return result;
  }
  // The built-in default configuration and the remove-unsafe baseline are a
  // constant 121-element / 139-attribute graph. Building them eagerly in every
  // realm (each iframe included) is wasteful when a frame never touches the API,
  // so parse the embedded JSON and build the graph only on first use. The
  // observable surface is unchanged — only a sanitizer that needs the default
  // (`new Sanitizer()`, a preset, or a safe default) pays for it, once per realm.
  let defaultConfigValue;
  const defaultConfig = () => {
    if (defaultConfigValue === undefined) {
      const data = parse(defaultData).configuration,
        config = create(null);
      config.attributes = copyList(data.attributes);
      config.comments = data.comments;
      config.dataAttributes = data.dataAttributes;
      config.elements = [];
      for (let index = 0; index < data.elements.length; index++) push(config.elements, copyElement(data.elements[index]));
      config.processingInstructions = copyList(data.processingInstructions);
      defaultConfigValue = config;
    }
    return defaultConfigValue;
  };
  let baselineValue;
  const baseline = () => {
    if (baselineValue === undefined) {
      const data = parse(baselineData);
      baselineValue = { elements: copyList(data.removeElements), attributes: copyList(data.removeAttributes) };
    }
    return baselineValue;
  };

  // "Canonicalize the configuration" (members were canonicalized on conversion).
  function canonicalize(config, allowCommentsAndDataAttributes) {
    if (config.elements === undefined && config.removeElements === undefined) config.removeElements = [];
    if (config.attributes === undefined && config.removeAttributes === undefined) config.removeAttributes = [];
    if (config.processingInstructions === undefined && config.removeProcessingInstructions === undefined)
      config.removeProcessingInstructions = [];
    if (config.comments === undefined) config.comments = allowCommentsAndDataAttributes;
    if (config.attributes !== undefined && config.dataAttributes === undefined)
      config.dataAttributes = allowCommentsAndDataAttributes;
    return config;
  }
  const listMembers = [
    "elements",
    "removeElements",
    "replaceWithChildrenElements",
    "attributes",
    "removeAttributes",
    "processingInstructions",
    "removeProcessingInstructions",
  ];
  // "Configuration is valid".
  function valid(config) {
    if (config.elements !== undefined && config.removeElements !== undefined) return false;
    if (config.attributes !== undefined && config.removeAttributes !== undefined) return false;
    if (config.processingInstructions !== undefined && config.removeProcessingInstructions !== undefined) return false;
    for (let index = 0; index < listMembers.length; index++) if (duplicates(config[listMembers[index]])) return false;
    const replace = config.replaceWithChildrenElements;
    if (replace !== undefined && !disjoint(replace, nonReplaceable)) return false;
    if (!disjoint(config.elements, replace) || !disjoint(config.removeElements, replace)) return false;
    const elements = config.elements || [];
    if (config.attributes !== undefined) {
      for (let index = 0; index < elements.length; index++) {
        const item = elements[index];
        if (duplicates(item.attributes) || duplicates(item.removeAttributes)) return false;
        if (!disjoint(config.attributes, item.attributes)) return false;
        const removed = item.removeAttributes || [];
        if (filter(removed, (attr) => !contains(config.attributes, attr)).length) return false;
        if (config.dataAttributes === true && item.attributes !== undefined && filter(item.attributes, isData).length)
          return false;
      }
      if (config.dataAttributes === true && filter(config.attributes, isData).length) return false;
    } else {
      for (let index = 0; index < elements.length; index++) {
        const item = elements[index];
        if (item.attributes !== undefined && item.removeAttributes !== undefined) return false;
        if (duplicates(item.attributes) || duplicates(item.removeAttributes)) return false;
        if (!disjoint(config.removeAttributes, item.attributes)) return false;
        if (!disjoint(config.removeAttributes, item.removeAttributes)) return false;
      }
      if (config.dataAttributes !== undefined) return false;
    }
    return true;
  }

  // ------------------------------------------------------- Sanitizer methods
  function allowElement(config, item) {
    if (config.elements !== undefined) {
      const modified = remove(config.replaceWithChildrenElements, item);
      if (config.attributes !== undefined) {
        if (item.attributes !== undefined) {
          item.attributes = filter(unique(item.attributes), (attr) => !contains(config.attributes, attr));
          if (config.dataAttributes === true) item.attributes = filter(item.attributes, (attr) => !isData(attr));
        }
        if (item.removeAttributes !== undefined)
          item.removeAttributes = filter(unique(item.removeAttributes), (attr) => contains(config.attributes, attr));
      } else {
        if (item.attributes !== undefined)
          item.attributes = filter(unique(item.attributes), (attr) => !contains(config.removeAttributes, attr));
        if (item.removeAttributes !== undefined)
          item.removeAttributes = filter(unique(item.removeAttributes), (attr) => !contains(config.removeAttributes, attr));
      }
      const index = indexOf(config.elements, item);
      if (index < 0) {
        push(config.elements, item);
        return true;
      }
      if (equalElements(config.elements[index], item)) return modified;
      remove(config.elements, item);
      push(config.elements, item);
      return true;
    }
    if (item.attributes !== undefined || (item.removeAttributes !== undefined && item.removeAttributes.length)) return false;
    const modified = remove(config.replaceWithChildrenElements, item);
    if (!contains(config.removeElements, item)) return modified;
    remove(config.removeElements, item);
    return true;
  }
  // Attribute lists compare as sets (both are duplicate-free here).
  function equalLists(a, b) {
    if (a === undefined || b === undefined) return a === b;
    if (a.length !== b.length) return false;
    for (let index = 0; index < a.length; index++) if (!contains(b, a[index])) return false;
    return true;
  }
  const equalElements = (a, b) =>
    same(a, b) && equalLists(a.attributes, b.attributes) && equalLists(a.removeAttributes, b.removeAttributes);
  function removeElement(config, item) {
    const modified = remove(config.replaceWithChildrenElements, item);
    if (config.elements !== undefined) {
      if (contains(config.elements, item)) {
        remove(config.elements, item);
        return true;
      }
      return modified;
    }
    if (contains(config.removeElements, item)) return modified;
    push(config.removeElements, item);
    return true;
  }
  function replaceElementWithChildren(config, item) {
    if (contains(nonReplaceable, item)) return false;
    if (contains(config.replaceWithChildrenElements, item)) return false;
    remove(config.removeElements, item);
    remove(config.elements, item);
    if (config.replaceWithChildrenElements === undefined) config.replaceWithChildrenElements = [];
    push(config.replaceWithChildrenElements, item);
    return true;
  }
  function allowAttribute(config, item) {
    if (config.attributes !== undefined) {
      if (config.dataAttributes === true && isData(item)) return false;
      if (contains(config.attributes, item)) return false;
      const elements = config.elements || [];
      for (let index = 0; index < elements.length; index++) remove(elements[index].attributes, item);
      push(config.attributes, item);
      return true;
    }
    if (!contains(config.removeAttributes, item)) return false;
    remove(config.removeAttributes, item);
    return true;
  }
  function removeAttribute(config, item) {
    const elements = config.elements || [];
    if (config.attributes !== undefined) {
      let modified = remove(config.attributes, item);
      for (let index = 0; index < elements.length; index++) {
        if (remove(elements[index].attributes, item)) modified = true;
        remove(elements[index].removeAttributes, item);
      }
      return modified;
    }
    if (contains(config.removeAttributes, item)) return false;
    for (let index = 0; index < elements.length; index++) {
      remove(elements[index].attributes, item);
      remove(elements[index].removeAttributes, item);
    }
    push(config.removeAttributes, item);
    return true;
  }
  function removeUnsafe(config) {
    const unsafe = baseline();
    for (let index = 0; index < unsafe.elements.length; index++) removeElement(config, copyName(unsafe.elements[index]));
    for (let index = 0; index < unsafe.attributes.length; index++)
      removeAttribute(config, copyName(unsafe.attributes[index]));
  }
  function allowInstruction(config, item) {
    if (config.processingInstructions !== undefined) {
      if (contains(config.processingInstructions, item)) return false;
      push(config.processingInstructions, item);
      return true;
    }
    if (!contains(config.removeProcessingInstructions, item)) return false;
    remove(config.removeProcessingInstructions, item);
    return true;
  }
  function removeInstruction(config, item) {
    if (config.processingInstructions !== undefined) {
      if (!contains(config.processingInstructions, item)) return false;
      remove(config.processingInstructions, item);
      return true;
    }
    if (contains(config.removeProcessingInstructions, item)) return false;
    if (config.removeProcessingInstructions === undefined) config.removeProcessingInstructions = [];
    push(config.removeProcessingInstructions, item);
    return true;
  }

  // get(): a sorted SanitizerConfig dictionary in IDL member order.
  const less = (a, b) => {
    if (a.target !== undefined) return a.target < b.target;
    if (a.namespace !== b.namespace) {
      if (a.namespace === null) return true;
      if (b.namespace === null) return false;
      return a.namespace < b.namespace;
    }
    return a.name < b.name;
  };
  function sorted(list, convert) {
    const result = [];
    for (let index = 0; index < list.length; index++) {
      const item = list[index];
      let cursor = result.length;
      while (cursor > 0 && less(item, result[cursor - 1].item)) cursor--;
      for (let move = result.length; move > cursor; move--) result[move] = result[move - 1];
      result[cursor] = { item };
    }
    const output = [];
    for (let index = 0; index < result.length; index++) push(output, convert(result[index].item));
    return output;
  }
  const outputName = (item) => (item.target !== undefined ? { target: item.target } : { name: item.name, namespace: item.namespace });
  function outputElement(item) {
    const result = { name: item.name, namespace: item.namespace };
    if (item.attributes !== undefined) result.attributes = sorted(item.attributes, outputName);
    if (item.removeAttributes !== undefined) result.removeAttributes = sorted(item.removeAttributes, outputName);
    return result;
  }
  function output(config) {
    const result = {};
    for (let index = 0; index < configMembers.length; index++) {
      const key = configMembers[index][0],
        value = config[key];
      if (value === undefined) continue;
      result[key] = typeof value === "boolean" ? value : sorted(value, key === "elements" ? outputElement : outputName);
    }
    return result;
  }

  // -------------------------------------------------------- Sanitizer interface
  const configurationOf = (value) => (object(value) ? native("record", value) : undefined);
  const state = (receiver) => {
    const config = configurationOf(receiver);
    if (config === undefined) throw new TypeErrorValue("Illegal invocation");
    return config;
  };
  const constructPrefix = "Failed to construct 'Sanitizer': ";
  function SanitizerImplementation(configuration) {
    if (new.target === undefined)
      fail(constructPrefix, "Please use the 'new' operator, this DOM object constructor cannot be called as a function.");
    let config;
    if (configuration === undefined) config = copyConfig(defaultConfig());
    else if (object(configuration) || configuration === null) {
      config = canonicalize(configDictionary(configuration, constructPrefix), true);
      if (!valid(config)) fail(constructPrefix, "Invalid Sanitizer configuration.");
    } else {
      preset(configuration, constructPrefix);
      config = copyConfig(defaultConfig());
    }
    native("record", this, config);
  }
  const Sanitizer = make(SanitizerImplementation, "Sanitizer", 0, true, true, false);
  const prototype = SanitizerImplementation.prototype;
  delete prototype.constructor;
  Sanitizer.prototype = prototype;
  define(Sanitizer, "prototype", { writable: false, enumerable: false, configurable: false });
  const method = (owner, name, length, body) =>
    define(owner, name, { value: make(body, name, length), writable: true, enumerable: true, configurable: true });
  const required = (args, prefix) => {
    if (args.length < 1) fail(prefix, "1 argument required, but only 0 present.");
  };
  const prefixFor = (name) => `Failed to execute '${name}' on 'Sanitizer': `;
  const operation = (name, convert, body) =>
    method(prototype, name, 1, function (value) {
      const config = state(this);
      const prefix = prefixFor(name);
      required(arguments, prefix);
      return body(config, convert(value, prefix));
    });
  // Chrome 156's own-key order: sorted core members, then the processing
  // instruction members, then the constructor.
  operation("allowAttribute", attribute, allowAttribute);
  operation("allowElement", element, allowElement);
  method(prototype, "get", 0, function get() {
    return output(state(this));
  });
  operation("removeAttribute", attribute, removeAttribute);
  operation("removeElement", plainElement, removeElement);
  method(prototype, "removeUnsafe", 0, function () {
    removeUnsafe(state(this));
  });
  operation("replaceElementWithChildren", plainElement, replaceElementWithChildren);
  operation(
    "setComments",
    (value) => !!value,
    (config, allow) => {
      config.comments = allow;
    },
  );
  operation(
    "setDataAttributes",
    (value) => !!value,
    (config, allow) => {
      config.dataAttributes = allow;
    },
  );
  operation("allowProcessingInstruction", instruction, allowInstruction);
  operation("removeProcessingInstruction", instruction, removeInstruction);
  define(prototype, "constructor", { value: Sanitizer, writable: true, enumerable: false, configurable: true });
  define(prototype, Symbol.toStringTag, { value: "Sanitizer", configurable: true });
  define(g, "Sanitizer", { value: Sanitizer, writable: true, enumerable: false, configurable: true });

  // ---------------------------------------------- options of the HTML entry points
  // SetHTMLOptions / SetHTMLUnsafeOptions member `sanitizer`:
  // (Sanitizer or SanitizerConfig or SanitizerPresets).
  function sanitizerSpec(options, prefix, dictionary, overloaded) {
    if (options === undefined || options === null) return undefined;
    if (!object(options)) fail(prefix, overloaded ? "Overload resolution failed." : `The provided value is not of type '${dictionary}'.`);
    const raw = options.sanitizer;
    if (raw === undefined) return undefined;
    const context = member(prefix, "sanitizer", dictionary);
    const existing = configurationOf(raw);
    if (existing !== undefined) return { config: existing };
    if (object(raw) || raw === null) return { dictionary: configDictionary(raw, context) };
    return { preset: preset(raw, context) };
  }
  // "Get a sanitizer instance from options", then (safe) remove unsafe.
  function resolve(spec, safe, prefix) {
    let config;
    if (spec === undefined) {
      if (!safe) return undefined;
      config = copyConfig(defaultConfig());
    } else if (spec.config !== undefined) config = copyConfig(spec.config);
    else if (spec.preset !== undefined) config = copyConfig(defaultConfig());
    else {
      config = canonicalize(copyConfig(spec.dictionary), !safe);
      if (!valid(config)) fail(prefix, "Invalid Sanitizer configuration.");
    }
    if (safe) removeUnsafe(config);
    return config;
  }
  // (TrustedHTML or DOMString) keeps a trusted value for the sink's own check.
  const trustedOrString = (value, prefix) => (native("trustedHTML.is", value) ? value : domString(value, prefix));
  // setHTMLUnsafe / parseHTMLUnsafe: convert both arguments, run the sink's
  // Trusted Types check, then validate the sanitizer configuration.
  ns.sanitizerUnsafeArguments = (receiver, html, options, owner, name) => {
    const prefix = `Failed to execute '${name}' on '${owner}': `;
    html = trustedOrString(html, prefix);
    const spec = sanitizerSpec(options, prefix, "SetHTMLUnsafeOptions", true);
    html = native("trustedHTML.convert", receiver, html, `${owner} ${name}`, prefix);
    return { html, config: resolve(spec, false, prefix) };
  };

  const shadowCheck = ns.domAccessor("__isShadowRoot", false, "Node"),
    elementBrand = ns.domAccessor("nodeType", false, "Element");
  const brands = {
    Element: (value) => apply(elementBrand, value, []),
    ShadowRoot: (value) => {
      let shadow = false;
      try {
        shadow = apply(shadowCheck, value, []) === true;
      } catch {}
      if (!shadow) throw new TypeErrorValue("Illegal invocation");
    },
  };
  for (const owner of ["Element", "ShadowRoot"]) {
    const check = brands[owner];
    method(g[owner].prototype, "setHTML", 1, function setHTML(html, options) {
      check(this);
      const prefix = `Failed to execute 'setHTML' on '${owner}': `;
      required(arguments, prefix);
      html = domString(html, prefix);
      const config = resolve(sanitizerSpec(options, prefix, "SetHTMLOptions", false), true, prefix);
      // A safe setHTML never alters script element children.
      if (native("scriptContext", this)) return;
      native("setHTML", this, html, config);
    });
  }
  const statics = {
    parseHTML(html, options) {
      const prefix = "Failed to execute 'parseHTML' on 'Document': ";
      required(arguments, prefix);
      html = domString(html, prefix);
      const config = resolve(sanitizerSpec(options, prefix, "SetHTMLOptions", false), true, prefix);
      return native("parse", true, html, config);
    },
    parseHTMLUnsafe(html, options) {
      required(arguments, "Failed to execute 'parseHTMLUnsafe' on 'Document': ");
      const converted = ns.sanitizerUnsafeArguments(undefined, html, options, "Document", "parseHTMLUnsafe");
      return native("parse", false, converted.html, converted.config);
    },
  };
  for (const name of ["parseHTML", "parseHTMLUnsafe"])
    define(g.Document, name, { value: make(statics[name], name, 1), writable: true, enumerable: true, configurable: true });
})();
