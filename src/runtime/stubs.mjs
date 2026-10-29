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

export class Storage {
  constructor() { this.__map = new Map(); }
  get length() { return this.__map.size; }
  key(i) { return [...this.__map.keys()][i] ?? null; }
  getItem(k) { return this.__map.has(String(k)) ? this.__map.get(String(k)) : null; }
  setItem(k, v) { this.__map.set(String(k), String(v)); }
  removeItem(k) { this.__map.delete(String(k)); }
  clear() { this.__map.clear(); }
}

// matchMedia: parse width/height/orientation features and evaluate them against
// the window's innerWidth/innerHeight (default 1024×768). Responsive components
// (`useMediaQuery`, MUI breakpoints) branch on this; a flat `false` made them
// mis-branch and thrash. `matches` is a getter so it re-evaluates if innerWidth
// is overridden — still stable for a given viewport. Unknown/feature-less queries
// stay honestly `false`.
const MQ_FEATURE = /\(\s*(min-width|max-width|min-height|max-height|orientation)\s*:\s*([^)]+?)\s*\)/g;
function evalMediaQuery(query, win) {
  const W = (win && win.innerWidth) || 1024;
  const H = (win && win.innerHeight) || 768;
  const q = String(query).toLowerCase();
  let matched = true, any = false, m;
  MQ_FEATURE.lastIndex = 0;
  while ((m = MQ_FEATURE.exec(q))) {
    any = true;
    const feat = m[1], val = m[2].trim();
    if (feat === 'min-width') matched = matched && W >= parseFloat(val);
    else if (feat === 'max-width') matched = matched && W <= parseFloat(val);
    else if (feat === 'min-height') matched = matched && H >= parseFloat(val);
    else if (feat === 'max-height') matched = matched && H <= parseFloat(val);
    else matched = matched && val === (H >= W ? 'portrait' : 'landscape'); // orientation: portrait iff height >= width (turbo-dom#138)
  }
  return any && matched;
}
export function makeMatchMedia(win) {
  return (query) => ({
    media: String(query),
    get matches() { return evalMediaQuery(query, win); },
    onchange: null,
    addEventListener() {},
    removeEventListener() {},
    addListener() {},               // deprecated, kept for compatibility
    removeListener() {},
    dispatchEvent() { return false; },
  });
}

// getComputedStyle lives in cascade.mjs — it resolves injected <style> sheets +
// inline styles (a strict superset of the old inline-only stub once here).

// Resize/Intersection observers fire their callback ONCE, async, with a single
// initial entry — never on a loop. Components that gate first render on the first
// observer callback (lazy lists, MUI autosize, visibility-gated mounts) then
// proceed and settle. The entry's rect is the element's synthetic box (dom.mjs).
function rectOf(el) {
  const r = el && typeof el.getBoundingClientRect === 'function' ? el.getBoundingClientRect() : { width: 0, height: 0 };
  return { x: 0, y: 0, top: 0, left: 0, right: r.width, bottom: r.height, width: r.width, height: r.height };
}
export class ResizeObserver {
  constructor(cb) { this.__cb = cb; }
  observe(el) {
    const cb = this.__cb;
    Promise.resolve().then(() => {
      const r = rectOf(el);
      const entry = { target: el, contentRect: r, borderBoxSize: [{ inlineSize: r.width, blockSize: r.height }], contentBoxSize: [{ inlineSize: r.width, blockSize: r.height }], devicePixelContentBoxSize: [{ inlineSize: r.width, blockSize: r.height }] };
      try { cb([entry], this); } catch { /* don't crash the render tier on a callback throw */ }
    });
  }
  unobserve() {}
  disconnect() {}
}
export class IntersectionObserver {
  constructor(cb, opts = {}) {
    this.__cb = cb;
    this.root = opts.root ?? null;
    this.rootMargin = opts.rootMargin ?? '0px';
    this.thresholds = Array.isArray(opts.threshold) ? opts.threshold : [opts.threshold ?? 0];
  }
  observe(el) {
    const cb = this.__cb;
    Promise.resolve().then(() => {
      const r = rectOf(el);
      const entry = { target: el, isIntersecting: true, intersectionRatio: 1, boundingClientRect: r, intersectionRect: r, rootBounds: null, time: 0 };
      try { cb([entry], this); } catch { /* same */ }
    });
  }
  unobserve() {}
  disconnect() {}
  takeRecords() { return []; }
}

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
