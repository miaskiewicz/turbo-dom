// Layer 3 — lazy `window`. A Proxy whose globals are factories that materialize
// on first `get` and self-replace with the concrete value (one-time Proxy cost
// per property). A test using only `document.querySelector` never constructs
// localStorage, IntersectionObserver, matchMedia, etc. The Proxy doubles as a
// tracer: it records which globals each test actually touches.

import {
  Storage, makeMatchMedia,
  IntersectionObserver, ResizeObserver,
  FileReader, makeCanvasStub, makeCustomElements,
  makeLocation, makeHistory,
} from './stubs.mjs';
import { makeGetComputedStyle } from './cascade.mjs';
import {
  Node, Element, Text, Comment, Document, DocumentFragment, DocumentType, Event, CustomEvent,
  MutationObserver, DOMParser, XMLSerializer, ShadowRoot,
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

// FormData that preserves File/Blob identity. Node's global FormData clones
// entries into anonymous Blobs (losing the File reference + filename); tests that
// assert `fd.get('file') === file` need the original kept.
class TurboFormData {
  constructor() { this.__entries = []; }
  append(name, value, filename) { this.__entries.push([String(name), this.__wrap(value, filename)]); }
  set(name, value, filename) { this.delete(name); this.append(name, value, filename); }
  __wrap(value, filename) {
    // string values coerce; File/Blob are kept by reference (optionally re-named)
    if (value == null || typeof value !== 'object') return String(value);
    if (filename !== undefined && globalThis.Blob && value instanceof globalThis.Blob && value.name !== filename) {
      const F = makeFile(); return new F([value], String(filename), { type: value.type });
    }
    return value;
  }
  get(name) { const e = this.__entries.find((x) => x[0] === String(name)); return e ? e[1] : null; }
  getAll(name) { return this.__entries.filter((x) => x[0] === String(name)).map((x) => x[1]); }
  has(name) { return this.__entries.some((x) => x[0] === String(name)); }
  delete(name) { this.__entries = this.__entries.filter((x) => x[0] !== String(name)); }
  forEach(cb, thisArg) { for (const [k, v] of this.__entries) cb.call(thisArg, v, k, this); }
  *entries() { for (const e of this.__entries) yield [e[0], e[1]]; }
  *keys() { for (const e of this.__entries) yield e[0]; }
  *values() { for (const e of this.__entries) yield e[1]; }
  [Symbol.iterator]() { return this.entries(); }
}

// A tag-specific HTML*Element interface. Not constructible — exists so
// `el instanceof HTMLXElement` is true only for elements with the matching
// localName (matcher = string or RegExp). Avoids aliasing every interface to
// Element (which made `instanceof HTMLAnchorElement` true for ALL elements).
function tagClass(matcher) {
  const test = typeof matcher === 'string' ? (n) => n === matcher : (n) => matcher.test(n);
  const C = function () { throw new TypeError('Illegal constructor'); };
  // share Element.prototype so prototype-level spies/reads resolve (e.g.
  // vi.spyOn(HTMLAnchorElement.prototype, 'click')); instanceof is decided by
  // the tag matcher below, not the prototype chain.
  C.prototype = Element.prototype;
  Object.defineProperty(C, Symbol.hasInstance, {
    value: (o) => o != null && o.nodeType === 1 && test(o.localName),
  });
  return C;
}

// Capture host functions at module load — BEFORE any installGlobals() can shadow
// the bare names on globalThis (which would make these delegates call themselves).
const hostSetTimeout = globalThis.setTimeout;
const hostClearTimeout = globalThis.clearTimeout;

// Env-independent lazy global factories — built ONCE at module load, shared by
// every window (none capture document/url/windowProxy). Materialization still
// self-replaces onto the per-env base, so each env gets its own instance; only
// the factory objects/closures are shared, saving ~28 allocations per test file.
const SHARED_LAZY = {
  customElements: () => makeCustomElements(), // fresh registry per env, built on first access
  localStorage: () => new Storage(),
  sessionStorage: () => new Storage(),
  matchMedia: () => makeMatchMedia(),
  getComputedStyle: () => makeGetComputedStyle(),
  IntersectionObserver: () => IntersectionObserver,
  ResizeObserver: () => ResizeObserver,
  requestAnimationFrame: () => (cb) => hostSetTimeout(() => cb(performanceNow()), 0),
  cancelAnimationFrame: () => (id) => hostClearTimeout(id),
  navigator: () => ({
    userAgent: 'Mozilla/5.0 (turbo-dom) AppleWebKit/537.36',
    platform: 'turbo-dom', vendor: '', language: 'en-US', languages: ['en-US'],
    onLine: true, cookieEnabled: true, doNotTrack: null, maxTouchPoints: 0,
    hardwareConcurrency: 4, deviceMemory: 8, webdriver: false,
    clipboard: { readText: async () => '', writeText: async () => {}, read: async () => [], write: async () => {} },
    permissions: { query: async () => ({ state: 'prompt', addEventListener() {}, removeEventListener() {} }) },
    sendBeacon: () => true, vibrate: () => false,
  }),
  performance: () => ({ now: performanceNow, timeOrigin: 0, mark() {}, measure() {}, getEntriesByName: () => [], getEntriesByType: () => [], clearMarks() {}, clearMeasures() {} }),
  Storage: () => Storage,
  devicePixelRatio: () => 1,
  innerWidth: () => 1024,
  innerHeight: () => 768,
  outerWidth: () => 1024,
  outerHeight: () => 768,
  scrollX: () => 0, scrollY: () => 0, pageXOffset: () => 0, pageYOffset: () => 0,
  screenX: () => 0, screenY: () => 0, screenLeft: () => 0, screenTop: () => 0,
  screen: () => ({ width: 1024, height: 768, availWidth: 1024, availHeight: 768, colorDepth: 24, pixelDepth: 24, orientation: { type: 'landscape-primary', angle: 0, addEventListener() {}, removeEventListener() {} } }),
  visualViewport: () => ({ width: 1024, height: 768, scale: 1, offsetLeft: 0, offsetTop: 0, pageLeft: 0, pageTop: 0, addEventListener() {}, removeEventListener() {} }),
};
const hostSetInterval = globalThis.setInterval;
const hostClearInterval = globalThis.clearInterval;
const hostQueueMicrotask = globalThis.queueMicrotask;
const hostStructuredClone = globalThis.structuredClone;

export function createWindow(document, { url = 'http://localhost/' } = {}) {
  const touched = new Set();
  let windowProxy;

  // Per-env globals: only the ~11 that capture document/url/windowProxy. Everything
  // stateless (constructors, timers, stateless window methods) lives in the shared
  // module-level STATIC_BASE, built ONCE — not rebuilt per createWindow (per test
  // file). The Proxy below reads base first, then STATIC_BASE, then lazy; a
  // `window.x = y` assignment writes to base, shadowing STATIC_BASE per-env.
  const base = {
    document,
    name: '',
    closed: false,
    origin: new URL(url).origin,
    // customElements is lazy (SHARED_LAZY) — env-independent + rarely used; building
    // its two Maps eagerly per createWindow was wasted for the typical test.
    Image: function Image(w, h) { const img = document.createElement('img'); if (w != null) img.setAttribute('width', w); if (h != null) img.setAttribute('height', h); return img; },
    Audio: function Audio(src) { const a = document.createElement('audio'); if (src) a.setAttribute('src', src); return a; },
    getSelection: () => document.getSelection(),
    dispatchEvent: (e) => document.dispatchEvent(e),
    addEventListener: (...a) => document.addEventListener(...a),
    removeEventListener: (...a) => document.removeEventListener(...a),
  };

  // Per-env lazy globals: ONLY those that capture document/url/windowProxy.
  // Everything env-independent lives in the module-level SHARED_LAZY (built once,
  // not re-allocated per test file) — see below. None constructed until touched.
  const lazy = {
    // subsystem grouping: history co-materializes with (and shares) location
    location: () => makeLocation(url),
    history: () => makeHistory(windowProxy.location),
  };

  windowProxy = new Proxy(base, {
    get(t, k) {
      if (k === 'window' || k === 'self' || k === 'globalThis' || k === 'parent' || k === 'top') return windowProxy;
      if (k in t) return t[k];            // per-env (incl. overrides + materialized lazy)
      if (k in STATIC_BASE) return STATIC_BASE[k];  // shared stateless globals
      const factory = lazy[k] || SHARED_LAZY[k];
      if (factory) {
        touched.add(k);
        const v = factory();
        t[k] = v;                 // self-replace: subsequent reads skip the factory
        return v;
      }
      return undefined;
    },
    set(t, k, v) { t[k] = v; return true; },  // writes to base → shadows STATIC_BASE per-env
    has(t, k) {
      return k in t || k in STATIC_BASE || k in lazy || k in SHARED_LAZY ||
        k === 'window' || k === 'self' || k === 'globalThis' || k === 'parent' || k === 'top';
    },
    // so vi.spyOn(window, 'scrollTo'/'open'/…) finds STATIC_BASE methods as own
    // props; spyOn then defineProperty's the spy onto base (target), shadowing it.
    getOwnPropertyDescriptor(t, k) {
      const own = Object.getOwnPropertyDescriptor(t, k);
      if (own) return own;
      if (k in STATIC_BASE) return { configurable: true, enumerable: true, writable: true, value: STATIC_BASE[k] };
      return undefined;
    },
  });

  document.defaultView = windowProxy;

  return {
    window: windowProxy,
    // which lazy globals this test materialized (the "DOM surface used" report)
    touched: () => [...touched],
    // every global name this window can provide (for environment adapters)
    globalKeys: [...Object.keys(base), ...Object.keys(STATIC_BASE), ...Object.keys(lazy), ...Object.keys(SHARED_LAZY)],
    // Layer 5: drop materialized global slots, keep the class machinery warm.
    resetGlobals() {
      for (const k of touched) delete base[k];
      touched.clear();
    },
  };
}

import { performance as nodePerformance } from 'node:perf_hooks';
function performanceNow() {
  return nodePerformance.now();
}

let __objUrlSeq = 0;
function makeURL() {
  class TurboURL extends URL {}
  TurboURL.createObjectURL = () => `blob:turbo-dom/${++__objUrlSeq}`;
  TurboURL.revokeObjectURL = () => {};
  return TurboURL;
}
function makeFile() {
  const B = globalThis.Blob;
  return class File extends B {
    constructor(bits = [], name = 'file', opts = {}) { super(bits, opts); this.name = String(name); this.lastModified = opts.lastModified || 0; }
  };
}
// stateless — build the classes ONCE, not per createWindow() (per test file).
const TURBO_URL = makeURL();
const TURBO_FILE = makeFile();

// Minimal XMLHttpRequest backed by fetch — enough that libraries that construct
// one and issue a request don't crash. No-network setups still get a clean object.
function makeXHR() {
  return class XMLHttpRequest {
    constructor() {
      this.readyState = 0; this.status = 0; this.statusText = ''; this.response = ''; this.responseText = '';
      this.responseType = ''; this.timeout = 0; this.withCredentials = false;
      this.onreadystatechange = null; this.onload = null; this.onerror = null; this.onabort = null;
      this.__headers = {}; this.__method = 'GET'; this.__url = ''; this.__listeners = new Map(); this.__aborted = false;
    }
    open(method, url) { this.__method = method; this.__url = url; this.readyState = 1; this.__fire('readystatechange'); }
    setRequestHeader(k, v) { this.__headers[k] = v; }
    getResponseHeader() { return null; }
    getAllResponseHeaders() { return ''; }
    addEventListener(t, cb) { const l = this.__listeners.get(t) || []; l.push(cb); this.__listeners.set(t, l); }
    removeEventListener(t, cb) { const l = this.__listeners.get(t); if (l) this.__listeners.set(t, l.filter((x) => x !== cb)); }
    __fire(type) { const ev = { type, target: this }; if (typeof this['on' + type] === 'function') this['on' + type](ev); for (const cb of this.__listeners.get(type) || []) cb(ev); }
    abort() { this.__aborted = true; this.readyState = 0; this.__fire('abort'); }
    send(body) {
      if (!globalThis.fetch) { this.readyState = 4; this.status = 0; this.__fire('error'); this.__fire('loadend'); return; }
      globalThis.fetch(this.__url, { method: this.__method, headers: this.__headers, body }).then(async (res) => {
        if (this.__aborted) return;
        this.status = res.status; this.statusText = res.statusText; this.responseText = await res.text(); this.response = this.responseText;
        this.readyState = 4; this.__fire('readystatechange'); this.__fire('load'); this.__fire('loadend');
      }).catch(() => { if (this.__aborted) return; this.readyState = 4; this.status = 0; this.__fire('error'); this.__fire('loadend'); });
    }
  };
}

// Stateless globals — identical for every window, so built ONCE at module load
// instead of per createWindow() (per test file). createWindow's Proxy falls back
// to this after its tiny per-env `base`. Nothing here captures document/url/window.
const STATIC_BASE = {
  // DOM + event constructors (cheap class refs)
  Node, Element, Text, Comment, Document, DocumentFragment, DocumentType, EventTarget,
  ShadowRoot,
  Event, CustomEvent,
  UIEvent, MouseEvent, PointerEvent, KeyboardEvent, InputEvent, FocusEvent,
  CompositionEvent, WheelEvent, TouchEvent, DragEvent, ProgressEvent, ClipboardEvent,
  DataTransfer,
  // every element is a plain Element → `el instanceof HTMLElement` is true.
  // Tag-specific interfaces match by localName via Symbol.hasInstance.
  HTMLElement: Element, SVGElement: Element,
  // SVG interface globals. Libs (and test setup mocks) do
  // `Object.defineProperty(SVGSVGElement.prototype, 'viewBox', …)` and
  // `el instanceof SVGSVGElement`; an undefined global is a hard ReferenceError.
  SVGSVGElement: tagClass('svg'), SVGPathElement: tagClass('path'),
  SVGGElement: tagClass('g'), SVGCircleElement: tagClass('circle'),
  SVGRectElement: tagClass('rect'), SVGLineElement: tagClass('line'),
  SVGEllipseElement: tagClass('ellipse'), SVGPolygonElement: tagClass('polygon'),
  SVGPolylineElement: tagClass('polyline'), SVGTextElement: tagClass('text'),
  SVGTSpanElement: tagClass('tspan'), SVGUseElement: tagClass('use'),
  SVGDefsElement: tagClass('defs'), SVGImageElement: tagClass('image'),
  SVGStopElement: tagClass('stop'), SVGSymbolElement: tagClass('symbol'),
  SVGMarkerElement: tagClass('marker'), SVGClipPathElement: tagClass('clipPath'),
  SVGMaskElement: tagClass('mask'), SVGPatternElement: tagClass('pattern'),
  SVGTitleElement: tagClass('title'), SVGDescElement: tagClass('desc'),
  SVGForeignObjectElement: tagClass('foreignObject'),
  SVGGraphicsElement: Element, SVGGeometryElement: Element, SVGTextContentElement: Element,
  HTMLAnchorElement: tagClass('a'), HTMLInputElement: tagClass('input'),
  HTMLTextAreaElement: tagClass('textarea'), HTMLSelectElement: tagClass('select'),
  HTMLOptionElement: tagClass('option'), HTMLButtonElement: tagClass('button'),
  HTMLFormElement: tagClass('form'), HTMLImageElement: tagClass('img'),
  HTMLCanvasElement: tagClass('canvas'), HTMLTemplateElement: tagClass('template'),
  HTMLLabelElement: tagClass('label'), HTMLDivElement: tagClass('div'),
  HTMLSpanElement: tagClass('span'), HTMLParagraphElement: tagClass('p'),
  HTMLUListElement: tagClass('ul'), HTMLLIElement: tagClass('li'),
  HTMLBodyElement: tagClass('body'), HTMLIFrameElement: tagClass('iframe'),
  HTMLHeadingElement: tagClass(/^h[1-6]$/),
  // Previously-missing interfaces: undefined globals make `el instanceof HTMLXElement`
  // throw "Right-hand side of 'instanceof' is not an object" — worse than returning
  // false. RTL/React probe these constantly.
  HTMLTableElement: tagClass('table'), HTMLTableRowElement: tagClass('tr'),
  HTMLTableCellElement: tagClass(/^(td|th)$/), HTMLTableSectionElement: tagClass(/^(thead|tbody|tfoot)$/),
  HTMLTableColElement: tagClass(/^(col|colgroup)$/), HTMLTableCaptionElement: tagClass('caption'),
  HTMLOListElement: tagClass('ol'), HTMLDListElement: tagClass('dl'),
  HTMLFieldSetElement: tagClass('fieldset'), HTMLLegendElement: tagClass('legend'),
  HTMLOptGroupElement: tagClass('optgroup'), HTMLDataListElement: tagClass('datalist'),
  HTMLOutputElement: tagClass('output'), HTMLProgressElement: tagClass('progress'),
  HTMLMeterElement: tagClass('meter'), HTMLDetailsElement: tagClass('details'),
  HTMLDialogElement: tagClass('dialog'), HTMLPreElement: tagClass('pre'),
  HTMLBRElement: tagClass('br'), HTMLHRElement: tagClass('hr'),
  HTMLQuoteElement: tagClass(/^(q|blockquote)$/), HTMLModElement: tagClass(/^(ins|del)$/),
  HTMLPictureElement: tagClass('picture'), HTMLSourceElement: tagClass('source'),
  HTMLTrackElement: tagClass('track'), HTMLVideoElement: tagClass('video'),
  HTMLAudioElement: tagClass('audio'), HTMLMediaElement: tagClass(/^(video|audio)$/),
  HTMLEmbedElement: tagClass('embed'), HTMLObjectElement: tagClass('object'),
  HTMLMapElement: tagClass('map'), HTMLAreaElement: tagClass('area'),
  HTMLScriptElement: tagClass('script'), HTMLStyleElement: tagClass('style'),
  HTMLLinkElement: tagClass('link'), HTMLMetaElement: tagClass('meta'),
  HTMLTitleElement: tagClass('title'), HTMLBaseElement: tagClass('base'),
  HTMLHeadElement: tagClass('head'), HTMLHtmlElement: tagClass('html'),
  HTMLDataElement: tagClass('data'), HTMLTimeElement: tagClass('time'),
  HTMLSlotElement: tagClass('slot'), HTMLMenuElement: tagClass('menu'),
  HTMLDocument: Document,
  MutationObserver, DOMParser, XMLSerializer,
  URL: TURBO_URL, URLSearchParams,
  Blob: globalThis.Blob, File: TURBO_FILE, FileReader,
  AbortController: globalThis.AbortController, AbortSignal: globalThis.AbortSignal,
  TextEncoder: globalThis.TextEncoder, TextDecoder: globalThis.TextDecoder,
  fetch: globalThis.fetch ? (...a) => globalThis.fetch(...a) : undefined,
  Headers: globalThis.Headers, Request: globalThis.Request, Response: globalThis.Response,
  FormData: TurboFormData, ReadableStream: globalThis.ReadableStream,
  crypto: globalThis.crypto, Crypto: globalThis.Crypto, SubtleCrypto: globalThis.SubtleCrypto,
  btoa: (s) => Buffer.from(String(s), 'binary').toString('base64'),
  atob: (s) => Buffer.from(String(s), 'base64').toString('binary'),
  MessageChannel: globalThis.MessageChannel, MessagePort: globalThis.MessagePort,
  BroadcastChannel: globalThis.BroadcastChannel, EventSource: globalThis.EventSource,
  reportError: (e) => { void e; },
  requestIdleCallback: (cb) => hostSetTimeout(() => cb({ didTimeout: false, timeRemaining: () => 50 }), 0),
  cancelIdleCallback: (id) => hostClearTimeout(id),
  CSS: { supports: () => true, escape: (s) => String(s).replace(/[^a-zA-Z0-9_-]/g, (c) => '\\' + c) },
  XMLHttpRequest: makeXHR(),
  Worker: class Worker { constructor() {} postMessage() {} terminate() {} addEventListener() {} removeEventListener() {} },
  // timers delegate to captured host fns (NOT bare names — installed bare names
  // would resolve back here → recursion)
  setTimeout: (...a) => hostSetTimeout(...a),
  clearTimeout: (...a) => hostClearTimeout(...a),
  setInterval: (...a) => hostSetInterval(...a),
  clearInterval: (...a) => hostClearInterval(...a),
  queueMicrotask: (...a) => hostQueueMicrotask(...a),
  structuredClone: (...a) => hostStructuredClone(...a),
  scrollTo() {}, scroll() {}, scrollBy() {},
  open: () => null, close() {}, stop() {}, print() {}, focus() {}, blur() {},
  moveTo() {}, moveBy() {}, resizeTo() {}, resizeBy() {},
  alert() {}, confirm: () => false, prompt: () => null,
};
