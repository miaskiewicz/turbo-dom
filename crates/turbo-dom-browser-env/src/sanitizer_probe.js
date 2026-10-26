(async () => {
  const out = {};
  const run = (fn) => {
    try {
      const value = fn();
      return { value: value === undefined ? "<undefined>" : value };
    } catch (error) {
      return { error: error.name, message: error.message, same: error === globalThis.__sanitizerThrown };
    }
  };
  const describe = (owner, key) => {
    const descriptor = Object.getOwnPropertyDescriptor(owner, key);
    if (!descriptor) return null;
    const row = { enumerable: descriptor.enumerable, configurable: descriptor.configurable };
    if ("writable" in descriptor) row.writable = descriptor.writable;
    for (const part of ["get", "set", "value"])
      if (typeof descriptor[part] === "function") row[part] = [descriptor[part].name, descriptor[part].length];
      else if (part in descriptor && part !== "value") row[part] = null;
      else if (part === "value" && part in descriptor) row.value = String(descriptor.value);
    return row;
  };
  const thrown = new TypeError("mine");
  globalThis.__sanitizerThrown = thrown;

  // ------------------------------------------------------------- surface
  out.surface = {
    global: describe(globalThis, "Sanitizer"),
    length: Sanitizer.length,
    prototypeDescriptor: describe(Sanitizer, "prototype"),
    own: Object.getOwnPropertyNames(Sanitizer.prototype),
    members: Object.fromEntries(Object.getOwnPropertyNames(Sanitizer.prototype).map((key) => [key, describe(Sanitizer.prototype, key)])),
    tag: describe(Sanitizer.prototype, Symbol.toStringTag),
    statics: Object.getOwnPropertyNames(Sanitizer).sort(),
    chain: [Object.getPrototypeOf(Sanitizer) === Function.prototype, Object.getPrototypeOf(Sanitizer.prototype) === Object.prototype],
    source: [Function.prototype.toString.call(Sanitizer), Function.prototype.toString.call(Element.prototype.setHTML), Function.prototype.toString.call(Document.parseHTML)],
    toString: Object.prototype.toString.call(new Sanitizer()),
    ownKeys: Reflect.ownKeys(new Sanitizer()).length,
    element: Object.fromEntries(["setHTML", "setHTMLUnsafe", "getHTML"].map((key) => [key, describe(Element.prototype, key)])),
    shadowRoot: Object.fromEntries(["setHTML", "setHTMLUnsafe", "getHTML"].map((key) => [key, describe(ShadowRoot.prototype, key)])),
    document: Object.fromEntries(["parseHTML", "parseHTMLUnsafe"].map((key) => [key, describe(Document, key)])),
    documentPrototype: ["parseHTML", "parseHTMLUnsafe", "setHTML"].map((key) => key in Document.prototype),
    otherHosts: ["setHTML" in DocumentFragment.prototype, "setHTML" in Document.prototype, "setHTML" in HTMLTemplateElement.prototype],
  };
  out.brands = {
    call: run(() => Sanitizer()),
    getPlain: run(() => Sanitizer.prototype.get.call({})),
    getProto: run(() => Sanitizer.prototype.get.call(Object.create(Sanitizer.prototype))),
    allowDocument: run(() => Sanitizer.prototype.allowElement.call(document, "b")),
    unsafeUndefined: run(() => Sanitizer.prototype.removeUnsafe.call(undefined)),
    setHTMLDocument: run(() => Element.prototype.setHTML.call(document, "")),
    setHTMLFragment: run(() => Element.prototype.setHTML.call(document.createDocumentFragment(), "")),
    shadowSetHTMLElement: run(() => ShadowRoot.prototype.setHTML.call(document.createElement("div"), "")),
    parseHTMLReceiver: run(() => Document.parseHTML.call(null, "<b>x</b>").body.innerHTML),
    parseUnsafeReceiver: run(() => Document.parseHTMLUnsafe.call(5, "<b>x</b>").body.innerHTML),
    subclass: run(() => {
      class Strict extends Sanitizer {
        constructor() {
          super({ removeElements: ["b"] });
        }
      }
      const strict = new Strict();
      const host = document.createElement("div");
      host.setHTML("<b>1</b><i>2</i>", { sanitizer: strict });
      return [host.innerHTML, Object.getPrototypeOf(strict) === Strict.prototype, strict.get().removeElements.length];
    }),
  };

  // ---------------------------------------------------- configuration
  const get = (config) => run(() => new Sanitizer(config).get());
  const defaults = new Sanitizer().get();
  out.defaultGet = defaults;
  out.defaults = {
    preset: run(() => JSON.stringify(new Sanitizer("default").get()) === JSON.stringify(defaults)),
    undefinedArg: run(() => JSON.stringify(new Sanitizer(undefined).get()) === JSON.stringify(defaults)),
    copy: run(() => {
      const sanitizer = new Sanitizer();
      const first = sanitizer.get();
      first.elements.length = 0;
      first.comments = true;
      return [sanitizer.get().elements.length, sanitizer.get().comments, sanitizer.get() !== sanitizer.get()];
    }),
    removeUnsafeNoop: run(() => {
      const sanitizer = new Sanitizer();
      sanitizer.removeUnsafe();
      return JSON.stringify(sanitizer.get()) === JSON.stringify(defaults);
    }),
  };
  out.config = {
    empty: get({}),
    nullConfig: get(null),
    functionConfig: get(function () {}),
    arrayConfig: get([]),
    badPreset: get("foo"),
    numberPreset: get(1),
    booleanPreset: get(true),
    bothElements: get({ elements: ["b"], removeElements: ["i"] }),
    bothAttributes: get({ attributes: ["x"], removeAttributes: ["y"] }),
    bothInstructions: get({ processingInstructions: ["a"], removeProcessingInstructions: ["b"] }),
    duplicateElements: get({ elements: ["b", "b"] }),
    duplicateRemove: get({ removeElements: ["b", "b"] }),
    duplicateReplace: get({ replaceWithChildrenElements: ["b", "b"] }),
    duplicateAttributes: get({ attributes: ["x", "x"] }),
    duplicateRemoveAttributes: get({ removeAttributes: ["x", "x"] }),
    duplicateInstructions: get({ processingInstructions: ["a", "a"] }),
    duplicateRemoveInstructions: get({ removeProcessingInstructions: ["a", "a"] }),
    elementAttributeDuplicate: get({ elements: [{ name: "b", attributes: ["x", "x"] }] }),
    elementRemoveDuplicate: get({ elements: [{ name: "b", removeAttributes: ["x", "x"] }] }),
    elementAttributeOverlap: get({ elements: [{ name: "b", attributes: ["x"] }], attributes: ["x"] }),
    elementRemoveNotSubset: get({ elements: [{ name: "b", removeAttributes: ["x", "y"] }], attributes: ["x"] }),
    elementBothWithRemoveList: get({ elements: [{ name: "b", attributes: ["x"], removeAttributes: ["y"] }] }),
    elementBothWithAllowList: get({ elements: [{ name: "b", attributes: ["y"], removeAttributes: ["x"] }], attributes: ["x"] }),
    elementAllowVsRemoveList: get({ elements: [{ name: "b", attributes: ["x"] }], removeAttributes: ["x"] }),
    elementRemoveVsRemoveList: get({ elements: [{ name: "b", removeAttributes: ["x"] }], removeAttributes: ["x"] }),
    elementRemoveOkRemoveList: get({ elements: [{ name: "b", removeAttributes: ["x"] }], removeAttributes: ["y"] }),
    replaceOnly: get({ replaceWithChildrenElements: ["b"] }),
    replaceHtml: get({ replaceWithChildrenElements: ["html"] }),
    replaceSvg: get({ replaceWithChildrenElements: [{ name: "svg", namespace: "http://www.w3.org/2000/svg" }] }),
    replaceMath: get({ replaceWithChildrenElements: [{ name: "math", namespace: "http://www.w3.org/1998/Math/MathML" }] }),
    replaceHtmlOtherNamespace: get({ replaceWithChildrenElements: [{ name: "html", namespace: null }] }),
    replaceAndElements: get({ elements: ["b"], replaceWithChildrenElements: ["b"] }),
    replaceAndRemove: get({ removeElements: ["b"], replaceWithChildrenElements: ["b"] }),
    dataWithRemoveList: get({ removeAttributes: [], dataAttributes: true }),
    dataFalseWithRemoveList: get({ removeAttributes: [], dataAttributes: false }),
    dataInAllowList: get({ attributes: ["data-x", "y"], dataAttributes: true }),
    dataInAllowListDefault: get({ attributes: ["data-x"] }),
    dataFalse: get({ attributes: ["data-x"], dataAttributes: false }),
    dataUpper: get({ attributes: ["data-X"] }),
    dataCaps: get({ attributes: ["DATA-x"] }),
    dataBare: get({ attributes: ["data-"] }),
    dataColon: get({ attributes: ["data-a:b"] }),
    dataNamespaced: get({ attributes: [{ name: "data-x", namespace: "urn:x" }] }),
    dataInElement: get({ elements: [{ name: "b", attributes: ["data-x"] }], attributes: [] }),
    dataInElementFalse: get({ elements: [{ name: "b", attributes: ["data-x"] }], attributes: [], dataAttributes: false }),
    dataInElementRemoveList: get({ elements: [{ name: "b", attributes: ["data-x"] }] }),
    namespaces: get({
      elements: [{ name: "x", namespace: "" }, { name: "y", namespace: null }, { name: "z", namespace: undefined }, { name: "q", namespace: "urn:a" }],
      attributes: [{ name: "a", namespace: "" }, { name: "b", namespace: "urn:b" }, "c"],
    }),
    order: get({
      elements: ["z", "a", { name: "m", namespace: "http://www.w3.org/2000/svg" }, { name: "m", namespace: null }, { name: "m", namespace: "a" }, { name: "b", attributes: ["z", { name: "a", namespace: "urn:q" }, "a"] }],
      attributes: [{ name: "b", namespace: "urn:z" }, "z", "a", { name: "a", namespace: "urn:a" }],
      replaceWithChildrenElements: ["y", "c"],
    }),
    instructions: get({ processingInstructions: ["xml-stylesheet", { target: "foo" }, "Foo"] }),
    removeInstructions: get({ removeProcessingInstructions: ["b", "a"] }),
    comments: get({ comments: false }),
    coerced: get({ comments: "x", dataAttributes: 0, attributes: [] }),
    unknownKey: get({ foo: 1, elements: ["b"] }),
    setIterable: get({ elements: new Set(["b", "i"]) }),
    sanitizerAsConfig: get(new Sanitizer({ elements: ["b"] })),
    caseSensitive: get({ removeElements: ["B", "b"], removeAttributes: ["X", "x"] }),
    emptyNames: get({ removeElements: [""], removeAttributes: [""] }),
    coercedNames: get({ removeElements: [5, { name: 6, namespace: 7 }, { name: null, namespace: null }, function () {}], removeAttributes: [true] }),
  };
  out.conversion = {
    order: run(() => {
      const log = [];
      const config = {};
      for (const key of ["replaceWithChildrenElements", "removeProcessingInstructions", "removeElements", "removeAttributes", "processingInstructions", "elements", "dataAttributes", "comments", "attributes"])
        Object.defineProperty(config, key, {
          get() {
            log.push(key);
            return undefined;
          },
        });
      new Sanitizer(config);
      return log;
    }),
    nested: run(() => {
      const log = [];
      const trace = (target, label) => new Proxy(target, { get(object, key) { log.push(label + "." + String(key)); return object[key]; } });
      new Sanitizer({ elements: [trace({ name: "b", attributes: [trace({ name: "x" }, "attr")], removeAttributes: undefined }, "el")], comments: { valueOf() { log.push("comments"); return 1; } } });
      return log;
    }),
    proxyConfig: run(() => {
      const log = [];
      new Sanitizer(new Proxy({}, { get(target, key) { log.push(String(key)); return undefined; }, has(target, key) { log.push("has:" + String(key)); return false; } }));
      return log;
    }),
    elementNull: get({ elements: [null] }),
    elementUndefined: get({ elements: [undefined] }),
    elementEmpty: get({ elements: [{}] }),
    elementSymbol: get({ elements: [Symbol()] }),
    elementNameSymbol: get({ elements: [{ name: Symbol() }] }),
    elementNamespaceSymbol: get({ elements: [{ name: "b", namespace: Symbol() }] }),
    elementAttributesString: get({ elements: [{ name: "b", attributes: "x" }] }),
    elementAttributesNull: get({ elements: [{ name: "b", attributes: [null] }] }),
    elementAttributesEmpty: get({ elements: [{ name: "b", attributes: [{}] }] }),
    elementRemoveAttributesEmpty: get({ elements: [{ name: "b", removeAttributes: [{}] }] }),
    elementToString: get({ elements: [{ toString() { throw thrown; } }] }),
    nameToString: get({ elements: [{ name: { toString() { throw thrown; } } }] }),
    attributeNull: get({ attributes: [null] }),
    attributeEmpty: get({ attributes: [{}] }),
    removeElementNull: get({ removeElements: [null] }),
    replaceEmpty: get({ replaceWithChildrenElements: [{}] }),
    instructionNull: get({ processingInstructions: [null] }),
    instructionEmpty: get({ processingInstructions: [{}] }),
    removeInstructionEmpty: get({ removeProcessingInstructions: [{}] }),
    elementsString: get({ elements: "b" }),
    elementsNull: get({ elements: null }),
    elementsUndefined: get({ elements: undefined }),
    elementsObject: get({ elements: {} }),
    iteratorNotCallable: get({ elements: { [Symbol.iterator]: 5 } }),
    iteratorNotObject: get({ elements: { [Symbol.iterator]() { return 5; } } }),
    stepNotObject: get({ elements: { [Symbol.iterator]() { return { next() { return 5; } }; } } }),
    iteratorThrows: get({ elements: { [Symbol.iterator]() { throw thrown; } } }),
    getterThrows: get({ get elements() { throw thrown; } }),
    nameGetterThrows: get({ elements: [{ get name() { throw thrown; } }] }),
  };

  // ---------------------------------------------------------- methods
  const steps = (config, list) =>
    run(() => {
      const sanitizer = new Sanitizer(config);
      const results = list.map(([name, ...args]) => run(() => sanitizer[name](...args)));
      return { results, get: sanitizer.get() };
    });
  const svg = (name) => ({ name, namespace: "http://www.w3.org/2000/svg" });
  out.methods = {
    removeListElements: steps({}, [["allowElement", "b"], ["allowElement", { name: "b", attributes: ["x"] }], ["allowElement", { name: "b", removeAttributes: [] }], ["allowElement", { name: "b", removeAttributes: ["x"] }], ["removeElement", "b"], ["removeElement", "b"], ["allowElement", "b"], ["allowElement", "b"], ["replaceElementWithChildren", "i"], ["allowElement", "i"], ["allowElement", "i"]]),
    allowListElements: steps({ elements: [] }, [["allowElement", "b"], ["allowElement", "b"], ["allowElement", { name: "b", attributes: ["x"] }], ["allowElement", { name: "b", attributes: ["x"] }], ["removeElement", "b"], ["removeElement", "b"], ["replaceElementWithChildren", "i"], ["replaceElementWithChildren", "i"], ["allowElement", "i"], ["replaceElementWithChildren", "html"], ["replaceElementWithChildren", svg("svg")], ["replaceElementWithChildren", { name: "math", namespace: "http://www.w3.org/1998/Math/MathML" }], ["replaceElementWithChildren", "u"], ["removeElement", "u"], ["replaceElementWithChildren", "u"]]),
    replaceMovesFromLists: steps({ elements: ["b"] }, [["replaceElementWithChildren", "b"], ["allowElement", "b"]]),
    replaceFromRemoveList: steps({ removeElements: ["b"] }, [["replaceElementWithChildren", "b"], ["removeElement", "b"]]),
    elementAttributeOrder: steps({ elements: [{ name: "b", attributes: ["x", "y"] }] }, [["allowElement", { name: "b", attributes: ["y", "x"] }], ["allowElement", { name: "b", attributes: ["y", "x"] }]]),
    allowListAttributes: steps({ attributes: [] }, [["allowAttribute", "x"], ["allowAttribute", "x"], ["removeAttribute", "x"], ["removeAttribute", "x"], ["setDataAttributes", true], ["allowAttribute", "data-y"], ["setDataAttributes", false], ["allowAttribute", "data-y"], ["setDataAttributes", true]]),
    removeListAttributes: steps({}, [["removeAttribute", "x"], ["removeAttribute", "x"], ["allowAttribute", "x"], ["allowAttribute", "x"], ["setDataAttributes", true], ["setComments", true], ["setComments", false], ["setComments", false]]),
    elementAttributeFixups: steps({ elements: [{ name: "b", attributes: ["x", "y"] }, { name: "i", removeAttributes: ["z"] }], attributes: ["z"] }, [["allowAttribute", "x"], ["removeAttribute", "z"], ["removeAttribute", "y"], ["setDataAttributes", false]]),
    removeListElementFixups: steps({ elements: [{ name: "b", attributes: ["x", "y"] }, { name: "i", removeAttributes: ["z"] }] }, [["removeAttribute", "x"], ["removeAttribute", "z"], ["allowAttribute", "z"]]),
    allowElementDedupe: steps({ elements: [], attributes: ["g"] }, [["allowElement", { name: "b", attributes: ["x", "x", "g", "data-a"], removeAttributes: ["g", "g", "q"] }]]),
    allowElementRemoveListDedupe: steps({ elements: [], removeAttributes: ["r"] }, [["allowElement", { name: "b", attributes: ["x", "x", "r"] }], ["allowElement", { name: "i", removeAttributes: ["r", "s", "s"] }]]),
    allowElementOnRemoveList: steps({ removeElements: ["b"], removeAttributes: ["r"] }, [["allowElement", { name: "b", attributes: ["x"] }], ["allowElement", "b"], ["allowElement", "b"]]),
    setDataAttributesKeepsLists: steps({ attributes: ["data-x", "title"], elements: [{ name: "b", attributes: ["data-y", "id"] }], dataAttributes: false }, [["setDataAttributes", true], ["allowElement", { name: "i", attributes: ["data-z", "w"] }]]),
    removeUnsafe: steps({}, [["removeUnsafe"], ["removeUnsafe"]]),
    removeUnsafeAllowList: steps({ elements: ["script", "b", "iframe", { name: "b", namespace: null }, svg("use"), { name: "x", attributes: ["onclick", "y"] }], attributes: ["onclick", "x", "onfoo"] }, [["removeUnsafe"]]),
    instructions: steps({}, [["allowProcessingInstruction", "a"], ["removeProcessingInstruction", "a"], ["removeProcessingInstruction", { target: "a" }], ["allowProcessingInstruction", "a"]]),
    instructionAllowList: steps({ processingInstructions: [] }, [["allowProcessingInstruction", "a"], ["allowProcessingInstruction", { target: "a" }], ["removeProcessingInstruction", "a"], ["removeProcessingInstruction", "a"]]),
    argumentErrors: steps({}, [["allowElement"], ["allowElement", {}], ["allowElement", 5], ["allowElement", null], ["allowElement", undefined], ["allowElement", { name: Symbol() }], ["removeElement", null], ["removeElement", ["b"]], ["removeElement", Symbol()], ["replaceElementWithChildren", null], ["allowAttribute", {}], ["allowAttribute", null], ["removeAttribute", undefined], ["setComments"], ["setComments", Symbol()], ["setDataAttributes"], ["allowProcessingInstruction", {}], ["allowProcessingInstruction", null], ["removeProcessingInstruction", undefined], ["allowProcessingInstruction", { target: Symbol() }], ["allowElement", { get name() { throw thrown; } }], ["get", 1], ["removeUnsafe", 1]]),
    getterOrder: run(() => {
      const logs = [];
      for (const [name, value] of [["allowElement", { name: "b" }], ["allowAttribute", { name: "x" }], ["allowProcessingInstruction", { target: "p" }]]) {
        const log = [];
        new Sanitizer({})[name](new Proxy(value, { get(target, key) { log.push(String(key)); return target[key]; } }));
        logs.push(log);
      }
      return logs;
    }),
  };

  // --------------------------------------------------- entry-point options
  const host = document.createElement("div");
  const html = (markup, options) => run(() => { host.setHTML(markup, options); return host.innerHTML; });
  const unsafe = (markup, options) => run(() => { host.setHTMLUnsafe(markup, options); return host.innerHTML; });
  out.options = {
    noArguments: run(() => host.setHTML()),
    shadowNoArguments: run(() => document.createElement("div").attachShadow({ mode: "open" }).setHTML()),
    parseNoArguments: run(() => Document.parseHTML()),
    parseUnsafeNoArguments: run(() => Document.parseHTMLUnsafe()),
    unsafeNoArguments: run(() => host.setHTMLUnsafe()),
    nullOptions: html("<b onclick=1>n</b><!--c-->", null),
    numberOptions: html("<b>", 5),
    stringOptions: html("<b>", "default"),
    functionOptions: html("<b onclick=1>f</b>", function () {}),
    sanitizerNull: html("<b onclick=1>n</b><!--c-->", { sanitizer: null }),
    sanitizerUndefined: html("<b onclick=1>n</b><!--c-->", { sanitizer: undefined }),
    sanitizerNumber: html("<b>", { sanitizer: 5 }),
    sanitizerBadPreset: html("<b>", { sanitizer: "nope" }),
    sanitizerSymbol: html("<b>", { sanitizer: Symbol() }),
    sanitizerInvalid: html("<b>", { sanitizer: { elements: [], removeElements: [] } }),
    sanitizerElementEmpty: html("<b>", { sanitizer: { elements: [{}] } }),
    sanitizerElementNull: html("<b>", { sanitizer: { elements: [null] } }),
    sanitizerAttributeEmpty: html("<b>", { sanitizer: { elements: [{ name: "b", attributes: [{}] }] } }),
    sanitizerGetterThrows: html("<b>", { get sanitizer() { throw thrown; } }),
    shadowBadPreset: run(() => document.createElement("div").attachShadow({ mode: "open" }).setHTML("<b>", { sanitizer: "x" })),
    unsafeInvalid: unsafe("<b>", { sanitizer: { elements: [], removeElements: [] } }),
    unsafeBadPreset: unsafe("<b>", { sanitizer: "x" }),
    unsafeNumber: unsafe("<b>", 5),
    unsafeTrue: unsafe("<b>", true),
    unsafeString: unsafe("<b>", "default"),
    unsafeNull: unsafe("<b onclick=1>x</b>", null),
    unsafeFunction: unsafe("<b onclick=1>x</b>", function () {}),
    unsafeSanitizerNull: unsafe("<b onclick=1>n</b><!--c-->", { sanitizer: null }),
    shadowUnsafeNumber: run(() => document.createElement("div").attachShadow({ mode: "open" }).setHTMLUnsafe("<b>", 5)),
    parseUnsafeNumber: run(() => Document.parseHTMLUnsafe("<b>", 5)),
    parseUnsafeInvalid: run(() => Document.parseHTMLUnsafe("<b>", { sanitizer: { elements: [], removeElements: [] } })),
    parseInvalid: run(() => Document.parseHTML("<b>", { sanitizer: { elements: [], removeElements: [] } })),
    parseBadPreset: run(() => Document.parseHTML("<b>", { sanitizer: "x" })),
    parseNumber: run(() => Document.parseHTML("<b>", true)),
    htmlSymbol: run(() => host.setHTML(Symbol())),
    htmlThrows: run(() => host.setHTML({ toString() { throw thrown; } })),
    unsafeHtmlSymbol: run(() => host.setHTMLUnsafe(Symbol())),
    parseHtmlSymbol: run(() => Document.parseHTML(Symbol())),
    parseUnsafeHtmlSymbol: run(() => Document.parseHTMLUnsafe(Symbol())),
    order: ["setHTML", "setHTMLUnsafe", "parseHTML", "parseHTMLUnsafe"].map((name) =>
      run(() => {
        const log = [];
        const markup = { toString() { log.push("html"); return "<b>x</b>"; } };
        const options = { get sanitizer() { log.push("sanitizer"); return {}; } };
        if (name.startsWith("parse")) Document[name](markup, options);
        else host[name](markup, options);
        return log;
      }),
    ),
    invalidKeepsChildren: run(() => {
      host.innerHTML = "<i>keep</i>";
      try {
        host.setHTML("<b>x</b>", { sanitizer: { elements: [], removeElements: [] } });
      } catch {}
      return host.innerHTML;
    }),
    instanceNotMutated: run(() => {
      const sanitizer = new Sanitizer({ elements: ["script", "b"], attributes: ["onclick"] });
      host.setHTML("<b onclick=1>x</b><script>y</script>", { sanitizer });
      return [host.innerHTML, sanitizer.get()];
    }),
    instanceComments: run(() => {
      const sanitizer = new Sanitizer({});
      host.setHTML("<script>x</script><!--c--><b>y</b>", { sanitizer });
      return [host.innerHTML, sanitizer.get().removeElements.length];
    }),
    fakeSanitizer: html("<b>1</b><i>2</i><!--c-->", { sanitizer: Object.create(Sanitizer.prototype) }),
  };
  out.contexts = {
    template: run(() => {
      const template = document.createElement("template");
      template.setHTML("<b>x</b><script>y</script>");
      return [template.innerHTML, template.childNodes.length, template.content.childNodes.length];
    }),
    script: run(() => {
      const script = document.createElement("script");
      script.textContent = "old";
      script.setHTML("<b>x</b>", { sanitizer: {} });
      return script.textContent;
    }),
    scriptInvalidConfig: run(() => {
      const script = document.createElement("script");
      script.textContent = "old";
      script.setHTML("<b>x</b>", { sanitizer: { elements: [], removeElements: [] } });
      return script.textContent;
    }),
    scriptBadOptions: run(() => document.createElement("script").setHTML("<b>x</b>", 5)),
    svgScript: run(() => {
      const script = document.createElementNS("http://www.w3.org/2000/svg", "script");
      script.textContent = "old";
      script.setHTML("<b>x</b>", { sanitizer: {} });
      return script.textContent;
    }),
    otherScript: run(() => {
      const script = document.createElementNS("urn:x", "script");
      script.setHTML("<b>x</b>", { sanitizer: {} });
      return script.innerHTML;
    }),
    scriptUnsafe: run(() => {
      const script = document.createElement("script");
      script.textContent = "old";
      script.setHTMLUnsafe("<b>x</b>", { sanitizer: {} });
      return script.textContent;
    }),
    style: run(() => {
      const style = document.createElement("style");
      style.setHTML("<b>x</b>");
      return style.innerHTML;
    }),
    textarea: run(() => {
      const textarea = document.createElement("textarea");
      textarea.setHTML("<b>x</b>");
      return textarea.innerHTML;
    }),
    shadowRoot: run(() => {
      const root = document.createElement("div").attachShadow({ mode: "open" });
      root.setHTML("<b onclick=1>x</b><script>y</script><!--c-->");
      return root.innerHTML;
    }),
    existingShadow: run(() => {
      const element = document.createElement("div");
      element.attachShadow({ mode: "open" }).innerHTML = "<i>x</i>";
      element.setHTML("<b>y</b>");
      return [element.innerHTML, element.shadowRoot.innerHTML];
    }),
    mutations: await (async () => {
      const target = document.createElement("div");
      document.body.append(target);
      target.innerHTML = "<i>a</i><i>b</i>";
      const records = [];
      const observer = new MutationObserver((list) => records.push(...list));
      observer.observe(target, { childList: true, subtree: true, attributes: true, characterData: true });
      target.setHTML("<b onclick=1>x</b><p>y<script>z</script><!--c--></p>");
      await Promise.resolve();
      observer.disconnect();
      target.remove();
      return records.map((record) => [record.type, record.addedNodes.length, record.removedNodes.length, record.target === target]);
    })(),
  };

  // ----------------------------------------------------- sanitization matrix
  const S = (config) => new Sanitizer(config);
  const anchors = ["javascript:a", " javascript:b", "\tjava\nscript:c", "JaVaScRiPt:d", "javascript&colon;e", "&#1;javascript:f", "javascript", "vbscript:g", "data:text/html,h", "java&#0;script:i", "   javascript:j", "javascript :k", "jav​ascript:l", "javascript&#x3a;m", "javascript:", "https://ok", "#frag", "javascript\u0001:n", " \u0007JAVASCRIPT:o"];
  out.safe = {
    defaultBasic: html('<b onclick="x" id=a class=c style="color:red" title=t>a</b><script>x</script><i data-x=1>i</i><!--c--><a href="javascript:x">l</a><a href=" JAVA\tscript:x">l2</a><a href="https://x">l3</a>'),
    permissive: html('<b onclick="x">a</b><script>x</script><iframe></iframe><!--c--><i data-x=1>i</i><a href="javascript:x">l</a><object></object><embed><frame><base href=x><svg><script>s</script><use href="#a"></use></svg>', { sanitizer: {} }),
    permissiveInstance: html('<b onclick="x">a</b><script>x</script><!--c-->', { sanitizer: S({}) }),
    allowScript: html('<script>x</script><b onclick=1 onmouseover=2 onfoo=3>b</b><iframe></iframe>', { sanitizer: { elements: ["script", "b", "iframe"], attributes: ["onclick", "onfoo"] } }),
    allowScriptInstance: html('<script>x</script><b onclick=1 onfoo=3>b</b>', { sanitizer: S({ elements: ["script", "b"], attributes: ["onclick", "onfoo"] }) }),
    elementsOnly: html('<p id=1 class=2><b data-x=1 title=t>x</b><span>s</span><!--c--></p>', { sanitizer: { elements: ["p", { name: "b", attributes: ["title"] }] } }),
    replace: html('<p><b>x<i>y</i></b></p>', { sanitizer: { replaceWithChildrenElements: ["b"] } }),
    removeVsReplace: html('<p>1<b>2<i>3</i></b><u>4<b>5</b></u></p>', { sanitizer: { removeElements: ["i"], replaceWithChildrenElements: ["b", "u"] } }),
    replaceTemplate: html('<p><template><b>t</b></template>after</p>', { sanitizer: { replaceWithChildrenElements: ["template"] } }),
    template: html('<template><b onclick=1>t</b><script>x</script><!--c--></template>', { sanitizer: {} }),
    templateNested: html('<template><template><i onclick=1>n</i><iframe></iframe></template></template>', { sanitizer: S({}) }),
    perElementRemove: html('<p title=t id=x>1</p><b title=t id=y>2</b>', { sanitizer: { elements: [{ name: "p", removeAttributes: ["title"] }, { name: "b", attributes: ["id"] }], attributes: ["title"] } }),
    perElementRemoveList: html('<p title=t id=x>1</p><b title=t id=y>2</b><i title=t id=z>3</i>', { sanitizer: { elements: [{ name: "p", removeAttributes: ["title"] }, { name: "b", attributes: ["id"] }, "i"], removeAttributes: ["id"] } }),
    attributeList: html('<b title=t data-x=1 id=i>x</b>', { sanitizer: { attributes: ["title"] } }),
    attributeListInstance: html('<b title=t data-x=1 id=i>x</b>', { sanitizer: S({ attributes: ["title"] }) }),
    dataAttributes: html('<b data-x=1 data-Y=2 data-=3 title=4 DATA-z=5>x</b>', { sanitizer: S({ attributes: ["title"], dataAttributes: true }) }),
    dataAttributesFalse: html('<b data-x=1 data-y=2 title=4>x</b>', { sanitizer: S({ attributes: ["title", "data-x"], dataAttributes: false }) }),
    dataForeign: html('<svg data-x=1 data-y=2><text data-z=3>t</text></svg><math data-m=1><mi data-n=2>x</mi></math><b data-b=1>b</b>', { sanitizer: S({ elements: [svg("svg"), svg("text"), { name: "math", namespace: "http://www.w3.org/1998/Math/MathML" }, { name: "mi", namespace: "http://www.w3.org/1998/Math/MathML" }, "b"], attributes: [], dataAttributes: true }) }),
    comments: html('<!--a--><b><!--b-->x</b><template><!--c--></template>', { sanitizer: S({}) }),
    commentsOff: html('<!--a--><b><!--b-->x</b>', { sanitizer: S({ comments: false }) }),
    commentsDictionary: html('<!--a--><b>x</b>', { sanitizer: { comments: true } }),
    bogusComments: html('<?xml x?><b>a</b><!doctype html><![CDATA[x]]>', { sanitizer: S({}) }),
    noscript: html('<noscript><b>x</b><p>y</p></noscript>', { sanitizer: {} }),
    customElement: html('<my-el foo=1><b>x</b></my-el>'),
    customAllowed: html('<my-el foo=1 onclick=1><b>x</b></my-el>', { sanitizer: {} }),
    style: html('<style>b{}</style><b style="color:red">x</b>', { sanitizer: {} }),
    anchors: html(anchors.map((url, index) => `<a id=a${index} href="${url}">${index}</a>`).join(""), { sanitizer: {} }),
    anchorsUnsafeDefault: unsafe(anchors.slice(0, 6).map((url, index) => `<a id=a${index} href="${url}">${index}</a>`).join(""), { sanitizer: "default" }),
    urlAttributes: html('<a href="javascript:a" ping="javascript:b" cite="javascript:x" xlink:href="javascript:h"></a><blockquote cite="javascript:c"></blockquote><q cite="javascript:d"></q><form action="javascript:e" formaction="javascript:x"></form><button formaction="javascript:f" action="javascript:x"></button><input formaction="javascript:g"><img src="javascript:h" srcset="javascript:i"><video poster="javascript:j"></video><object data="javascript:k"></object><area href="javascript:l"><link href="javascript:m"><div href="javascript:n"></div><iframe src="javascript:o"></iframe>', { sanitizer: {} }),
    svgUrls: html('<svg><a href="javascript:a" xlink:href="javascript:b">x</a><a href="https://ok" xlink:href="https://ok2">y</a><image href="javascript:c" xlink:href="javascript:d"></image><use href="javascript:e"></use><feImage href="javascript:f"></feImage><animate attributeName="href"></animate><animate attributeName="HREF"></animate><animate attributeName=" href"></animate><animate attributeName="xlink:href"></animate><animateMotion attributeName="href"></animateMotion><animateTransform attributeName="href"></animateTransform><set attributeName="href"></set><set attributeName="fill"></set><set attributename="xlink:href"></set><discard attributeName="href"></discard></svg>', { sanitizer: {} }),
    mathUrls: html('<math href="javascript:a"><mi href="https://ok">x</mi><mo xlink:href="javascript:b">y</mo><mtext href=" javascript:c">z</mtext><mn src="javascript:d">1</mn></math>', { sanitizer: {} }),
    namespacedAttributes: html('<svg xml:lang="en" xmlns:foo="urn:x" foo:bar="1" xlink:title="t"><text xml:space="preserve">a</text></svg><p xml:lang="en" xlink:href="javascript:x">p</p>', { sanitizer: { attributes: [{ name: "lang", namespace: "http://www.w3.org/XML/1998/namespace" }, "foo:bar", { name: "xlink:href" }] } }),
    caseNames: html('<svg><foreignObject><div>x</div></foreignObject><foreignobject>y</foreignobject></svg><B>b</B>', { sanitizer: { elements: ["svg", "B", { name: "foreignobject", namespace: "http://www.w3.org/2000/svg" }, "div"] } }),
    elementNamespaces: html('<svg><a>svga</a></svg><a>htmla</a><math><mi>m</mi></math>', { sanitizer: { elements: [svg("svg"), svg("a"), "math", { name: "mi", namespace: "http://www.w3.org/1998/Math/MathML" }] } }),
    nullNamespace: html('<b>x</b>', { sanitizer: { elements: [{ name: "b", namespace: null }] } }),
    defaultMatrix: html('<h1 id=h class=c title=t lang=en dir=rtl>h</h1><a href="https://x" target=_blank rel=noopener hreflang=en type=t download>a</a><img src=x alt=y><table border=1><caption>c</caption><colgroup span=2><col span=1></colgroup><thead><tr><th colspan=2 rowspan=1 scope=col headers=h abbr=a>t</th></tr></thead><tbody><tr><td colspan=2 headers=x>d</td></tr></tbody><tfoot><tr><td>f</td></tr></tfoot></table><ol start=3 reversed type=a><li value=5>l</li></ol><ul><li>u</li></ul><q cite="https://c">q</q><blockquote cite="https://c">b</blockquote><time datetime="2020">t</time><data value=1>d</data><abbr title=t>a</abbr><del cite=x datetime=y>d</del><ins cite=x datetime=y>i</ins><form><input value=x></form><details open><summary>s</summary>d</details><button>b</button><select><option>o</option></select><textarea>t</textarea><video src=x></video><canvas></canvas><dialog open>d</dialog><br><hr><wbr><pre>p</pre><bdo dir=rtl>b</bdo><bdi>i</bdi><ruby>r<rp>(</rp><rt>t</rt></ruby><search>s</search><menu><li>m</li></menu><hgroup><h2>h</h2></hgroup><figure><figcaption>f</figcaption></figure><main>m</main><nav>n</nav><article>a</article><aside>a</aside><section>s</section><header>h</header><footer>f</footer><address>a</address><dl><dt>t</dt><dd>d</dd></dl><span style="x">s</span><div class=x>d</div><em>e</em><strong>s</strong><small>s</small><s>s</s><cite>c</cite><dfn>d</dfn><code>c</code><var>v</var><samp>s</samp><kbd>k</kbd><sub>s</sub><sup>s</sup><i>i</i><u>u</u><mark>m</mark><p>p</p><h3>3</h3><h4>4</h4><h5>5</h5><h6>6</h6><label>l</label><fieldset>f</fieldset><slot>s</slot><map><area></map><picture><source></picture><audio></audio><meter></meter><progress></progress><output></output><font>f</font><center>c</center><marquee>m</marquee>'),
    svgDefault: html('<svg viewBox="0 0 10 10" width=10 height=10 xmlns="http://www.w3.org/2000/svg" fill=red style="x" class=c id=i><circle cx=1 cy=2 r=3 fill=red stroke=blue transform="t" onclick=x></circle><path d="M0" pathLength=1></path><text x=1 y=2 dx=1 text-anchor=middle font-size=3>t<tspan>u</tspan></text><foreignObject width=1><div>x</div></foreignObject><g><rect x=1 width=2 rx=1></rect></g><defs><linearGradient id=g><stop offset=1></stop></linearGradient></defs><filter><feGaussianBlur></feGaussianBlur></filter><a href="https://x">a</a><image href=x></image><title>t</title><desc>d</desc><style>x</style><mask></mask><clipPath></clipPath><pattern></pattern><marker markerWidth=1></marker><polygon points="1"></polygon><polyline points="1"></polyline><line x1=1></line><ellipse rx=1></ellipse><textPath href="#x">p</textPath><metadata>m</metadata><symbol></symbol><switch></switch><view></view></svg>'),
    mathDefault: html('<math display=block xmlns="x" dir=rtl mathcolor=red><mrow><mi mathvariant=bold>x</mi><mo stretchy=true fence=true form=prefix lspace=1>+</mo><mn>1</mn><mfrac linethickness=1><mi>a</mi><mi>b</mi></mfrac><msqrt><mi>c</mi></msqrt><mroot><mi>d</mi><mn>3</mn></mroot><mtable><mtr><mtd columnspan=2 rowspan=1>e</mtd></mtr></mtable><mspace width=1></mspace><mtext>t</mtext><ms>s</ms><mstyle scriptlevel=1>s</mstyle><mpadded width=1>p</mpadded><mphantom>p</mphantom><msub><mi>a</mi><mn>1</mn></msub><msup><mi>a</mi><mn>2</mn></msup><msubsup><mi>a</mi><mn>1</mn><mn>2</mn></msubsup><munder><mi>a</mi><mi>b</mi></munder><mover><mi>a</mi><mi>b</mi></mover><munderover><mi>a</mi><mi>b</mi><mi>c</mi></munderover><mmultiscripts><mi>a</mi><mprescripts></mprescripts><mi>b</mi></mmultiscripts><semantics><mi>x</mi><annotation>a</annotation><annotation-xml>b</annotation-xml></semantics><merror>e</merror><menclose>e</menclose><maction>a</maction></mrow></math>'),
    headElements: html('<title>t</title><meta charset=x><link rel=x><base href=x><style>s</style>x', { sanitizer: S({}) }),
  };
  out.declarative = {
    host: run(() => {
      host.setHTML('<div><template shadowrootmode=open><b onclick=1>x</b><script>s</script><!--c--></template></div><p><template shadowrootmode=closed shadowrootserializable><i>z</i></template></p>', { sanitizer: {} });
      return [host.firstChild.shadowRoot.innerHTML, host.getHTML({ serializableShadowRoots: true })];
    }),
    defaultConfig: run(() => {
      host.setHTML('<div><template shadowrootmode=open><b>in</b><img src=x></template>light</div>');
      return [host.innerHTML, host.firstChild.shadowRoot && host.firstChild.shadowRoot.innerHTML];
    }),
    nested: run(() => {
      host.setHTML('<div><template shadowrootmode=open><span><template shadowrootmode=open><b onclick=1>deep</b><iframe></iframe></template></span></template></div>', { sanitizer: {} });
      return host.firstChild.shadowRoot.firstChild.shadowRoot.innerHTML;
    }),
    hostRemoved: html('<div><template shadowrootmode=open><b>x</b></template></div>', { sanitizer: { removeElements: ["div"] } }),
    allowListFilters: run(() => {
      host.setHTML('<div><template shadowrootmode=open><b>x</b><i>y</i></template></div>', { sanitizer: { elements: ["div", "i"] } });
      return [host.innerHTML, host.firstChild.shadowRoot && host.firstChild.shadowRoot.innerHTML];
    }),
    // The template element (and its shadowroot* attributes) is sanitized before
    // the declarative shadow root is attached.
    templateRules: (() => {
      const markup = '<div><template shadowrootmode=open><b>x</b><i>y</i></template>light</div>';
      const closed = '<div><template shadowrootmode=closed shadowrootserializable><b>x</b><i>y</i></template>light</div>';
      const nested = '<div><template shadowrootmode=open><span><template shadowrootmode=open><b>deep</b><i>no</i></template>s</span></template></div>';
      const probe = (sanitizer, source = markup, unsafeCall = false) =>
        run(() => {
          const options = sanitizer === undefined ? undefined : { sanitizer };
          if (unsafeCall) host.setHTMLUnsafe(source, options);
          else host.setHTML(source, options);
          const shadowHost = host.firstChild;
          const root = shadowHost.shadowRoot;
          const inner = root && root.firstChild && root.firstChild.shadowRoot;
          const shadow = root ? [root.innerHTML, inner ? inner.innerHTML : null] : null;
          let attach;
          try {
            const attached = shadowHost.attachShadow({ mode: "open" });
            attach = ["ok", attached.innerHTML, attached === shadowHost.shadowRoot];
          } catch (error) {
            attach = [error.name];
          }
          return [host.innerHTML, shadow, attach, host.getHTML({ serializableShadowRoots: true })];
        });
      return {
        defaults: probe(undefined),
        empty: probe({}),
        divOnly: probe({ elements: ["div"] }),
        divAndB: probe({ elements: ["div", "b"] }),
        divTemplate: probe({ elements: ["div", "template"] }),
        divTemplateB: probe({ elements: ["div", "template", "b"] }),
        removeTemplate: probe({ removeElements: ["template"] }),
        replaceTemplate: probe({ replaceWithChildrenElements: ["template"] }),
        templateWithoutAttributes: probe({ elements: ["div", { name: "template", attributes: [] }, "b"] }),
        templateKeepsMode: probe({ elements: ["div", { name: "template", attributes: ["shadowrootmode"] }, "b"] }),
        removeModeAttribute: probe({ removeAttributes: ["shadowrootmode"] }),
        removeSerializable: probe({ removeAttributes: ["shadowrootserializable"] }, closed),
        closedDefault: probe(undefined, closed),
        closedTemplate: probe({ elements: ["div", "template", "b"] }, closed),
        nested: probe({ elements: ["div", "span", "template", "b"] }, nested),
        unsafeNoTemplate: probe({ elements: ["div", "b"] }, markup, true),
        unsafeRemoveTemplate: probe({ removeElements: ["template"] }, markup, true),
        unsafeReplaceTemplate: probe({ replaceWithChildrenElements: ["template"] }, markup, true),
        unsafeDefault: probe("default", markup, true),
        unsafeNone: probe(undefined, markup, true),
      };
    })(),
    parsedTemplateRules: (() => {
      const markup = '<div><template shadowrootmode=open><b>x</b><i>y</i></template>light</div>';
      const probe = (sanitizer, safe) =>
        run(() => {
          const options = sanitizer === undefined ? undefined : { sanitizer };
          const parsedDocument = safe ? Document.parseHTML(markup, options) : Document.parseHTMLUnsafe(markup, options);
          const shadowHost = parsedDocument.body && parsedDocument.body.firstChild;
          return [parsedDocument.body ? parsedDocument.body.innerHTML : null, shadowHost && shadowHost.shadowRoot ? shadowHost.shadowRoot.innerHTML : null];
        });
      return {
        safeDefault: probe(undefined, true),
        safeEmpty: probe({}, true),
        safeNoTemplate: probe({ elements: ["html", "head", "body", "div", "b"] }, true),
        safeTemplate: probe({ elements: ["html", "head", "body", "div", "b", "template"] }, true),
        unsafeNone: probe(undefined, false),
        unsafeNoTemplate: probe({ elements: ["html", "head", "body", "div", "b"] }, false),
        unsafeRemoveTemplate: probe({ removeElements: ["template"] }, false),
        unsafeReplaceTemplate: probe({ replaceWithChildrenElements: ["template"] }, false),
      };
    })(),
    replacedHost: html('<div><template shadowrootmode=open><b>x</b></template>light</div>', { sanitizer: { replaceWithChildrenElements: ["div"] } }),
    templateAttributes: run(() => {
      host.setHTML('<div><template shadowrootmode=open onclick=1 id=x><b>in</b></template></div>', { sanitizer: {} });
      return [host.innerHTML, host.firstChild.shadowRoot.innerHTML];
    }),
    invalidHost: html('<a><template shadowrootmode=open><b onclick=1>in</b><script>x</script></template></a>', { sanitizer: {} }),
    topLevel: run(() => {
      host.setHTML('<template shadowrootmode=open><b>in</b></template>', { sanitizer: {} });
      return [host.innerHTML, !!host.shadowRoot];
    }),
    flags: run(() => {
      host.setHTML('<div><template shadowrootmode=closed shadowrootserializable shadowrootclonable shadowrootdelegatesfocus><b>in</b></template></div><section><template shadowrootmode=open shadowrootslotassignment=MANUAL><slot></slot></template></section>', { sanitizer: {} });
      return [host.getHTML({ serializableShadowRoots: true }), host.lastChild.shadowRoot.slotAssignment];
    }),
    unsafeSanitized: run(() => {
      host.setHTMLUnsafe('<div><template shadowrootmode=open shadowrootslotassignment=manual><b onclick=1>x</b><i>y</i><!--c--></template></div>', { sanitizer: { removeElements: ["i"] } });
      return [host.firstChild.shadowRoot.innerHTML, host.firstChild.shadowRoot.slotAssignment];
    }),
  };
  out.unsafe = {
    empty: unsafe('<a href="javascript:x" onclick=1>l</a><script>s</script><!--c--><i data-x=1>i</i><iframe></iframe>', { sanitizer: {} }),
    defaultPreset: unsafe('<a href="javascript:x" title=t>l</a><b onclick=1>x</b><script>s</script><!--c-->', { sanitizer: "default" }),
    instance: unsafe('<a href="javascript:x">l</a><script>s</script><i>i</i>', { sanitizer: S({ removeElements: ["i"] }) }),
    elements: unsafe('<b onclick=1>x</b><i>y</i><!--c-->', { sanitizer: { elements: ["b"] } }),
    attributes: unsafe('<b title=t data-x=1 id=i>x</b>', { sanitizer: { attributes: ["title"] } }),
    svgAnimation: unsafe('<svg><set attributeName="href"></set><a href="javascript:x">a</a></svg><math href="javascript:y"></math>', { sanitizer: {} }),
  };
  const shape = (document) => [document.compatMode, document.doctype && document.doctype.name, Array.from(document.childNodes, (node) => node.nodeType), document.documentElement ? document.documentElement.outerHTML : null, document.URL, document.contentType, document.characterSet, document instanceof Document, document === window.document];
  const parsed = (markup, options, safe) => run(() => shape(safe ? Document.parseHTML(markup, options) : Document.parseHTMLUnsafe(markup, options)));
  out.parse = {
    safeDefault: parsed('<html><head><title>t</title><meta charset=utf8><link rel=x><style>s</style><base href=x></head><body class=c><b onclick=1>x</b><script>s</script><!--c--></body></html>', undefined, true),
    safeDoctype: parsed('<!doctype html><b onclick=1>x</b><script>s</script><!--c-->', undefined, true),
    safeComments: parsed('<!--a--><!doctype html><!--b--><html><!--c--><body>x<!--d--></body></html><!--e-->', { sanitizer: {} }, true),
    safeInstanceComments: parsed('<!--a--><!doctype html><html><body>x<!--d--></body></html>', { sanitizer: S({}) }, true),
    safeNoComments: parsed('<!--a--><!doctype html><b>x</b>', { sanitizer: S({ comments: false }) }, true),
    removeHtml: parsed('<!--a--><!doctype html><b>x</b>', { sanitizer: { removeElements: ["html"] } }, true),
    elementsOnly: parsed('<b onclick=1>x</b><i>y</i>', { sanitizer: { elements: ["b"] } }, true),
    removeElements: parsed('<b onclick=1>x</b><i>y</i>', { sanitizer: { removeElements: ["i"] } }, true),
    replaceBody: parsed('<p>a</p>', { sanitizer: { replaceWithChildrenElements: ["body"] } }, true),
    removeHead: parsed('<title>t</title><p>a</p>', { sanitizer: { removeElements: ["head"] } }, true),
    frameset: parsed('<frameset><frame src=x></frameset>', { sanitizer: {} }, true),
    unsafeNone: parsed('<!--a--><!doctype html><html><body><script>x</script><b onclick=1>x</b></body></html>', undefined, false),
    unsafeDefault: parsed('<!doctype html><html><head><title>t</title><style>s</style></head><body><b onclick=1>x</b><!--c--></body></html>', { sanitizer: "default" }, false),
    unsafeElements: parsed('<b onclick=1>x</b><i>y</i><!--c-->', { sanitizer: { elements: ["html", "head", "body", "b"] } }, false),
    unsafeNoComments: parsed('<!--a--><!doctype html><b>x</b><!--b-->', { sanitizer: { comments: false } }, false),
    trustedLikeObject: parsed({ toString: () => "<p>obj</p>" }, undefined, false),
    scriptsInert: run(() => {
      window.__sanitizerRan = 0;
      const parsedDocument = Document.parseHTMLUnsafe("<body><script>window.__sanitizerRan++</script>");
      const script = parsedDocument.querySelector("script");
      document.body.append(document.adoptNode(script));
      script.remove();
      return [window.__sanitizerRan, parsedDocument.scripts.length, script.ownerDocument === document];
    }),
  };
  return out;
})()
