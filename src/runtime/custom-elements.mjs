// HTML custom elements for the JS runtime (miaskiewicz/turbo-dom#26 item 2): each element's
// custom element state (what `:defined` reads), the window's CustomElementRegistry, element
// upgrade / construction, and the lifecycle callbacks. Kept in lockstep with the Rust engine,
// whose tree records the same state (crates/turbo-dom/src/rtdom/custom_element_state.rs) and
// whose V8 binding upgrades like Chrome — both checked against the Chrome 156 control
// crates/turbo-dom-browser-env/src/custom_elements_defined_chrome156.json
// (test/custom-elements-chrome.test.mjs here).
//
// State. An element's state is fixed when it is created and changes only through upgrade
// or construction. Without either, it is the creation-time state: an HTML element whose
// local name is a valid custom element name, or that was created with an `is` value (the
// parser's `is` attribute, `createElement(name, {is})`, a clone of one), is "undefined";
// every other element is "uncustomized". A later `is` attribute edit never changes it.
// `el.__ce` holds an explicit state (unset = the creation-time one) and `el.__ceIs` an `is`
// value the buffer cannot tell (created, cloned and innerHTML-parsed elements); both live
// only on the rare elements that need them, so plain elements carry nothing.
//
// Pay-for-what-you-use: the DOM's hooks (dom.mjs) run only once the document has a
// registry with a definition (`Document.__ceRegistry`, set when `window.customElements` is
// first touched); until then every mutation path is unchanged.
//
// Reactions run synchronously at the mutation (Chrome runs them when the DOM call returns):
// upgrades, then attributeChangedCallback for observed attributes and connectedCallback;
// disconnectedCallback on removal; adoptedCallback on adoptNode. Exceptions from
// constructors and callbacks are reported (`window.reportError`), never thrown into the
// DOM call, like Chrome; a throwing constructor leaves the element "failed".

export const UNDEFINED = 'undefined';
export const FAILED = 'failed';
export const UNCUSTOMIZED = 'uncustomized';
export const PRECUSTOMIZED = 'precustomized';
export const CUSTOM = 'custom';
const STATES = new Set([UNDEFINED, FAILED, UNCUSTOMIZED, PRECUSTOMIZED, CUSTOM]);

// HTML's "valid custom element name" for the names the parser and createElement produce.
const RESERVED = new Set(['annotation-xml', 'color-profile', 'font-face', 'font-face-src', 'font-face-uri',
  'font-face-format', 'font-face-name', 'missing-glyph']);
export function isCustomName(name) {
  return /^[a-z][^A-Z]*-[^A-Z]*$/.test(name) && !RESERVED.has(name);
}

// The `is` value the parser saw (the immutable buffer's attribute, whatever later edits did).
function parsedIs(el) {
  const doc = el.ownerDocument, buf = doc && doc.__buf;
  return el.__attrIdx >= 0 && buf ? buf.attrGet(el.__attrIdx, 'is') : null;
}

// Keep the parser's `is` value when the element loses its buffer link (adoption).
export function freezeParsedIs(el) {
  const is = parsedIs(el);
  if (is === null) return;
  el.__ceIs = is;
  if (el.__ce === undefined) el.__ce = UNDEFINED;
}

// A fragment-parsed (innerHTML) element: its `is` attribute is its `is` value.
export function parsedIsAttribute(el) {
  const attrs = el.__attrs;
  for (let i = 0; i < attrs.length; i++) {
    if (attrs[i].name === 'is' && attrs[i].prefix === '') { el.__ceIs = attrs[i].value; el.__ce = UNDEFINED; return; }
  }
}

// The element's `is` value (HTML: read once, at creation).
export function isValue(el) {
  return el.__ceIs !== undefined ? el.__ceIs : parsedIs(el);
}

// The element's custom element state: the recorded one, else the creation-time state.
export function customElementState(el) {
  if (!el || el.nodeType !== 1 || el.__ns !== '') return UNCUSTOMIZED;
  const state = el.__ce;
  if (state !== undefined) return state;
  const name = el.localName;
  return (name.indexOf('-') !== -1 && isCustomName(name)) || parsedIs(el) !== null ? UNDEFINED : UNCUSTOMIZED;
}

// :defined — uncustomized or custom.
export function isDefinedElement(el) {
  const state = customElementState(el);
  return state === UNCUSTOMIZED || state === CUSTOM;
}

// Record an element's state (an embedder's upgrade report, or this module's own). Returns
// whether the state changed; a change bumps the DOM version so cached queries and
// computed styles see it. Non-elements and non-HTML elements are ignored.
export function setCustomElementState(el, state) {
  if (!STATES.has(state)) throw new TypeError(`unknown custom element state: ${state}`);
  if (!el || el.nodeType !== 1 || el.__ns !== '') return false;
  const before = customElementState(el);
  el.__ce = state;
  if (before === state) return false;
  const doc = el.ownerDocument;
  if (doc) doc.__version = (doc.__version || 0) + 1;
  return true;
}

