// Layer 2 — lazy node inflation + copy-on-write tree + live collections + identity.
//
// The native parser hands us an immutable nested "buffer" (plain JS objects).
// DOM node handles inflate from it lazily: a node's children aren't built until
// something reads them. First access memoizes the handle (=== identity preserved).
// Mutation promotes the affected node to fully-owned (COW). Reads are transparent
// across the boundary — a buffer-backed read and an owned read are indistinguishable.

import { createRequire } from 'node:module';
import { EventTarget, Event, CustomEvent } from './events.mjs';
import { liveNodeList, liveHTMLCollection } from './collections.mjs';
import { matchesSelector, querySelector as qsel, querySelectorAll as qselAll } from './selectors.mjs';
import { serializeInner, serializeOuter } from './html-serialize.mjs';
import { Buffer } from './buffer.mjs';

const require = createRequire(import.meta.url);
const native = require('../../index.js');

export const ELEMENT_NODE = 1;
export const TEXT_NODE = 3;
export const COMMENT_NODE = 8;
export const DOCUMENT_NODE = 9;
export const DOCUMENT_TYPE_NODE = 10;
export const DOCUMENT_FRAGMENT_NODE = 11;

const SVG_NS = 'http://www.w3.org/2000/svg';
const MATHML_NS = 'http://www.w3.org/1998/Math/MathML';
const HTML_NS = 'http://www.w3.org/1999/xhtml';
const nsUri = (short) => (short === 'svg' ? SVG_NS : short === 'math' ? MATHML_NS : HTML_NS);

// ------------------------------------------------------------------ Node ----
export class Node extends EventTarget {
  static ELEMENT_NODE = ELEMENT_NODE;
  static TEXT_NODE = TEXT_NODE;
  static COMMENT_NODE = COMMENT_NODE;
  static DOCUMENT_NODE = DOCUMENT_NODE;
  static DOCUMENT_TYPE_NODE = DOCUMENT_TYPE_NODE;
  static DOCUMENT_FRAGMENT_NODE = DOCUMENT_FRAGMENT_NODE;

  constructor(ownerDocument) {
    super();
    this.ownerDocument = ownerDocument || null;
    this.parentNode = null;
    this.__idx = -1;              // backing buffer index, or -1 if owned/created
    this.__kids = null;          // owned child array once inflated/promoted
  }

  // Lazy inflation: walk the SoA buffer's firstChild/nextSib for this node, build
  // handles on first access, memoize for identity. Mutation then operates on __kids
  // (COW promotion). A buffer-backed read and an owned read are indistinguishable.
  __children() {
    if (this.__kids) return this.__kids;
    const kids = [];
    const doc = this.ownerDocument;
    if (this.__idx >= 0 && doc && doc.__buf) {
      const buf = doc.__buf;
      for (let c = buf.firstChild(this.__idx); c !== -1; c = buf.nextSib(c)) {
        // template content fragment is not a child — it's `.content`
        if (buf.nodeType(c) === DOCUMENT_FRAGMENT_NODE && buf.tagName(c) === 'content') continue;
        const child = doc.__nodeAt(c);
        child.parentNode = this;
        kids.push(child);
      }
    }
    this.__kids = kids;
    return kids;
  }

  get childNodes() {
    const self = this;
    return liveNodeList(() => self.__children());
  }
  get firstChild() { const k = this.__children(); return k[0] ?? null; }
  get lastChild() { const k = this.__children(); return k[k.length - 1] ?? null; }
  hasChildNodes() { return this.__children().length > 0; }

  get nextSibling() {
    const p = this.parentNode; if (!p) return null;
    const k = p.__children(); const i = k.indexOf(this);
    return i >= 0 ? k[i + 1] ?? null : null;
  }
  get previousSibling() {
    const p = this.parentNode; if (!p) return null;
    const k = p.__children(); const i = k.indexOf(this);
    return i > 0 ? k[i - 1] : null;
  }
  get parentElement() {
    return this.parentNode && this.parentNode.nodeType === ELEMENT_NODE ? this.parentNode : null;
  }

  // ---- mutation (COW promotion happens implicitly: __children() owns the array) ----
  appendChild(node) { return this.insertBefore(node, null); }

