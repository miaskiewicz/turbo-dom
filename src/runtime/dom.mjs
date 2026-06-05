// Layer 2 — lazy node inflation + copy-on-write tree + live collections + identity.
//
// The native parser hands us an immutable nested "buffer" (plain JS objects).
// DOM node handles inflate from it lazily: a node's children aren't built until
// something reads them. First access memoizes the handle (=== identity preserved).
// Mutation promotes the affected node to fully-owned (COW). Reads are transparent
// across the boundary — a buffer-backed read and an owned read are indistinguishable.

import { createRequire } from 'node:module';
import {
  EventTarget, Event, CustomEvent,
  UIEvent, MouseEvent, KeyboardEvent, FocusEvent,
} from './events.mjs';
import { liveNodeList, liveHTMLCollection } from './collections.mjs';
import { matchesSelector, querySelector as qsel, querySelectorAll as qselAll } from './selectors.mjs';
import { serializeInner, serializeOuter } from './html-serialize.mjs';
import { Buffer } from './buffer.mjs';
import { makeCanvasStub } from './stubs.mjs';

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
    notifyMutation(this, { type: 'childList', target: this, addedNodes: [node], removedNodes: [], nextSibling: ref || null });
    return node;
  }

  removeChild(node) {
    const kids = this.__children();
    const i = kids.indexOf(node);
    if (i === -1) throw new Error('NotFoundError: node is not a child');
    const next = kids[i + 1] || null;
    kids.splice(i, 1);
    node.parentNode = null;
    notifyMutation(this, { type: 'childList', target: this, addedNodes: [], removedNodes: [node], nextSibling: next });
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

  get isConnected() {
    let n = this;
    while (n.parentNode) n = n.parentNode;
    return n.nodeType === DOCUMENT_NODE;
  }
  getRootNode() {
    let n = this;
    while (n.parentNode) n = n.parentNode;
    return n;
  }
  normalize() {
    const kids = this.__children();
    for (let i = kids.length - 1; i > 0; i--) {
      if (kids[i].nodeType === TEXT_NODE && kids[i - 1].nodeType === TEXT_NODE) {
        kids[i - 1]._data += kids[i].data; kids[i].parentNode = null; kids.splice(i, 1);
      }
    }
    for (let i = kids.length - 1; i >= 0; i--) {
      if (kids[i].nodeType === TEXT_NODE && kids[i].data === '') { kids[i].parentNode = null; kids.splice(i, 1); }
      else if (kids[i].nodeType === ELEMENT_NODE) kids[i].normalize();
    }
  }
  replaceChildren(...nodes) { this.__kids = []; for (const n of nodes) this.appendChild(typeof n === 'string' ? this.ownerDocument.createTextNode(n) : n); }
  // bitmask: 1 DISCONNECTED, 2 PRECEDING, 4 FOLLOWING, 8 CONTAINS, 16 CONTAINED_BY
  compareDocumentPosition(other) {
    if (other === this) return 0;
    if (this.contains(other)) return 16 + 4;
    if (other.contains(this)) return 8 + 2;
    // document order via a flat walk from the common root
    const root = this.getRootNode();
    const order = [];
    (function walk(n) { order.push(n); for (const c of (n.__children ? n.__children() : [])) walk(c); })(root);
    const a = order.indexOf(this), b = order.indexOf(other);
    if (a === -1 || b === -1) return 1;
    return a < b ? 4 : 2;
  }
  cloneNode(deep = false) {
    // base fallback; Element/Text/Comment override
    const n = new this.constructor(this.ownerDocument);
    if (deep) for (const c of this.__children()) n.appendChild(c.cloneNode(true));
    return n;
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

// nodeType + document-position constants on the prototype, so `node.TEXT_NODE`
// works on instances (dom-accessibility-api and others read them off the node).
Object.assign(Node.prototype, {
  ELEMENT_NODE: 1, ATTRIBUTE_NODE: 2, TEXT_NODE: 3, CDATA_SECTION_NODE: 4,
  ENTITY_REFERENCE_NODE: 5, ENTITY_NODE: 6, PROCESSING_INSTRUCTION_NODE: 7,
  COMMENT_NODE: 8, DOCUMENT_NODE: 9, DOCUMENT_TYPE_NODE: 10,
  DOCUMENT_FRAGMENT_NODE: 11, NOTATION_NODE: 12,
  DOCUMENT_POSITION_DISCONNECTED: 1, DOCUMENT_POSITION_PRECEDING: 2,
  DOCUMENT_POSITION_FOLLOWING: 4, DOCUMENT_POSITION_CONTAINS: 8,
  DOCUMENT_POSITION_CONTAINED_BY: 16, DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC: 32,
});

// ----------------------------------------------------- CharacterData ----
class CharacterData extends Node {
  constructor(ownerDocument, data) { super(ownerDocument); this._data = data ?? ''; }
  get data() { return this._data; }
  set data(v) { const old = this._data; this._data = String(v); notifyMutation(this, { type: 'characterData', target: this, oldValue: old, addedNodes: [], removedNodes: [] }); }
  get nodeValue() { return this._data; }
  set nodeValue(v) { this.data = v; }
  get length() { return this._data.length; }
  get textContent() { return this._data; }
  set textContent(v) { this.data = v; }
  substringData(offset, count) { return this._data.slice(offset, offset + count); }
  appendData(s) { this.data = this._data + s; }
  insertData(offset, s) { this.data = this._data.slice(0, offset) + s + this._data.slice(offset); }
  deleteData(offset, count) { this.data = this._data.slice(0, offset) + this._data.slice(offset + count); }
  replaceData(offset, count, s) { this.data = this._data.slice(0, offset) + s + this._data.slice(offset + count); }
  before(...nodes) { const p = this.parentNode; if (p) for (const n of nodes) p.insertBefore(typeof n === 'string' ? this.ownerDocument.createTextNode(n) : n, this); }
  after(...nodes) { const p = this.parentNode; if (!p) return; const ref = this.nextSibling; for (const n of nodes) p.insertBefore(typeof n === 'string' ? this.ownerDocument.createTextNode(n) : n, ref); }
  replaceWith(...nodes) { const p = this.parentNode; if (!p) return; const ref = this.nextSibling; this.remove(); for (const n of nodes) p.insertBefore(typeof n === 'string' ? this.ownerDocument.createTextNode(n) : n, ref); }
}

export class Text extends CharacterData {
  get nodeType() { return TEXT_NODE; }
  get nodeName() { return '#text'; }
  get wholeText() {
    let s = this._data, n = this.previousSibling;
    while (n && n.nodeType === TEXT_NODE) { s = n.data + s; n = n.previousSibling; }
    n = this.nextSibling;
    while (n && n.nodeType === TEXT_NODE) { s += n.data; n = n.nextSibling; }
    return s;
  }
  splitText(offset) {
    const rest = this._data.slice(offset);
    this.data = this._data.slice(0, offset);
    const node = new Text(this.ownerDocument, rest);
    if (this.parentNode) this.parentNode.insertBefore(node, this.nextSibling);
    return node;
  }
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
    this.shadowRoot = null;           // open shadow root, if attached
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
    const old = a ? a.value : null;
    if (a) a.value = String(value);
    else this.__attrs.push({ name, value: String(value), prefix: '' });
    notifyMutation(this, { type: 'attributes', target: this, attributeName: name, oldValue: old, addedNodes: [], removedNodes: [] });
  }
  removeAttribute(name) {
    const a = this.__attrs.find((x) => x.name === name);
    this.__attrs = this.__attrs.filter((x) => x.name !== name);
    if (a) notifyMutation(this, { type: 'attributes', target: this, attributeName: name, oldValue: a.value, addedNodes: [], removedNodes: [] });
  }
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

  get dataset() {
    if (!this.__dataset) this.__dataset = makeDataset(this);
    return this.__dataset;
  }

  // ---- form-control properties (on the prototype so libraries that read
  //      element.constructor.prototype descriptors — e.g. user-event — find them) ----
  get value() {
    const t = this.localName;
    if (t === 'select') {
      const list = Array.from(this.getElementsByTagName('option'));
      const s = list.find((o) => o.selected);
      if (s) return s.value;
      return list.length && !this.multiple ? list[0].value : '';
    }
    if (t === 'option') return this.hasAttribute('value') ? this.getAttribute('value') : this.textContent;
    if (t === 'input' || t === 'textarea') return this.__value !== undefined ? this.__value : (this.getAttribute('value') ?? '');
    return undefined;
  }
  set value(x) {
    const t = this.localName;
    if (t === 'select') { for (const o of this.getElementsByTagName('option')) o.selected = (o.value === String(x)); return; }
    if (t === 'option') { this.setAttribute('value', x); return; }
    // typed <input> sanitizes per the value-sanitization algorithm: invalid
    // values for date/time/number/etc are rejected (value left unchanged), so
    // user-event typing them char-by-char doesn't emit bogus partial values.
    if (t === 'input') {
      const sanitized = sanitizeInputValue((this.getAttribute('type') || 'text').toLowerCase(), String(x));
      if (sanitized === null) return; // invalid → reject, keep current value
      this.__value = sanitized;
    } else {
      this.__value = String(x);
    }
    if (this.__selStart != null) { this.__selStart = Math.min(this.__selStart, this.__value.length); this.__selEnd = Math.min(this.__selEnd, this.__value.length); }
  }
  get defaultValue() { return this.getAttribute('value') ?? ''; }
  set defaultValue(v) { this.setAttribute('value', v); }
  get valueAsNumber() { const v = this.value; return v === '' || v == null ? NaN : Number(v); }
  set valueAsNumber(n) { this.value = String(n); }
  get valueAsDate() { const v = this.value; const d = v ? new Date(v) : null; return d && !isNaN(d) ? d : null; }
  set valueAsDate(d) { this.value = d instanceof Date ? d.toISOString().slice(0, 10) : ''; }

  get selectionStart() { return this.__selStart ?? null; }
  set selectionStart(v) { this.__selStart = v; }
  get selectionEnd() { return this.__selEnd ?? null; }
  set selectionEnd(v) { this.__selEnd = v; }
  get selectionDirection() { return this.__selDir ?? 'none'; }
  set selectionDirection(v) { this.__selDir = v; }
  setSelectionRange(s, e, dir = 'none') { this.__selStart = s; this.__selEnd = e; this.__selDir = dir; }
  setRangeText(repl, start = this.__selStart ?? 0, end = this.__selEnd ?? 0) {
    const v = this.value ?? ''; this.value = v.slice(0, start) + repl + v.slice(end);
  }
  select() { this.__selStart = 0; this.__selEnd = (this.value ?? '').length; }

  get checked() { return this.__checked !== undefined ? this.__checked : this.hasAttribute('checked'); }
  set checked(x) { this.__checked = !!x; }
  get defaultChecked() { return this.hasAttribute('checked'); }
  set defaultChecked(x) { if (x) this.setAttribute('checked', ''); else this.removeAttribute('checked'); }

  get type() {
    if (this.localName === 'input') return (this.getAttribute('type') || 'text').toLowerCase();
    if (this.localName === 'button') return (this.getAttribute('type') || 'submit').toLowerCase();
    return this.getAttribute('type') || undefined;
  }
  set type(x) { this.setAttribute('type', x); }
  get disabled() { return this.hasAttribute('disabled'); }
  set disabled(x) { if (x) this.setAttribute('disabled', ''); else this.removeAttribute('disabled'); }
  get readOnly() { return this.hasAttribute('readonly'); }
  set readOnly(x) { if (x) this.setAttribute('readonly', ''); else this.removeAttribute('readonly'); }
  get required() { return this.hasAttribute('required'); }
  set required(x) { if (x) this.setAttribute('required', ''); else this.removeAttribute('required'); }
  get name() { return this.getAttribute('name') ?? ''; }
  set name(x) { this.setAttribute('name', x); }
  get placeholder() { return this.getAttribute('placeholder') ?? ''; }
  set placeholder(x) { this.setAttribute('placeholder', x); }
  get href() { return this.getAttribute('href') ?? ''; }
  set href(x) { this.setAttribute('href', x); }
  get download() { return this.getAttribute('download') ?? ''; }
  set download(x) { this.setAttribute('download', x); }
  get rel() { return this.getAttribute('rel') ?? ''; }
  set rel(x) { this.setAttribute('rel', x); }
  get referrerPolicy() { return this.getAttribute('referrerpolicy') ?? ''; }
  set referrerPolicy(x) { this.setAttribute('referrerpolicy', x); }
  get src() { return this.getAttribute('src') ?? ''; }
  set src(x) { this.setAttribute('src', x); }
  get alt() { return this.getAttribute('alt') ?? ''; }
  set alt(x) { this.setAttribute('alt', x); }

  // option
  get selected() { return this.__selected !== undefined ? this.__selected : this.hasAttribute('selected'); }
  set selected(x) {
    this.__selected = !!x;
    // single-select: selecting one option deselects the others (exclusive)
    if (x && this.localName === 'option') {
      let sel = this.parentNode;
      while (sel && sel.localName !== 'select') sel = sel.parentNode;
      if (sel && !sel.multiple) {
        for (const o of sel.getElementsByTagName('option')) if (o !== this) o.__selected = false;
      }
    }
  }
  get defaultSelected() { return this.hasAttribute('selected'); }
  get text() { return this.textContent; }
  set text(v) { this.textContent = v; }

  // select
  get options() { return this.localName === 'select' ? this.getElementsByTagName('option') : undefined; }
  get multiple() { return this.hasAttribute('multiple'); }
  set multiple(x) { if (x) this.setAttribute('multiple', ''); else this.removeAttribute('multiple'); }
  get selectedOptions() { return Array.from(this.getElementsByTagName('option')).filter((o) => o.selected); }
  get selectedIndex() {
    const list = Array.from(this.getElementsByTagName('option'));
    const i = list.findIndex((o) => o.selected);
    if (i >= 0) return i;
    return list.length && !this.multiple ? 0 : -1;
  }
  set selectedIndex(idx) { Array.from(this.getElementsByTagName('option')).forEach((o, i) => { o.selected = (i === Number(idx)); }); }

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

  click() {
    // WHATWG "click in progress" flag: a nested .click() on the same element
    // (e.g. a parent onClick that re-clicks this element on the bubble path) is
    // a no-op, which breaks the otherwise-infinite re-entrancy.
    if (this.__clickInProgress) return;
    this.__clickInProgress = true;
    try { this.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, detail: 1 })); }
    finally { this.__clickInProgress = false; }
  }
  // pre-click activation (runs BEFORE click listeners; undone if preventDefault)
  __preClickActivation() {
    if (this.localName !== 'input') return null;
    const t = (this.getAttribute('type') || 'text').toLowerCase();
    // Write the internal field directly — NOT via the `checked` setter, which
    // React wraps with its value-tracker. Going through the setter would update
    // React's tracked value too, hiding the change and suppressing onChange.
    if (t === 'checkbox') {
      const old = this.checked; this.__checked = !old;
      return { undo: () => { this.__checked = old; } };
    }
    if (t === 'radio') {
      if (this.checked) return null;
      const group = this.__radioGroup();
      const prev = group.find((r) => r.checked) || null;
      group.forEach((r) => { r.__checked = false; });
      this.__checked = true;
      return { undo: () => { this.__checked = false; if (prev) prev.__checked = true; } };
    }
    return null;
  }
  __radioGroup() {
    const name = this.getAttribute('name');
    let root = this; while (root.parentNode) root = root.parentNode;
    const form = this.closest && this.closest('form');
    const scope = form || root;
    if (!name) return [this];
    return (scope.getElementsByTagName ? Array.from(scope.getElementsByTagName('input')) : [])
      .filter((i) => i.localName === 'input' && (i.getAttribute('type') || '').toLowerCase() === 'radio' && i.getAttribute('name') === name);
  }

  // default actions applied post-dispatch when not preventDefault'd
  __runDefaultAction(e) {
    if (e.type !== 'click') return;
    if (this.localName === 'input') {
      const t = (this.getAttribute('type') || 'text').toLowerCase();
      if (t === 'submit') { const f = this.closest('form'); if (f) f.requestSubmit(); }
    } else if (this.localName === 'button') {
      const t = (this.getAttribute('type') || 'submit').toLowerCase();
      if (t === 'submit') { const f = this.closest('form'); if (f) f.requestSubmit(); }
    } else if (this.localName === 'label') {
      const c = this.control;
      if (c && c !== e.target) c.click();
    }
  }
  focus() {
    const doc = this.ownerDocument;
    const prev = doc.__active;
    if (prev === this) return;
    // moving focus blurs the previously-focused element first
    if (prev && prev !== doc.body && typeof prev.dispatchEvent === 'function') {
      doc.__active = null;
      prev.dispatchEvent(new FocusEvent('blur', { relatedTarget: this }));
      prev.dispatchEvent(new FocusEvent('focusout', { bubbles: true, relatedTarget: this }));
    }
    doc.__setActive(this);
    this.dispatchEvent(new FocusEvent('focus', { relatedTarget: prev || null }));
    this.dispatchEvent(new FocusEvent('focusin', { bubbles: true, relatedTarget: prev || null }));
  }
  blur() {
    const doc = this.ownerDocument;
    if (doc.__active !== this) return;
    doc.__setActive(doc.body);
    this.dispatchEvent(new FocusEvent('blur'));
    this.dispatchEvent(new FocusEvent('focusout', { bubbles: true }));
  }
  getBoundingClientRect() { return zeroRect(); }
  getClientRects() { return []; }
  scrollIntoView() {}
  scroll() {} scrollTo() {} scrollBy() {}
  // honest zero geometry (no layout)
  get offsetWidth() { return 0; } get offsetHeight() { return 0; }
  get offsetTop() { return 0; } get offsetLeft() { return 0; }
  get offsetParent() { return null; }
  get clientWidth() { return 0; } get clientHeight() { return 0; }
  get clientTop() { return 0; } get clientLeft() { return 0; }
  get scrollWidth() { return 0; } get scrollHeight() { return 0; }
  get scrollTop() { return 0; } set scrollTop(_v) {} get scrollLeft() { return 0; } set scrollLeft(_v) {}

  // namespaced attributes
  getAttributeNS(_ns, name) { return this.getAttribute(name); }
  setAttributeNS(_ns, name, value) { this.setAttribute(name, value); }
  hasAttributeNS(_ns, name) { return this.hasAttribute(name); }
  removeAttributeNS(_ns, name) { this.removeAttribute(name); }
  getAttributeNode(name) { const a = this.__attrs.find((x) => x.name === name); return a ? { name: a.name, value: a.value, ownerElement: this } : null; }

  // adjacency
  insertAdjacentElement(position, el) {
    const p = this.parentNode;
    switch (position) {
      case 'beforebegin': if (p) p.insertBefore(el, this); break;
      case 'afterbegin': this.insertBefore(el, this.firstChild); break;
      case 'beforeend': this.appendChild(el); break;
      case 'afterend': if (p) p.insertBefore(el, this.nextSibling); break;
    }
    return el;
  }
  insertAdjacentText(position, text) { this.insertAdjacentElement(position, this.ownerDocument.createTextNode(text)); }

  // commonly-reflected properties
  get tabIndex() { return this.hasAttribute('tabindex') ? parseInt(this.getAttribute('tabindex'), 10) || 0 : (/^(a|button|input|select|textarea)$/.test(this.localName) ? 0 : -1); }
  set tabIndex(v) { this.setAttribute('tabindex', String(v)); }
  get title() { return this.getAttribute('title') ?? ''; } set title(v) { this.setAttribute('title', v); }
  get lang() { return this.getAttribute('lang') ?? ''; } set lang(v) { this.setAttribute('lang', v); }
  get dir() { return this.getAttribute('dir') ?? ''; } set dir(v) { this.setAttribute('dir', v); }
  get hidden() { return this.hasAttribute('hidden'); } set hidden(v) { if (v) this.setAttribute('hidden', ''); else this.removeAttribute('hidden'); }
  get role() { return this.getAttribute('role'); } set role(v) { this.setAttribute('role', v); }
  get contentEditable() { return this.getAttribute('contenteditable') ?? 'inherit'; } set contentEditable(v) { this.setAttribute('contenteditable', v); }
  get isContentEditable() { return this.getAttribute('contenteditable') === 'true' || this.getAttribute('contenteditable') === ''; }
  get hreflang() { return this.getAttribute('hreflang') ?? ''; }
  get target() { return this.getAttribute('target') ?? ''; } set target(v) { this.setAttribute('target', v); }

  // pointer capture + animations (no-op honest stubs)
  setPointerCapture() {} releasePointerCapture() {} hasPointerCapture() { return false; }
  animate() { return { play() {}, pause() {}, cancel() {}, finish() {}, finished: Promise.resolve(), onfinish: null, cancel_: null }; }
  getAnimations() { return []; }
  requestFullscreen() { return Promise.resolve(); }

  // canvas (no raster backend — honest no-op context)
  getContext(type) { return this.localName === 'canvas' ? (this.__ctx ||= makeCanvasStub()) : null; }
  toDataURL() { return 'data:,'; }

  // shadow DOM (open by default; a detached fragment with a host back-reference)
  attachShadow(init = {}) {
    const root = new DocumentFragment(this.ownerDocument);
    root.host = this;
    root.mode = init.mode || 'open';
    root.querySelector = (s) => qsel(root, s);
    root.querySelectorAll = (s) => qselAll(root, s);
    this.__shadow = root;
    if (root.mode === 'open') this.shadowRoot = root;
    return root;
  }

  // forms
  get form() { return this.closest ? this.closest('form') : null; }
  get elements() {
    if (this.localName !== 'form') return undefined;
    return liveHTMLCollection(() => collectByTag(this, '*').filter((e) => /^(input|select|textarea|button|fieldset|output)$/.test(e.localName)));
  }
  submit() { if (this.localName === 'form') this.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); }
  requestSubmit() { this.submit(); }
  reset() {
    if (this.localName !== 'form') return;
    this.dispatchEvent(new Event('reset', { bubbles: true, cancelable: true }));
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

// Range — functional enough for selection bookkeeping (user-event), zero geometry.
class Range {
  constructor(doc) {
    this.__doc = doc;
    this.startContainer = doc; this.endContainer = doc;
    this.startOffset = 0; this.endOffset = 0; this.collapsed = true;
  }
  setStart(node, offset) { this.startContainer = node; this.startOffset = offset; this.__sync(); }
  setEnd(node, offset) { this.endContainer = node; this.endOffset = offset; this.__sync(); }
  setStartBefore(node) { this.setStart(node.parentNode, 0); }
  setStartAfter(node) { this.setStart(node.parentNode, 0); }
  setEndBefore(node) { this.setEnd(node.parentNode, 0); }
  setEndAfter(node) { this.setEnd(node.parentNode, 0); }
  selectNode(node) { this.startContainer = this.endContainer = node; this.__sync(); }
  selectNodeContents(node) { this.startContainer = this.endContainer = node; this.startOffset = 0; this.endOffset = node.childNodes ? node.childNodes.length : 0; this.__sync(); }
  collapse(toStart) { if (toStart) { this.endContainer = this.startContainer; this.endOffset = this.startOffset; } else { this.startContainer = this.endContainer; this.startOffset = this.endOffset; } this.collapsed = true; }
  __sync() { this.collapsed = this.startContainer === this.endContainer && this.startOffset === this.endOffset; }
  get commonAncestorContainer() { return this.startContainer; }
  cloneRange() { const r = new Range(this.__doc); Object.assign(r, this); return r; }
  cloneContents() { return this.__doc.createDocumentFragment(); }
  deleteContents() {}
  insertNode(node) { if (this.startContainer && this.startContainer.insertBefore) this.startContainer.insertBefore(node, this.startContainer.childNodes[this.startOffset] ?? null); }
  surroundContents(node) { this.insertNode(node); }
  getBoundingClientRect() { return zeroRect(); }
  getClientRects() { return []; }
  detach() {}
}

function makeSelection() {
  let ranges = [];
  return {
    get rangeCount() { return ranges.length; },
    get isCollapsed() { return ranges.every((r) => r.collapsed); },
    get anchorNode() { return ranges[0] ? ranges[0].startContainer : null; },
    get focusNode() { return ranges[0] ? ranges[0].endContainer : null; },
    get anchorOffset() { return ranges[0] ? ranges[0].startOffset : 0; },
    get focusOffset() { return ranges[0] ? ranges[0].endOffset : 0; },
    get type() { return ranges.length ? 'Range' : 'None'; },
    addRange(r) { ranges.push(r); },
    removeAllRanges() { ranges = []; },
    removeRange(r) { ranges = ranges.filter((x) => x !== r); },
    getRangeAt(i) { return ranges[i]; },
    collapse() {}, extend() {}, selectAllChildren() {}, setBaseAndExtent() {}, empty() { ranges = []; },
    toString() { return ''; },
  };
}

// TreeWalker / NodeIterator over the DOM (doubles for both — common subset).
class TreeWalker {
  constructor(root, whatToShow, filter) {
    this.root = root; this.whatToShow = whatToShow >>> 0; this.filter = filter; this.currentNode = root;
    this.referenceNode = root; this.pointerBeforeReferenceNode = true;
  }
  __show(node) {
    const bit = node.nodeType === ELEMENT_NODE ? 1 : node.nodeType === TEXT_NODE ? 4 : node.nodeType === COMMENT_NODE ? 128 : 0xffffffff;
    if (!(this.whatToShow & bit) && this.whatToShow !== 0xffffffff) return 3; // FILTER_SKIP
    if (this.filter) {
      const fn = typeof this.filter === 'function' ? this.filter : this.filter.acceptNode;
      return fn.call(this.filter, node);
    }
    return 1; // FILTER_ACCEPT
  }
  __flat() {
    const out = [];
    (function walk(n) { for (const c of (n.__children ? n.__children() : [])) { out.push(c); walk(c); } })(this.root);
    return out.filter((n) => this.__show(n) === 1);
  }
  nextNode() { const all = this.__flat(); const i = all.indexOf(this.currentNode); const next = all[i + 1] ?? (this.currentNode === this.root ? all[0] : null); this.currentNode = next; this.referenceNode = next; return next ?? null; }
  previousNode() { const all = this.__flat(); const i = all.indexOf(this.currentNode); const prev = i > 0 ? all[i - 1] : null; if (prev) { this.currentNode = prev; this.referenceNode = prev; } return prev ?? null; }
  firstChild() { const k = (this.currentNode.__children ? this.currentNode.__children() : []).filter((n) => this.__show(n) === 1); if (k[0]) { this.currentNode = k[0]; return k[0]; } return null; }
  lastChild() { const k = (this.currentNode.__children ? this.currentNode.__children() : []).filter((n) => this.__show(n) === 1); const last = k[k.length - 1]; if (last) { this.currentNode = last; return last; } return null; }
  parentNode() { const p = this.currentNode.parentNode; if (p && p !== this.root.parentNode) { this.currentNode = p; return p; } return null; }
  nextSibling() { let n = this.currentNode.nextSibling; while (n && this.__show(n) !== 1) n = n.nextSibling; if (n) this.currentNode = n; return n ?? null; }
  previousSibling() { let n = this.currentNode.previousSibling; while (n && this.__show(n) !== 1) n = n.previousSibling; if (n) this.currentNode = n; return n ?? null; }
  detach() {}
}

// DOMParser — parses a string into a real Document via the native parser.
export class DOMParser {
  parseFromString(str, type = 'text/html') {
    if (type === 'text/html') return parseDocument(String(str));
    // XML-ish: parse as a fragment wrapped in a document
    const doc = parseDocument(`<!doctype html><html><body>${str}</body></html>`);
    return doc;
  }
}

// XMLSerializer — serializes a node back to markup.
export class XMLSerializer {
  serializeToString(node) {
    if (node.nodeType === DOCUMENT_NODE || node.nodeType === DOCUMENT_FRAGMENT_NODE) return serializeInner(node);
    return serializeOuter(node);
  }
}

// minimal inline-style CSSOM (honest: only inline + explicitly set props)
function makeStyle(el) {
  // parse → { values: Map<prop,value>, prio: Map<prop,'important'|''> } (handles !important)
  const parse = () => {
    const values = new Map();
    const prio = new Map();
    for (const decl of (el.getAttribute('style') || '').split(';')) {
      const i = decl.indexOf(':');
      if (i === -1) continue;
      const prop = decl.slice(0, i).trim();
      let val = decl.slice(i + 1).trim();
      if (!prop) continue;
      const m = /\s*!\s*important\s*$/i.exec(val);
      if (m) { val = val.slice(0, m.index).trim(); prio.set(prop, 'important'); }
      values.set(prop, val);
    }
    return { values, prio };
  };
  const write = ({ values, prio }) => {
    el.setAttribute('style', [...values].map(([k, v]) => `${k}: ${v}${prio.get(k) === 'important' ? ' !important' : ''}`).join('; '));
  };
  return new Proxy({}, {
    get(_t, key) {
      if (key === 'getPropertyValue') return (p) => parse().values.get(p) ?? '';
      if (key === 'getPropertyPriority') return (p) => parse().prio.get(p) ?? '';
      if (key === 'setProperty') return (p, v, priority) => { const s = parse(); s.values.set(p, String(v)); if (priority) s.prio.set(p, 'important'); else s.prio.delete(p); write(s); };
      if (key === 'removeProperty') return (p) => { const s = parse(); const v = s.values.get(p) ?? ''; s.values.delete(p); s.prio.delete(p); write(s); return v; };
      if (key === 'item') return (i) => [...parse().values.keys()][i] ?? '';
      if (key === 'length') return parse().values.size;
      if (key === 'cssText') return el.getAttribute('style') || '';
      if (key === 'cssFloat') return parse().values.get('float') ?? '';
      if (key === Symbol.iterator) { const keys = [...parse().values.keys()]; return keys[Symbol.iterator].bind(keys); }
      if (typeof key !== 'string') return undefined;
      return parse().values.get(kebab(key)) ?? '';
    },
    set(_t, key, value) {
      if (key === 'cssText') { el.setAttribute('style', String(value)); return true; }
      const s = parse(); s.values.set(kebab(key), String(value)); write(s); return true;
    },
    has(_t, key) { return typeof key === 'string'; },
  });
}
const kebab = (s) => s.replace(/[A-Z]/g, (m) => '-' + m.toLowerCase());

// WHATWG value-sanitization per input type. Returns the accepted value, or null
// if invalid (caller leaves the current value unchanged — like a real browser,
// which rejects partial/garbage input rather than storing it).
function sanitizeInputValue(type, v) {
  if (v === '') return '';
  switch (type) {
    case 'date': return /^\d{4}-\d{2}-\d{2}$/.test(v) ? v : null;
    case 'month': return /^\d{4}-\d{2}$/.test(v) ? v : null;
    case 'week': return /^\d{4}-W\d{2}$/.test(v) ? v : null;
    case 'time': return /^\d{2}:\d{2}(:\d{2}(\.\d{1,3})?)?$/.test(v) ? v : null;
    case 'datetime-local': return /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(:\d{2})?$/.test(v) ? v : null;
    case 'number': return /^-?\d*\.?\d+(e[-+]?\d+)?$/i.test(v) ? v : null;
    case 'range': return /^-?\d*\.?\d+$/.test(v) ? v : '50';
    case 'color': return /^#[0-9a-fA-F]{6}$/.test(v) ? v.toLowerCase() : null;
    default: return v; // text, email, password, search, url, tel, hidden, etc.
  }
}

// element.dataset — camelCase <-> data-* attribute mapping.
const dataAttr = (key) => 'data-' + key.replace(/[A-Z]/g, (m) => '-' + m.toLowerCase());
const dataKey = (attr) => attr.slice(5).replace(/-([a-z])/g, (_, c) => c.toUpperCase());
function makeDataset(el) {
  return new Proxy({}, {
    get(_t, k) { if (typeof k !== 'string') return undefined; const v = el.getAttribute(dataAttr(k)); return v === null ? undefined : v; },
    set(_t, k, v) { el.setAttribute(dataAttr(k), String(v)); return true; },
    deleteProperty(_t, k) { el.removeAttribute(dataAttr(k)); return true; },
    has(_t, k) { return el.hasAttribute(dataAttr(k)); },
    ownKeys() { return el.getAttributeNames().filter((n) => n.startsWith('data-')).map(dataKey); },
    getOwnPropertyDescriptor(_t, k) {
      const n = dataAttr(k);
      if (el.hasAttribute(n)) return { configurable: true, enumerable: true, value: el.getAttribute(n) };
      return undefined;
    },
  });
}

// --- MutationObserver, wired to the mutation methods above via notifyMutation ---
function isDescendant(node, ancestor) {
  let n = node;
  while (n) { if (n === ancestor) return true; n = n.parentNode; }
  return false;
}
function notifyMutation(target, record) {
  const doc = target.ownerDocument;
  if (!doc || !doc.__mo || doc.__mo.length === 0) return;
  for (const reg of doc.__mo) {
    const { obs, target: obsTarget, options } = reg;
    const onTarget = record.target === obsTarget;
    const inSubtree = options.subtree && isDescendant(record.target, obsTarget);
    if (!onTarget && !inSubtree) continue;
    if (record.type === 'childList' && !options.childList) continue;
    if (record.type === 'attributes' && !options.attributes) continue;
    if (record.type === 'attributes' && options.attributeFilter && !options.attributeFilter.includes(record.attributeName)) continue;
    if (record.type === 'characterData' && !options.characterData) continue;
    const rec = {
      type: record.type, target: record.target,
      addedNodes: record.addedNodes || [], removedNodes: record.removedNodes || [],
      previousSibling: record.previousSibling || null, nextSibling: record.nextSibling || null,
      attributeName: record.attributeName || null, attributeNamespace: null,
      oldValue: (record.type === 'attributes' && options.attributeOldValue) ||
                (record.type === 'characterData' && options.characterDataOldValue) ? (record.oldValue ?? null) : null,
    };
    obs.__enqueue(rec);
    doc.__scheduleMO(obs);
  }
}

export class MutationObserver {
  constructor(callback) { this.__cb = callback; this.__records = []; this.__regs = []; }
  observe(target, options = {}) {
    const opts = {
      childList: !!options.childList,
      attributes: options.attributes ?? (options.attributeFilter || options.attributeOldValue ? true : false),
      characterData: options.characterData ?? (options.characterDataOldValue ? true : false),
      subtree: !!options.subtree,
      attributeOldValue: !!options.attributeOldValue,
      characterDataOldValue: !!options.characterDataOldValue,
      attributeFilter: options.attributeFilter || null,
    };
    const doc = target.ownerDocument || target;
    doc.__moRegister(this, target, opts);
    this.__regs.push(doc);
  }
  disconnect() { for (const doc of this.__regs) doc.__moUnregister(this); this.__regs = []; this.__records = []; }
  takeRecords() { const r = this.__records; this.__records = []; return r; }
  __enqueue(rec) { this.__records.push(rec); }
}

// ----------------------------------------------------------- Document ----
export class Document extends Node {
  constructor() {
    super(null);
    this.ownerDocument = this;
    this.__buf = null;            // SoA buffer accessor
    this.__cache = [];            // idx -> handle (identity memoization / nodeAt)
    this.__active = null;         // activeElement
    this.defaultView = null;      // set by environment (window)
    this.__mo = [];               // registered MutationObservers
    this.__moPending = null;      // observers with queued records awaiting microtask
  }
  get nodeType() { return DOCUMENT_NODE; }
  get nodeName() { return '#document'; }

  // ---- MutationObserver registry ----
  __moRegister(obs, target, options) {
    // replace existing registration for (obs,target) per spec
    this.__mo = this.__mo.filter((r) => !(r.obs === obs && r.target === target));
    this.__mo.push({ obs, target, options });
  }
  __moUnregister(obs) { this.__mo = this.__mo.filter((r) => r.obs !== obs); }
  __scheduleMO(obs) {
    if (!this.__moPending) this.__moPending = new Set();
    if (this.__moPending.has(obs)) return;
    this.__moPending.add(obs);
    queueMicrotask(() => {
      this.__moPending.delete(obs);
      const recs = obs.takeRecords();
      if (recs.length) { try { obs.__cb(recs, obs); } catch (e) { /* observer callbacks must not break the mutator */ } }
    });
  }

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
    this.__mo = [];          // drop observers
    this.__moPending = null;
  }

  get documentElement() { return this.__children().find((n) => n.nodeType === ELEMENT_NODE && n.localName === 'html') ?? null; }
  get doctype() { return this.__children().find((n) => n.nodeType === DOCUMENT_TYPE_NODE) ?? null; }
  get head() { const html = this.documentElement; return html ? html.__children().find((n) => n.localName === 'head') ?? null : null; }
  get body() { const html = this.documentElement; return html ? html.__children().find((n) => n.localName === 'body') ?? null : null; }
  get activeElement() { return this.__active || this.body || null; }
  __setActive(el) { this.__active = el; }

  // ---- factories (owned nodes, no buffer) ----
  createElement(tag) { return new Element(this, String(tag).toLowerCase(), ''); }
  createElementNS(ns, qualified) {
    const short = ns === SVG_NS ? 'svg' : ns === MATHML_NS ? 'math' : '';
    const local = qualified.includes(':') ? qualified.split(':')[1] : qualified;
    return new Element(this, local, short);
  }
  createTextNode(data) { return new Text(this, String(data)); }
  createComment(data) { return new Comment(this, String(data)); }
  createDocumentFragment() { return new DocumentFragment(this); }
  createEvent(type = 'Event') {
    switch (String(type)) {
      case 'CustomEvent': return new CustomEvent('');
      case 'MouseEvent': case 'MouseEvents': return new MouseEvent('');
      case 'KeyboardEvent': case 'KeyEvents': return new KeyboardEvent('');
      case 'UIEvent': case 'UIEvents': return new UIEvent('');
      case 'FocusEvent': return new FocusEvent('');
      default: return new Event('');
    }
  }
  createRange() { return new Range(this); }
  createAttribute(name) { return { name, value: '', ownerElement: null }; }
  createComment(data) { return new Comment(this, String(data)); }
  getSelection() { if (!this.__selection) this.__selection = makeSelection(); return this.__selection; }
  importNode(node, deep) { return node.cloneNode(deep); }
  adoptNode(node) { if (node.parentNode) node.parentNode.removeChild(node); node.ownerDocument = this; return node; }

  // TreeWalker / NodeIterator (whatToShow: 1 elements, 4 text, 0xFFFFFFFF all)
  createTreeWalker(root, whatToShow = 0xffffffff, filter = null) { return new TreeWalker(root, whatToShow, filter); }
  createNodeIterator(root, whatToShow = 0xffffffff, filter = null) { return new TreeWalker(root, whatToShow, filter); }

  getElementsByName(name) {
    const self = this; return liveHTMLCollection(() => collectByTag(self, '*').filter((e) => e.getAttribute('name') === name));
  }
  elementFromPoint() { return null; }
  elementsFromPoint() { return []; }
  execCommand() { return false; }
  queryCommandSupported() { return false; }
  queryCommandEnabled() { return false; }
  hasFocus() { return true; }
  write() {} writeln() {} open() { return this; } close() {}

  get title() { const t = this.querySelector('title'); return t ? t.textContent : (this.__title || ''); }
  set title(v) { const t = this.querySelector('title'); if (t) t.textContent = v; else this.__title = String(v); }
  get location() { return this.defaultView ? this.defaultView.location : null; }
  get baseURI() { return (this.defaultView && this.defaultView.location && this.defaultView.location.href) || 'about:blank'; }
  get URL() { return this.baseURI; }
  get documentURI() { return this.baseURI; }
  get scrollingElement() { return this.documentElement; }
  get fullscreenElement() { return null; }
  get implementation() {
    const self = this;
    return {
      createHTMLDocument(title) { const d = parseDocument(`<!doctype html><html><head><title>${title || ''}</title></head><body></body></html>`); return d; },
      createDocumentType: (name, pub, sys) => new DocumentType(self, name, pub, sys),
      hasFeature: () => true,
    };
  }

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

  // cookie jar: store name=value, strip attributes (path/Secure/SameSite/…),
  // dedupe by name, honor deletion via max-age<=0 or a past expires.
  get cookie() {
    if (!this.__cookieJar) return '';
    return [...this.__cookieJar].map(([k, v]) => `${k}=${v}`).join('; ');
  }
  set cookie(str) {
    if (!this.__cookieJar) this.__cookieJar = new Map();
    const parts = String(str).split(';');
    const pair = parts.shift();
    const eq = pair.indexOf('=');
    if (eq === -1) return;
    const name = pair.slice(0, eq).trim();
    if (!name) return;
    const value = pair.slice(eq + 1).trim();
    const attrs = parts.map((a) => a.trim().toLowerCase());
    const maxAge = attrs.find((a) => a.startsWith('max-age='));
    const expires = attrs.find((a) => a.startsWith('expires='));
    const deleted = (maxAge && parseInt(maxAge.slice(8), 10) <= 0) ||
      (expires && new Date(expires.slice(8)).getTime() <= Date.now());
    if (deleted) this.__cookieJar.delete(name);
    else this.__cookieJar.set(name, value);
  }

  // document state (honest defaults for a headless, focused, loaded page)
  get visibilityState() { return 'visible'; }
  get hidden() { return false; }
  get readyState() { return 'complete'; }
  hasFocus() { return true; }
  get characterSet() { return 'UTF-8'; }
  get compatMode() { return 'CSS1Compat'; }
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
