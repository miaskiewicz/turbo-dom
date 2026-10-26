(() => {
  'use strict';
  // miaskiewicz/turbo-dom#29: (1) createElementNS in a namespace other than HTML/SVG/MathML,
  // (2) insertion across documents adopts the node, (3) template contents live in an inert
  // document (no custom element upgrades until inserted into the document), (4) custom
  // element reactions are queued per element and run when the DOM call returns ([CEReactions]).
  // Recorded in Chrome for Testing 156 by
  // crates/turbo-dom-browser-env/scripts/select-element-chrome-control.mjs; asserted by
  // test/dom-adoption-reactions-chrome.test.mjs (the npm JS runtime). Each section records
  // an exception as `errors[section]` instead of stopping the probe (Chrome: none).
  const out = {};
  const errors = {};
  const section = (name, body) => {
    try { body(); } catch (e) { errors[name] = String((e && e.stack) || e); }
  };
  const SVG = 'http://www.w3.org/2000/svg';
  const HTML = 'http://www.w3.org/1999/xhtml';
  const MATH = 'http://www.w3.org/1998/Math/MathML';
  const errName = (fn) => { try { fn(); return 'ok'; } catch (e) { return e.name; } };
  const byId = (id) => document.getElementById(id);
  const nsView = (el) => [el.namespaceURI, el.prefix, el.localName, el.tagName, el.nodeName,
    el instanceof HTMLElement, el.matches(':defined')];

  // ---- 1. createElementNS -------------------------------------------------------------
  section('ns', () => {
    const made = {
      unknown: document.createElementNS('urn:x', 'Foo'),
      prefixed: document.createElementNS('urn:x', 'p:Foo'),
      nullNs: document.createElementNS(null, 'Bar'),
      emptyNs: document.createElementNS('', 'baz'),
      svgPrefixed: document.createElementNS(SVG, 'svg:rect'),
      svgCamel: document.createElementNS(SVG, 'foreignObject'),
      math: document.createElementNS(MATH, 'mi'),
      html: document.createElementNS(HTML, 'section'),
      customName: document.createElementNS('urn:x', 'x-foo'),
    };
    out.ns = Object.fromEntries(Object.entries(made).map(([k, el]) => [k, nsView(el)]));
    const prefixed = made.prefixed;
    prefixed.setAttribute('Data-X', '1');
    out.nsAttrs = [prefixed.getAttributeNames(), prefixed.getAttribute('data-x'), prefixed.getAttribute('Data-X'),
      prefixed.hasAttribute('data-x')];
    out.nsClone = nsView(prefixed.cloneNode(true));
    out.nsImport = nsView(document.importNode(prefixed, false));
    const holder = document.createElement('div');
    holder.append(made.unknown, prefixed, made.nullNs);
    out.nsSerialized = holder.innerHTML;
    made.unknown.innerHTML = '<b>x</b>';
    out.nsInner = [made.unknown.firstChild.namespaceURI, made.unknown.firstChild.localName, made.unknown.innerHTML];
    out.nsErrors = {
      nullPrefix: errName(() => document.createElementNS(null, 'p:a')),
      xmlPrefix: errName(() => document.createElementNS('urn:x', 'xml:a')),
      xmlNs: errName(() => document.createElementNS('http://www.w3.org/XML/1998/namespace', 'xml:a')),
      xmlnsName: errName(() => document.createElementNS('urn:x', 'xmlns')),
      xmlnsPrefix: errName(() => document.createElementNS('urn:x', 'xmlns:a')),
      xmlnsNs: errName(() => document.createElementNS('http://www.w3.org/2000/xmlns/', 'xmlns')),
      xmlnsNsOther: errName(() => document.createElementNS('http://www.w3.org/2000/xmlns/', 'a')),
      twoColons: errName(() => document.createElementNS('urn:x', 'a:b:c')),
      emptyPrefix: errName(() => document.createElementNS('urn:x', ':a')),
      emptyLocal: errName(() => document.createElementNS('urn:x', 'a:')),
      empty: errName(() => document.createElementNS('urn:x', '')),
      space: errName(() => document.createElementNS('urn:x', 'a b')),
      slash: errName(() => document.createElementNS('urn:x', 'a/b')),
      digit: errName(() => document.createElementNS('urn:x', '1a')),
      underscore: errName(() => document.createElementNS('urn:x', '_a')),
      nonAscii: errName(() => document.createElementNS('urn:x', 'été')),
      htmlSpace: errName(() => document.createElement('a b')),
      htmlDigit: errName(() => document.createElement('1a')),
      htmlEmpty: errName(() => document.createElement('')),
    };
    out.nsTwoColons = nsView(document.createElementNS('urn:x', 'a:b:c'));
    // tag-name lookup: HTML elements by the lowercased name, others by the exact qualified name
    document.body.append(holder);
    const svg = document.createElementNS(SVG, 'svg');
    svg.append(made.svgCamel);
    holder.append(svg);
    out.nsByTag = ['Foo', 'foo', 'p:Foo', 'Bar', 'foreignObject', 'foreignobject', 'FOREIGNOBJECT', 'DIV']
      .map((name) => [name, document.getElementsByTagName(name).length, holder.getElementsByTagName(name).length]);
    holder.remove();
  });

  // ---- 2. insertion across documents adopts --------------------------------------------
  const other = new DOMParser().parseFromString(
    '<div id="src" class="c"><p id="p1" title="t">one<b>bold</b></p><p id="p2">two</p></div><span id="keep"></span>',
    'text/html');
  section('adopt', () => {
    const live = other.getElementsByTagName('p');
    const before = [other.querySelectorAll('p').length, live.length];
    const src = other.getElementById('src');
    const target = document.createElement('section');
    document.body.append(target);
    target.insertBefore(src, null);
    const owners = [];
    for (const n of [src, ...src.querySelectorAll('*')]) owners.push(n.ownerDocument === document);
    out.adopt = {
      before,
      after: [other.querySelectorAll('p').length, live.length],
      oldLookup: [other.getElementById('src'), other.getElementById('p1'), other.getElementById('keep') !== null],
      oldBody: other.body.innerHTML,
      owners,
      newLookup: [byId('p1') === src.firstElementChild, byId('p2') === src.lastElementChild],
      read: [src.textContent, src.outerHTML, src.querySelector('b').textContent, byId('p1').getAttribute('title')],
    };
    const frag = other.createDocumentFragment();
    frag.append(other.createElement('i'), 'txt');
    target.append(frag);
    target.replaceChild(other.createElement('u'), target.lastChild);
    out.adoptFragment = [target.lastChild.localName, target.lastChild.ownerDocument === document,
      target.lastChild.previousSibling.ownerDocument === document, frag.childNodes.length];
    target.remove();
  });

  // ---- 3 + 4. custom element reactions -------------------------------------------------
  const log = [];
  const mark = (step) => log.push(['--', step]);
  const label = (el) => el.id || el.localName;
  const host = byId('host');
  let inert = null;
  const docName = (d) => (d === document ? 'document' : d === other ? 'other' : d === inert ? 'inert' : '?');
  class Logged extends HTMLElement {
    static get observedAttributes() { return ['a']; }
    constructor() {
      super();
      log.push(['ctor', label(this), this.isConnected, this.childElementCount, this.parentNode !== null]);
    }
    connectedCallback() {
      log.push(['conn', label(this), this.parentNode.childNodes.length]);
      if (this.hasAttribute('data-nest')) { this.removeAttribute('data-nest'); this.setAttribute('a', 'nested'); }
    }
    disconnectedCallback() { log.push(['disc', label(this), host.childNodes.length]); }
    adoptedCallback(oldDoc, newDoc) { log.push(['adopt', label(this), docName(oldDoc), docName(newDoc)]); }
    attributeChangedCallback(name, oldValue, newValue) { log.push(['attr', label(this), name, oldValue, newValue]); }
  }
  mark('define');
  customElements.define('x-r', Logged);

  // 3. template contents: an inert document, upgraded only once inserted into the document
  section('template', () => {
    inert = document.createElement('template').content.ownerDocument;
    const t2 = byId('t2');
    out.inert = [inert !== document, t2.content.ownerDocument === inert, t2.content.firstChild.ownerDocument === inert,
      inert.defaultView, t2.content.firstChild.matches(':defined')];
    mark('content-clone');
    const clone = t2.content.cloneNode(true);
    out.contentClone = [clone.ownerDocument === inert, clone.firstChild.ownerDocument === inert,
      clone.firstChild.matches(':defined')];
    mark('upgrade-inert');
    customElements.upgrade(clone);
    out.upgradeInert = clone.firstChild.matches(':defined');
    mark('append-detached');
    const box = document.createElement('div');
    box.append(clone);
    out.detached = [box.firstChild.ownerDocument === document, box.firstChild.matches(':defined')];
    mark('connect');
    host.append(box);
    out.connected = [box.firstChild.matches(':defined'), box.firstChild.firstChild.matches(':defined')];
    box.remove();
    mark('import');
    const imported = document.importNode(t2.content, true);
    out.imported = [imported.ownerDocument === document, imported.firstChild.ownerDocument === document,
      imported.firstChild.matches(':defined')];
    mark('template-innerHTML');
    const t3 = document.createElement('template');
    t3.innerHTML = '<x-r id="w1"></x-r>';
    out.templateInner = [t3.childNodes.length, t3.content.childNodes.length, t3.content.firstChild.matches(':defined'),
      t3.innerHTML, t3.content.ownerDocument === inert];
    mark('into-content');
    const fresh = document.createElement('x-r');
    fresh.id = 'fresh';
    t3.content.appendChild(fresh);
    out.intoContent = [fresh.ownerDocument === inert, fresh.matches(':defined')];
    mark('template-clone');
    const deepCopy = t2.cloneNode(true);
    out.templateClone = [deepCopy.content.childNodes.length, t2.cloneNode(false).content.childNodes.length,
      deepCopy.content.firstChild.matches(':defined'), deepCopy.content.ownerDocument === inert, deepCopy.innerHTML];
    mark('move-content');
    const t4 = byId('t4');
    host.append(t4.content);
    out.moveContent = [t4.content.childNodes.length, byId('v1').matches(':defined')];
  });
  host.textContent = '';

  // 4. reactions run when the DOM call returns, element by element
  section('reactions', () => {
    mark('innerHTML');
    host.innerHTML = '<x-r id="r1" a="1" data-nest=""><x-r id="r2" a="2"></x-r></x-r><x-r id="r3"></x-r>';
    mark('innerHTML-replace');
    host.innerHTML = '<x-r id="r4"></x-r>';
    mark('append-many');
    const m1 = document.createElement('x-r');
    m1.id = 'm1';
    const m2 = document.createElement('x-r');
    m2.id = 'm2';
    mark('append-many-insert');
    host.append(m1, m2);
    mark('replaceChildren');
    host.replaceChildren(document.createTextNode('t'));
    mark('detached-innerHTML');
    const tree = document.createElement('div');
    tree.innerHTML = '<x-r id="c1"><x-r id="c2"></x-r><span></span></x-r>';
    mark('clone-deep');
    const treeCopy = tree.cloneNode(true);
    out.cloneDeep = [treeCopy.firstChild.matches(':defined'), treeCopy.firstChild.firstChild.matches(':defined')];
    mark('insertAdjacentHTML');
    host.insertAdjacentHTML('beforeend', '<x-r id="h1"></x-r>');
    mark('insertAdjacentHTML-custom');
    byId('h1').insertAdjacentHTML('afterbegin', '<i></i>');
    mark('attributes');
    byId('h1').setAttribute('a', 'x');
    byId('h1').setAttribute('b', 'y');
    byId('h1').removeAttribute('a');
    mark('replaceChild');
    const repl = document.createElement('x-r');
    repl.id = 'repl';
    host.replaceChild(repl, byId('h1'));
  });
  section('crossDocument', () => {
    mark('cross-document');
    const mover = document.createElement('x-r');
    mover.id = 'mv';
    host.append(mover);
    other.body.appendChild(mover);
    out.moverInOther = [mover.ownerDocument === other, mover.matches(':defined')];
    host.append(mover);
    mark('adoptNode');
    other.adoptNode(mover);
    document.adoptNode(mover);
    out.mover = [mover.ownerDocument === document, mover.isConnected];
    mark('other-createElement');
    out.otherCreate = other.createElement('x-r').matches(':defined');
  });
  // ---- pre-existing bugs found on the way (each filed as its own turbo-dom issue) --------
  section('extra', () => {
    // insertBefore(node, node): the reference child becomes node's next sibling (no throw)
    const list = document.createElement('ol');
    list.append(document.createElement('li'), document.createElement('li'), document.createElement('li'));
    const middle = list.children[1];
    middle.id = 'middle';
    out.insertSelf = [errName(() => list.insertBefore(middle, middle)), list.children[1] === middle,
      errName(() => list.replaceChild(middle, middle)), list.children.length, list.children[1] === middle];
    // a doctype clone keeps its name and ids
    const doctype = document.implementation.createDocumentType('svg', '-//W3C//DTD SVG 1.1//EN', 'urn:dtd');
    const copy = doctype.cloneNode();
    out.doctypeClone = [copy.name, copy.publicId, copy.systemId, document.doctype.cloneNode().name];
    // only HTML elements serialize as void
    const foreign = document.createElementNS('urn:x', 'br');
    foreign.append('x');
    const wrap = document.createElement('div');
    wrap.append(foreign);
    out.foreignVoid = wrap.innerHTML;
    // <meta>.content reflects the attribute; other elements have no `content` property
    const meta = document.createElement('meta');
    const metaBefore = meta.content;
    meta.content = 'width=device-width';
    const plain = document.createElement('div');
    const plainBefore = typeof plain.content;
    plain.content = 5;
    out.contentProperty = [metaBefore, meta.getAttribute('content'), meta.content, plainBefore, plain.content,
      Object.keys(plain).includes('content'), typeof document.createElementNS(SVG, 'svg').content];
    // adoption reaches shadow-including descendants
    const shadowHost = other.createElement('div');
    const shadowRoot = shadowHost.attachShadow({ mode: 'open' });
    shadowRoot.innerHTML = '<p>x</p>';
    document.body.append(shadowHost);
    out.shadowAdopt = [shadowRoot.ownerDocument === document, shadowRoot.firstChild.ownerDocument === document];
    shadowHost.remove();
  });
  mark('end');
  host.textContent = '';
  out.log = log;
  out.errors = errors;
  return out;
})();