  insertBefore(node, ref) {
    if (node.nodeType === DOCUMENT_FRAGMENT_NODE) {
      for (const c of node.__children().slice()) this.insertBefore(c, ref);
      return node;
    }
    if (node.parentNode) node.parentNode.removeChild(node);
    const kids = this.__children();
    const i = ref ? kids.indexOf(ref) : -1;
    if (ref && i === -1) throw new Error('NotFoundError: ref is not a child');
    if (ref) kids.splice(i, 0, node); else kids.push(node);
    node.parentNode = this;
    node.ownerDocument = this.ownerDocument;
    return node;
  }

  removeChild(node) {
    const kids = this.__children();
    const i = kids.indexOf(node);
    if (i === -1) throw new Error('NotFoundError: node is not a child');
    kids.splice(i, 1);
    node.parentNode = null;
    return node;
  }

  replaceChild(newNode, oldNode) {
    this.insertBefore(newNode, oldNode);
    this.removeChild(oldNode);
    return oldNode;
  }

  remove() { if (this.parentNode) this.parentNode.removeChild(this); }

  contains(other) {
    let n = other;
    while (n) { if (n === this) return true; n = n.parentNode; }
    return false;
  }

  get textContent() {
    let s = '';
    for (const c of this.__children()) {
      if (c.nodeType === TEXT_NODE) s += c.data;
      else if (c.nodeType === ELEMENT_NODE || c.nodeType === DOCUMENT_FRAGMENT_NODE) s += c.textContent;
    }
    return s;
  }
  set textContent(value) {
    this.__kids = [];
    if (value !== '') this.appendChild(this.ownerDocument.createTextNode(String(value)));
  }

  // window/document path for event propagation past the document
  get __owner() { return null; }
}

// ----------------------------------------------------- CharacterData ----
class CharacterData extends Node {
  constructor(ownerDocument, data) { super(ownerDocument); this._data = data ?? ''; }
  get data() { return this._data; }
  set data(v) { this._data = String(v); }
  get nodeValue() { return this._data; }
  set nodeValue(v) { this._data = String(v); }
  get length() { return this._data.length; }
  get textContent() { return this._data; }
  set textContent(v) { this._data = String(v); }
}

export class Text extends CharacterData {
  get nodeType() { return TEXT_NODE; }
  get nodeName() { return '#text'; }
  cloneNode() { return new Text(this.ownerDocument, this._data); }
}

export class Comment extends CharacterData {
  get nodeType() { return COMMENT_NODE; }
  get nodeName() { return '#comment'; }
  cloneNode() { return new Comment(this.ownerDocument, this._data); }
}

export class DocumentType extends Node {
  constructor(ownerDocument, name, publicId, systemId) {
    super(ownerDocument);
    this.name = name; this.publicId = publicId || ''; this.systemId = systemId || '';
  }
  get nodeType() { return DOCUMENT_TYPE_NODE; }
  get nodeName() { return this.name; }
}

// ------------------------------------------------------------ Element ----
class ClassList {
  constructor(el) { this.__el = el; }
  __list() { return (this.__el.getAttribute('class') || '').split(/\s+/).filter(Boolean); }
  __set(list) { this.__el.setAttribute('class', list.join(' ')); }
  contains(c) { return this.__list().includes(c); }
  add(...cs) { const l = this.__list(); for (const c of cs) if (!l.includes(c)) l.push(c); this.__set(l); }
  remove(...cs) { this.__set(this.__list().filter((c) => !cs.includes(c))); }
  toggle(c, force) {
    const has = this.contains(c);
    if (force === true || (force === undefined && !has)) { this.add(c); return true; }
    this.remove(c); return false;
  }
  replace(a, b) { const l = this.__list(); const i = l.indexOf(a); if (i === -1) return false; l[i] = b; this.__set(l); return true; }
  get length() { return this.__list().length; }
  item(i) { return this.__list()[i] ?? null; }
  get value() { return this.__el.getAttribute('class') || ''; }
  toString() { return this.value; }
  [Symbol.iterator]() { return this.__list()[Symbol.iterator](); }
}

export class Element extends Node {
  constructor(ownerDocument, localName, namespace = '') {
    super(ownerDocument);
    this.localName = localName;
    this.__ns = namespace;            // '', 'svg', 'math'
    this.__attrs = [];                // [{name, value, prefix}]
    this.content = null;              // <template> content fragment
  }

  get nodeType() { return ELEMENT_NODE; }
  get tagName() { return this.__ns ? this.localName : this.localName.toUpperCase(); }
  get nodeName() { return this.tagName; }
  get namespaceURI() { return nsUri(this.__ns); }

