(() => {
  const rows = {},
    signals = globalThis.__scriptSignals;
  const source = (name) =>
    "globalThis.__scriptSignals.push([" +
    JSON.stringify(name) +
    ",document.currentScript.id,document.currentScript.ownerDocument===document]);";
  const attach = (script, name) => {
    script.id = name;
    document.body.appendChild(script);
    return signals.slice();
  };
  const created = document.createElement("script");
  rows.created = [created.async, created.hasAttribute("async")];
  created.async = false;
  rows.propertyFalse = [created.async, created.hasAttribute("async")];
  created.setAttribute("async", "");
  rows.attributeTrue = [created.async, created.hasAttribute("async")];
  created.removeAttribute("async");
  rows.attributeRemovedAfterIDL = [created.async, created.hasAttribute("async")];
  const fresh = document.createElement("script");
  fresh.setAttribute("async", "");
  fresh.removeAttribute("async");
  rows.attributeRemovedFresh = [fresh.async, fresh.hasAttribute("async")];
  rows.parser = [
    document.getElementById("parser").async,
    document.getElementById("parser").cloneNode(true).async,
  ];
  const single = document.createElement("script");
  single.textContent = source("single");
  rows.synchronous = attach(single, "single");
  single.remove();
  rows.reinsert = attach(single, "again");
  rows.startedClone = attach(single.cloneNode(true), "started-clone");
  const unstarted = document.createElement("script");
  unstarted.textContent = source("unstarted");
  const clone = unstarted.cloneNode(true);
  rows.unstartedClone = attach(clone, "unstarted-clone");
  rows.unstartedOriginal = attach(unstarted, "unstarted");
  const empty = document.createElement("script");
  document.body.appendChild(empty);
  empty.id = "empty";
  empty.textContent = source("empty");
  rows.emptyThenText = signals.slice();
  const parser = new DOMParser().parseFromString(
    "<script id=parsed>" + source("parsed") + "</script>",
    "text/html",
  );
  const parsed = parser.querySelector("script");
  rows.domParserAsync = [parsed.async, parsed.cloneNode(true).async];
  rows.domParserEmptyAsync = new DOMParser()
    .parseFromString("<script></script>", "text/html")
    .querySelector("script").async;
  rows.domParserDataAsync = new DOMParser()
    .parseFromString("<script type=application/json>0</script>", "text/html")
    .querySelector("script").async;
  rows.domParserAdopt = attach(parsed, "parsed");
  rows.domParserClone = attach(parsed.cloneNode(true), "parsed-clone");
  const inertCreated = parser.createElement("script");
  inertCreated.textContent = source("inert-created");
  parser.head.appendChild(inertCreated);
  rows.inertCreatedBefore = signals.slice();
  rows.inertCreatedAdopt = attach(inertCreated, "inert-created");
  const container = document.createElement("div");
  container.innerHTML = "<script>" + source("innerhtml") + "</script>";
  const fragmentScript = container.firstChild;
  rows.innerHTMLAsync = [fragmentScript.async, fragmentScript.cloneNode(true).async];
  rows.innerHTMLAdopt = attach(fragmentScript, "innerhtml");
  rows.innerHTMLClone = attach(fragmentScript.cloneNode(true), "innerhtml-clone");
  const range = document.createRange();
  range.selectNodeContents(document.body);
  const fragment = range.createContextualFragment("<script>" + source("contextual") + "</script>");
  rows.contextualAsync = [fragment.firstChild.async, fragment.firstChild.cloneNode(true).async];
  fragment.firstChild.id = "contextual";
  document.body.appendChild(fragment);
  rows.contextual = signals.slice();
  const template = document.createElement("template");
  template.innerHTML = "<script>" + source("template") + "</script>";
  rows.templateAsync = [
    template.content.firstChild.async,
    template.content.firstChild.cloneNode(true).async,
  ];
  const templateClone = template.content.cloneNode(true);
  templateClone.firstChild.id = "template-clone";
  document.body.appendChild(templateClone);
  rows.templateClone = signals.slice();
  const originalClone = document.getElementById("parser").cloneNode(true);
  rows.parserClone = attach(originalClone, "parser-clone");
  const absent = document.createElement("script");
  absent.removeAttribute("async");
  rows.removeAbsent = [absent.async, absent.hasAttribute("async")];
  const detachedInert = parser.createElement("script");
  detachedInert.textContent = source("detached-inert");
  rows.detachedInertAdopt = attach(detachedInert, "detached-inert");
  const literal = document.getElementById("literal-template");
  rows.literalTemplateAsync = [
    literal.content.firstChild.async,
    literal.content.firstChild.cloneNode(true).async,
  ];
  const literalClone = literal.content.cloneNode(true);
  literalClone.firstChild.id = "literal-clone";
  document.body.appendChild(literalClone);
  rows.literalTemplateClone = signals.slice();
  const literalImport = document.importNode(literal.content, true);
  literalImport.firstChild.id = "literal-import";
  document.body.appendChild(literalImport);
  rows.literalTemplateImport = signals.slice();
  const getter = Object.getOwnPropertyDescriptor(HTMLScriptElement.prototype, "async").get;
  for (const [name, obj] of [
    ["wrong", document.createElement("div")],
    ["forged", Object.create(HTMLScriptElement.prototype)],
  ]) {
    try {
      getter.call(obj);
      rows[name] = "unexpected";
    } catch (error) {
      rows[name] = error.name;
    }
  }
  rows.mime = [];
  for (const type of [
    "text/javascript",
    "application/ecmascript",
    "application/x-javascript",
    "text/javascript1.0",
    "text/jscript",
    "text/livescript",
    "text/javascript; charset=utf-8",
    " TEXT/JAVASCRIPT ",
    "application/json",
    "\u00a0text/javascript\u00a0",
  ]) {
    const script = document.createElement("script");
    script.type = type;
    script.textContent = source("mime-" + type);
    const before = signals.length;
    document.body.append(script);
    rows.mime.push([type, signals.length === before + 1]);
    script.remove();
  }
  const changing = document.createElement("script");
  changing.type = "application/json";
  changing.textContent = source("changed-type");
  document.body.append(changing);
  const beforeChange = signals.length;
  changing.type = "text/javascript";
  const asyncOriginal = document.createElement("script");
  asyncOriginal.setAttribute("async", "");
  const asyncClone = asyncOriginal.cloneNode(true);
  asyncClone.removeAttribute("async");
  rows.cloneAsyncRemoved = asyncClone.async;
  rows.changedTypeOnly = signals.length - beforeChange;
  changing.firstChild.data = changing.firstChild.data;
  rows.sameCharacterData = signals.length - beforeChange;
  changing.firstChild.data += "\n";
  rows.changedCharacterData = signals.length - beforeChange;
  changing.appendChild(document.createTextNode(""));
  rows.appendEmptyText = signals.length - beforeChange;
  rows.languages = [];
  for (const language of ["JavaScript", "jscript", "javascript1.0", "ecmascript", "wrong"]) {
    const script = document.createElement("script");
    script.setAttribute("language", language);
    script.textContent = source("language-" + language);
    const before = signals.length;
    document.body.append(script);
    rows.languages.push([language, signals.length === before + 1]);
    script.remove();
  }
  const shadowHost = document.createElement("div"),
    shadow = shadowHost.attachShadow({ mode: "open" }),
    shadowScript = document.createElement("script");
  shadowScript.textContent =
    'globalThis.__scriptSignals.push(["shadow",document.currentScript===null,true]);';
  const beforeShadow = signals.length;
  shadow.append(shadowScript);
  rows.detachedShadow = signals.length - beforeShadow;
  document.body.append(shadowHost);
  rows.connectedShadow = signals.slice(beforeShadow);
  const outer = document.createElement("script"),
    inner = document.createElement("script");
  const beforeNested = signals.length;
  inner.textContent = source("inner");
  document.body.append(outer);
  outer.append(
    document.createTextNode(source("outer-first")),
    inner,
    document.createTextNode(source("outer-last")),
  );
  rows.nestedScripts = signals.slice(beforeNested);
  return rows;
})();
