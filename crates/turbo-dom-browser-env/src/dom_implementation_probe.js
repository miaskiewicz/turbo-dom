(() => {
  const impl = document.implementation;
  const output = {
    stable: impl === document.implementation,
    brand: Object.prototype.toString.call(impl),
    prototype: Object.getPrototypeOf(impl) === DOMImplementation.prototype,
    keys: Reflect.ownKeys(impl).map(String),
    methods: ["createDocumentType", "createDocument", "createHTMLDocument", "hasFeature"].map((name) => [name, impl[name].length, impl[name].name, Function.prototype.toString.call(impl[name])]),
    html: [], xml: [], errors: [],
  };
  const summary = (doc) => ({
    tag: Object.prototype.toString.call(doc),
    prototype: Object.getPrototypeOf(doc) === (doc.contentType === "text/html" ? HTMLDocument.prototype : XMLDocument.prototype),
    independent: doc !== document && doc.documentElement?.ownerDocument === doc,
    contentType: doc.contentType, url: doc.URL, view: doc.defaultView === null,
    root: doc.documentElement ? [doc.documentElement.nodeName, doc.documentElement.localName, doc.documentElement.prefix, doc.documentElement.namespaceURI] : null,
    doctype: doc.doctype ? [doc.doctype.name, doc.doctype.publicId, doc.doctype.systemId, doc.doctype.ownerDocument === doc] : null,
    head: doc.head ? doc.head.innerHTML : null,
    body: doc.body ? doc.body.innerHTML : null,
  });
  for (const args of [[], [undefined], [null], [""], [" a  b\n c "], ["<b>safe&</b>"]]) {
    const doc = impl.createHTMLDocument(...args);
    output.html.push(summary(doc));
  }
  for (const [ns, name] of [[null, ""], [null, "Camel"], ["urn:fixture", "p:Root"], ["http://www.w3.org/1999/xhtml", "html"], ["http://www.w3.org/2000/svg", "svg"]]) {
    const dt = impl.createDocumentType("fixture", "PUBLIC-id", "system-id");
    const doc = impl.createDocument(ns, name, dt);
    output.xml.push({ ...summary(doc), identity: dt === doc.doctype });
  }
  output.constructors = [];
  for (const Constructor of [Document, XMLDocument, HTMLDocument]) {
    try { const doc = new Constructor(); output.constructors.push({ result: summary(doc) }); }
    catch (error) { output.constructors.push({ error: error.name }); }
  }
  output.hasFeature = impl.hasFeature() && impl.hasFeature(Symbol("ignored"), { toString() { throw Error("must not coerce"); } });
  for (const operation of [
    () => new DOMImplementation(),
    () => DOMImplementation.prototype.hasFeature.call({}),
    () => DOMImplementation.prototype.createHTMLDocument.call({}),
    () => impl.createHTMLDocument(Symbol("bad")),
    () => impl.createDocumentType(),
    () => impl.createDocumentType("", "", ""),
    () => impl.createDocumentType("bad name", "", ""),
    () => impl.createDocumentType("valid", Symbol("bad"), ""),
    () => impl.createDocument(),
    () => impl.createDocument(null, "p:Root"),
    () => impl.createDocument("urn:fixture", "xml:Root"),
    () => impl.createDocument("urn:fixture", "xmlns"),
    () => impl.createDocument("urn:fixture", "bad name"),
    () => impl.createDocument(null, "", {}),
  ]) {try {operation(); output.errors.push(null);} catch(error) {output.errors.push(error.name);}}
  const retained = impl.createHTMLDocument("retained");
  retained.body.innerHTML = '<div id="native">real</div>';
  output.retained = retained.getElementById("native").ownerDocument === retained && retained.title === "retained" && retained.implementation !== impl;
  const sheet = new CSSStyleSheet();
  try { retained.adoptedStyleSheets = [sheet]; output.foreignSheet = null; }
  catch(error) { output.foreignSheet = error.name; }
  const define = Object.defineProperty, get = Object.getOwnPropertyDescriptor, apply = Reflect.apply;
  const intrinsics = [[RegExp.prototype, "test"], [RegExp.prototype, "exec"], [String.prototype, "split"], [String.prototype, "includes"], [Array.prototype, "some"], [globalThis, "Map"], [Object, "defineProperty"]];
  const descriptors = [];
  for (let index = 0; index < intrinsics.length; index++) define(descriptors, index, { value: get(intrinsics[index][0], intrinsics[index][1]), configurable: true });
  let hooks = 0, protectedHTML, protectedXML, protectedType, failure;
  const poison = function () { hooks++; throw Error("unexpected guest intrinsic"); };
  for (let index = 0; index < intrinsics.length; index++) define(intrinsics[index][0], intrinsics[index][1], { value: poison, writable: true, configurable: true });
  try {
    protectedHTML = impl.createHTMLDocument("protected");
    protectedType = impl.createDocumentType("fixture", "", "");
    protectedXML = impl.createDocument("urn:fixture", "p:Root", protectedType);
  } catch (error) { failure = error.name; }
  finally { for (let index = 0; index < intrinsics.length; index++) apply(define, Object, [intrinsics[index][0], intrinsics[index][1], descriptors[index]]); }
  output.intrinsicImmunity = { hooks, created: !!protectedHTML && !!protectedXML, failed: failure || null, title: protectedHTML?.title, owner: protectedXML?.documentElement?.ownerDocument === protectedXML, identity: protectedXML?.doctype === protectedType };
  {
    const fragment = document.createDocumentFragment(), first = document.createElement('div'), second = document.createElement('span');
    first.id = second.id = 'duplicate'; fragment.append(first, second);
    const fn = DocumentFragment.prototype.getElementById, descriptor = Object.getOwnPropertyDescriptor(DocumentFragment.prototype, 'getElementById');
    const attempt = callback => { try { return callback(); } catch (error) { return error.name; } };
    output.fragmentId = {
      descriptor: [fn.name, fn.length, descriptor.writable, descriptor.enumerable, descriptor.configurable, Object.hasOwn(fn, 'prototype'), Function.prototype.toString.call(fn).includes('[native code]')],
      order: [fragment.getElementById('duplicate') === first, fragment.getElementById('missing') === null, fragment.getElementById('') === null],
      errors: [attempt(() => fn.call({})), attempt(() => fn.call(document, 'duplicate')), attempt(() => fn.call(fragment)), attempt(() => fn.call(fragment, Symbol()))]
    };
    first.id = '\ud800'; second.id = ''; output.fragmentId.units = [fragment.getElementById('\ud800') === first, fragment.getElementById('') === null];
    const originals = [DocumentFragment.prototype.querySelectorAll, Element.prototype.getAttribute, Array.prototype.includes, globalThis.TypeError];
    let idHooks = 0, idValue, idFailure;
    const idPoison = () => { idHooks++; throw Error('unexpected ID lookup hook'); };
    DocumentFragment.prototype.querySelectorAll = Element.prototype.getAttribute = Array.prototype.includes = globalThis.TypeError = idPoison;
    try { idValue = fn.call(fragment, '\ud800') === first; try { fn.call(fragment); } catch (error) { idFailure = error.name; } }
    finally { [DocumentFragment.prototype.querySelectorAll, Element.prototype.getAttribute, Array.prototype.includes, globalThis.TypeError] = originals; }
    output.fragmentId.intrinsics = [idHooks, idValue, idFailure];
    const host = document.createElement('div'); host.id = 'host'; document.body.append(host);
    const root = host.attachShadow({mode: 'closed'}), inner = document.createElement('b'); inner.id = 'nested'; root.append(inner);
    const nestedHost = document.createElement('div'); root.append(nestedHost); const nested = nestedHost.attachShadow({mode:'closed'}); nested.innerHTML = '<b id="leak"></b>';
    output.fragmentId.shadow = [root.getElementById('nested') === inner, root.getElementById('host') === null, root.getElementById('leak') === null, fn.call(root, 'nested') === inner];
    inner.id = 'changed'; output.fragmentId.changed = [root.getElementById('nested') === null, root.getElementById('changed') === inner];
    host.remove();
  }
  return output;
})();