  // ---- attributes ----
  getAttribute(name) { const a = this.__attrs.find((x) => x.name === name); return a ? a.value : null; }
  hasAttribute(name) { return this.__attrs.some((x) => x.name === name); }
  getAttributeNames() { return this.__attrs.map((a) => a.name); }
  setAttribute(name, value) {
    const a = this.__attrs.find((x) => x.name === name);
    if (a) a.value = String(value);
    else this.__attrs.push({ name, value: String(value), prefix: '' });
  }
  removeAttribute(name) { this.__attrs = this.__attrs.filter((x) => x.name !== name); }
  toggleAttribute(name, force) {
    const has = this.hasAttribute(name);
    if (force === true || (force === undefined && !has)) { this.setAttribute(name, ''); return true; }
    this.removeAttribute(name); return false;
  }
  get attributes() {
    return this.__attrs.map((a) => ({
      name: a.name, localName: a.name, value: a.value, prefix: a.prefix || null,
      namespaceURI: a.prefix === 'xlink' ? 'http://www.w3.org/1999/xlink' : null,
    }));
  }

  get id() { return this.getAttribute('id') || ''; }
  set id(v) { this.setAttribute('id', v); }

  // label / form-control association (used by RTL getByLabelText)
  get htmlFor() { return this.getAttribute('for') || ''; }
  set htmlFor(v) { this.setAttribute('for', v); }
  get control() {
    if (this.localName !== 'label') return null;
    const id = this.getAttribute('for');
    if (id) return this.ownerDocument.getElementById(id);
    return this.querySelector('button,input,select,textarea,meter,output,progress') || null;
  }
  get labels() {
    const labelable = /^(button|input|meter|output|progress|select|textarea)$/.test(this.localName) &&
      !(this.localName === 'input' && this.getAttribute('type') === 'hidden');
    if (!labelable) return undefined;
    const out = [];
    if (this.id) {
      for (const l of this.ownerDocument.getElementsByTagName('label')) {
        if (l.getAttribute('for') === this.id) out.push(l);
      }
    }
    let p = this.parentNode;
    while (p) { if (p.localName === 'label' && !out.includes(p)) out.push(p); p = p.parentNode; }
    return out;
  }
  get className() { return this.getAttribute('class') || ''; }
  set className(v) { this.setAttribute('class', v); }
  get classList() { return new ClassList(this); }

  get style() {
    // minimal honest CSSOM: parse/serialize the inline style attribute
    if (!this.__style) this.__style = makeStyle(this);
    return this.__style;
  }

  // ---- element-only traversal (live) ----
  get children() {
    const self = this;
    return liveHTMLCollection(() => self.__children().filter((n) => n.nodeType === ELEMENT_NODE));
  }
  get childElementCount() { return this.__children().filter((n) => n.nodeType === ELEMENT_NODE).length; }
  get firstElementChild() { return this.__children().find((n) => n.nodeType === ELEMENT_NODE) ?? null; }
  get lastElementChild() { const e = this.__children().filter((n) => n.nodeType === ELEMENT_NODE); return e[e.length - 1] ?? null; }
  get nextElementSibling() { let n = this.nextSibling; while (n && n.nodeType !== ELEMENT_NODE) n = n.nextSibling; return n || null; }
  get previousElementSibling() { let n = this.previousSibling; while (n && n.nodeType !== ELEMENT_NODE) n = n.previousSibling; return n || null; }

  // ---- modern insertion ----
  append(...nodes) { for (const n of nodes) this.appendChild(toNode(this.ownerDocument, n)); }
  prepend(...nodes) { const first = this.firstChild; for (const n of nodes) this.insertBefore(toNode(this.ownerDocument, n), first); }
  before(...nodes) { const p = this.parentNode; if (!p) return; for (const n of nodes) p.insertBefore(toNode(this.ownerDocument, n), this); }
  after(...nodes) { const p = this.parentNode; if (!p) return; const ref = this.nextSibling; for (const n of nodes) p.insertBefore(toNode(this.ownerDocument, n), ref); }
  replaceWith(...nodes) { const p = this.parentNode; if (!p) return; const ref = this.nextSibling; this.remove(); for (const n of nodes) p.insertBefore(toNode(this.ownerDocument, n), ref); }
  replaceChildren(...nodes) { this.__kids = []; for (const n of nodes) this.appendChild(toNode(this.ownerDocument, n)); }

