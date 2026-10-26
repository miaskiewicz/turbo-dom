// Layer 2 — lazy node inflation + copy-on-write tree + live collections + identity.
//
// The native parser hands us an immutable nested "buffer" (plain JS objects).
// DOM node handles inflate from it lazily: a node's children aren't built until
// something reads them. First access memoizes the handle (=== identity preserved).
// Mutation promotes the affected node to fully-owned (COW). Reads are transparent
// across the boundary — a buffer-backed read and an owned read are indistinguishable.

import { getParser } from './parser.mjs';
import {
  EventTarget, Event, CustomEvent,
  UIEvent, MouseEvent, KeyboardEvent, FocusEvent,
} from './events.mjs';
import { liveNodeList, liveHTMLCollection, liveNamedNodeMap } from './collections.mjs';
import { matchesSelector, querySelector as qsel, querySelectorAll as qselAll } from './selectors.mjs';
import { serializeInner, serializeOuter } from './html-serialize.mjs';
import { CSSStyleSheet, styleSheetList } from './cssom.mjs';
import { COLOR_PROPS, canonicalizeColor } from './color.mjs';
import { blockText, canonical, declarations, expand, longhandAlias, longhands, shorthandValue } from './css-shorthand.mjs';
import { SVGAnimatedString, SVGAnimatedLength, SVGAnimatedRect, SVG_LENGTH_ATTRS } from './svg.mjs';

// Per-node query-result cache keyed by (selector, document version). querySelectorAll
// returns a STATIC list per spec, so caching is safe until the next mutation bumps
// Document.__version (which invalidates every cached query). Big win for the repeated
// identical queries RTL/findBy/waitFor run against an unchanged tree.
function cachedQSA(node, sel) {
  const doc = node.ownerDocument || node;
  const v = doc.__version || 0;
  let cache = node.__qaCache;
  if (cache) { const c = cache.get(sel); if (c !== undefined && c.v === v) return c.r; }
  else cache = node.__qaCache = new Map();
  if (cache.size > 512) cache.clear();
  const r = qselAll(node, sel);
  cache.set(sel, { v, r });
  return r;
}
// memoize the className → class-list split (pure; the regex split showed up per
// getElementsByClassName call in profiles)
const __classSplit = new Map();
function splitClasses(cls) {
  let c = __classSplit.get(cls);
  if (c === undefined) {
    c = cls.split(/\s+/).filter(Boolean);
    if (__classSplit.size > 2000) __classSplit.clear();
    __classSplit.set(cls, c);
  }
  return c;
}
function cachedQS(node, sel) {
  const doc = node.ownerDocument || node;
  const v = doc.__version || 0;
  let cache = node.__qsCache;
  if (cache) { const c = cache.get(sel); if (c !== undefined && c.v === v) return c.r; }
  else cache = node.__qsCache = new Map();
  if (cache.size > 512) cache.clear();
  const r = qsel(node, sel);
  cache.set(sel, { v, r });
  return r;
}
import { Buffer } from './buffer.mjs';
import { makeCanvasStub } from './stubs.mjs';
import {
  constructCustomElement, createCustomElement, clonedElement, insertedNode, parsedFragment, removedNode,
  attributeChanged, adoptedNode, freezeParsedIs, parsedIsAttribute, reactionDepth, withReactions,
} from './custom-elements.mjs';

// Custom element reactions ([CEReactions], custom-elements.mjs): a DOM call that can queue
// reactions re-enters itself inside one reaction scope when `doc` has the hooks armed and no
// scope is open yet, so its reactions run when it returns. One predicted-false read
// otherwise. `METHOD` holds the original implementations (spy-proof re-entry).
const ceScope = (doc) => doc.__ceRegistry !== undefined && reactionDepth === 0;
const METHOD = {};

// DOM versions come from ONE process-wide counter, so a version value is never reused by
// another document: a node adopted into another document cannot hit a cache entry (keyed on
// its old document's version) by coincidence (miaskiewicz/turbo-dom#29 item 2).
let VERSION = 0;

export const ELEMENT_NODE = 1;
export const TEXT_NODE = 3;
export const COMMENT_NODE = 8;
export const DOCUMENT_NODE = 9;
export const DOCUMENT_TYPE_NODE = 10;
export const DOCUMENT_FRAGMENT_NODE = 11;

const SVG_NS = 'http://www.w3.org/2000/svg';
const MATHML_NS = 'http://www.w3.org/1998/Math/MathML';
const HTML_NS = 'http://www.w3.org/1999/xhtml';
const XML_NS = 'http://www.w3.org/XML/1998/namespace';
const XMLNS_NS = 'http://www.w3.org/2000/xmlns/';
// An element's `__ns` is '' (HTML), 'svg' or 'math' — what the parser produces — or, for
// createElementNS in any other namespace, 'ns:' + the namespace URI ('ns:' alone is the
// null namespace). Every '' / truthy test ("HTML or not") stays a single string check.
const nsUri = (short) => (short === '' ? HTML_NS : short === 'svg' ? SVG_NS : short === 'math' ? MATHML_NS
  : short.length > 3 ? short.slice(3) : null);
const nsShort = (uri) => (uri === HTML_NS ? '' : uri === SVG_NS ? 'svg' : uri === MATHML_NS ? 'math' : 'ns:' + (uri ?? ''));

