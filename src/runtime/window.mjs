// Layer 3 — lazy `window`. A Proxy whose globals are factories that materialize
// on first `get` and self-replace with the concrete value (one-time Proxy cost
// per property). A test using only `document.querySelector` never constructs
// localStorage, IntersectionObserver, matchMedia, etc. The Proxy doubles as a
// tracer: it records which globals each test actually touches.

import {
  Storage, makeMatchMedia, makeGetComputedStyle,
  IntersectionObserver, ResizeObserver,
  FileReader, makeCanvasStub, makeCustomElements,
  makeLocation, makeHistory,
} from './stubs.mjs';
import {
  Node, Element, Text, Comment, Document, DocumentFragment, DocumentType, Event, CustomEvent,
  MutationObserver,
} from './dom.mjs';
import {
  EventTarget,
  UIEvent, MouseEvent, PointerEvent, KeyboardEvent, InputEvent, FocusEvent,
  CompositionEvent, WheelEvent, TouchEvent, DragEvent, ProgressEvent,
} from './events.mjs';

// Minimal DataTransfer / clipboard primitives some libraries (user-event) need.
class DataTransfer {
  constructor() { this.__data = new Map(); this.dropEffect = 'none'; this.effectAllowed = 'all'; this.items = []; this.files = []; this.types = []; }
  setData(fmt, data) { this.__data.set(fmt, String(data)); if (!this.types.includes(fmt)) this.types.push(fmt); }
  getData(fmt) { return this.__data.get(fmt) ?? ''; }
  clearData(fmt) { if (fmt) this.__data.delete(fmt); else this.__data.clear(); }
}
class ClipboardEvent extends Event {
  constructor(type, init = {}) { super(type, init); this.clipboardData = init.clipboardData ?? new DataTransfer(); }
}

export function createWindow(document, { url = 'http://localhost/' } = {}) {
  const touched = new Set();
  let windowProxy;

  // Universal globals (touched by ~every test) — eager, no point lazifying.
  const base = {
    document,
    name: '',
    closed: false,
    origin: new URL(url).origin,
    // constructors are cheap class refs — expose eagerly
    Node, Element, Text, Comment, Document, DocumentFragment, DocumentType,
    EventTarget,
    Event, CustomEvent,
    UIEvent, MouseEvent, PointerEvent, KeyboardEvent, InputEvent, FocusEvent,
    CompositionEvent, WheelEvent, TouchEvent, DragEvent, ProgressEvent, ClipboardEvent,
    DataTransfer,
    HTMLElement: Element, SVGElement: Element,
    MutationObserver,
    URL: makeURL(), URLSearchParams,
    Blob: globalThis.Blob, File: makeFile(), FileReader,
    customElements: makeCustomElements(),
    AbortController: globalThis.AbortController, AbortSignal: globalThis.AbortSignal,
    TextEncoder: globalThis.TextEncoder, TextDecoder: globalThis.TextDecoder,
    // timers delegate to the host (Node) — already lazy at the OS level
    setTimeout: (...a) => setTimeout(...a),
    clearTimeout: (...a) => clearTimeout(...a),
    setInterval: (...a) => setInterval(...a),
    clearInterval: (...a) => clearInterval(...a),
    queueMicrotask: (...a) => queueMicrotask(...a),
    structuredClone: (...a) => structuredClone(...a),
    getSelection: () => document.getSelection(),
    scrollTo() {}, scroll() {}, scrollBy() {},
    alert() {}, confirm: () => false, prompt: () => null,
    dispatchEvent: (e) => document.dispatchEvent(e),
    addEventListener: (...a) => document.addEventListener(...a),
    removeEventListener: (...a) => document.removeEventListener(...a),
  };

  // Lazy globals — none constructed until touched.
  const lazy = {
    localStorage: () => new Storage(),
    sessionStorage: () => new Storage(),
    matchMedia: () => makeMatchMedia(),
    getComputedStyle: () => makeGetComputedStyle(),
    IntersectionObserver: () => IntersectionObserver,
    ResizeObserver: () => ResizeObserver,
    requestAnimationFrame: () => (cb) => setTimeout(() => cb(performanceNow()), 0),
    cancelAnimationFrame: () => (id) => clearTimeout(id),
    // subsystem grouping: history co-materializes with (and shares) location
    location: () => makeLocation(url),
    history: () => makeHistory(windowProxy.location),
    navigator: () => ({ userAgent: 'turbodom/0.0.1', platform: 'turbodom', language: 'en-US', languages: ['en-US'], onLine: true }),
    performance: () => ({ now: performanceNow, timeOrigin: 0, mark() {}, measure() {} }),
    Storage: () => Storage,
    devicePixelRatio: () => 1,
    innerWidth: () => 1024,
    innerHeight: () => 768,
  };

  windowProxy = new Proxy(base, {
    get(t, k) {
      if (k === 'window' || k === 'self' || k === 'globalThis' || k === 'parent' || k === 'top') return windowProxy;
      if (k in t) return t[k];
      const factory = lazy[k];
      if (factory) {
        touched.add(k);
        const v = factory();
        t[k] = v;                 // self-replace: subsequent reads skip the factory
        return v;
      }
      return undefined;
    },
    set(t, k, v) { t[k] = v; return true; },
    has(t, k) {
      return k in t || k in lazy ||
        k === 'window' || k === 'self' || k === 'globalThis' || k === 'parent' || k === 'top';
    },
  });

  document.defaultView = windowProxy;

  return {
    window: windowProxy,
    // which lazy globals this test materialized (the "DOM surface used" report)
    touched: () => [...touched],
    // Layer 5: drop materialized global slots, keep the class machinery warm.
    resetGlobals() {
      for (const k of touched) delete base[k];
      touched.clear();
    },
  };
}

function performanceNow() {
  const [s, ns] = process.hrtime();
  return s * 1000 + ns / 1e6;
}

let __objUrlSeq = 0;
function makeURL() {
  class TurboURL extends URL {}
  TurboURL.createObjectURL = () => `blob:turbodom/${++__objUrlSeq}`;
  TurboURL.revokeObjectURL = () => {};
  return TurboURL;
}
function makeFile() {
  const B = globalThis.Blob;
  return class File extends B {
    constructor(bits = [], name = 'file', opts = {}) { super(bits, opts); this.name = String(name); this.lastModified = opts.lastModified || 0; }
  };
}
