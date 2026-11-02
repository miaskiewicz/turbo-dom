(async () => {
  const out = {};
  const run = (fn) => {
    try {
      const value = fn();
      return { value: value === undefined ? "<undefined>" : value };
    } catch (error) {
      return { error: error.name, message: error.message };
    }
  };
  const flags = (root) => root && [root.mode, root.clonable, root.serializable, root.delegatesFocus, root.slotAssignment, root.innerHTML, root.host.localName];
  const ids = ["open", "closed", "flags", "upper", "bad", "badhost", "twice", "nested", "script", "custom", "later", "cell", "manual", "manualUpper", "named", "para", "withform"];
  // Main-document parse: declarative roots attach where the parser runs.
  out.document = {};
  for (const id of ids) {
    const element = document.getElementById(id);
    out.document[id] = { shadow: flags(element.shadowRoot), html: element.innerHTML, serialized: element.getHTML({ serializableShadowRoots: true }) };
  }
  out.documentState = {
    templates: document.querySelectorAll("template").length,
    scripts: document.scripts.length,
    inHead: run(() => [document.getElementById("inhead") && document.getElementById("inhead").localName, document.head.innerHTML]),
    plainTemplate: run(() => {
      const contents = document.getElementById("plain").content;
      const host = contents.getElementById("intpl");
      return [flags(host.shadowRoot), host.innerHTML, contents.ownerDocument === document];
    }),
    nestedInner: run(() => flags(document.getElementById("nested").shadowRoot.getElementById("inner").shadowRoot)),
    shadowScript: run(() => {
      const script = document.getElementById("script").shadowRoot.querySelector("script");
      return [script.textContent, script.getRootNode() instanceof ShadowRoot, script.isConnected];
    }),
    shadowForm: run(() => {
      const root = document.getElementById("withform").shadowRoot;
      const field = root.getElementById("field");
      return [field.value, field.isConnected, root.styleSheets.length, document.getElementById("field")];
    }),
    closedReattach: run(() => {
      const root = document.getElementById("closed").attachShadow({ mode: "closed" });
      return [root.innerHTML, root.mode, root.childNodes.length];
    }),
    flagsMismatch: run(() => document.getElementById("flags").attachShadow({ mode: "open" }).mode),
    openReattach: run(() => {
      const host = document.getElementById("open");
      const root = host.attachShadow({ mode: "open" });
      return [root === host.shadowRoot, root.innerHTML, flags(root)];
    }),
    openReattachTwice: run(() => document.getElementById("open").attachShadow({ mode: "open" }).innerHTML),
    clone: run(() => document.getElementById("flags").cloneNode(true).getHTML({ serializableShadowRoots: true })),
    cloneClonableOpen: run(() => {
      const copy = document.getElementById("later").cloneNode(true);
      return [flags(copy.shadowRoot), copy.innerHTML];
    }),
    cloneUnclonable: run(() => {
      const copy = document.getElementById("twice").cloneNode(true);
      return [copy.shadowRoot, copy.innerHTML];
    }),
    slotted: run(() => {
      const root = document.getElementById("open").shadowRoot;
      return root ? root.querySelector("slot").assignedNodes().length : null;
    }),
  };
  // Fragment-parsing entry points never attach declarative roots.
  const markup = '<div id="h"><template shadowrootmode="open"><b>x</b></template></div>';
  const host = document.createElement("section");
  document.body.append(host);
  out.fragments = {
    innerHTML: run(() => {
      host.innerHTML = markup;
      return [host.firstChild.shadowRoot, host.innerHTML];
    }),
    outerHTML: run(() => {
      host.innerHTML = "<p></p>";
      host.firstChild.outerHTML = markup;
      return [host.firstChild.shadowRoot, host.innerHTML];
    }),
    insertAdjacentHTML: run(() => {
      host.textContent = "";
      host.insertAdjacentHTML("beforeend", markup);
      return [host.firstChild.shadowRoot, host.innerHTML];
    }),
    createContextualFragment: run(() => {
      const range = document.createRange();
      range.selectNodeContents(host);
      const fragment = range.createContextualFragment(markup);
      return [fragment.firstChild.shadowRoot, fragment.firstChild.innerHTML];
    }),
    templateInnerHTML: run(() => {
      const template = document.createElement("template");
      template.innerHTML = markup;
      return [template.content.firstChild.shadowRoot, template.innerHTML];
    }),
    createdDocument: run(() => {
      const created = document.implementation.createHTMLDocument("");
      created.body.innerHTML = markup;
      return [created.body.firstChild.shadowRoot, created.body.innerHTML];
    }),
    setHTMLUnsafe: run(() => {
      host.setHTMLUnsafe(markup);
      return [flags(host.firstChild.shadowRoot), host.innerHTML];
    }),
  };
  // Document.parseHTMLUnsafe / parseHTML parse with declarative roots allowed.
  const parsed = (source, safe, options) =>
    run(() => {
      const result = safe ? Document.parseHTML(source, options) : Document.parseHTMLUnsafe(source, options);
      const shadows = [];
      const walk = (root) => {
        for (const element of root.querySelectorAll("*")) {
          const shadowRoot = element.shadowRoot;
          if (shadowRoot) {
            shadows.push([element.id || element.localName, ...flags(shadowRoot)]);
            walk(shadowRoot);
          }
        }
      };
      walk(result);
      return [result.body ? result.body.getHTML({ serializableShadowRoots: true }) : null, shadows, result.querySelectorAll("template").length];
    });
  const page = document.documentElement.outerHTML;
  out.parsed = {
    unsafeMatrix: parsed('<div id="a"><template shadowrootmode="open"><b>a</b><slot></slot></template>light</div><div id="b"><template shadowrootmode="closed" shadowrootserializable shadowrootclonable shadowrootdelegatesfocus><b>b</b></template></div><div id="c"><template shadowrootmode="open"><span id="c2"><template shadowrootmode="open" shadowrootserializable><i>deep</i></template></span></template></div><a id="d"><template shadowrootmode="open">no</template></a><div id="e"><template shadowrootmode="open">1</template><template shadowrootmode="open">2</template></div><div id="f"><template shadowrootmode="open" shadowrootslotassignment="manual"><slot></slot></template></div>', false),
    unsafeBody: parsed('<body><template shadowrootmode="open"><b>body root</b></template>text</body>', false),
    unsafeHead: parsed('<head><template shadowrootmode="open"><b>head</b></template></head><body>x</body>', false),
    unsafeTemplateContents: parsed('<template id="t"><div><template shadowrootmode="open"><b>in contents</b></template></div></template>', false),
    unsafeSanitized: parsed('<div id="s"><template shadowrootmode="open" shadowrootserializable><b onclick="x">s</b><script>y</script><i>i</i></template></div>', false, { sanitizer: { removeElements: ["i"] } }),
    safeDefault: parsed('<div id="s"><template shadowrootmode="open"><b onclick="x">s</b></template></div>', true),
    safeEmpty: parsed('<div id="s"><template shadowrootmode="open" shadowrootserializable><b onclick="x">s</b><script>y</script><!--c--></template></div>', true, { sanitizer: {} }),
    pageReparse: parsed(page, false),
  };
  out.parsedState = {
    closedReattach: run(() => {
      const result = Document.parseHTMLUnsafe('<div id="h"><template shadowrootmode="closed"><b>x</b></template></div>');
      const element = result.getElementById("h");
      const root = element.attachShadow({ mode: "closed" });
      return [root.innerHTML, element.shadowRoot];
    }),
    mismatch: run(() => Document.parseHTMLUnsafe('<div id="h"><template shadowrootmode="closed"><b>x</b></template></div>').getElementById("h").attachShadow({ mode: "open" })),
    adopted: run(() => {
      const result = Document.parseHTMLUnsafe('<div id="h"><template shadowrootmode="open" shadowrootclonable><b>x</b></template></div>');
      const element = document.adoptNode(result.getElementById("h"));
      host.replaceChildren(element);
      return [flags(element.shadowRoot), element.shadowRoot.ownerDocument === document, element.cloneNode(true).shadowRoot !== null];
    }),
  };
  return out;
})()
