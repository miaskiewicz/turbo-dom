(async () => {
  const getter = Object.getOwnPropertyDescriptor(Element.prototype, "customElementRegistry").get;
  const local = customElements,
    scoped = new CustomElementRegistry();
  const registries = new Map([
    [local, "global"],
    [scoped, "scoped"],
    [null, "null"],
  ]);
  const label = (node) => registries.get(getter.call(node)) || "other";
  const rows = [];
  const check = (name, run) => {
    try {
      rows.push([name, run()]);
    } catch (error) {
      rows.push([name, { error: error.name }]);
    }
  };
  check("document-element", () => label(document.documentElement));
  check("detached-html", () => label(document.createElement("span")));
  check("detached-svg", () => label(document.createElementNS("http://www.w3.org/2000/svg", "svg")));
  const globalNode = document.createElement("x-registry-global");
  const scopedNode = document.createElement("x-registry-scoped", { customElementRegistry: scoped });
  const nullNode = document.createElement("x-registry-null", { customElementRegistry: null });
  check("explicit-scoped", () => label(scopedNode));
  check("explicit-null", () => label(nullNode));
  const parser = document.createElement("div");
  parser.innerHTML = "<x-registry-parser></x-registry-parser>";
  check("detached-parser", () => label(parser.firstChild));
  const host = document.createElement("div");
  document.body.append(host);
  const shadow = host.attachShadow({ mode: "closed", customElementRegistry: scoped });
  shadow.innerHTML = "<x-registry-inner></x-registry-inner>";
  const shadowParsed = shadow.firstChild;
  check("scoped-shadow-parser", () => label(shadowParsed));
  document.body.append(shadowParsed);
  check("scoped-parser-moved-global", () => label(shadowParsed));
  shadow.append(globalNode, scopedNode, nullNode);
  check("global-moved-scoped", () => label(globalNode));
  check("scoped-moved-scoped", () => label(scopedNode));
  check("null-moved-scoped", () => label(nullNode));
  document.body.append(scopedNode);
  check("scoped-moved-global", () => label(scopedNode));
  const unreadHost = document.createElement("div");
  document.body.append(unreadHost);
  const unreadShadow = unreadHost.attachShadow({ mode: "closed", customElementRegistry: scoped });
  unreadShadow.innerHTML = "<x-registry-unread><span></span></x-registry-unread>";
  const unread = unreadShadow.firstChild;
  document.body.append(unread);
  check("unread-scoped-parser-moved-global", () => label(unread));
  check("unread-scoped-descendant", () => label(unread.firstChild));
  const nullHost = document.createElement("div");
  document.body.append(nullHost);
  const nullShadow = nullHost.attachShadow({ mode: "closed", customElementRegistry: null });
  nullShadow.innerHTML = "<x-registry-null-parser></x-registry-null-parser>";
  const nullParsed = nullShadow.firstChild;
  document.body.append(nullParsed);
  check("unread-null-parser-moved-global", () => label(nullParsed));
  unreadShadow.innerHTML = "<section></section>";
  unreadShadow.firstChild.insertAdjacentHTML(
    "beforeend",
    "<x-registry-adjacent></x-registry-adjacent>",
  );
  const adjacent = unreadShadow.firstChild.firstChild;
  document.body.append(adjacent);
  check("unread-scoped-adjacent-moved-global", () => label(adjacent));
  const range = document.createRange();
  range.selectNodeContents(unreadShadow);
  const fragment = range.createContextualFragment("<x-registry-range></x-registry-range>");
  check("scoped-contextual-fragment", () => label(fragment.firstChild));
  const globalHost = document.createElement("div");
  document.body.append(globalHost);
  const globalShadow = globalHost.attachShadow({ mode: "closed" });
  check("global-shadow-root", () => globalShadow.customElementRegistry === local);
  globalShadow.innerHTML = "<x-registry-global-shadow></x-registry-global-shadow>";
  check("global-shadow-parser", () => label(globalShadow.firstChild));
  const template = document.createElement("template", { customElementRegistry: scoped });
  unreadShadow.append(template);
  template.innerHTML = "<x-registry-template><span></span></x-registry-template>";
  check("scoped-template", () => label(template));
  const templateNode = template.content.firstChild;
  check("template-content-parser", () => label(templateNode));
  document.body.append(templateNode);
  check("template-content-moved-global", () => label(templateNode));
  check("template-descendant-moved-global", () => label(templateNode.firstChild));
  const inert = document.implementation.createHTMLDocument("");
  check("inert-document-element", () => label(inert.documentElement));
  check("inert-created", () => label(inert.createElement("x-registry-inert")));
  const inertCreated = inert.createElement("x-registry-inert-created"),
    inertExplicitNull = inert.createElement("x-registry-inert-null", {
      customElementRegistry: null,
    });
  document.adoptNode(inertCreated);
  document.adoptNode(inertExplicitNull);
  check("default-inert-created-adopted-global", () => label(inertCreated));
  check("explicit-null-inert-created-adopted-global", () => label(inertExplicitNull));
  inert.adoptNode(globalNode);
  check("global-adopted-inert", () => label(globalNode));
  check("scoped-adopted-inert", () => label(inert.adoptNode(scopedNode)));
  check("null-adopted-inert", () => label(inert.adoptNode(nullNode)));
  document.adoptNode(nullNode);
  check("null-inert-roundtrip-global", () => label(nullNode));
  const unreadNull = document.createElement("section", { customElementRegistry: null });
  unreadNull.append(document.createElement("span", { customElementRegistry: null }));
  inert.adoptNode(unreadNull);
  document.adoptNode(unreadNull);
  check("unread-null-roundtrip-root", () => label(unreadNull));
  check("unread-null-roundtrip-descendant", () => label(unreadNull.firstChild));
  const implicitNull = inert.createElement("section", { customElementRegistry: null });
  document.body.append(implicitNull);
  check("implicit-inert-null-adoption", () => label(implicitNull));
  const inertHost = inert.createElement("div");
  const inertShadow = inertHost.attachShadow({ mode: "closed", customElementRegistry: null });
  inertShadow.innerHTML = "<x-registry-inert-shadow><span></span></x-registry-inert-shadow>";
  document.adoptNode(inertHost);
  check("inert-shadow-root-adopted-global", () => inertShadow.customElementRegistry === local);
  check("unread-inert-shadow-child-adopted-global", () => label(inertShadow.firstChild));
  check("unread-inert-shadow-descendant-adopted-global", () =>
    label(inertShadow.firstChild.firstChild),
  );
  let frame;
  if (!globalThis.child) {
    frame = document.createElement("iframe");
    document.body.append(frame);
    globalThis.child = frame.contentWindow;
  }
  child.eval("void 0");
  const foreign = child;
  registries.set(foreign.customElements, "foreign");
  check("explicit-foreign-global", () =>
    label(
      document.createElement("x-registry-foreign-option", {
        customElementRegistry: foreign.customElements,
      }),
    ),
  );
  const foreignScoped = foreign.eval("new CustomElementRegistry()");
  registries.set(foreignScoped, "foreign-scoped");
  check("explicit-foreign-scoped", () =>
    label(
      document.createElement("x-registry-foreign-scoped", { customElementRegistry: foreignScoped }),
    ),
  );
  const other = foreign.document.createElement("x-registry-foreign");
  check("foreign-parent-getter", () => label(other));
  document.adoptNode(other);
  check("foreign-adopted-parent", () => label(other));
  const foreignGetter = Object.getOwnPropertyDescriptor(
    foreign.Element.prototype,
    "customElementRegistry",
  ).get;
  check("foreign-getter-on-parent", () => foreignGetter.call(scopedNode) === scoped);
  check("forged-object", () => getter.call({}));
  check("document-receiver", () => getter.call(document));
  const descriptor = Object.getOwnPropertyDescriptor(Element.prototype, "customElementRegistry");
  const shape = {
    enumerable: descriptor.enumerable,
    configurable: descriptor.configurable,
    setter: typeof descriptor.set,
    getterName: getter.name,
    getterLength: getter.length,
    constructible: (() => {
      try {
        Reflect.construct(getter, []);
        return true;
      } catch {
        return false;
      }
    })(),
  };
  templateNode.remove();
  globalHost.remove();
  unread.remove();
  nullParsed.remove();
  adjacent.remove();
  unreadHost.remove();
  nullHost.remove();
  if (frame) frame.remove();
  host.remove();
  scopedNode.remove();
  shadowParsed.remove();
  return { shape, rows };
})();
