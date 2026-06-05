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
  MutationObserver, DOMParser, XMLSerializer,
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

// Capture host functions at module load — BEFORE any installGlobals() can shadow
// the bare names on globalThis (which would make these delegates call themselves).
const hostSetTimeout = globalThis.setTimeout;
const hostClearTimeout = globalThis.clearTimeout;
const hostSetInterval = globalThis.setInterval;
const hostClearInterval = globalThis.clearInterval;
const hostQueueMicrotask = globalThis.queueMicrotask;
const hostStructuredClone = globalThis.structuredClone;

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
    // generic elements are plain Element → `el instanceof HTMLElement` is true.
    // HTMLIFrameElement MUST be a distinct class so React's iframe-descent loop
    // (`while (el instanceof HTMLIFrameElement)`) terminates on normal elements.
    HTMLElement: Element, SVGElement: Element,
    HTMLIFrameElement: class HTMLIFrameElement extends Element {},
    HTMLInputElement: Element, HTMLTextAreaElement: Element, HTMLSelectElement: Element,
    HTMLOptionElement: Element, HTMLButtonElement: Element, HTMLAnchorElement: Element,
    HTMLFormElement: Element, HTMLImageElement: Element, HTMLCanvasElement: Element,
    HTMLTemplateElement: Element, HTMLLabelElement: Element, HTMLDivElement: Element,
    HTMLSpanElement: Element, HTMLParagraphElement: Element, HTMLUListElement: Element,
    HTMLLIElement: Element, HTMLHeadingElement: Element, HTMLBodyElement: Element,
    HTMLDocument: Document, DocumentFragment, ShadowRoot: DocumentFragment,
    MutationObserver, DOMParser, XMLSerializer,
    URL: makeURL(), URLSearchParams,
    Blob: globalThis.Blob, File: makeFile(), FileReader,
    customElements: makeCustomElements(),
    AbortController: globalThis.AbortController, AbortSignal: globalThis.AbortSignal,
    TextEncoder: globalThis.TextEncoder, TextDecoder: globalThis.TextDecoder,
    // web platform globals Node already provides
    fetch: globalThis.fetch ? (...a) => globalThis.fetch(...a) : undefined,
    Headers: globalThis.Headers, Request: globalThis.Request, Response: globalThis.Response,
    FormData: globalThis.FormData, ReadableStream: globalThis.ReadableStream,
    crypto: globalThis.crypto, Crypto: globalThis.Crypto, SubtleCrypto: globalThis.SubtleCrypto,
    btoa: (s) => Buffer.from(String(s), 'binary').toString('base64'),
    atob: (s) => Buffer.from(String(s), 'base64').toString('binary'),
    MessageChannel: globalThis.MessageChannel, MessagePort: globalThis.MessagePort,
    BroadcastChannel: globalThis.BroadcastChannel, EventSource: globalThis.EventSource,
    reportError: (e) => { /* swallow; tests assert via handlers */ void e; },
    requestIdleCallback: (cb) => hostSetTimeout(() => cb({ didTimeout: false, timeRemaining: () => 50 }), 0),
    cancelIdleCallback: (id) => hostClearTimeout(id),
    CSS: { supports: () => true, escape: (s) => String(s).replace(/[^a-zA-Z0-9_-]/g, (c) => '\\' + c) },
    XMLHttpRequest: makeXHR(),
    Image: function Image(w, h) { const img = document.createElement('img'); if (w != null) img.setAttribute('width', w); if (h != null) img.setAttribute('height', h); return img; },
    Audio: function Audio(src) { const a = document.createElement('audio'); if (src) a.setAttribute('src', src); return a; },
    Worker: class Worker { constructor() {} postMessage() {} terminate() {} addEventListener() {} removeEventListener() {} },
    // timers delegate to the captured host fns (NOT the bare names — once these
    // are installed on globalThis the bare names resolve back here → recursion)
    setTimeout: (...a) => hostSetTimeout(...a),
    clearTimeout: (...a) => hostClearTimeout(...a),
    setInterval: (...a) => hostSetInterval(...a),
    clearInterval: (...a) => hostClearInterval(...a),
    queueMicrotask: (...a) => hostQueueMicrotask(...a),
    structuredClone: (...a) => hostStructuredClone(...a),
    getSelection: () => document.getSelection(),
    scrollTo() {}, scroll() {}, scrollBy() {},
    // window methods libraries/tests spy on — must exist as own props for vi.spyOn
    open: () => null, close() {}, stop() {}, print() {}, focus() {}, blur() {},
    moveTo() {}, moveBy() {}, resizeTo() {}, resizeBy() {},
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
    requestAnimationFrame: () => (cb) => hostSetTimeout(() => cb(performanceNow()), 0),
    cancelAnimationFrame: () => (id) => hostClearTimeout(id),
    // subsystem grouping: history co-materializes with (and shares) location
    location: () => makeLocation(url),
    history: () => makeHistory(windowProxy.location),
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
    // every global name this window can provide (for environment adapters)
    globalKeys: [...Object.keys(base), ...Object.keys(lazy)],
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