// A DOMException (a plain Error with its name where the host has none, e.g. a bare V8).
function domError(name, message) {
  if (typeof DOMException === 'function') return new DOMException(message, name);
  const error = new Error(message);
  error.name = name;
  return error;
}
// DOM "valid element local name" (createElement / createElementNS, Chrome 156): an ASCII
// letter followed by anything but ASCII whitespace, NULL, '/' and '>'; or ':', '_' or a
// non-ASCII code point followed by ASCII alphanumerics, '-', '.', ':', '_' and non-ASCII.
function validLocalName(name) {
  const n = name.length;
  if (n === 0) return false;
  const c0 = name.charCodeAt(0) | 0x20;
  if (c0 >= 97 && c0 <= 122) {
    for (let i = 1; i < n; i++) {
      const c = name.charCodeAt(i);
      if (c <= 32 ? c === 32 || c === 9 || c === 10 || c === 12 || c === 13 || c === 0 : c === 47 || c === 62) return false;
    }
    return true;
  }
  const first = name.charCodeAt(0);
  if (first !== 58 && first !== 95 && first < 0x80) return false;
  for (let i = 1; i < n; i++) {
    const c = name.charCodeAt(i);
    if (c >= 0x80 || c === 45 || c === 46 || c === 58 || c === 95 || (c >= 48 && c <= 57) || ((c | 0x20) >= 97 && (c | 0x20) <= 122)) continue;
    return false;
  }
  return true;
}
// DOM "valid namespace prefix": non-empty, no ASCII whitespace, NULL, '/' or '>'.
function validPrefix(prefix) {
  if (prefix.length === 0) return false;
  for (let i = 0; i < prefix.length; i++) {
    const c = prefix.charCodeAt(i);
    if (c === 32 || c === 9 || c === 10 || c === 12 || c === 13 || c === 0 || c === 47 || c === 62) return false;
  }
  return true;
}
const invalidName = (name) => domError('InvalidCharacterError', `The qualified name provided ('${name}') contains the invalid name-start character or invalid character.`);

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
        const nt = buf.nodeType(c); // read once, pass to __nodeAt (was re-read in its switch)
        // template content fragment is not a child — it's `.content`
        if (nt === DOCUMENT_FRAGMENT_NODE && buf.tagName(c) === 'content') continue;
        const child = doc.__nodeAt(c, nt);
        child.parentNode = this;
        kids.push(child);
      }
    }
    this.__kids = kids;
    return kids;
  }

  get childNodes() {
    // the NodeList reads __children() live, so one cached object per node is
    // always correct — avoids re-allocating a Proxy on every .childNodes access
    // (React/RTL hit this constantly).
    if (this.__childNodesList) return this.__childNodesList;
    const self = this;
    return (this.__childNodesList = liveNodeList(() => self.__children()));
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
    if ((this.ownerDocument.__ceRegistry !== undefined || node.ownerDocument.__ceRegistry !== undefined) && reactionDepth === 0) return withReactions(METHOD.insertBefore, this, [node, ref]);
    // DOM "pre-insert": check the reference child before touching the tree, and a node
    // inserted before itself goes before its next sibling (miaskiewicz/turbo-dom#34)
    if (ref && ref.parentNode !== this) throw new Error('NotFoundError: ref is not a child');
    if (ref === node) ref = node.nextSibling;
    if (node.nodeType === DOCUMENT_FRAGMENT_NODE) {
      for (const c of node.__children().slice()) this.insertBefore(c, ref);
      return node;
    }
    if (node.parentNode) node.parentNode.removeChild(node);
    const kids = this.__children();
    const i = ref ? kids.indexOf(ref) : -1;
    // a node from another document is adopted first (DOM "insert" step 7.1)
    if (node.ownerDocument !== this.ownerDocument) adopt(node, this.ownerDocument);
    if (ref) kids.splice(i, 0, node); else kids.push(node);
    node.parentNode = this;
    notifyMutation(this, 'childList', node, null, ref || null);
    // custom elements (custom-elements.mjs): upgrades + connectedCallback, only once a
    // registry exists — one predicted-false read otherwise
    const doc = this.ownerDocument;
    if (doc !== null && doc.__ceRegistry !== undefined) insertedNode(node);
    return node;
  }

  removeChild(node) {
    if (this.ownerDocument.__ceRegistry !== undefined && reactionDepth === 0) return withReactions(METHOD.removeChild, this, [node]);
    const kids = this.__children();
    const i = kids.indexOf(node);
    if (i === -1) throw new Error('NotFoundError: node is not a child');
    const next = kids[i + 1] || null;
    const doc = this.ownerDocument, ce = doc !== null && doc.__ceRegistry !== undefined && this.isConnected;
    kids.splice(i, 1);
    node.parentNode = null;
    notifyMutation(this, 'childList', null, node, next);
    if (ce) removedNode(node);
    return node;
  }

  // DOM "replace": remove the old child, then insert the new one before the old child's
  // next sibling (or the new node's own next sibling when that is it) — so replacing a
  // node with itself leaves it in place (miaskiewicz/turbo-dom#34)
  replaceChild(newNode, oldNode) {
    if ((this.ownerDocument.__ceRegistry !== undefined || newNode.ownerDocument.__ceRegistry !== undefined) && reactionDepth === 0) return withReactions(METHOD.replaceChild, this, [newNode, oldNode]);
    if (oldNode.parentNode !== this) throw new Error('NotFoundError: node is not a child');
    let ref = oldNode.nextSibling;
    if (ref === newNode) ref = newNode.nextSibling;
    this.removeChild(oldNode);
    this.insertBefore(newNode, ref);
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
    for (;;) {
      while (n.parentNode) n = n.parentNode;
      // a node inside a shadow tree is connected iff its host is connected
      if (n.__isShadowRoot) { n = n.host; continue; }
      return n.nodeType === DOCUMENT_NODE;
    }
  }
  getRootNode(options) {
    let n = this;
    if (options && options.composed) {
      // climb through shadow boundaries to the topmost root (document/detached)
      for (;;) {
        while (n.parentNode) n = n.parentNode;
        if (n.__isShadowRoot) { n = n.host; continue; }
        return n;
      }
    }
    // non-composed: stop at the enclosing shadow root if there is one
    while (n.parentNode) n = n.parentNode;
    return n;
  }
  normalize() {
    const kids = this.__children();
    let changed = false;
    for (let i = kids.length - 1; i > 0; i--) {
      if (kids[i].nodeType === TEXT_NODE && kids[i - 1].nodeType === TEXT_NODE) {
        kids[i - 1]._data += kids[i].data; kids[i].parentNode = null; kids.splice(i, 1); changed = true;
      }
    }
    for (let i = kids.length - 1; i >= 0; i--) {
      if (kids[i].nodeType === TEXT_NODE && kids[i].data === '') { kids[i].parentNode = null; kids.splice(i, 1); changed = true; }
      else if (kids[i].nodeType === ELEMENT_NODE) kids[i].normalize();
    }
    // normalize mutates __kids in place — bump __version (invalidate cached queries)
    // and feed MutationObservers, like every other childList mutation path.
    if (changed) notifyMutation(this, 'childList', null, null, null);
  }
  replaceChildren(...nodes) {
    if (ceScope(this.ownerDocument)) return withReactions(METHOD.replaceChildren, this, nodes);
    this.__detachKids(); this.__kids = []; this.__touch(); for (const n of nodes) this.appendChild(toNode(this.ownerDocument, n));
  }
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
  // cloneNode = DOM "clone a node" into the node's own document; importNode clones into
  // another one. Every node type implements `__clone(document, deep)`.
  cloneNode(deep = false) {
    if (ceScope(this.ownerDocument)) return withReactions(METHOD.cloneNode, this, [deep]);
    return this.__clone(this.ownerDocument, !!deep);
  }

  get textContent() {
    // <style> whose CSSOM sheet received insertRule() (emotion/styled-components
    // "speedy" mode injects rules WITHOUT touching the node's text): reflect those
    // rules in the serialized text, as browsers/jsdom do, so tests that scrape
    // `querySelectorAll('style').map(s => s.textContent)` see the injected CSS.
    // __sheet is only ever set on <style> elements (sheet getter gates on it), so
    // this is one predicted-false read for every other element on the hot path.
    const sheet = this.__sheet;
    if (sheet && sheet.cssRules.length) {
      const kids = this.__children();
      let css = '';
      for (let i = 0; i < kids.length; i++) if (kids[i].nodeType === TEXT_NODE) css += kids[i].data;
      for (let i = 0; i < sheet.cssRules.length; i++) css += (css ? '\n' : '') + sheet.cssRules[i].cssText;
      return css;
    }
    const kids = this.__children();
    // hot leaf case (RTL getByText reads textContent on every element): a single
    // text child → return its data directly, skipping the iterator + concat.
    if (kids.length === 1 && kids[0].nodeType === TEXT_NODE) return kids[0].data;
    let s = '';
    for (let i = 0; i < kids.length; i++) {
      const c = kids[i];
      if (c.nodeType === TEXT_NODE) s += c.data;
      else if (c.nodeType === ELEMENT_NODE || c.nodeType === DOCUMENT_FRAGMENT_NODE) s += c.textContent;
    }
    return s;
  }
  set textContent(value) {
    if (ceScope(this.ownerDocument)) { withReactions(METHOD.setTextContent, this, [value]); return; }
    this.__detachKids();
    this.__kids = [];
    this.__touch();
    if (value !== '') this.appendChild(this.ownerDocument.createTextNode(String(value)));
  }

  // bump the document version so getElementsBy* caches invalidate after direct
  // __kids reassignments (innerHTML/textContent/replaceChildren) that bypass
  // insertBefore/removeChild.
  __touch() { const d = this.ownerDocument; if (d) d.__version = ++VERSION; }

  // Clear the outgoing children's parentNode before a wholesale `__kids = []` rewrite
  // (innerHTML/textContent/replaceChildren) — removeChild does this one at a time, these
  // paths bypass it. Without it a reference held into the replaced subtree keeps reporting
  // the (now unrelated) old parent and walks back up into the live tree.
  // Reads __kids DIRECTLY, never __children(): an un-inflated buffer-backed child has no
  // node object yet, so nothing can be holding it — inflating a whole child list just to
  // null out parent pointers would put an allocation on the React container-clear path.
  __detachKids() {
    const kids = this.__kids;
    if (!kids) return;
    const doc = this.ownerDocument, ce = doc !== null && doc.__ceRegistry !== undefined && kids.length !== 0 && this.isConnected;
    for (let i = 0; i < kids.length; i++) kids[i].parentNode = null;
    if (ce) for (let i = 0; i < kids.length; i++) removedNode(kids[i]);
  }

  // Element/Document/Fragment nodeValue is null per spec (CharacterData overrides).
  get nodeValue() { return null; }
  set nodeValue(_v) { /* no-op for non-CharacterData nodes, per spec */ }

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

// Viable next sibling (DOM spec): the first following sibling that is NOT itself
// one of the nodes being inserted. Anchoring after()/replaceWith() on the raw
// nextSibling breaks when that sibling is also an argument (e.g. a.after(b, c)
// where b already follows a) — insertBefore(b, b) then throws NotFoundError.
// Mirrors rtdom Tree::viable_next_sibling.
function viableNextSibling(node, nodes) {
  let s = node.nextSibling;
  while (s && nodes.includes(s)) s = s.nextSibling;
  return s;
}

// ----------------------------------------------------- CharacterData ----
class CharacterData extends Node {
  constructor(ownerDocument, data) { super(ownerDocument); this._data = data ?? ''; }
  get data() { return this._data; }
  set data(v) { const old = this._data; this._data = String(v); notifyMutation(this, 'characterData', null, null, null, null, old); }
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
  before(...nodes) { childBefore(this, nodes); }
  after(...nodes) { childAfter(this, nodes); }
  replaceWith(...nodes) { childReplaceWith(this, nodes); }
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
  __clone(doc) { return new Text(doc, this._data); }
}

export class Comment extends CharacterData {
  get nodeType() { return COMMENT_NODE; }
  get nodeName() { return '#comment'; }
  __clone(doc) { return new Comment(doc, this._data); }
}

