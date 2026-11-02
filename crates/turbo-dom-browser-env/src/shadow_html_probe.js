(async () => {
  const out = {};
  const settle = async () => {
    for (let i = 0; i < 4; i++) await Promise.resolve();
  };
  const run = (fn) => {
    try {
      return { value: fn() };
    } catch (error) {
      return { error: error.name, message: error.message };
    }
  };
  const named = (fn) => {
    try {
      return { value: fn() };
    } catch (error) {
      return { error: error.name };
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
    return row;
  };
  const rootMembers = ["activeElement", "clonable", "delegatesFocus", "fullscreenElement", "getHTML", "onslotchange", "pictureInPictureElement", "pointerLockElement", "serializable", "setHTMLUnsafe", "styleSheets"];
  out.descriptors = {
    ShadowRoot: Object.fromEntries(rootMembers.map((key) => [key, describe(ShadowRoot.prototype, key)])),
    Element: Object.fromEntries(["getHTML", "setHTMLUnsafe"].map((key) => [key, describe(Element.prototype, key)])),
  };
  const fragment = document.createDocumentFragment();
  const plain = document.createElement("div");
  out.brands = {};
  for (const key of rootMembers) {
    const descriptor = Object.getOwnPropertyDescriptor(ShadowRoot.prototype, key);
    const call = descriptor.get || descriptor.value;
    out.brands[key] = [fragment, plain, {}, undefined].map((receiver) => named(() => call.call(receiver, "")));
  }
  for (const key of ["getHTML", "setHTMLUnsafe"])
    out.brands["Element." + key] = [fragment, {}, document].map((receiver) => named(() => Element.prototype[key].call(receiver, "")));
  out.brands.setters = ["fullscreenElement", "onslotchange"].map((key) => named(() => Object.getOwnPropertyDescriptor(ShadowRoot.prototype, key).set.call(fragment, null)));

  const flags = (root) => [root.mode, root.clonable, root.serializable, root.delegatesFocus, root.slotAssignment];
  out.init = [
    { mode: "open" },
    { mode: "closed", clonable: true, serializable: true, delegatesFocus: true, slotAssignment: "manual" },
    { mode: "open", clonable: 1, serializable: "", delegatesFocus: "x" },
    { mode: "open", clonable: {}, serializable: 0, delegatesFocus: null },
  ].map((init) => flags(document.createElement("section").attachShadow(init)));

  const host = document.createElement("div");
  document.body.append(host);
  const root = host.attachShadow({ mode: "open", serializable: true, delegatesFocus: true, clonable: true });
  root.innerHTML = "<span>in<b>x</b></span><slot></slot>";
  host.innerHTML = "<p>light &amp; <i>shade</i></p>";
  const inner = root.firstChild.attachShadow({ mode: "closed" });
  inner.innerHTML = "<i>closed</i><style>i{color:red}</style>";
  const other = document.createElement("div").attachShadow({ mode: "closed" });
  out.getHTML = {
    plain: host.getHTML(),
    serializable: host.getHTML({ serializableShadowRoots: true }),
    truthy: host.getHTML({ serializableShadowRoots: 1 }),
    falsy: host.getHTML({ serializableShadowRoots: "" }),
    listed: host.getHTML({ shadowRoots: [root] }),
    listedClosed: host.getHTML({ serializableShadowRoots: true, shadowRoots: [inner] }),
    listedOnlyInner: host.getHTML({ shadowRoots: [inner] }),
    unrelated: host.getHTML({ shadowRoots: new Set([other, other]) }),
    root: root.getHTML(),
    rootSerializable: root.getHTML({ shadowRoots: [inner] }),
    innerHTML: host.innerHTML,
    nullOptions: host.getHTML(null),
    template: (() => {
      const template = document.createElement("template");
      template.innerHTML = "<b>t</b>";
      return [template.getHTML(), template.getHTML({ serializableShadowRoots: true })];
    })(),
    void: (() => {
      const image = document.createElement("img");
      image.append("x");
      return [document.createElement("br").getHTML(), image.getHTML(), image.childNodes.length];
    })(),
  };
  out.getHTMLErrors = [5, "x", { shadowRoots: 5 }, { shadowRoots: "ab" }, { shadowRoots: [{}] }, { shadowRoots: [fragment] }, { shadowRoots: [root, host] }].map((value) => run(() => host.getHTML(value)));
  out.getHTMLRootErrors = [run(() => root.getHTML(7))];
  const reads = [];
  host.getHTML(new Proxy({}, { get: (target, key) => (reads.push(String(key)), undefined) }));
  out.getHTMLReads = reads;

  const parsed = document.createElement("div");
  document.body.append(parsed);
  const records = [];
  const observer = new MutationObserver((list) => {
    for (const record of list) records.push([record.type, record.addedNodes.length, record.removedNodes.length]);
  });
  parsed.innerHTML = "<em>old</em><em>old</em>";
  await settle();
  observer.observe(parsed, { childList: true, subtree: true });
  parsed.setHTMLUnsafe(
    '<div id=a><template shadowrootmode=open shadowrootclonable shadowrootserializable shadowrootdelegatesfocus><b>s</b><span><template shadowrootmode=CLOSED shadowrootserializable="false">deep</template></span></template>tail</div><template shadowrootmode=open>top-level</template>',
  );
  await settle();
  observer.disconnect();
  const declared = parsed.querySelector("#a");
  const deep = declared.shadowRoot.querySelector("span");
  out.setHTMLUnsafe = {
    records,
    innerHTML: parsed.innerHTML,
    declared: flags(declared.shadowRoot),
    declaredHTML: declared.shadowRoot.innerHTML,
    deepClosed: deep.shadowRoot,
    serialized: parsed.getHTML({ serializableShadowRoots: true }),
    contextRoot: parsed.shadowRoot,
    topLevel: parsed.lastChild.localName + ":" + parsed.lastChild.content.textContent,
  };
  const duplicate = document.createElement("div");
  duplicate.setHTMLUnsafe(
    "<div><template shadowrootmode=open>one</template><template shadowrootmode=open>two</template><template shadowrootmode=closed>three</template></div><p><template shadowrootmode=bogus>b</template></p><input><template shadowrootmode=open>void</template><a><template shadowrootmode=open>anchor</template></a><x-host><template shadowrootmode=open>custom</template></x-host>",
  );
  out.duplicates = [duplicate.innerHTML, duplicate.firstChild.shadowRoot.innerHTML, duplicate.querySelector("x-host").shadowRoot.innerHTML];
  const reuse = document.createElement("div");
  reuse.setHTMLUnsafe("<span><template shadowrootmode=open shadowrootclonable>decl</template></span>");
  const declaredHost = reuse.firstChild,
    declaredRoot = declaredHost.shadowRoot;
  out.reattach = [
    run(() => declaredHost.attachShadow({ mode: "closed" })),
    (() => {
      const again = declaredHost.attachShadow({ mode: "open", serializable: true });
      return [again === declaredRoot, again.innerHTML, flags(again)];
    })(),
    run(() => declaredHost.attachShadow({ mode: "open" })),
  ];
  const inTemplate = document.createElement("div");
  inTemplate.setHTMLUnsafe("<template><div><template shadowrootmode=open>inside</template></div></template>");
  out.templateContent = [inTemplate.innerHTML, inTemplate.firstChild.content.firstChild.shadowRoot.innerHTML];
  const template = document.createElement("template");
  template.setHTMLUnsafe("<div><template shadowrootmode=open>tt</template></div>");
  out.templateTarget = [template.childNodes.length, template.content.childNodes.length, template.content.firstChild.shadowRoot.innerHTML, template.innerHTML];
  const rootHost = document.createElement("div");
  const rootTarget = rootHost.attachShadow({ mode: "open", serializable: true });
  rootTarget.innerHTML = "<i>gone</i>";
  rootTarget.setHTMLUnsafe("<span><template shadowrootmode=open shadowrootserializable>nested</template></span><td>cell</td>");
  out.rootTarget = [rootTarget.innerHTML, rootHost.getHTML({ serializableShadowRoots: true })];
  out.setHTMLUnsafeArgs = [
    run(() => parsed.setHTMLUnsafe()),
    run(() => rootTarget.setHTMLUnsafe()),
    run(() => (parsed.setHTMLUnsafe(null), parsed.innerHTML)),
    run(() => (parsed.setHTMLUnsafe(undefined), parsed.innerHTML)),
    run(() => (parsed.setHTMLUnsafe({ toString: () => "<b>object</b>" }), parsed.innerHTML)),
    named(() => parsed.setHTMLUnsafe(Symbol("x"))),
  ];
  const innerHTMLHost = document.createElement("div");
  innerHTMLHost.innerHTML = "<div><template shadowrootmode=open>x</template></div>";
  out.innerHTMLIgnoresDeclarative = [innerHTMLHost.innerHTML, innerHTMLHost.firstChild.shadowRoot];

  const closedHost = document.createElement("div");
  const closedRoot = closedHost.attachShadow({ mode: "closed", clonable: true, delegatesFocus: true, slotAssignment: "manual" });
  closedRoot.innerHTML = "<slot></slot><b>c</b>";
  const closedCopy = closedHost.cloneNode();
  out.clone = {
    closed: [closedCopy.shadowRoot, run(() => closedCopy.attachShadow({ mode: "closed" }))],
    shallow: (() => {
      const copy = host.cloneNode(false);
      return [copy.childNodes.length, copy.shadowRoot.innerHTML, flags(copy.shadowRoot), copy.shadowRoot === root];
    })(),
    deep: (() => {
      const copy = host.cloneNode(true);
      return [copy.innerHTML, copy.shadowRoot.innerHTML, copy.shadowRoot.firstChild.shadowRoot];
    })(),
    declarative: (() => {
      const copy = reuse.cloneNode(true);
      const copyHost = copy.firstChild;
      return [copyHost.shadowRoot.innerHTML, copyHost.shadowRoot.clonable, run(() => copyHost.attachShadow({ mode: "open" }))];
    })(),
    declarativeFresh: (() => {
      const source = document.createElement("div");
      source.setHTMLUnsafe("<span><template shadowrootmode=open shadowrootclonable>fresh</template></span>");
      const copyHost = source.cloneNode(true).firstChild;
      const again = copyHost.attachShadow({ mode: "open" });
      return [again === copyHost.shadowRoot, again.innerHTML, run(() => copyHost.attachShadow({ mode: "open" }))];
    })(),
    nonClonable: (() => {
      const source = document.createElement("div");
      source.attachShadow({ mode: "open" }).innerHTML = "x";
      return source.cloneNode(true).shadowRoot;
    })(),
    nested: (() => {
      const outer = document.createElement("section");
      const nestHost = document.createElement("div");
      outer.append(nestHost);
      const first = nestHost.attachShadow({ mode: "open", clonable: true });
      first.innerHTML = "<p>1</p>";
      first.firstChild.attachShadow({ mode: "open", clonable: true, serializable: true }).innerHTML = "<i>2</i>";
      const copy = outer.cloneNode(true);
      const foreign = document.implementation.createHTMLDocument("");
      const imported = foreign.importNode(nestHost, true);
      const adopted = document.importNode(nestHost, false);
      return [
        copy.firstChild.shadowRoot.firstChild.shadowRoot.innerHTML,
        copy.getHTML({ serializableShadowRoots: true }),
        outer.cloneNode(false).childNodes.length,
        imported.ownerDocument === foreign,
        imported.shadowRoot.firstChild.ownerDocument === foreign,
        imported.shadowRoot.firstChild.shadowRoot.innerHTML,
        adopted.shadowRoot.innerHTML,
      ];
    })(),
    manual: (() => {
      const source = document.createElement("div");
      source.attachShadow({ mode: "open", clonable: true, slotAssignment: "manual", serializable: true }).innerHTML = "<slot></slot>";
      return flags(source.cloneNode().shadowRoot);
    })(),
  };

  const focusHost = document.createElement("div");
  document.body.append(focusHost);
  const focusRoot = focusHost.attachShadow({ mode: "open" });
  focusRoot.innerHTML = "<input id=a><p><button>b</button></p>";
  const deepRoot = focusRoot.querySelector("p").attachShadow({ mode: "closed" });
  deepRoot.innerHTML = "<input id=deep>";
  const unrelatedRoot = document.createElement("div").attachShadow({ mode: "open" });
  const active = () => [focusRoot.activeElement && focusRoot.activeElement.localName, deepRoot.activeElement && deepRoot.activeElement.localName, unrelatedRoot.activeElement, document.activeElement && document.activeElement.localName];
  out.activeElement = [active()];
  focusRoot.querySelector("input").focus();
  out.activeElement.push(active());
  deepRoot.querySelector("input").focus();
  out.activeElement.push(active());
  const light = document.createElement("input");
  document.body.append(light);
  light.focus();
  out.activeElement.push(active());
  light.remove();

  out.fullscreen = [
    focusRoot.fullscreenElement,
    focusRoot.pointerLockElement,
    focusRoot.pictureInPictureElement,
    run(() => {
      focusRoot.fullscreenElement = focusHost;
      return focusRoot.fullscreenElement;
    }),
  ];

  const sheetHost = document.createElement("div");
  const sheetRoot = sheetHost.attachShadow({ mode: "open" });
  sheetRoot.innerHTML = "<style>a{}</style><div><style media=print>b{}</style></div>";
  const list = sheetRoot.styleSheets;
  const sheets = { detached: list.length, documentBefore: document.styleSheets.length };
  document.body.append(sheetHost);
  sheets.connected = [list.length, list === sheetRoot.styleSheets, list[0].ownerNode === sheetRoot.firstChild, list.item(1).media.mediaText, document.styleSheets.length === sheets.documentBefore];
  const added = document.createElement("style");
  added.textContent = "c{}";
  sheetRoot.prepend(added);
  sheets.live = [list.length, list[0].ownerNode === added];
  const nestedSheetHost = document.createElement("p");
  sheetRoot.append(nestedSheetHost);
  nestedSheetHost.attachShadow({ mode: "open" }).innerHTML = "<style>d{}</style>";
  sheets.nested = [list.length, nestedSheetHost.shadowRoot.styleSheets.length];
  sheetRoot.adoptedStyleSheets = [new CSSStyleSheet()];
  sheets.adopted = list.length;
  const link = document.createElement("link");
  link.rel = "stylesheet";
  sheetRoot.append(link);
  sheets.link = [list.length, link.sheet];
  added.remove();
  sheets.removed = list.length;
  sheets.shape = [Object.keys(list), Object.prototype.toString.call(list), run(() => list.item()), [...list].length];
  sheetHost.remove();
  sheets.disconnected = list.length;
  out.styleSheets = sheets;

  const slotHost = document.createElement("div");
  const slotRoot = slotHost.attachShadow({ mode: "open" });
  slotRoot.innerHTML = "<slot></slot>";
  document.body.append(slotHost);
  const seen = [];
  const handler = function (event) {
    seen.push([this === slotRoot, event.type, event.target === slotRoot.firstChild, event.bubbles]);
    return false;
  };
  const slot = { initial: slotRoot.onslotchange };
  slotRoot.onslotchange = handler;
  slot.assigned = slotRoot.onslotchange === handler;
  slotHost.append(document.createElement("span"));
  await settle();
  slot.seen = seen.slice();
  const stored = {};
  slotRoot.onslotchange = stored;
  slot.object = slotRoot.onslotchange === stored;
  slotRoot.onslotchange = 5;
  slot.primitive = slotRoot.onslotchange;
  slotRoot.onslotchange = handler;
  slotRoot.onslotchange = null;
  slot.cleared = slotRoot.onslotchange;
  slotHost.append(document.createElement("span"));
  await settle();
  slot.afterClear = seen.length;
  slotRoot.firstChild.setAttribute("onslotchange", "void 0");
  slot.attribute = slotRoot.onslotchange;
  out.onslotchange = slot;
  for (const node of [host, parsed, focusHost, slotHost]) node.remove();
  return out;
})()
