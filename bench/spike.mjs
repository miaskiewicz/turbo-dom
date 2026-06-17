// Phase-1 boundary spike bench (RUST_PORT_PLAN §7).
//
// Question: does a Rust-core-in-WASM DOM, accessed from JS, stay within ~10% of
// (or beat) the current pure-JS runtime on the chattiest hot paths?
//
//   wasm-pack build --target nodejs --out-dir pkg-spike --no-default-features --features wasm-runtime
//   node bench/spike.mjs
//
// Measures three impls over an IDENTICAL workload (qsa-once, then per-node chatter:
// getAttribute x2 + tagName + parent-walk + listener-less dispatch walk):
//   * JS      — current src/runtime (DOM is JS objects; access is free in-process)
//   * WASM-id — Option A mitigated: ints/handles + ids, strings resolved JS-side
//   * WASM-str— naive: per-call String marshal for tagName/getAttribute
// Also reports JS<->WASM boundary crossings per workload iteration.

import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);

let wasm;
try {
  wasm = require('../pkg-spike/turbo_dom_parser.js');
} catch (e) {
  console.error('! spike wasm not built. Run:\n  wasm-pack build --target nodejs --out-dir pkg-spike --no-default-features --features wasm-runtime');
  process.exit(1);
}

import { createEnvironment } from '../src/runtime/index.mjs';

// ---- fixture: RTL-style card grid ----
const N = 300;
const cards = Array.from({ length: N }, (_, i) =>
  `<div class="card sx-${i % 7}" data-testid="card-${i}" id="c${i}"><h2 class="title">T${i}</h2><p>body <a href="/x/${i}">link</a></p><button type="button">Go</button></div>`
).join('');
const html = `<!doctype html><html><body><main class="grid">${cards}</main></body></html>`;

// ---- boundary-crossing counter: wrap every wasm export ----
let CROSS = 0;
const W = {};
for (const k of Object.keys(wasm)) {
  if (typeof wasm[k] === 'function') {
    W[k] = (...a) => { CROSS++; return wasm[k](...a); };
  }
}

// =========================================================================
// WASM JS-front-end shim (Option A): lazy memoized handle wrappers + JS string table
// =========================================================================
const SEP = '';
function makeWasmDoc(html, mode) {                  // mode: 'id' | 'str' | 'rec'
  const naive = mode === 'str';
  const prefetch = mode === 'rec';
  const doc = W.create(html);                       // 1 crossing/parse
  const tagNames = W.tag_names_blob(doc).split(SEP); // pulled ONCE
  const attrNames = W.attr_names_blob(doc).split(SEP);
  const attrValues = W.attr_values_blob(doc).split(SEP);
  let wrappersMade = 0;
  const cache = new Map();                           // handle -> wrapper (identity)

  class Node {
    constructor(h) { this.h = h; this._tag = undefined; this._rec = undefined; }
    rec() {                                          // 1 crossing, then cached
      if (this._rec === undefined) {
        const r = W.node_record(doc, this.h);
        const tagId = r[0], parent = r[1], ac = r[2];
        const attrs = new Map();
        for (let k = 0; k < ac; k++) attrs.set(attrNames[r[3 + k * 2]], attrValues[r[4 + k * 2]]);
        this._rec = { tag: tagId < 0 ? null : tagNames[tagId], parent, attrs };
      }
      return this._rec;
    }
    get tagName() {
      if (naive) return W.tag_name_str(doc, this.h);
      if (prefetch) return this.rec().tag;
      if (this._tag === undefined) {                 // immutable -> cache once
        const id = W.tag_id(doc, this.h);
        this._tag = id < 0 ? null : tagNames[id];
      }
      return this._tag;
    }
    getAttribute(name) {
      if (naive) return W.get_attr_str(doc, this.h, name) ?? null;
      if (prefetch) { const v = this.rec().attrs.get(name); return v === undefined ? null : v; }
      const id = W.get_attr_id(doc, this.h, name);
      return id < 0 ? null : attrValues[id];
    }
    get parentNode() {
      const p = prefetch ? this.rec().parent : W.parent(doc, this.h);
      return p < 0 ? null : nodeAt(p);
    }
    dispatchWalk() { return W.dispatch_walk(doc, this.h); }
  }
  function nodeAt(h) {
    let n = cache.get(h);
    if (n === undefined) { n = new Node(h); cache.set(h, n); wrappersMade++; }
    return n;
  }
  return {
    querySelectorAll(sel) {                          // 1 crossing -> packed Uint32Array
      const handles = W.qsa(doc, sel);
      const out = new Array(handles.length);
      for (let i = 0; i < handles.length; i++) out[i] = nodeAt(handles[i]);
      return out;
    },
    stats: () => ({ wrappersMade }),
  };
}

// =========================================================================
// shared workload — identical sequence for every impl
// =========================================================================
function workload(els) {
  let sink = 0;
  for (let i = 0; i < els.length; i++) {
    const el = els[i];
    const cls = el.getAttribute('class');
    if (cls) sink += cls.length;
    const tid = el.getAttribute('data-testid');
    if (tid) sink += tid.length;
    const tag = el.tagName;
    if (tag) sink += tag.length;
    // parent walk to root
    let p = el.parentNode, depth = 0;
    while (p) { depth++; p = p.parentNode; }
    sink += depth;
    // listener-less dispatch path
    sink += el.dispatchWalk ? el.dispatchWalk() : dispatchJS(el);
  }
  return sink;
}