  // ---- queries ----
  matches(sel) { return matchesSelector(this, sel); }
  closest(sel) { let n = this; while (n && n.nodeType === ELEMENT_NODE) { if (n.matches(sel)) return n; n = n.parentNode; } return null; }
  querySelector(sel) { return qsel(this, sel); }
  querySelectorAll(sel) { return qselAll(this, sel); }
  getElementsByTagName(tag) { const self = this; return liveHTMLCollection(() => collectByTag(self, tag.toLowerCase())); }
  getElementsByClassName(cls) { const self = this; const classes = cls.split(/\s+/).filter(Boolean); return liveHTMLCollection(() => collectByClass(self, classes)); }

  // ---- innerHTML / outerHTML ----
  get innerHTML() { return serializeInner(this); }
  set innerHTML(html) {
    const frag = native.parseFragment(String(html), this.__ns ? `${this.__ns} ${this.localName}` : this.localName);
    this.__kids = [];
    for (const rawChild of frag.children) {
      if (rawChild.nodeType === DOCUMENT_FRAGMENT_NODE && rawChild.name === 'content') continue;
      const child = this.ownerDocument.__inflateNested(rawChild);
      child.parentNode = this;
      this.__kids.push(child);
    }
  }
  get outerHTML() { return serializeOuter(this); }
  get innerText() { return this.textContent; }
  set innerText(v) { this.textContent = v; }

  insertAdjacentHTML(position, html) {
    const tmp = this.ownerDocument.createElement(this.localName);
    tmp.innerHTML = html;
    const nodes = tmp.__children().slice();
    const p = this.parentNode;
    switch (position) {
      case 'beforebegin': for (const n of nodes) p.insertBefore(n, this); break;
      case 'afterbegin': { const first = this.firstChild; for (const n of nodes) this.insertBefore(n, first); break; }
      case 'beforeend': for (const n of nodes) this.appendChild(n); break;
      case 'afterend': { const ref = this.nextSibling; for (const n of nodes) p.insertBefore(n, ref); break; }
      default: throw new Error(`bad insertAdjacentHTML position: ${position}`);
    }
  }

  cloneNode(deep = false) {
    const el = new Element(this.ownerDocument, this.localName, this.__ns);
    el.__attrs = this.__attrs.map((a) => ({ ...a }));
    if (deep) for (const c of this.__children()) el.appendChild(c.cloneNode(true));
    return el;
  }

  click() { this.dispatchEvent(new Event('click', { bubbles: true, cancelable: true })); }
  focus() { this.ownerDocument.__setActive(this); }
  blur() { this.ownerDocument.__setActive(this.ownerDocument.body); }
  getBoundingClientRect() { return zeroRect(); }
  getClientRects() { return []; }
  scrollIntoView() {}
}

// form-ish value reflection for inputs (common in RTL/user-event)
function defineValueProp(el) {
  if (!('value' in el) && (el.localName === 'input' || el.localName === 'textarea' || el.localName === 'select' || el.localName === 'option')) {
    let v = el.getAttribute('value') ?? '';
    Object.defineProperty(el, 'value', { get: () => v, set: (x) => { v = String(x); }, configurable: true });
    if (el.localName === 'input') {
      let checked = el.hasAttribute('checked');
      Object.defineProperty(el, 'checked', { get: () => checked, set: (x) => { checked = !!x; }, configurable: true });
    }
  }
}

// ---------------------------------------------------- DocumentFragment ----
export class DocumentFragment extends Node {
  get nodeType() { return DOCUMENT_FRAGMENT_NODE; }
  get nodeName() { return '#document-fragment'; }
  querySelector(sel) { return qsel(this, sel); }
  querySelectorAll(sel) { return qselAll(this, sel); }
  get children() { const self = this; return liveHTMLCollection(() => self.__children().filter((n) => n.nodeType === ELEMENT_NODE)); }
  append(...nodes) { for (const n of nodes) this.appendChild(toNode(this.ownerDocument, n)); }
  cloneNode(deep = false) { const f = new DocumentFragment(this.ownerDocument); if (deep) for (const c of this.__children()) f.appendChild(c.cloneNode(true)); return f; }
}

// ------------------------------------------------------------ helpers ----
function toNode(doc, n) { return typeof n === 'string' ? doc.createTextNode(n) : n; }

