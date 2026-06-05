// Layer 4 — aggressively but HONESTLY stub the unobservable. Headless runners
// have no layout, so we don't pretend. Honest absence over a plausible lie.

// FileReader — resolves async with a result, enough for upload-handling tests.
export class FileReader {
  constructor() { this.result = null; this.error = null; this.readyState = 0; this.onload = null; this.onloadend = null; this.onerror = null; this.__listeners = new Map(); }
  addEventListener(t, cb) { (this.__listeners.get(t) || this.__listeners.set(t, []).get(t)).push(cb); }
  removeEventListener(t, cb) { const l = this.__listeners.get(t); if (l) this.__listeners.set(t, l.filter((x) => x !== cb)); }
  __fire(type) {
    const ev = { type, target: this };
    if (typeof this['on' + type] === 'function') this['on' + type](ev);
    for (const cb of this.__listeners.get(type) || []) cb(ev);
  }
  __read(blob, makeResult) {
    this.readyState = 1;
    Promise.resolve().then(async () => {
      try { this.result = await makeResult(blob); } catch (e) { this.error = e; this.readyState = 2; this.__fire('error'); this.__fire('loadend'); return; }
      this.readyState = 2; this.__fire('load'); this.__fire('loadend');
    });
  }
  readAsText(blob) { this.__read(blob, (b) => (b && b.text ? b.text() : String(b))); }
  readAsDataURL(blob) { this.__read(blob, async (b) => { const buf = Buffer.from(b && b.arrayBuffer ? await b.arrayBuffer() : []); return `data:${(b && b.type) || ''};base64,${buf.toString('base64')}`; }); }
  readAsArrayBuffer(blob) { this.__read(blob, (b) => (b && b.arrayBuffer ? b.arrayBuffer() : new ArrayBuffer(0))); }
  abort() { this.readyState = 2; this.__fire('abort'); }
}

// Canvas 2D context — no raster backend; methods are no-ops, measureText returns 0.
export function makeCanvasStub() {
  const noop = () => {};
  return new Proxy({}, {
    get(_t, k) {
      if (k === 'measureText') return (s) => ({ width: String(s).length * 6 });
      if (k === 'getImageData') return () => ({ data: new Uint8ClampedArray(0), width: 0, height: 0 });
      if (k === 'createLinearGradient' || k === 'createRadialGradient' || k === 'createPattern') return () => ({ addColorStop: noop });
      if (k === 'canvas') return null;
      return noop;
    },
  });
}

// CustomElementRegistry — define/get/whenDefined. No upgrade of generic elements,
// but enough that defining and awaiting elements doesn't throw.
export function makeCustomElements() {
  const defs = new Map();
  const waiters = new Map();
  return {
    define(name, ctor) {
      if (defs.has(name)) throw new Error(`'${name}' already defined`);
      defs.set(name, ctor);
      const w = waiters.get(name); if (w) { w.forEach((r) => r(ctor)); waiters.delete(name); }
    },
    get(name) { return defs.get(name); },
    getName(ctor) { for (const [n, c] of defs) if (c === ctor) return n; return null; },
    whenDefined(name) { if (defs.has(name)) return Promise.resolve(defs.get(name)); return new Promise((res) => { const a = waiters.get(name) || []; a.push(res); waiters.set(name, a); }); },
    upgrade() {},
  };
}

export class Storage {
  constructor() { this.__map = new Map(); }
  get length() { return this.__map.size; }
  key(i) { return [...this.__map.keys()][i] ?? null; }
  getItem(k) { return this.__map.has(String(k)) ? this.__map.get(String(k)) : null; }
  setItem(k, v) { this.__map.set(String(k), String(v)); }
  removeItem(k) { this.__map.delete(String(k)); }
  clear() { this.__map.clear(); }
}

export function makeMatchMedia() {
  return (query) => ({
    matches: false,                 // honest: no real media context in a headless runner
    media: query,
    onchange: null,
    addEventListener() {},
    removeEventListener() {},
    addListener() {},               // deprecated, kept for compatibility
    removeListener() {},
    dispatchEvent() { return false; },
  });
}

// getComputedStyle: honest — reflects ONLY inline + explicitly-set values, never
// invents cascade/layout numbers. A property that wasn't set reads as ''.
export function makeGetComputedStyle() {
  return (el) => {
    const style = el && el.style ? el.style : null;
    return new Proxy({}, {
      get(_t, key) {
        if (key === 'getPropertyValue') return (p) => (style ? style.getPropertyValue(p) : '');
        if (key === '__honest') return 'computed style is inline-only; no layout/cascade available';
        if (typeof key !== 'string') return undefined;
        return style ? style[key] : '';
      },
    });
  };
}

class ObserverStub {
  constructor(cb) { this.__cb = cb; }
  observe() {}
  unobserve() {}
  disconnect() {}
  takeRecords() { return []; }
}
export class IntersectionObserver extends ObserverStub {
  constructor(cb) { super(cb); this.root = null; this.rootMargin = '0px'; this.thresholds = [0]; }
}
export class ResizeObserver extends ObserverStub {}

// MutationObserver: real enough to be useful — fires on observed mutations would
// require hooking every mutation; v1 is an honest queue-based stub that records
// nothing automatically. Documented as opt-in faithful later.
export class MutationObserver {
  constructor(cb) { this.__cb = cb; this.__records = []; }
  observe() {}
  disconnect() {}
  takeRecords() { const r = this.__records; this.__records = []; return r; }
}

export function makeLocation(href = 'http://localhost/') {
  const u = new URL(href);
  return {
    get href() { return u.href; }, set href(v) { /* navigation is a no-op in headless */ },
    get protocol() { return u.protocol; },
    get host() { return u.host; },
    get hostname() { return u.hostname; },
    get port() { return u.port; },
    get pathname() { return u.pathname; },
    get search() { return u.search; },
    get hash() { return u.hash; },
    get origin() { return u.origin; },
    assign() {}, replace() {}, reload() {}, toString() { return u.href; },
  };
}

export function makeHistory(location) {
  const stack = [{ state: null, url: location.href }];
  let idx = 0;
  return {
    get length() { return stack.length; },
    get state() { return stack[idx].state; },
    pushState(state, _title, url) { stack.splice(idx + 1); stack.push({ state, url }); idx++; },
    replaceState(state, _title, url) { stack[idx] = { state, url: url ?? stack[idx].url }; },
    back() { if (idx > 0) idx--; },
    forward() { if (idx < stack.length - 1) idx++; },
    go(n) { idx = Math.max(0, Math.min(stack.length - 1, idx + (n || 0))); },
  };
}