// JS-runtime dispatch (listener-less, bubbling) path length proxy
function dispatchJS(el) {
  let p = el, d = 1;
  while (p.parentNode) { d++; p = p.parentNode; }
  return d;
}

function bestOf(fn, n = 6, ms = 500) {
  let best = 0, sink = 0;
  for (let r = 0; r < n; r++) {
    // warm
    for (let i = 0; i < 200; i++) sink += fn();
    let iters = 0;
    const start = process.hrtime.bigint();
    const deadline = start + BigInt(ms) * 1_000_000n;
    while (process.hrtime.bigint() < deadline) { sink += fn(); iters++; }
    const elapsed = Number(process.hrtime.bigint() - start) / 1e6;
    const ops = (iters / elapsed) * 1000;
    if (ops > best) best = ops;
  }
  return { best, sink };
}

// ---- setup each impl: parse once, qsa once -> fixed element list (isolates per-node chatter) ----
console.log(`fixture: ${N} cards, html ${html.length} bytes\n`);

// JS runtime
const env = createEnvironment(html);
const jsEls = [...env.document.querySelectorAll('div.card')];

// WASM mitigated (per-call ids)
const wdoc = makeWasmDoc(html, 'id');
const widEls = wdoc.querySelectorAll('div.card');

// WASM naive (per-call strings)
const wdocN = makeWasmDoc(html, 'str');
const wstrEls = wdocN.querySelectorAll('div.card');

// WASM prefetch-record + cache (Option A scalars-on-wrapper)
const wdocR = makeWasmDoc(html, 'rec');
const wrecEls = wdocR.querySelectorAll('div.card');

console.log(`qsa('div.card') results: JS=${jsEls.length} WASM=${widEls.length} (match: ${jsEls.length === widEls.length})`);

// correctness: first card attrs must agree
const a = { js: jsEls[0].getAttribute('class'), wid: widEls[0].getAttribute('class'), wstr: wstrEls[0].getAttribute('class') };
console.log(`first .card class — JS:"${a.js}" WASM-id:"${a.wid}" WASM-str:"${a.wstr}"  match:${a.js === a.wid && a.js === a.wstr}`);
console.log(`first .card tagName — JS:${jsEls[0].tagName} WASM-id:${widEls[0].tagName}`);

// boundary-crossing count for one workload pass (mitigated)
// crossings: count the FIRST pass (cold) and a SECOND pass (warm cache) for 'rec'
CROSS = 0; workload(widEls);
const crossId = CROSS;
CROSS = 0; workload(wstrEls);
const crossStr = CROSS;
const wdocR2 = makeWasmDoc(html, 'rec');
const wrec2 = wdocR2.querySelectorAll('div.card');
CROSS = 0; workload(wrec2);               // cold: 1 record fetch per touched node
const crossRecCold = CROSS;
CROSS = 0; workload(wrec2);               // warm: records cached -> only dispatch_walk crosses
const crossRecWarm = CROSS;
console.log(`\nboundary crossings / workload pass (${N} nodes):`);
console.log(`  WASM-id=${crossId} (${(crossId / N).toFixed(1)}/node)  WASM-str=${crossStr} (${(crossStr / N).toFixed(1)}/node)`);
console.log(`  WASM-rec cold=${crossRecCold} (${(crossRecCold / N).toFixed(1)}/node)  warm=${crossRecWarm} (${(crossRecWarm / N).toFixed(1)}/node)  JS=0`);

// ---- benchmark (steady-state: warm caches, the RTL repeated-assert pattern) ----
console.log('\nbest-of-6 ops/s (full workload pass over all cards, steady-state):');
const rJs = bestOf(() => workload(jsEls));
const rId = bestOf(() => workload(widEls));
const rStr = bestOf(() => workload(wstrEls));
const rRec = bestOf(() => workload(wrecEls));   // warm after first pass
const fmt = (n) => Math.round(n).toLocaleString();
console.log(`  JS runtime   : ${fmt(rJs.best)}  (sink ${rJs.sink % 100003})`);
console.log(`  WASM-id      : ${fmt(rId.best)}  (${(rId.best / rJs.best).toFixed(2)}x JS)`);
console.log(`  WASM-str     : ${fmt(rStr.best)}  (${(rStr.best / rJs.best).toFixed(2)}x JS)`);
console.log(`  WASM-rec     : ${fmt(rRec.best)}  (${(rRec.best / rJs.best).toFixed(2)}x JS)  <- prefetch+cache`);

console.log(`\nlaziness: WASM-rec wrappers materialized = ${wdocR.stats().wrappersMade} (cards=${N})`);

const ratio = Math.max(rId.best, rRec.best) / rJs.best;
console.log(`\nVERDICT INPUT: best WASM variant is ${ratio.toFixed(2)}x the JS runtime on chatty access.`);
console.log(ratio >= 0.9 ? '=> within ~10% or faster — CONTINUE candidate' : '=> boundary tax dominates — PIVOT candidate (see plan §7)');