function collectByTag(root, tag) {
  const out = [];
  const visit = (node) => {
    for (const c of node.__children()) {
      if (c.nodeType === ELEMENT_NODE) {
        if (tag === '*' || c.localName === tag) out.push(c);
        visit(c);
      }
    }
  };
  visit(root);
  return out;
}
function collectByClass(root, classes) {
  const out = [];
  const visit = (node) => {
    for (const c of node.__children()) {
      if (c.nodeType === ELEMENT_NODE) {
        if (classes.every((cl) => c.classList.contains(cl))) out.push(c);
        visit(c);
      }
    }
  };
  visit(root);
  return out;
}

function zeroRect() {
  return { x: 0, y: 0, top: 0, left: 0, right: 0, bottom: 0, width: 0, height: 0, toJSON() { return this; } };
}

// minimal inline-style CSSOM (honest: only inline + explicitly set props)
function makeStyle(el) {
  const parse = () => {
    const map = new Map();
    for (const decl of (el.getAttribute('style') || '').split(';')) {
      const i = decl.indexOf(':');
      if (i === -1) continue;
      const prop = decl.slice(0, i).trim();
      const val = decl.slice(i + 1).trim();
      if (prop) map.set(prop, val);
    }
    return map;
  };
  const write = (map) => el.setAttribute('style', [...map].map(([k, v]) => `${k}: ${v}`).join('; '));
  return new Proxy({}, {
    get(_t, key) {
      if (key === 'getPropertyValue') return (p) => parse().get(p) ?? '';
      if (key === 'setProperty') return (p, v) => { const m = parse(); m.set(p, v); write(m); };
      if (key === 'removeProperty') return (p) => { const m = parse(); const v = m.get(p) ?? ''; m.delete(p); write(m); return v; };
      if (key === 'cssText') return el.getAttribute('style') || '';
      if (typeof key !== 'string') return undefined;
      return parse().get(kebab(key)) ?? '';
    },
    set(_t, key, value) {
      if (key === 'cssText') { el.setAttribute('style', String(value)); return true; }
      const m = parse(); m.set(kebab(key), String(value)); write(m); return true;
    },
  });
}
const kebab = (s) => s.replace(/[A-Z]/g, (m) => '-' + m.toLowerCase());

// ----------------------------------------------------------- Document ----
export class Document extends Node {
  constructor() {
    super(null);
    this.ownerDocument = this;
    this.__buf = null;            // SoA buffer accessor
    this.__cache = [];            // idx -> handle (identity memoization / nodeAt)
    this.__active = null;         // activeElement
    this.defaultView = null;      // set by environment (window)
  }
  get nodeType() { return DOCUMENT_NODE; }
  get nodeName() { return '#document'; }

  // nodeAt: one handle per buffer index, memoized → preserves === identity.
  __nodeAt(idx) {
    if (idx < 0) return null;
    const cached = this.__cache[idx];
    if (cached !== undefined) return cached;
    const buf = this.__buf;
    let node;
    switch (buf.nodeType(idx)) {
      case ELEMENT_NODE: {
        node = new Element(this, buf.tagName(idx), buf.ns(idx));
        node.__idx = idx;
        node.__attrs = buf.attrs(idx);
        // template content fragment: a child node typed 11 named "content"
        if (buf.tagName(idx) === 'template') {
          for (let c = buf.firstChild(idx); c !== -1; c = buf.nextSib(c)) {
            if (buf.nodeType(c) === DOCUMENT_FRAGMENT_NODE && buf.tagName(c) === 'content') {
              node.content = this.__nodeAt(c);
              break;
            }
          }
        }
        defineValueProp(node);
        break;
      }
      case TEXT_NODE: node = new Text(this, buf.text(idx)); node.__idx = idx; break;
      case COMMENT_NODE: node = new Comment(this, buf.text(idx)); node.__idx = idx; break;
      case DOCUMENT_TYPE_NODE:
        node = new DocumentType(this, buf.text(idx), buf.publicId(idx), buf.systemId(idx));
        node.__idx = idx;
        break;
      case DOCUMENT_FRAGMENT_NODE: node = new DocumentFragment(this); node.__idx = idx; break;
      default: node = new Comment(this, ''); node.__idx = idx; break;
    }
    this.__cache[idx] = node;
    return node;
  }