// The state a copy (clone / import) starts in: "undefined" when the source is a custom
// element or was created with an `is` value, else uncustomized; the `is` value travels.
export function copyCustomElementState(source, copy) {
  const is = isValue(source);
  if (is !== null) copy.__ceIs = is;
  if (customElementState(source) !== UNCUSTOMIZED) copy.__ce = UNDEFINED;
}

// ---------------------------------------------------------------- registry ----

const definitions = new WeakMap(); // constructor -> definition (its registry's)
const ALREADY_CONSTRUCTED = {};

function report(doc, error) {
  const win = doc && doc.defaultView;
  const reportError = win && win.reportError;
  if (typeof reportError === 'function') reportError(error);
}

// The live registry state of a document, when it has at least one definition.
function active(doc) {
  const reg = doc && doc.__ceRegistry;
  return reg !== undefined && reg !== null && reg.defs.size !== 0 ? reg : null;
}

function lookup(reg, localName, is) {
  const autonomous = reg.defs.get(localName);
  if (autonomous !== undefined && autonomous.localName === localName) return autonomous;
  if (is === null) return null;
  const customized = reg.defs.get(is);
  return customized !== undefined && customized.localName === localName ? customized : null;
}

function definitionOf(el) {
  if (customElementState(el) !== UNDEFINED) return null;
  const reg = active(el.ownerDocument);
  return reg && lookup(reg, el.localName, isValue(el));
}

function callback(el, def, name, args) {
  const fn = def[name];
  if (fn === undefined) return;
  try { fn.apply(el, args); } catch (error) { report(el.ownerDocument, error); }
}

// HTML "upgrade an element": run the definition's constructor on the existing element.
function upgrade(el, def) {
  if (customElementState(el) !== UNDEFINED) return;
  setCustomElementState(el, PRECUSTOMIZED);
  def.stack.push(el);
  let result, error, threw = false;
  try { result = Reflect.construct(def.ctor, []); } catch (e) { threw = true; error = e; }
  def.stack.pop();
  if (!threw && result !== el) {
    threw = true;
    error = new TypeError('Custom element constructors must call super() first and must not return a different object');
  }
  if (threw) {
    setCustomElementState(el, FAILED);
    report(el.ownerDocument, error);
    return;
  }
  el.__ceDef = def;
  setCustomElementState(el, CUSTOM);
  if (def.attributeChangedCallback !== undefined) {
    for (const name of def.observed) {
      const value = el.getAttribute(name);
      if (value !== null) callback(el, def, 'attributeChangedCallback', [name, null, value, null]);
    }
  }
  if (el.isConnected) callback(el, def, 'connectedCallback', []);
}

// HTML "try to upgrade an element".
function tryUpgrade(el) {
  const def = definitionOf(el);
  if (def) upgrade(el, def);
}

// Shadow-including inclusive descendants in shadow-including tree order (elements only).
function eachElement(root, visit) {
  if (root.nodeType === 1) {
    visit(root);
    const shadow = root.__shadow;
    if (shadow) eachElement(shadow, visit);
  }
  const kids = root.__children();
  for (let i = 0; i < kids.length; i++) if (kids[i].nodeType === 1 || kids[i].nodeType === 11) eachElement(kids[i], visit);
}

// The HTML element constructors (HTMLElement and the tag interfaces): `new C()` for a
// defined C makes a new custom element; during an upgrade it returns the element being
// upgraded. Returns null for a constructor that is not defined (the caller's fallback).
export function constructCustomElement(newTarget) {
  const def = definitions.get(newTarget);
  if (def === undefined || def.registry.document.__ceRegistry !== def.registry) return null;
  const proto = newTarget.prototype;
  const stack = def.stack;
  if (stack.length === 0) {
    const el = def.registry.document.__newHtmlElement(def.localName);
    if (def.localName !== def.name) el.__ceIs = def.name;
    Object.setPrototypeOf(el, proto);
    el.__ceDef = def;
    el.__ce = CUSTOM;
    return el;
  }
  const el = stack[stack.length - 1];
  if (el === ALREADY_CONSTRUCTED) throw new TypeError("Failed to construct 'HTMLElement': This instance is already constructed");
  Object.setPrototypeOf(el, proto);
  stack[stack.length - 1] = ALREADY_CONSTRUCTED;
  return el;
}

// HTML "create an element" for an HTML element of `doc` (createElement, cloneNode):
// `synchronous` runs an autonomous definition's constructor directly (createElement),
// else the element is created "undefined" and upgraded (clones).
export function createCustomElement(doc, localName, is, synchronous) {
  const reg = active(doc);
  const def = reg && lookup(reg, localName, is);
  if (def && def.localName === def.name && synchronous) {
    try {
      const el = Reflect.construct(def.ctor, []);
      if (!el || el.nodeType !== 1 || el.localName !== localName || el.ownerDocument !== doc || el.parentNode
        || el.__children().length !== 0 || (el.__attrs !== undefined && el.__attrs.length !== 0)) {
        throw new TypeError(`Failed to execute 'createElement' on 'Document': The result must be a fresh <${localName}> element`);
      }
      return el;
    } catch (error) {
      report(doc, error);
      const el = doc.__newHtmlElement(localName);
      el.__ce = FAILED;
      return el;
    }
  }
  const el = doc.__newHtmlElement(localName);
  if (is !== null) { el.__ceIs = is; el.__ce = UNDEFINED; }
  if (def) upgrade(el, def);
  return el;
}