export class DocumentType extends Node {
  constructor(ownerDocument, name, publicId, systemId) {
    super(ownerDocument);
    this.name = name; this.publicId = publicId || ''; this.systemId = systemId || '';
  }
  get nodeType() { return DOCUMENT_TYPE_NODE; }
  get nodeName() { return this.name; }
  __clone(doc) { return new DocumentType(doc, this.name, this.publicId, this.systemId); } // turbo-dom#35
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
    // HTMLElement (=== Element here) as a custom element constructor: `new C()` / an
    // upgrade's super() for a defined C (custom-elements.mjs); internal construction never
    // takes this branch.
    if (new.target !== Element && new.target !== SVGElement && new.target !== MathMLElement) {
      const custom = constructCustomElement(new.target);
      if (custom !== null) return custom;
    }
    super(ownerDocument);
    this.localName = localName;
    this.__ns = namespace;            // '', 'svg', 'math'
    this.__attrs = undefined;         // lazily built (buffer or []) on first attr touch
    this.__attrIdx = -1;              // buffer index for lazy attr inflation
    this.__content = undefined;       // <template> contents (a fragment in the inert document), lazy
    this.shadowRoot = null;           // open shadow root, if attached
    // Predeclare the hot lazy fields (left at their "absent" sentinel) so every
    // Element shares ONE hidden class. Without this, fields added on demand in
    // varying order across elements make the shared query/match/style/event code
    // megamorphic on property loads. Costs a few in-object slots, buys monomorphism.
    this.__childNodesList = undefined; // childNodes NodeList (memoized live view)
    this.__childrenList = undefined;   // children HTMLCollection (memoized live view)
    this.__qaCache = undefined;        // querySelectorAll cache (selector → {v,r})
    this.__qsCache = undefined;        // querySelector cache
    this.__computedStyle = undefined;  // getComputedStyle snapshot (version-keyed)
    this.__shadow = undefined;         // attached shadow root (open or closed)
    this.__sheet = undefined;          // CSSOM CSSStyleSheet for <style> (lazy, CSS-in-JS)
  }

  get nodeType() { return ELEMENT_NODE; }
  // HTML <template> contents: a fragment owned by the document's inert "template contents
  // owner document" (no browsing context, no custom element registry), created on first
  // read. <meta>'s `content` reflects its attribute; no other element has the property.
  get content() {
    if (this.__ns === '') {
      if (this.localName === 'template') return this.__content ?? (this.__content = new DocumentFragment(this.ownerDocument.__templateDocument()));
      if (this.localName === 'meta') return this.getAttribute('content') ?? '';
    }
    return undefined;
  }
  set content(value) {
    if (this.__ns === '') {
      if (this.localName === 'meta') { this.setAttribute('content', value); return; }
      if (this.localName === 'template') return; // read-only
    }
    Object.defineProperty(this, 'content', { value, writable: true, enumerable: true, configurable: true });
  }
  // the qualified name, uppercased for HTML elements (an HTML document)
  get tagName() {
    const p = this.__prefix, q = p === undefined ? this.localName : `${p}:${this.localName}`;
    return this.__ns ? q : q.toUpperCase();
  }
  get nodeName() { return this.tagName; }
  get namespaceURI() { return nsUri(this.__ns); }
  // createElementNS's prefix (`__prefix`, set only on the elements that have one)
  get prefix() { return this.__prefix ?? null; }

  // ---- attributes ----
  // attrs inflate lazily: a buffer-backed element leaves __attrs undefined and
  // builds the array from the SoA only when an attribute is first touched (many
  // elements are inflated for traversal but never have attrs read).
  __buildAttrs() { const doc = this.ownerDocument, buf = doc && doc.__buf; return (this.__attrIdx >= 0 && buf) ? buf.attrs(this.__attrIdx) : []; }
  getAttribute(name) {
    if (!this.__ns) name = ('' + name).toLowerCase(); // HTML attr names are ASCII-lowercased (SVG/MathML keep case)
    const at = this.__attrs;
    if (at !== undefined) { for (let i = 0; i < at.length; i++) if (at[i].name === name) return at[i].value; return null; }
    const doc = this.ownerDocument, buf = doc && doc.__buf;        // lazy: read column, don't materialize
    return (this.__attrIdx >= 0 && buf) ? buf.attrGet(this.__attrIdx, name) : null;
  }
  hasAttribute(name) {
    if (!this.__ns) name = ('' + name).toLowerCase();
    const at = this.__attrs;
    if (at !== undefined) { for (let i = 0; i < at.length; i++) if (at[i].name === name) return true; return false; }
    const doc = this.ownerDocument, buf = doc && doc.__buf;
    return (this.__attrIdx >= 0 && buf) ? buf.attrHas(this.__attrIdx, name) : false;
  }
  getAttributeNames() { return (this.__attrs ?? (this.__attrs = this.__buildAttrs())).map((a) => a.name); }
  setAttribute(name, value) {
    if (!this.__ns) name = ('' + name).toLowerCase();
    if (this.__attrs === undefined) this.__attrs = this.__buildAttrs();
    const a = this.__attrs.find((x) => x.name === name);
    const old = a ? a.value : null;
    if (a) a.value = String(value);
    else this.__attrs.push({ name, value: String(value), prefix: '' });
    notifyMutation(this, 'attributes', null, null, null, name, old);
  }
  removeAttribute(name) {
    if (!this.__ns) name = ('' + name).toLowerCase();
    if (this.__attrs === undefined) this.__attrs = this.__buildAttrs();
    const a = this.__attrs.find((x) => x.name === name);
    this.__attrs = this.__attrs.filter((x) => x.name !== name);
    if (a) notifyMutation(this, 'attributes', null, null, null, name, a.value);
  }
  toggleAttribute(name, force) {
    const has = this.hasAttribute(name);
    if (force === true || (force === undefined && !has)) { this.setAttribute(name, ''); return true; }
    this.removeAttribute(name); return false;
  }
  get attributes() {
    // Live NamedNodeMap (memoized for identity): length/indexing read __attrs on
    // every access, so a captured reference shrinks as attributes are removed —
    // React 19's releaseSingletonInstance `while (attrs.length)` loop terminates.
    return this.__attributesMap ?? (this.__attributesMap = liveNamedNodeMap(this, () =>
      (this.__attrs ?? (this.__attrs = this.__buildAttrs())).map((a) => ({
        name: a.name, localName: a.name, value: a.value, prefix: a.prefix || null,
        namespaceURI: a.prefix === 'xlink' ? 'http://www.w3.org/1999/xlink' : null,
        ownerElement: this,
      }))));
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
    // explicit association: <label for="thisId">
    if (this.id) {
      for (const l of this.ownerDocument.getElementsByTagName('label')) {
        if (l.getAttribute('for') === this.id) out.push(l);
      }
    }
    // implicit association: an ancestor <label> — but ONLY if THIS element is
    // that label's labeled control (the first labelable descendant). A second
    // control nested in the same label is NOT labeled by it.
    let p = this.parentNode;
    while (p) {
      if (p.localName === 'label') { if (p.control === this && !out.includes(p)) out.push(p); break; }
      p = p.parentNode;
    }
    return out;
  }
  get className() { return this.getAttribute('class') || ''; }
  set className(v) { this.setAttribute('class', v); }
  get classList() { return new ClassList(this); }

  get dataset() {
    if (!this.__dataset) this.__dataset = makeDataset(this);
    return this.__dataset;
  }

  // CSSOM seam for CSS-in-JS (emotion/styled-components/MUI): a <style> exposes a
  // CSSStyleSheet so engines can insertRule() at runtime. Created lazily on first
  // read (only emotion-touched <style>s ever get one), so plain documents pay
  // nothing. Rules inserted here flow into the cascade via cascade.mjs reading
  // this.__sheet. Non-<style> elements have no sheet (null), matching the platform.
  get sheet() {
    if (this.localName !== 'style') return null;
    return this.__sheet ?? (this.__sheet = new CSSStyleSheet(this));
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
    // textarea has no value attribute — its raw value defaults to the child text
    // content (the WHATWG "default value") until edited; input falls back to the
    // value attribute (defaultValue).
    if (t === 'textarea') return this.__value !== undefined ? this.__value : this.textContent;
    if (t === 'input') return this.__value !== undefined ? this.__value : (this.getAttribute('value') ?? '');
    return undefined;
  }
  set value(x) {
    const t = this.localName;
    if (t === 'select') { for (const o of this.getElementsByTagName('option')) o.selected = (o.value === String(x)); return; }
    if (t === 'option') { this.setAttribute('value', x); return; }
    // typed <input> runs the WHATWG value-sanitization algorithm: a value that
    // isn't valid for date/time/number/etc becomes the EMPTY string (not the prior
    // value). Matches real browsers — React's value tracker then sees the change and
    // fires onChange, where retaining the old value would silently swallow it.
    if (t === 'input') {
      const sanitized = sanitizeInputValue((this.getAttribute('type') || 'text').toLowerCase(), String(x));
      this.__value = sanitized === null ? '' : sanitized;
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
  // IDL-attribute reflection: React assigns these as element PROPERTIES; without a
  // reflecting setter the value never reaches a content attribute, so getAttribute/
  // toHaveAttribute see nothing. Mirrors the canonical lowercased attribute name.
  get inputMode() { return this.getAttribute('inputmode') ?? ''; }
  set inputMode(x) { this.setAttribute('inputmode', x); }
  get spellcheck() { const v = this.getAttribute('spellcheck'); return v == null ? true : v !== 'false'; }
  set spellcheck(x) { this.setAttribute('spellcheck', x ? 'true' : 'false'); }
  get autocomplete() { return this.getAttribute('autocomplete') ?? ''; }
  set autocomplete(x) { this.setAttribute('autocomplete', x); }
  get accept() { return this.getAttribute('accept') ?? ''; }
  set accept(x) { this.setAttribute('accept', x); }
  get min() { return this.getAttribute('min') ?? ''; }
  set min(x) { this.setAttribute('min', x); }
  get max() { return this.getAttribute('max') ?? ''; }
  set max(x) { this.setAttribute('max', x); }
  get step() { return this.getAttribute('step') ?? ''; }
  set step(x) { this.setAttribute('step', x); }
  get pattern() { return this.getAttribute('pattern') ?? ''; }
  set pattern(x) { this.setAttribute('pattern', x); }
  get maxLength() { return this.hasAttribute('maxlength') ? parseInt(this.getAttribute('maxlength'), 10) : -1; }
  set maxLength(x) { this.setAttribute('maxlength', String(x)); }
  get minLength() { return this.hasAttribute('minlength') ? parseInt(this.getAttribute('minlength'), 10) : -1; }
  set minLength(x) { this.setAttribute('minlength', String(x)); }
  get colSpan() { const v = parseInt(this.getAttribute('colspan'), 10); return v > 0 ? v : 1; }
  set colSpan(x) { this.setAttribute('colspan', String(x)); }
  get rowSpan() { const v = parseInt(this.getAttribute('rowspan'), 10); return v > 0 ? v : 1; }
  set rowSpan(x) { this.setAttribute('rowspan', String(x)); }
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
  // version-cache the element-filtered child array (like cachedQSA): re-filter only
  // when the DOM version changes, not on every children/childElementCount/first/last
  // access. Live — every mutation bumps Document.__version, invalidating the cache.
  __elementChildren() {
    const d = this.ownerDocument, v = d ? (d.__version || 0) : 0;
    if (this.__childrenArr !== undefined && this.__childrenArrV === v) return this.__childrenArr;
    this.__childrenArrV = v;
    return (this.__childrenArr = this.__children().filter((n) => n.nodeType === ELEMENT_NODE));
  }
  get children() {
    if (this.__childrenList) return this.__childrenList;
    const self = this;
    return (this.__childrenList = liveHTMLCollection(() => self.__elementChildren()));
  }
  get childElementCount() { return this.__elementChildren().length; }
  get firstElementChild() { return this.__elementChildren()[0] ?? null; }
  get lastElementChild() { const e = this.__elementChildren(); return e[e.length - 1] ?? null; }
  // one indexOf + scan, not nextSibling-per-step (each of which re-runs indexOf → O(n²)
  // when text nodes sit between elements). Reads __children() live — no cache.
  get nextElementSibling() {
    const p = this.parentNode; if (!p) return null;
    const k = p.__children(); let i = k.indexOf(this); if (i < 0) return null;
    for (i++; i < k.length; i++) if (k[i].nodeType === ELEMENT_NODE) return k[i];
    return null;
  }
  get previousElementSibling() {
    const p = this.parentNode; if (!p) return null;
    const k = p.__children(); let i = k.indexOf(this);
    for (i--; i >= 0; i--) if (k[i].nodeType === ELEMENT_NODE) return k[i];
    return null;
  }

  // ---- modern insertion ----
  append(...nodes) { parentAppend(this, nodes); }
  prepend(...nodes) {
    if (ceScope(this.ownerDocument)) { withReactions(METHOD.prepend, this, nodes); return; }
    const first = this.firstChild; for (const n of nodes) this.insertBefore(toNode(this.ownerDocument, n), first);
  }
  before(...nodes) { childBefore(this, nodes); }
  after(...nodes) { childAfter(this, nodes); }
  replaceWith(...nodes) { childReplaceWith(this, nodes); }

  // ---- queries ----
  matches(sel) { return matchesSelector(this, sel); }
  closest(sel) { let n = this; while (n && n.nodeType === ELEMENT_NODE) { if (n.matches(sel)) return n; n = n.parentNode; } return null; }
  querySelector(sel) { return cachedQS(this, sel); }
  querySelectorAll(sel) { return cachedQSA(this, sel); }
  getElementsByTagName(tag) {
    const self = this, raw = String(tag), t = raw.toLowerCase();
    let cv = -1, ca; // version-cache the subtree walk within this collection (live; re-walk on mutation)
    return liveHTMLCollection(() => { const d = self.ownerDocument, v = d ? (d.__version || 0) : 0; if (v === cv) return ca; cv = v; return (ca = collectByTag(self, raw, t)); });
  }
  getElementsByClassName(cls) {
    const self = this, classes = splitClasses(cls);
    let cv = -1, ca;
    return liveHTMLCollection(() => { const d = self.ownerDocument, v = d ? (d.__version || 0) : 0; if (v === cv) return ca; cv = v; return (ca = collectByClass(self, classes)); });
  }

  // ---- innerHTML / outerHTML ----
  get innerHTML() { return serializeInner(this); }
  set innerHTML(html) {
    if (ceScope(this.ownerDocument)) { withReactions(METHOD.setInnerHTML, this, [html]); return; }
    // a <template>'s markup replaces its contents (in the inert document), not its children
    const target = this.__ns === '' && this.localName === 'template' ? this.content : this;
    replaceAllWithMarkup(target, this, html);
  }
  get outerHTML() { return serializeOuter(this); }
  get innerText() { return this.textContent; }
  set innerText(v) { this.textContent = v; }

  // The markup is parsed with this element (afterbegin / beforeend) or its parent as the
  // context, straight into nodes of this document (turbo-dom#39: it used to parse through a
  // createElement(this.localName) scratch element — a constructed custom element, the
  // namespace lost).
  insertAdjacentHTML(position, html) {
    if (ceScope(this.ownerDocument)) { withReactions(METHOD.insertAdjacentHTML, this, [position, html]); return; }
    const where = String(position).toLowerCase();
    let parent = this, ref = null;
    switch (where) {
      case 'beforebegin': parent = this.parentNode; ref = this; break;
      case 'afterbegin': ref = this.firstChild; break;
      case 'beforeend': break;
      case 'afterend': parent = this.parentNode; ref = this.nextSibling; break;
      default: throw domError('SyntaxError', `The value provided ('${position}') is not one of 'beforeBegin', 'afterBegin', 'beforeEnd', or 'afterEnd'.`);
    }
    if (parent === null || parent.nodeType === DOCUMENT_NODE) throw domError('NoModificationAllowedError', 'The element has no parent.');
    const nodes = parseMarkup(parent.nodeType === ELEMENT_NODE ? parent : null, html, this.ownerDocument);
    for (const n of nodes) parent.insertBefore(n, ref);
  }

  __clone(doc, deep) {
    const el = newElement(doc, this.localName, this.__ns);
    if (this.__prefix !== undefined) el.__prefix = this.__prefix;
    el.__attrs = (this.__attrs ?? (this.__attrs = this.__buildAttrs())).map((a) => ({ ...a }));
    // custom element state travels (an `is` value / custom name -> "undefined", upgraded
    // when the clone's document has its definition — never in a template's inert
    // document); plain HTML elements and foreign ones skip the call
    if (this.__ns === '' && (this.__ce !== undefined || this.__ceIs !== undefined || this.__attrIdx >= 0 || this.localName.indexOf('-') !== -1)) clonedElement(this, el);
    if (deep) {
      // build the cloned child array directly — the clone is detached, so per-child
      // appendChild's notifyMutation (version bump) + reparent check are pure waste.
      el.__kids = cloneKids(this, el, doc);
      // HTML template "cloning steps": the contents are cloned into the copy's contents
      if (this.__ns === '' && this.localName === 'template') {
        const content = el.content;
        content.__kids = cloneKids(this.content, content, content.ownerDocument);
      }
    }
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
      return { undo: () => { this.__checked = old; }, fireChange: true };
    }
    if (t === 'radio') {
      if (this.checked) return null;
      const group = this.__radioGroup();
      const prev = group.find((r) => r.checked) || null;
      group.forEach((r) => { r.__checked = false; });
      this.__checked = true;
      return { undo: () => { this.__checked = false; if (prev) prev.__checked = true; }, fireChange: true };
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
  getBoundingClientRect() { const w = synthWidth(this), h = synthHeight(this); return { x: 0, y: 0, top: 0, left: 0, right: w, bottom: h, width: w, height: h, toJSON() { return this; } }; }
  getClientRects() { const r = this.getBoundingClientRect(); return (r.width || r.height) ? [r] : []; }
  scrollIntoView() {}
  scroll() {} scrollTo() {} scrollBy() {}
  // synthetic box model (no real layout) — see synthWidth/synthHeight below
  get offsetWidth() { return synthWidth(this); } get offsetHeight() { return synthHeight(this); }
  get offsetTop() { return 0; } get offsetLeft() { return 0; }
  get offsetParent() { return null; }
  get clientWidth() { return synthWidth(this); } get clientHeight() { return synthHeight(this); }
  get clientTop() { return 0; } get clientLeft() { return 0; }
  get scrollWidth() { return synthWidth(this); } get scrollHeight() { return synthHeight(this); }
  get scrollTop() { return 0; } set scrollTop(_v) {} get scrollLeft() { return 0; } set scrollLeft(_v) {}

  // namespaced attributes
  getAttributeNS(_ns, name) { return this.getAttribute(name); }
  setAttributeNS(_ns, name, value) { this.setAttribute(name, value); }
  hasAttributeNS(_ns, name) { return this.hasAttribute(name); }
  removeAttributeNS(_ns, name) { this.removeAttribute(name); }
  getAttributeNode(name) { const a = (this.__attrs ?? (this.__attrs = this.__buildAttrs())).find((x) => x.name === name); return a ? { name: a.name, value: a.value, ownerElement: this } : null; }
  getAttributeNodeNS(_ns, name) { return this.getAttributeNode(name); }
  // Attr-node methods are thin shims over the name-based API (no real Attr class).
  // node.attributes is a LIVE NamedNodeMap (see the getter), so React 19's
  // releaseSingletonInstance `while (attrs.length) removeAttributeNode(attrs[0])`
  // shrinks the map each pass and terminates — not a snapshot that loops forever.
  setAttributeNode(attr) { const prev = this.getAttributeNode(attr.name); this.setAttribute(attr.name, attr.value); return prev; }
  setAttributeNodeNS(attr) { return this.setAttributeNode(attr); }
  removeAttributeNode(attr) { const removed = this.getAttributeNode(attr && attr.name); if (attr && attr.name != null) this.removeAttribute(attr.name); return removed; }

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

  // shadow DOM. Flipping doc.__hasShadow arms the gated slow paths (event
  // retargeting in events.mjs, scoped cascade in cascade.mjs); before the first
  // attachShadow every benchmarked path runs exactly as it did with no shadow code.
  attachShadow(init = {}) {
    if (this.__shadow) throw new Error('NotSupportedError: shadow root already attached');
    const root = new ShadowRoot(this, init.mode || 'open', init.delegatesFocus);
    this.__shadow = root;
    if (root.mode === 'open') this.shadowRoot = root;
    const doc = this.ownerDocument;
    if (doc) doc.__hasShadow = true;
    return root;
  }

  // ---- <slot> projection (all on-demand; never touched by parse/query/events) ----
  // A light-DOM child's assigned slot: the matching <slot> in its parent host's
  // shadow tree, or null when the parent isn't a shadow host / no slot matches.
  get assignedSlot() {
    const host = this.parentNode;
    if (!host || !host.__shadow) return null;
    return findSlot(host.__shadow, this.getAttribute('slot') || '');
  }
  // <slot>.assignedNodes(): the host's light children routed to this slot by name
  // (default slot ← unnamed children + text). With {flatten:true} an empty slot
  // falls back to its own children (the slot's default content).
  assignedNodes(options) {
    if (this.localName !== 'slot') return [];
    const root = this.getRootNode();
    if (!root.__isShadowRoot) return [];
    const slotName = this.getAttribute('name') || '';
    const out = [];
    for (const c of root.host.__children()) {
      if (c.nodeType === ELEMENT_NODE) { if ((c.getAttribute('slot') || '') === slotName) out.push(c); }
      else if (c.nodeType === TEXT_NODE && slotName === '') out.push(c); // text routes only to the default slot
    }
    if (out.length === 0 && options && options.flatten) {
      return this.__children().filter((n) => n.nodeType === ELEMENT_NODE || n.nodeType === TEXT_NODE);
    }
    return out;
  }
  assignedElements(options) { return this.assignedNodes(options).filter((n) => n.nodeType === ELEMENT_NODE); }

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
    // abortable: if a listener preventDefault()s the reset event, controls keep state
    if (!this.dispatchEvent(new Event('reset', { bubbles: true, cancelable: true }))) return;
    for (const el of this.elements) {
      const tag = el.localName;
      if (tag === 'input') {
        const t = (el.getAttribute('type') || 'text').toLowerCase();
        if (t === 'checkbox' || t === 'radio') el.__checked = el.hasAttribute('checked');
        else el.__value = undefined; // fall back to the value attribute (defaultValue)
      } else if (tag === 'textarea') {
        el.__value = el.textContent;
      } else if (tag === 'select') {
        for (const o of el.getElementsByTagName('option')) o.__selected = o.hasAttribute('selected');
      }
    }
  }
}


// ------------------------------------------------------------- SVGElement ----
// SVG elements expose their geometry/identity attributes as SVGAnimated{String,Length,
// Rect} objects (el.className.baseVal, rect.width.baseVal.value, svg.viewBox.baseVal),
// not the plain strings/numbers HTML uses. A real subclass keeps these off every HTML
// element (zero regression, zero hot-path cost — the wrappers are built on access only).
// Instantiated by newElement() whenever the namespace is "svg".
export class SVGElement extends Element {
  // className is a readonly SVGAnimatedString in the spec; we still allow assigning a
  // string (common habit) by writing the class attribute, but reads return the wrapper.
  get className() { return new SVGAnimatedString(this, 'class'); }
  set className(v) { this.setAttribute('class', String(v)); }

  get viewBox() { return new SVGAnimatedRect(this, 'viewBox'); }

  // Honest geometry stubs (consistent with stubs.mjs — no layout engine, so zeros).
  getBBox() { return { x: 0, y: 0, width: 0, height: 0 }; }
  getCTM() { return null; }
  getScreenCTM() { return null; }
  get ownerSVGElement() {
    let p = this.parentNode;
    while (p && p.nodeType === ELEMENT_NODE) { if (p.localName === 'svg') return p; p = p.parentNode; }
    return null;
  }
}
// Length-valued geometry attributes → SVGAnimatedLength (built lazily per read).
for (const attr of SVG_LENGTH_ATTRS) {
  Object.defineProperty(SVGElement.prototype, attr, {
    configurable: true,
    get() { return new SVGAnimatedLength(this, attr); },
    set(v) { this.setAttribute(attr, v != null && v.baseVal ? String(v.baseVal.value) : String(v)); },
  });
}

// MathML-namespace elements are MathMLElements (turbo-dom#87); the interface adds no members
// over Element here (style / dataset / focus live on Element in this runtime).
export class MathMLElement extends Element {}

// createElement's ElementCreationOptions `is` (a string options argument is ignored, as Chrome).
const isOption = (options) => (options !== null && typeof options === 'object' && options.is !== undefined ? String(options.is) : null);

// Element factory: SVG namespace → SVGElement, MathML → MathMLElement, everything else → Element. One place so
// every construction site (buffer inflation, innerHTML, cloneNode, createElementNS) agrees.
function newElement(doc, tag, ns) {
  return ns === 'svg' ? new SVGElement(doc, tag, ns) : ns === 'math' ? new MathMLElement(doc, tag, ns) : new Element(doc, tag, ns);
}


// ---------------------------------------------------- DocumentFragment ----
export class DocumentFragment extends Node {
  get nodeType() { return DOCUMENT_FRAGMENT_NODE; }
  get nodeName() { return '#document-fragment'; }
  querySelector(sel) { return cachedQS(this, sel); }
  querySelectorAll(sel) { return cachedQSA(this, sel); }
  get children() { if (this.__childrenList) return this.__childrenList; const self = this; return (this.__childrenList = liveHTMLCollection(() => self.__children().filter((n) => n.nodeType === ELEMENT_NODE))); }
  // ParentNode element traversal (template content, fragments)
  get firstElementChild() { return this.__children().find((n) => n.nodeType === ELEMENT_NODE) ?? null; }
  get lastElementChild() { return this.__children().findLast((n) => n.nodeType === ELEMENT_NODE) ?? null; }
  get childElementCount() { return this.__children().filter((n) => n.nodeType === ELEMENT_NODE).length; }
  append(...nodes) { parentAppend(this, nodes); }
  __clone(doc, deep) {
    const f = new DocumentFragment(doc);
    if (deep) f.__kids = cloneKids(this, f, doc); // direct __kids build (see Element.__clone)
    return f;
  }
}

// A real shadow root: a detached document-fragment subtree with a back-reference
// to its host element. `__isShadowRoot` is the duck-typed flag events.mjs and
// Node.getRootNode/isConnected branch on (no import cycle). Encapsulation is free
// — the host's __children() never includes this subtree, so querySelector /
// getElementsBy* / matching never reach in.
export class ShadowRoot extends DocumentFragment {
  constructor(host, mode = 'open', delegatesFocus = false) {
    super(host.ownerDocument);
    this.host = host;
    this.mode = mode;
    this.delegatesFocus = !!delegatesFocus;
    this.__isShadowRoot = true;
  }
  get nodeName() { return '#document-fragment'; }
  get innerHTML() { return serializeInner(this); }
  set innerHTML(html) {
    if (ceScope(this.ownerDocument)) { withReactions(METHOD.setShadowInnerHTML, this, [html]); return; }
    replaceAllWithMarkup(this, null, html); // no context element → body
  }
  getElementById(id) {
    let found = null;
    const visit = (node) => {
      const kids = node.__children();
      for (let i = 0; i < kids.length; i++) {
        const c = kids[i];
        if (c.nodeType !== ELEMENT_NODE) continue;
        if (c.getAttribute('id') === id) { found = c; return; }
        visit(c);
        if (found) return;
      }
    };
    visit(this);
    return found;
  }
  getElementsByTagName(tag) {
    const self = this, raw = String(tag), t = raw.toLowerCase();
    let cv = -1, ca; // version-cache the subtree walk within this collection (live; re-walk on mutation)
    return liveHTMLCollection(() => { const d = self.ownerDocument, v = d ? (d.__version || 0) : 0; if (v === cv) return ca; cv = v; return (ca = collectByTag(self, raw, t)); });
  }
  getElementsByClassName(cls) {
    const self = this, classes = splitClasses(cls);
    let cv = -1, ca;
    return liveHTMLCollection(() => { const d = self.ownerDocument, v = d ? (d.__version || 0) : 0; if (v === cv) return ca; cv = v; return (ca = collectByClass(self, classes)); });
  }
  // the topmost root of any node within is this shadow root, never the document
  get activeElement() { return null; }
}

// ------------------------------------------------------------ helpers ----
function toNode(doc, n) { return typeof n === 'string' ? doc.createTextNode(n) : n; }

// Fragment-parse `html` with `context` (an element; null = <body>) into owned nodes of
// `doc`. Elements in other namespaces parse like <body> too.
function parseMarkup(context, html, doc) {
  const ns = context === null ? '' : context.__ns;
  const name = context === null ? '' : ns === '' ? context.localName : ns === 'svg' || ns === 'math' ? `${ns} ${context.localName}` : '';
  const frag = getParser().parseFragment(String(html), name);
  const nodes = [];
  // (template contents never sit at the top level: they hang off their <template>)
  for (const rawChild of frag.children) nodes.push(doc.__inflateNested(rawChild));
  // custom elements: upgrades queued at creation (custom-elements.mjs parsedFragment)
  if (nodes.length !== 0 && doc.__ceRegistry !== undefined) parsedFragment(nodes);
  return nodes;
}

// innerHTML: the parsed nodes replace all of `target`'s children (created first, so their
// upgrades queue before the old children's disconnectedCallbacks, as in Chrome).
function replaceAllWithMarkup(target, context, html) {
  const kids = parseMarkup(context, html, target.ownerDocument);
  target.__detachKids();
  for (let i = 0; i < kids.length; i++) kids[i].parentNode = target;
  target.__kids = kids;
  target.__touch();
}

// ParentNode append / ChildNode before / after / replaceWith, shared by elements, text,
// comments and fragments; one reaction scope for the whole call.
function parentAppend(parent, nodes) {
  if (ceScope(parent.ownerDocument)) { withReactions(parentAppend, null, [parent, nodes]); return; }
  for (const n of nodes) parent.appendChild(toNode(parent.ownerDocument, n));
}
function childBefore(child, nodes) {
  const p = child.parentNode;
  if (!p) return;
  if (ceScope(child.ownerDocument)) { withReactions(childBefore, null, [child, nodes]); return; }
  for (const n of nodes) p.insertBefore(toNode(child.ownerDocument, n), child);
}
function childAfter(child, nodes) {
  const p = child.parentNode;
  if (!p) return;
  if (ceScope(child.ownerDocument)) { withReactions(childAfter, null, [child, nodes]); return; }
  const ref = viableNextSibling(child, nodes);
  for (const n of nodes) p.insertBefore(toNode(child.ownerDocument, n), ref);
}
// DOM "replaceWith": the node is removed before the replacements go in (a node passed among
// its own replacements just moves; turbo-dom#40)
function childReplaceWith(child, nodes) {
  const p = child.parentNode;
  if (!p) return;
  if (ceScope(child.ownerDocument)) { withReactions(childReplaceWith, null, [child, nodes]); return; }
  const ref = viableNextSibling(child, nodes);
  if (!nodes.includes(child)) p.removeChild(child);
  for (const n of nodes) p.insertBefore(toNode(child.ownerDocument, n), ref);
}

// Deep clones of `source`'s children, created in `doc` and parented to `parent`.
function cloneKids(source, parent, doc) {
  const src = source.__children(), kids = new Array(src.length);
  for (let i = 0; i < src.length; i++) { const cl = src[i].__clone(doc, true); cl.parentNode = parent; kids[i] = cl; }
  return kids;
}

// First <slot> within a shadow tree whose name matches (default slot: name '').
function findSlot(shadowRoot, name) {
  let found = null;
  const visit = (node) => {
    const kids = node.__children();
    for (let i = 0; i < kids.length; i++) {
      const c = kids[i];
      if (c.nodeType !== ELEMENT_NODE) continue;
      if (c.localName === 'slot' && (c.getAttribute('name') || '') === name) { found = c; return; }
      visit(c);
      if (found) return;
    }
  };
  visit(shadowRoot);
  return found;
}

// The elements with qualified name `raw` (DOM "list of elements with qualified name"): HTML
// elements compare against the lowercased `tag`, others (SVG camelCase, other namespaces)
// against `raw` exactly. Only a name with a colon can match a prefixed element.
function collectByTag(root, raw, tag = raw) {
  const out = [];
  const all = raw === '*';
  const colon = raw.indexOf(':') !== -1;
  const visit = (node) => {
    const kids = node.__children();
    for (let i = 0; i < kids.length; i++) {
      const c = kids[i];
      if (c.nodeType !== ELEMENT_NODE) continue;
      if (all) out.push(c);
      else if (c.__ns === '') { if (c.localName === tag ? c.__prefix === undefined : colon && c.__prefix !== undefined && `${c.__prefix}:${c.localName}` === tag) out.push(c); }
      else if (c.localName === raw ? c.__prefix === undefined : colon && c.__prefix !== undefined && `${c.__prefix}:${c.localName}` === raw) out.push(c);
      visit(c);
    }
  };
  visit(root);
  return out;
}
// allocation-free whole-word class membership (no ClassList, no split)
function elHasClass(el, cls) {
  const cn = el.getAttribute('class');
  if (!cn) return false;
  if (cn === cls) return true;
  const L = cls.length;
  let idx = cn.indexOf(cls);
  while (idx !== -1) {
    if ((idx === 0 || cn.charCodeAt(idx - 1) <= 32) && (idx + L === cn.length || cn.charCodeAt(idx + L) <= 32)) return true;
    idx = cn.indexOf(cls, idx + 1);
  }
  return false;
}
function collectByClass(root, classes) {
  const out = [];
  const visit = (node) => {
    const kids = node.__children();
    for (let i = 0; i < kids.length; i++) {
      const c = kids[i];
      if (c.nodeType !== ELEMENT_NODE) continue;
      let ok = true;
      for (let j = 0; j < classes.length; j++) if (!elHasClass(c, classes[j])) { ok = false; break; }
      if (ok) out.push(c);
      visit(c);
    }
  };
  visit(root);
  return out;
}

function zeroRect() {
  return { x: 0, y: 0, top: 0, left: 0, right: 0, bottom: 0, width: 0, height: 0, toJSON() { return this; } };
}

// ---- synthetic geometry --------------------------------------------------
// Honest zero geometry deadlocks layout-driven React: a component measures 0,
// setState()s to "fix" it, re-renders, measures 0 again — forever (MUI Popper /
// autosizers / virtual lists / useLayoutEffect). So instead of a real layout
// engine we return a cheap synthetic box model whose values are (1) non-zero &
// plausible, (2) STABLE for the same DOM state, (3) internally consistent
// (children fit parents; right-left === width). Width depends only on ancestors,
// height only on descendants — no cycle. Memoized per node keyed on
// Document.__version (`__rw`/`__rh`), like __computedStyle, so a mutation
// invalidates it and a measure→setState→measure cycle sees the same value twice
// and settles. Test-time only — parse/query/match/events never call these.
const INLINE_TAGS = new Set(['a', 'abbr', 'b', 'bdi', 'bdo', 'br', 'cite', 'code', 'data', 'dfn', 'em', 'i', 'img', 'input', 'kbd', 'label', 'mark', 'output', 'q', 'rp', 'rt', 'ruby', 's', 'samp', 'select', 'small', 'span', 'strong', 'sub', 'sup', 'textarea', 'time', 'u', 'var', 'wbr']);
const LINE_HEIGHT = 18;
const CHAR_W = 8;
function viewportWidth(el) { const w = el.ownerDocument && el.ownerDocument.defaultView; return (w && w.innerWidth) || 1024; }
function synthWidth(el) {
  const doc = el.ownerDocument || el;
  const v = doc.__version || 0;
  const c = el.__rw;
  if (c && c.v === v) return c.w;
  const p = el.parentNode;
  const pw = (p && p.nodeType === ELEMENT_NODE) ? synthWidth(p) : viewportWidth(el);
  // inline boxes shrink-wrap their text (capped at the parent); blocks fill it
  const w = INLINE_TAGS.has(el.localName)
    ? Math.min(pw, Math.max(CHAR_W, (el.textContent || '').length * CHAR_W))
    : pw;
  el.__rw = { v, w };
  return w;
}
function synthHeight(el) {
  const doc = el.ownerDocument || el;
  const v = doc.__version || 0;
  const c = el.__rh;
  if (c && c.v === v) return c.h;
  const kids = el.__children ? el.__children() : [];
  let elemKids = 0, stacked = 0;
  for (let i = 0; i < kids.length; i++) { const k = kids[i]; if (k.nodeType === ELEMENT_NODE) { elemKids++; stacked += synthHeight(k); } }
  let h;
  if (elemKids) h = Math.max(LINE_HEIGHT, stacked);  // block flow: element children stack
  else {
    const chars = (el.textContent || '').length;
    const perLine = Math.max(1, Math.floor(synthWidth(el) / CHAR_W));
    h = Math.max(1, Math.ceil(chars / perLine)) * LINE_HEIGHT;
  }
  el.__rh = { v, h };
  return h;
}

// child index of `node` within its parent (per spec: the node's "index")
function nodeIndex(node) {
  const p = node.parentNode;
  if (!p) return 0;
  return p.__children().indexOf(node);
}

// Range — functional DOM Range. Tree-mutating ops (extract/clone/delete/insert/
// surround) implement the common case (a single container with element/text
// children, offsets as child indices). Zero geometry (no layout).
class Range {
  constructor(doc) {
    this.__doc = doc;
    this.startContainer = doc; this.endContainer = doc;
    this.startOffset = 0; this.endOffset = 0; this.collapsed = true;
  }
  setStart(node, offset) { this.startContainer = node; this.startOffset = offset; this.__sync(); }
  setEnd(node, offset) { this.endContainer = node; this.endOffset = offset; this.__sync(); }
  setStartBefore(node) { this.setStart(node.parentNode, nodeIndex(node)); }
  setStartAfter(node) { this.setStart(node.parentNode, nodeIndex(node) + 1); }
  setEndBefore(node) { this.setEnd(node.parentNode, nodeIndex(node)); }
  setEndAfter(node) { this.setEnd(node.parentNode, nodeIndex(node) + 1); }
  // selectNode: container is the node's parent, offsets bracket the node.
  selectNode(node) { const p = node.parentNode, i = nodeIndex(node); this.startContainer = this.endContainer = p; this.startOffset = i; this.endOffset = i + 1; this.__sync(); }
  selectNodeContents(node) { this.startContainer = this.endContainer = node; this.startOffset = 0; this.endOffset = node.nodeType === TEXT_NODE || node.nodeType === COMMENT_NODE ? node.length : node.__children().length; this.__sync(); }
  collapse(toStart) { if (toStart) { this.endContainer = this.startContainer; this.endOffset = this.startOffset; } else { this.startContainer = this.endContainer; this.startOffset = this.endOffset; } this.collapsed = true; }
  __sync() { this.collapsed = this.startContainer === this.endContainer && this.startOffset === this.endOffset; }
  // common ancestor: walk up from start until it contains end (per spec).
  get commonAncestorContainer() {
    let container = this.startContainer;
    while (container && !(container === this.endContainer || (container.contains && container.contains(this.endContainer)))) {
      container = container.parentNode;
    }
    return container || this.startContainer;
  }
  cloneRange() { const r = new Range(this.__doc); r.startContainer = this.startContainer; r.endContainer = this.endContainer; r.startOffset = this.startOffset; r.endOffset = this.endOffset; r.collapsed = this.collapsed; return r; }
  // The common single-container case: start === end container, offsets are child
  // indices (element children) or string offsets (a text/comment container).
  __sameContainerKids() { return this.startContainer === this.endContainer; }
  cloneContents() {
    const frag = this.__doc.createDocumentFragment();
    const c = this.startContainer;
    if (this.__sameContainerKids() && c && c.nodeType !== TEXT_NODE && c.nodeType !== COMMENT_NODE && c.__children) {
      const kids = c.__children();
      for (let i = this.startOffset; i < this.endOffset && i < kids.length; i++) frag.appendChild(kids[i].cloneNode(true));
    }
    return frag;
  }
  extractContents() {
    if (ceScope(this.__doc)) return withReactions(METHOD.extractContents, this, []);
    const frag = this.__doc.createDocumentFragment();
    const c = this.startContainer;
    if (this.__sameContainerKids() && c && c.nodeType !== TEXT_NODE && c.nodeType !== COMMENT_NODE && c.__children) {
      const kids = c.__children().slice(this.startOffset, this.endOffset);
      for (const k of kids) frag.appendChild(k); // moves out of the tree
    }
    this.collapse(true);
    return frag;
  }
  deleteContents() {
    if (ceScope(this.__doc)) { withReactions(METHOD.deleteContents, this, []); return; }
    const c = this.startContainer;
    if (this.__sameContainerKids() && c && c.nodeType !== TEXT_NODE && c.nodeType !== COMMENT_NODE && c.__children) {
      const kids = c.__children().slice(this.startOffset, this.endOffset);
      for (const k of kids) c.removeChild(k);
    }
    this.collapse(true);
  }
  insertNode(node) { if (this.startContainer && this.startContainer.insertBefore) this.startContainer.insertBefore(node, this.startContainer.__children ? (this.startContainer.__children()[this.startOffset] ?? null) : null); }
  surroundContents(node) {
    if (ceScope(this.__doc)) { withReactions(METHOD.surroundContents, this, [node]); return; }
    const frag = this.extractContents();
    this.insertNode(node);
    node.appendChild(frag);
  }
  // textual content of the range: the common single-container case.
  toString() {
    const c = this.startContainer;
    if (c && (c.nodeType === TEXT_NODE || c.nodeType === COMMENT_NODE)) {
      if (this.__sameContainerKids()) return c.data.slice(this.startOffset, this.endOffset);
      return c.data.slice(this.startOffset);
    }
    if (this.__sameContainerKids() && c && c.__children) {
      let s = '';
      const kids = c.__children();
      for (let i = this.startOffset; i < this.endOffset && i < kids.length; i++) s += kids[i].textContent;
      return s;
    }
    return '';
  }
  getBoundingClientRect() { return zeroRect(); }
  getClientRects() { return []; }
  detach() {}
}

function makeSelection(doc) {
  let ranges = [];
  const sel = {
    get rangeCount() { return ranges.length; },
    get isCollapsed() { return ranges.length === 0 || ranges.every((r) => r.collapsed); },
    get anchorNode() { return ranges[0] ? ranges[0].startContainer : null; },
    get focusNode() { return ranges[0] ? ranges[0].endContainer : null; },
    get anchorOffset() { return ranges[0] ? ranges[0].startOffset : 0; },
    get focusOffset() { return ranges[0] ? ranges[0].endOffset : 0; },
    get type() { return ranges.length === 0 ? 'None' : ranges[0].collapsed ? 'Caret' : 'Range'; },
    addRange(r) { ranges.push(r); },
    removeAllRanges() { ranges = []; },
    removeRange(r) { ranges = ranges.filter((x) => x !== r); },
    getRangeAt(i) { return ranges[i]; },
    // collapse(node, offset): a single collapsed range at the point.
    collapse(node, offset = 0) { if (node == null) { ranges = []; return; } const r = new Range(doc); r.setStart(node, offset); r.setEnd(node, offset); ranges = [r]; },
    collapseToStart() { if (ranges[0]) ranges[0].collapse(true); },
    collapseToEnd() { if (ranges[0]) ranges[0].collapse(false); },
    // extend(node, offset): move the focus (range end) to the new point.
    extend(node, offset = 0) { if (!ranges[0]) ranges = [new Range(doc)]; ranges[0].setEnd(node, offset); },
    selectAllChildren(node) { const r = new Range(doc); r.selectNodeContents(node); ranges = [r]; },
    setBaseAndExtent(anchorNode, anchorOffset, focusNode, focusOffset) { const r = new Range(doc); r.setStart(anchorNode, anchorOffset); r.setEnd(focusNode, focusOffset); ranges = [r]; },
    empty() { ranges = []; },
    toString() { return ranges[0] ? ranges[0].toString() : ''; },
  };
  return sel;
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
  // The inline declaration block, re-parsed whenever the `style` attribute changed:
  // [{ name, value, important, pending }] with shorthands expanded into their longhands
  // (miaskiewicz/turbo-dom#90, css-shorthand.mjs; the V8 binding's lockstep twin). Longhand
  // values are kept as authored (colors canonicalized), with no further validation.
  let source = null, entries = [];
  const parse = () => {
    const text = el.getAttribute('style') || '';
    if (text !== source) {
      source = text;
      entries = [];
      for (const [name, value, important] of declarations(text)) {
        const set = declarationEntries(name, value);
        if (!set) continue;
        for (const entry of set) {
          const index = entries.findIndex((e) => e.name === entry.name);
          if (index >= 0) {
            // A repeated property moves to the end; an !important one keeps out a later normal one.
            if (entries[index].important && !important) continue;
            entries.splice(index, 1);
          }
          entries.push({ ...entry, important });
        }
      }
    }
    return entries;
  };
  const write = () => { source = blockText(entries); el.setAttribute('style', source); };
  const value = (prop) => {
    const name = propertyName(prop);
    const found = parse().find((e) => e.name === name);
    if (found) return found.value;
    return canonical(name) ? shorthandValue(name, entries) : '';
  };
  const priority = (prop) => {
    const name = propertyName(prop);
    const list = longhands(name);
    const all = parse();
    const important = (n) => all.some((e) => e.name === n && e.important);
    return (list ? list.every(important) : important(name)) ? 'important' : '';
  };
  const setProperty = (prop, v, prio) => {
    const text = v === null || v === undefined ? '' : String(v);
    if (!text.trim()) { removeProperty(prop); return; }
    const p = prio === null || prio === undefined ? '' : String(prio).toLowerCase();
    if (p && p !== 'important') return;
    const set = declarationEntries(prop, text);
    if (!set) return;
    const all = parse();
    for (const entry of set) {
      const index = all.findIndex((e) => e.name === entry.name);
      const next = { ...entry, important: p === 'important' };
      if (index >= 0) all[index] = next; else all.push(next);
    }
    write();
  };
  // removeProperty: a shorthand removes its longhands and returns '' (Chrome).
  const removeProperty = (prop) => {
    const name = propertyName(prop);
    const all = parse();
    const list = longhands(name);
    if (list) {
      const kept = all.filter((e) => !list.includes(e.name));
      if (kept.length !== all.length) { entries = kept; write(); }
      return '';
    }
    const index = all.findIndex((e) => e.name === name);
    if (index < 0) return '';
    const [removed] = all.splice(index, 1);
    write();
    return removed.value;
  };
  return new Proxy({}, {
    get(_t, key) {
      if (key === 'getPropertyValue') return value;
      if (key === 'getPropertyPriority') return priority;
      if (key === 'setProperty') return setProperty;
      if (key === 'removeProperty') return removeProperty;
      if (key === 'item') return (i) => parse()[i]?.name ?? '';
      if (key === 'length') return parse().length;
      if (key === 'cssText') return blockText(parse());
      if (key === 'cssFloat') return value('float');
      if (key === Symbol.iterator) { const keys = parse().map((e) => e.name); return keys[Symbol.iterator].bind(keys); }
      if (typeof key !== 'string') return undefined;
      return value(kebab(key));
    },
    set(_t, key, v) {
      if (key === 'cssText') { el.setAttribute('style', String(v)); return true; }
      setProperty(key === 'cssFloat' ? 'float' : kebab(key), v, '');
      return true;
    },
    has(_t, key) { return typeof key === 'string'; },
  });
}
const kebab = (s) => s.replace(/[A-Z]/g, (m) => '-' + m.toLowerCase()).replace(/^webkit-/, '-webkit-');
// A property name as CSSOM keys it: custom properties as authored, others lowercase with
// longhand aliases (`word-wrap` -> `overflow-wrap`) resolved.
function propertyName(name) {
  const text = String(name);
  if (text.startsWith('--')) return text;
  const lower = text.toLowerCase();
  return longhandAlias(lower) ?? lower;
}
// The entries a declaration sets: a shorthand's longhands, or the longhand as authored (a
// <color> canonicalized the way Chrome's inline CSSOM spells it: hex / rgb() / hsl() ->
// rgb()/rgba(), named keywords as written); null for an invalid shorthand value.
function declarationEntries(prop, text) {
  const name = propertyName(prop);
  if (canonical(name)) return expand(name, text);
  let v = String(text).trim();
  if (COLOR_PROPS.has(name)) { const c = canonicalizeColor(v, false); if (c) v = c; }
  return [{ name, value: v, pending: null }];
}

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

// adoptNode/importNode helper: a buffer-backed node carries __idx/__attrIdx into
// the SoA buffer of its ORIGINAL document. Re-homing it to another document would
// leave those indices reading through the NEW document's buffer (wrong/garbage, or
// corruption). Materialize attrs + children off the current buffer FIRST, then sever
// the buffer linkage, then switch ownerDocument — depth-first so each level reads its
// own (old) buffer before being re-homed.
function adoptInto(node, doc) {
  if (node.ownerDocument === doc) return;
  const old = node.ownerDocument;
  // a document and its inert template document share one buffer: keep the lazy links
  if (old.__buf !== null && old.__buf === doc.__buf) { rehome(node, doc); return; }
  if (node.__idx >= 0) old.__cache[node.__idx] = undefined; // the old document no longer holds it
  if (node.nodeType === ELEMENT_NODE) {
    if (node.__attrs === undefined && node.__attrIdx >= 0) node.__attrs = node.__buildAttrs();
    if (node.__attrIdx >= 0 && node.__ns === '') freezeParsedIs(node); // the parser's `is` outlives the buffer link
    node.__attrIdx = -1;
  }
  const kids = node.__children ? node.__children() : []; // inflate off the OLD buffer
  node.__idx = -1;
  node.ownerDocument = doc;
  for (const c of kids) adoptInto(c, doc);
  if (node.__shadow) adoptInto(node.__shadow, doc); // shadow-including descendants
}
// Same buffer: switch the owner of the inflated nodes; the rest inflate in `doc` later
// (a node's children only exist once its __kids is built, so none is left behind).
function rehome(node, doc) {
  node.ownerDocument = doc;
  const kids = node.__kids;
  if (kids) for (let i = 0; i < kids.length; i++) rehome(kids[i], doc);
  if (node.__shadow) rehome(node.__shadow, doc);
}

// DOM "adopt" (after the node left its old parent): re-home the subtree, bump the old
// document's version (its caches must not keep reporting the node), and run the custom
// element adoption hooks when either document has them armed. A custom element adopted
// into a document without a registry arms that document's hooks (`__ceRegistry = null`:
// no definitions, but its custom elements still get connected / disconnected callbacks).
function adopt(node, doc) {
  const old = node.ownerDocument;
  adoptInto(node, doc);
  old.__version = ++VERSION;
  if (old.__ceRegistry !== undefined || doc.__ceRegistry !== undefined) {
    if (doc.__ceRegistry === undefined) doc.__ceRegistry = null;
    adoptedNode(node, old, doc);
  }
}

// --- MutationObserver, wired to the mutation methods above via notifyMutation ---
function isDescendant(node, ancestor) {
  let n = node;
  while (n) { if (n === ancestor) return true; n = n.parentNode; }
  return false;
}
const EMPTY_NODES = []; // shared read-only addedNodes/removedNodes for empty records

// Mutation notification — takes primitives (a single added/removed node, etc) so
// the common case (no MutationObservers — most test files) allocates NOTHING: it
// only bumps __version and returns. The MutationRecord object + its node arrays
// are built solely when an observer is actually registered.
function notifyMutation(target, type, added, removed, nextSibling, attributeName, oldValue) {
  const doc = target.ownerDocument;
  if (!doc) return;
  // bump the DOM version on every structural/attribute mutation — invalidates
  // the document's getElementsBy* caches. Unconditional (independent of observers).
  doc.__version = ++VERSION;
  const mo = doc.__mo;
  if (mo && mo.length !== 0) queueRecords(mo, doc, target, type, added, removed, nextSibling, attributeName, oldValue);
  // attributeChangedCallback: queued after the mutation record, like Chrome
  if (type === 'attributes' && doc.__ceRegistry !== undefined) attributeChanged(target, attributeName, oldValue);
}
function queueRecords(mo, doc, target, type, added, removed, nextSibling, attributeName, oldValue) {
  const addedNodes = added ? [added] : EMPTY_NODES;
  const removedNodes = removed ? [removed] : EMPTY_NODES;
  for (const reg of mo) {
    const { obs, target: obsTarget, options } = reg;
    const onTarget = target === obsTarget;
    const inSubtree = options.subtree && isDescendant(target, obsTarget);
    if (!onTarget && !inSubtree) continue;
    if (type === 'childList' && !options.childList) continue;
    if (type === 'attributes' && !options.attributes) continue;
    if (type === 'attributes' && options.attributeFilter && !options.attributeFilter.includes(attributeName)) continue;
    if (type === 'characterData' && !options.characterData) continue;
    const rec = {
      type, target,
      addedNodes, removedNodes,
      previousSibling: null, nextSibling: nextSibling || null,
      attributeName: attributeName || null, attributeNamespace: null,
      oldValue: (type === 'attributes' && options.attributeOldValue) ||
                (type === 'characterData' && options.characterDataOldValue) ? (oldValue ?? null) : null,
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
    this.__mo = null;             // registered MutationObservers (lazy — most files never observe())
    this.__moPending = null;      // observers with queued records awaiting microtask
    this.__ceRegistry = undefined; // the window's custom element registry, once touched (custom-elements.mjs)
    this.__inert = undefined;     // the template contents owner document (lazy), or true on that document
  }
  // HTML "appropriate template contents owner document": one inert document per document
  // (itself for an inert one) — no browsing context and no registry, so template contents
  // and their clones never upgrade; it shares the buffer, so buffer-backed contents inflate
  // lazily from it and move to and from the document without being materialized.
  __templateDocument() {
    const inert = this.__inert;
    if (inert === true) return this;
    if (inert !== undefined) return inert;
    const doc = new Document();
    doc.__buf = this.__buf;
    doc.__inert = true;
    doc.__version = ++VERSION;
    return (this.__inert = doc);
  }
  get nodeType() { return DOCUMENT_NODE; }
  get nodeName() { return '#document'; }
  // a document clone is a new document; its children are cloned into it
  __clone(_doc, deep) {
    const copy = new Document();
    if (deep) copy.__kids = cloneKids(this, copy, copy);
    return copy;
  }

  // ---- MutationObserver registry ----
  __moRegister(obs, target, options) {
    // replace existing registration for (obs,target) per spec
    if (!this.__mo) this.__mo = [];
    else this.__mo = this.__mo.filter((r) => !(r.obs === obs && r.target === target));
    this.__mo.push({ obs, target, options });
  }
  __moUnregister(obs) { if (this.__mo) this.__mo = this.__mo.filter((r) => r.obs !== obs); }
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
  // `nt` (nodeType) may be passed by callers that already read it (__children),
  // avoiding a second buffer read; other callers leave it undefined.
  __nodeAt(idx, nt) {
    if (idx < 0) return null;
    const cached = this.__cache[idx];
    if (cached !== undefined) return cached;
    const buf = this.__buf;
    let node;
    if (nt === undefined) nt = buf.nodeType(idx);
    switch (nt) {
      case ELEMENT_NODE: {
        const tag = buf.tagName(idx); // read once (was read twice — ctor + template check)
        node = newElement(this, tag, buf.ns(idx));
        node.__idx = idx;
        node.__attrIdx = idx; // attrs lazy (constructor left __attrs undefined)
        // template content fragment: a child node typed 11 named "content"
        if (tag === 'template') {
          for (let c = buf.firstChild(idx); c !== -1; c = buf.nextSib(c)) {
            if (buf.nodeType(c) === DOCUMENT_FRAGMENT_NODE && buf.tagName(c) === 'content') {
              node.__content = this.__templateDocument().__nodeAt(c); // owned by the inert document
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
        node = newElement(this, raw.name, raw.namespace || '');
        node.__attrs = raw.attrs.map((a) => ({ name: a.name, value: a.value, prefix: a.prefix || '' }));
        // the fragment parser reads `is` at creation, like the initial parse (custom-elements.mjs)
        if (node.__ns === '' && raw.attrs.length !== 0) parsedIsAttribute(node);
        if (raw.name === 'template') {
          const contentRaw = raw.children.find((c) => c.nodeType === DOCUMENT_FRAGMENT_NODE && c.name === 'content');
          if (contentRaw) node.__content = this.__templateDocument().__inflateNested(contentRaw);
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
    this.__version = ++VERSION; // invalidate getElementsBy* caches
    this.__tagCache = null;
    this.__classCache = null;
    this.__ceRegistry = undefined; // the window's globals (customElements) reset with the document
    this.__inert = undefined;      // a new buffer gets a new template contents owner document
  }

  get documentElement() { return this.__children().find((n) => n.nodeType === ELEMENT_NODE && n.localName === 'html') ?? null; }
  get doctype() { return this.__children().find((n) => n.nodeType === DOCUMENT_TYPE_NODE) ?? null; }
  get head() { const html = this.documentElement; return html ? html.__children().find((n) => n.localName === 'head') ?? null : null; }
  get body() { const html = this.documentElement; return html ? html.__children().find((n) => n.localName === 'body') ?? null : null; }
  get activeElement() { return this.__active || this.body || null; }
  __setActive(el) { this.__active = el; }

  // Live StyleSheetList. emotion iterates this to find its own <style>'s sheet by
  // ownerNode. Touching `.sheet` here lazily creates a CSSStyleSheet per <style>
  // (test-time only — never on parse/query paths).
  get styleSheets() {
    const styles = this.getElementsByTagName('style');
    const sheets = [];
    for (let i = 0; i < styles.length; i++) sheets.push(styles[i].sheet);
    return styleSheetList(sheets);
  }

  // ---- factories (owned nodes, no buffer) ----
  createElement(tag, options) {
    const raw = String(tag);
    if (!validLocalName(raw)) throw invalidName(raw);
    const name = raw.toLowerCase();
    if (this.__ceRegistry === undefined && options === undefined) return new Element(this, name, '');
    return createCustomElement(this, name, isOption(options), true);
  }
  // DOM "create an element NS": validate and extract the qualified name (Chrome 156 keeps
  // the local name up to a second colon: 'a:b:c' -> prefix 'a', local name 'b'), then an
  // element in that namespace — HTML (custom elements), SVG, MathML, or any other / null
  // namespace (a plain, case-preserving Element).
  createElementNS(namespace, qualifiedName, options) {
    const ns = namespace === null || namespace === undefined || namespace === '' ? null : String(namespace);
    const qualified = String(qualifiedName);
    let prefix = null, local = qualified;
    const colon = qualified.indexOf(':');
    if (colon !== -1) {
      prefix = qualified.slice(0, colon);
      const end = qualified.indexOf(':', colon + 1);
      local = end === -1 ? qualified.slice(colon + 1) : qualified.slice(colon + 1, end);
      if (!validPrefix(prefix)) throw invalidName(qualified);
    }
    if (!validLocalName(local)) throw invalidName(qualified);
    if ((prefix !== null && ns === null) || (prefix === 'xml' && ns !== XML_NS)
      || ((qualified === 'xmlns' || prefix === 'xmlns') !== (ns === XMLNS_NS))) {
      throw domError('NamespaceError', `The namespace and qualified name ('${qualified}') do not match.`);
    }
    const el = ns === HTML_NS && (this.__ceRegistry !== undefined || options !== undefined)
      ? createCustomElement(this, local, isOption(options), true)
      : newElement(this, local, nsShort(ns));
    if (prefix !== null) el.__prefix = prefix;
    return el;
  }
  __newHtmlElement(localName) { return new Element(this, localName, ''); }
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
  getSelection() { if (!this.__selection) this.__selection = makeSelection(this); return this.__selection; }
  // a clone created in THIS document (custom elements upgrade here, not in the source's)
  importNode(node, deep) {
    if (ceScope(this)) return withReactions(METHOD.importNode, this, [node, deep]);
    return node.__clone(this, !!deep);
  }
  adoptNode(node) {
    if ((this.__ceRegistry !== undefined || node.ownerDocument.__ceRegistry !== undefined) && reactionDepth === 0) return withReactions(METHOD.adoptNode, this, [node]);
    if (node.parentNode) node.parentNode.removeChild(node);
    if (node.ownerDocument !== this) adopt(node, this);
    return node;
  }

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
    const v = this.__version || 0;
    const cache = this.__idCache;
    if (cache) { const c = cache.get(id); if (c !== undefined && c.v === v) return c.el; }
    let found = null;
    const visit = (node) => {
      const kids = node.__children();
      for (let i = 0; i < kids.length; i++) {
        const c = kids[i];
        if (c.nodeType !== ELEMENT_NODE) continue;
        if (c.getAttribute('id') === id) { found = c; return; }
        visit(c);
        if (found) return;
      }
    };
    visit(this);
    (this.__idCache || (this.__idCache = new Map())).set(id, { v, el: found });
    return found;
  }
  querySelector(sel) { return cachedQS(this, sel); }
  querySelectorAll(sel) { return cachedQSA(this, sel); }
  // version-keyed cache: getElementsBy* is called repeatedly within a single
  // query (e.g. RTL getByLabelText calls element.labels per element, each doing
  // document.getElementsByTagName('label')). Without caching that's O(n²) tree
  // walks. The cache invalidates whenever the DOM version bumps (any mutation).
  __byTag(raw, t) {
    const v = this.__version || 0;
    const c = this.__tagCache && this.__tagCache.get(raw);
    if (c && c.v === v) return c.arr;
    const arr = collectByTag(this, raw, t);
    (this.__tagCache || (this.__tagCache = new Map())).set(raw, { v, arr });
    return arr;
  }
  __byClass(key, classes) {
    const v = this.__version || 0;
    const c = this.__classCache && this.__classCache.get(key);
    if (c && c.v === v) return c.arr;
    const arr = collectByClass(this, classes);
    (this.__classCache || (this.__classCache = new Map())).set(key, { v, arr });
    return arr;
  }
  // Memoize the live HTMLCollection Proxy per key. Its getArray closure re-reads
  // the version-cached __byTag/__byClass on every access, so a memoized Proxy still
  // reflects mutations — only the per-call Proxy allocation is saved.
  getElementsByTagName(tag) {
    const raw = String(tag);
    const m = this.__tagColl || (this.__tagColl = new Map());
    let coll = m.get(raw);
    if (!coll) { const self = this, t = raw.toLowerCase(); coll = liveHTMLCollection(() => self.__byTag(raw, t)); m.set(raw, coll); }
    return coll;
  }
  getElementsByClassName(cls) {
    const m = this.__classColl || (this.__classColl = new Map());
    let coll = m.get(cls);
    if (!coll) { const self = this; const classes = splitClasses(cls); coll = liveHTMLCollection(() => self.__byClass(cls, classes)); m.set(cls, coll); }
    return coll;
  }
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

// The reaction scopes' re-entry targets (see `ceScope`).
const setterOf = (proto, name) => Object.getOwnPropertyDescriptor(proto, name).set;
Object.assign(METHOD, {
  insertBefore: Node.prototype.insertBefore,
  removeChild: Node.prototype.removeChild,
  replaceChild: Node.prototype.replaceChild,
  replaceChildren: Node.prototype.replaceChildren,
  cloneNode: Node.prototype.cloneNode,
  setTextContent: setterOf(Node.prototype, 'textContent'),
  prepend: Element.prototype.prepend,
  setInnerHTML: setterOf(Element.prototype, 'innerHTML'),
  insertAdjacentHTML: Element.prototype.insertAdjacentHTML,
  setShadowInnerHTML: setterOf(ShadowRoot.prototype, 'innerHTML'),
  extractContents: Range.prototype.extractContents,
  deleteContents: Range.prototype.deleteContents,
  surroundContents: Range.prototype.surroundContents,
  importNode: Document.prototype.importNode,
  adoptNode: Document.prototype.adoptNode,
});

// Parse an HTML string into a fresh Document over the immutable SoA buffer.
export function parseDocument(html) {
  const soa = getParser().parseBuffer(String(html));
  const doc = new Document();
  doc.__load(soa);
  return doc;
}