  // Inflate an OWNED subtree from a nested parse tree (used by innerHTML= ).
  __inflateNested(raw) {
    let node;
    switch (raw.nodeType) {
      case ELEMENT_NODE:
        node = new Element(this, raw.name, raw.namespace || '');
        node.__attrs = raw.attrs.map((a) => ({ name: a.name, value: a.value, prefix: a.prefix || '' }));
        defineValueProp(node);
        if (raw.name === 'template') {
          const contentRaw = raw.children.find((c) => c.nodeType === DOCUMENT_FRAGMENT_NODE && c.name === 'content');
          if (contentRaw) { node.content = this.__inflateNested(contentRaw); }
        }
        break;
      case TEXT_NODE: node = new Text(this, raw.value); break;
      case COMMENT_NODE: node = new Comment(this, raw.value); break;
      case DOCUMENT_TYPE_NODE: node = new DocumentType(this, raw.name, raw.publicId, raw.systemId); break;
      case DOCUMENT_FRAGMENT_NODE: node = new DocumentFragment(this); break;
      default: node = new Comment(this, ''); break;
    }
    if (raw.nodeType !== TEXT_NODE && raw.nodeType !== COMMENT_NODE && raw.nodeType !== DOCUMENT_TYPE_NODE) {
      const kids = [];
      for (const rc of raw.children) {
        if (rc.nodeType === DOCUMENT_FRAGMENT_NODE && rc.name === 'content') continue;
        const child = this.__inflateNested(rc);
        child.parentNode = node;
        kids.push(child);
      }
      node.__kids = kids;
    }
    return node;
  }

  // Layer 5: (re)point at the SoA buffer; children inflate lazily. Arena reset.
  __load(soa) {
    this.__buf = new Buffer(soa);
    this.__idx = 0;          // node 0 is the document
    this.__kids = null;      // drop overlay
    this.__cache = [];       // drop node cache
    this.__active = null;
  }

  get documentElement() { return this.__children().find((n) => n.nodeType === ELEMENT_NODE && n.localName === 'html') ?? null; }
  get doctype() { return this.__children().find((n) => n.nodeType === DOCUMENT_TYPE_NODE) ?? null; }
  get head() { const html = this.documentElement; return html ? html.__children().find((n) => n.localName === 'head') ?? null : null; }
  get body() { const html = this.documentElement; return html ? html.__children().find((n) => n.localName === 'body') ?? null : null; }
  get activeElement() { return this.__active || this.body || null; }
  __setActive(el) { this.__active = el; }

  // ---- factories (owned nodes, no buffer) ----
  createElement(tag) { const el = new Element(this, String(tag).toLowerCase(), ''); defineValueProp(el); return el; }
  createElementNS(ns, qualified) {
    const short = ns === SVG_NS ? 'svg' : ns === MATHML_NS ? 'math' : '';
    const local = qualified.includes(':') ? qualified.split(':')[1] : qualified;
    return new Element(this, local, short);
  }
  createTextNode(data) { return new Text(this, String(data)); }
  createComment(data) { return new Comment(this, String(data)); }
  createDocumentFragment() { return new DocumentFragment(this); }
  createEvent() { return new Event(''); }
  importNode(node, deep) { return node.cloneNode(deep); }
  adoptNode(node) { if (node.parentNode) node.parentNode.removeChild(node); node.ownerDocument = this; return node; }

  // ---- queries ----
  getElementById(id) {
    let found = null;
    const visit = (node) => {
      for (const c of node.__children()) {
        if (found) return;
        if (c.nodeType === ELEMENT_NODE) { if (c.getAttribute('id') === id) { found = c; return; } visit(c); }
      }
    };
    visit(this);
    return found;
  }
  querySelector(sel) { return qsel(this, sel); }
  querySelectorAll(sel) { return qselAll(this, sel); }
  getElementsByTagName(tag) { const self = this; return liveHTMLCollection(() => collectByTag(self, tag.toLowerCase())); }
  getElementsByClassName(cls) { const self = this; const classes = cls.split(/\s+/).filter(Boolean); return liveHTMLCollection(() => collectByClass(self, classes)); }
  contains(node) { return Node.prototype.contains.call(this, node); }

  get cookie() { return this.__cookie || ''; }
  set cookie(v) { this.__cookie = (this.__cookie ? this.__cookie + '; ' : '') + v; }
  get __owner() { return this.defaultView; }
}

export { Event, CustomEvent };

// Parse an HTML string into a fresh Document over the immutable SoA buffer.
export function parseDocument(html) {
  const soa = native.parseBuffer(String(html));
  const doc = new Document();
  doc.__load(soa);
  return doc;
}