// ------------------------------------------------------------- DOM hooks ----
// Called by dom.mjs only when `doc.__ceRegistry` is set (one predicted-false read otherwise).

// A clone (or import) of `source`: copy the state, upgrade when a definition exists.
export function clonedElement(source, copy) {
  copyCustomElementState(source, copy);
  if (active(copy.ownerDocument)) tryUpgrade(copy);
}

// `node` was inserted: connectedCallback for custom elements, try to upgrade the others.
export function insertedNode(node) {
  if (!active(node.ownerDocument) || !node.isConnected) return;
  eachElement(node, (el) => {
    if (el.__ce === CUSTOM) { if (el.__ceDef) callback(el, el.__ceDef, 'connectedCallback', []); }
    else tryUpgrade(el);
  });
}

// Elements created by an HTML fragment parse (innerHTML): upgraded when their definition
// exists, connected or not (Chrome creates them in the context element's document).
export function parsedFragment(nodes) {
  if (nodes.length === 0 || !active(nodes[0].ownerDocument)) return;
  for (const node of nodes) eachElement(node, tryUpgrade);
}

// `node` was removed from a connected tree: disconnectedCallback for custom elements.
export function removedNode(node) {
  if (!active(node.ownerDocument)) return;
  eachElement(node, (el) => { if (el.__ce === CUSTOM && el.__ceDef) callback(el, el.__ceDef, 'disconnectedCallback', []); });
}

// An attribute of `el` changed (`oldValue` null when it was added).
export function attributeChanged(el, name, oldValue) {
  const def = el.__ce === CUSTOM ? el.__ceDef : undefined;
  if (def === undefined || def.attributeChangedCallback === undefined || !def.observed.includes(name)) return;
  callback(el, def, 'attributeChangedCallback', [name, oldValue, el.getAttribute(name), null]);
}

// `node` was adopted from `oldDocument`: adoptedCallback for custom elements.
export function adoptedNode(node, oldDocument, newDocument) {
  eachElement(node, (el) => {
    if (el.__ce === CUSTOM && el.__ceDef) callback(el, el.__ceDef, 'adoptedCallback', [oldDocument, newDocument]);
  });
}

// --------------------------------------------------- CustomElementRegistry ----

// The window's registry over `document`: define / get / getName / whenDefined / upgrade.
export function makeCustomElements(document) {
  const reg = { document, defs: new Map() };
  const waiters = new Map();
  if (document) document.__ceRegistry = reg;
  return {
    define(name, ctor, options) {
      name = String(name);
      if (typeof ctor !== 'function') throw new TypeError("Failed to execute 'define' on 'CustomElementRegistry': The provided value is not a constructor");
      if (reg.defs.has(name)) throw new Error(`'${name}' already defined`);
      const localName = options && options.extends != null ? String(options.extends) : name;
      const proto = ctor.prototype;
      const def = { registry: reg, name, localName, ctor, stack: [], observed: [] };
      for (const key of ['connectedCallback', 'disconnectedCallback', 'adoptedCallback', 'attributeChangedCallback']) {
        const fn = proto[key];
        def[key] = typeof fn === 'function' ? fn : undefined;
      }
      if (def.attributeChangedCallback !== undefined) {
        const observed = ctor.observedAttributes;
        if (observed != null) def.observed = Array.from(observed, String);
      }
      reg.defs.set(name, def);
      definitions.set(ctor, def);
      // upgrade the document's candidates in shadow-including tree order
      if (document && document.__ceRegistry === reg) {
        const candidates = [];
        eachElement(document, (el) => {
          if (el.localName === localName && customElementState(el) === UNDEFINED && lookup(reg, localName, isValue(el)) === def) candidates.push(el);
        });
        for (const el of candidates) upgrade(el, def);
      }
      const w = waiters.get(name);
      if (w) { w.forEach((resolve) => resolve(ctor)); waiters.delete(name); }
    },
    get(name) { const def = reg.defs.get(String(name)); return def ? def.ctor : undefined; },
    getName(ctor) { for (const [n, def] of reg.defs) if (def.ctor === ctor) return n; return null; },
    whenDefined(name) {
      name = String(name);
      const def = reg.defs.get(name);
      if (def) return Promise.resolve(def.ctor);
      return new Promise((resolve) => { const a = waiters.get(name) || []; a.push(resolve); waiters.set(name, a); });
    },
    upgrade(root) {
      if (!root || typeof root.__children !== 'function') return;
      const pending = [];
      eachElement(root, (el) => pending.push(el));
      for (const el of pending) {
        const def = el.ownerDocument && el.ownerDocument.__ceRegistry === reg && customElementState(el) === UNDEFINED
          ? lookup(reg, el.localName, isValue(el)) : null;
        if (def) upgrade(el, def);
      }
    },
  };
}
