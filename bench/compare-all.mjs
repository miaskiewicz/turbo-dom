// Comprehensive cross-DOM benchmark across 4 dimensions, all four impls:
// jsdom, happy-dom, the turbo-dom JS runtime, and the rtdom native Rust runtime.
//
//   npm run bench:compare:all      (wrapper runs the Rust bench first, injects RTDOM_JSON)
//
// Dimensions (300-card grid):
//   parse            build a queryable document from HTML (unique each iter → no cache)
//   construct+query  parse + querySelectorAll('div.card')
//   per-node         qsa once, then per-node getAttribute x2 + tagName + parent-walk
//   repeated-query   qsa('div.card') on an unchanged tree (cached for turbo-dom/rtdom)

import { JSDOM } from 'jsdom';
import { Window } from 'happy-dom';
import { createEnvironment } from '../src/runtime/index.mjs';

const N = 300;
function fixture(suffix = '') {
  let h = '<!doctype html><html><body><main class="grid">';
  for (let i = 0; i < N; i++) {
    h += `<div class="card sx-${i % 7}" data-testid="card-${i}" id="c${i}">` +
         `<h2 class="title">T${i}</h2><p>body</p><button type="button">Go</button></div>`;
  }
  return h + '</main></body></html>' + suffix;
}
const HTML = fixture();

// time-boxed (cheap, alloc-free dims): per-node, repeated-query
function bench(thunk, ms = 400, runs = 6) {
  let best = 0, sink = 0;
  for (let r = 0; r < runs; r++) {
    for (let i = 0; i < 30; i++) sink += thunk(i);
    let iters = 0;
    const start = process.hrtime.bigint();
    const deadline = start + BigInt(ms) * 1_000_000n;
    while (process.hrtime.bigint() < deadline) { sink += thunk(iters); iters++; }
    const ops = (iters / (Number(process.hrtime.bigint() - start) / 1e6)) * 1000;
    if (ops > best) best = ops;
  }
  globalThis.__sink = (globalThis.__sink || 0) + sink;
  return best;
}

// fixed-count (heavy: builds a fresh document per call — jsdom/happy-dom leak under a
// tight time-box → OOM). Bounded iters + GC between runs keeps memory sane.
// Times each build individually — GC happens OUTSIDE the timed span (so it never
// pollutes the measurement) and reclaims leaky jsdom/happy-dom docs between samples.
// ops/s = 1000 / best (min) single-build ms.
function benchFixed(thunk, samples = 30) {
  let bestMs = Infinity, sink = 0, base = 0;
  for (let i = 0; i < 3; i++) sink += thunk(base++); // warmup
  for (let s = 0; s < samples; s++) {
    if (globalThis.gc) globalThis.gc();              // reclaim before timing (untimed)
    const t0 = process.hrtime.bigint();
    sink += thunk(base++);
    const ms = Number(process.hrtime.bigint() - t0) / 1e6;
    if (ms < bestMs) bestMs = ms;
  }
  globalThis.__sink = (globalThis.__sink || 0) + sink;
  return 1000 / bestMs;
}

function perNode(els) {
  let s = 0;
  for (const el of els) {
    const c = el.getAttribute('class'); if (c) s += c.length;
    const t = el.getAttribute('data-testid'); if (t) s += t.length;
    const tag = el.tagName; if (tag) s += tag.length;
    let p = el.parentNode, d = 0; while (p) { d++; p = p.parentNode; }
    s += d;
  }
  return s;
}

// per-impl document builders + element-list getter
const impls = {
  jsdom: {
    doc: (h) => new JSDOM(h).window.document,
  },
  'happy-dom': {
    doc: (h) => { const w = new Window(); return new w.DOMParser().parseFromString(h, 'text/html'); },
  },
  'turbo-dom (JS)': {
    doc: (h) => createEnvironment(h).document,
  },
};

const dims = ['parse', 'construct+query', 'per-node', 'repeated-query'];
const table = {}; // impl -> {dim: ops}

for (const [name, { doc }] of Object.entries(impls)) {
  table[name] = {};
  // parse (unique html → defeat turbo-dom parse cache) — fixed-count to bound memory
  table[name].parse = benchFixed((i) => doc(fixture(`<!--${name}${i}-->`)).body.childElementCount);
  // construct + query
  table[name]['construct+query'] = benchFixed((i) => doc(fixture(`<!--q${name}${i}-->`)).querySelectorAll('div.card').length);
  // per-node (fixed tree)
  const els = [...doc(HTML).querySelectorAll('div.card')];
  table[name]['per-node'] = bench(() => perNode(els));
  // repeated query (fixed tree)
  const d = doc(HTML);
  table[name]['repeated-query'] = bench(() => d.querySelectorAll('div.card').length);
}

// rtdom native (Rust) numbers via RTDOM_JSON
if (process.env.RTDOM_JSON) {
  try {
    const r = JSON.parse(process.env.RTDOM_JSON);
    table['rtdom (native Rust)'] = {
      parse: r.parse,
      'construct+query': r.construct,
      'per-node': r.per_node,
      'repeated-query': r.repeated_query,
    };
  } catch {}
}

// --- print matrix ---
const names = Object.keys(table);
const fmt = (n) => (n ? Math.round(n).toLocaleString() : '—');
console.log(`\nfixture: ${N} cards, ${HTML.length} bytes — ops/s (higher = faster), best-of-6\n`);
const head = 'impl'.padEnd(24) + dims.map((d) => d.padStart(16)).join('');
console.log(head);
console.log('-'.repeat(head.length));
for (const name of names) {
  console.log(name.padEnd(24) + dims.map((d) => fmt(table[name][d]).padStart(16)).join(''));
}

// normalized vs turbo-dom-JS per dimension
const base = table['turbo-dom (JS)'];
console.log(`\n× vs turbo-dom (JS):`);
console.log('impl'.padEnd(24) + dims.map((d) => d.padStart(16)).join(''));
for (const name of names) {
  console.log(name.padEnd(24) + dims.map((d) => {
    const v = table[name][d], b = base[d];
    return (v && b ? (v / b).toFixed(2) + '×' : '—').padStart(16);
  }).join(''));
}
console.log(`\n(jsdom/happy-dom/turbo-dom-JS are an in-process A/B; rtdom is cross-runtime — directional.)`);
