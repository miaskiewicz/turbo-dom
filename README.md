# fast-dom-parser

Native HTML parser for a lazy, copy-on-write DOM test runtime — **Layer 1** of the
[fast-dom design](./fast-dom-spec.md). One Rust core backed by
[`html5ever`](https://github.com/servo/html5ever) (Servo's spec-compliant WHATWG tree
constructor), exposed through two interchangeable front-ends:

| Front-end | Build | Use |
|---|---|---|
| **napi-rs** native addon | `npm run build` | default fast path |
| **wasm-bindgen** `wasm32` | `npm run build:wasm` | StackBlitz / WebContainers / locked-down CI |

Both expose the same API and the boundary is crossed **exactly once per parse**. The runtime
uses `parseBuffer()` — a compact **Structure-of-Arrays** typed-array buffer that the JS DOM
inflates node handles from lazily (the plan's SoA optimization, shipped). A full lazy
copy-on-write DOM + lazy window (Layers 2–5) is assembled on top.

## Thesis, restated

happy-dom traded **correctness** for speed → a permanent compatibility tax, and its
hand-rolled parser will never catch the WHATWG tree-construction edge cases (table
foster-parenting, optional end tags, `<template>` content, SVG/MathML foreign content).
We **inherit** Servo's correctness instead of chasing it.

## Conformance

Gated against [`html5lib-tests`](https://github.com/html5lib/html5lib-tests)
tree-construction (the WHATWG conformance suite, fuzzed against browsers):

```
PASS 1778  FAIL 5  ERROR 0  SKIP 8
Conformance: 99.72%   (55/57 fixture files fully clean, 0 crashes)
```

**All 5 remaining misses are bleeding-edge `<select>`-family proposals** the newest
html5lib-tests track but `html5ever` 0.39 hasn't adopted — the experimental
`<selectedcontent>` / customizable-`<select>` element and `<input>`/`<button>`-in-select
edge cases. **Zero are marshaling or serializer bugs** — a regression test asserts any
non-`<select>` failure fails the build (`test/conformance.test.mjs`). Chasing them means
patching the parser, against the "inherit Servo's correctness" thesis.

### Delta vs happy-dom / jsdom (same suite, same serializer)

| engine | pass | fail | crash | rate |
|---|---:|---:|---:|---:|
| **fast-dom** | 1778 | 5 | 0 | **99.72%** |
| jsdom | 1730 | 53 | 0 | 97.03% |
| happy-dom | 666 | 1116 | 1 | **37.35%** |

happy-dom's 1116 failures are concentrated in the **adoption agency algorithm**
(`<a><p></a></p>` reparenting) and **table foster-parenting** — the exact "messy input"
bug class the design autopsy predicted from a hand-rolled parser. fast-dom inherits
Servo's tree constructor and sidesteps all of it.

Run it:

```bash
npm run conformance              # our summary + per-file table
npm run conformance:delta        # fast-dom vs happy-dom vs jsdom
node harness/conformance.mjs --verbose          # show failing diffs
node harness/conformance.mjs --file tests1.dat  # one fixture
```

## Speed

Parse throughput, ops/sec (higher is better), `npm run bench`. `parseBuffer()` is the
**SoA fast path** (typed arrays, the runtime uses it); `parse()` is the old full JS-tree
marshaling; `parseRaw()` is parse-only (the floor):

| fixture | `parseBuffer()` | `parse()` | `parseRaw()` | parse5 | happy-dom | jsdom |
|---|---:|---:|---:|---:|---:|---:|
| small (122 B) | 70.8k | 49.4k | 308k | 325k | 17.7k | 17.8k |
| ssr-large (56 KB) | 502 | 119 | 707 | 651 | 46 | 23 |
| deep-nested (4.4 KB) | 2,459 | 1,171 | 2,706 | 1,143 | 714 | 84 |
| malformed (92 B) | 27.1k | 21.4k | 229k | 222k | 6.2k | 2.2k |
| real-storybook (20 KB) | 3,912 | 1,806 | 6,448 | 2,200 | 353 | 100 |

- **SoA win:** `parseBuffer()` is **1.3–4.2× faster than the old `parse()`** (4.2× on 56 KB SSR,
  2.2× on real HTML) and now within **1.4–1.65× of the `parseRaw` floor** (full marshaling was
  4–8× off). It beats parse5 on the large fixtures and crushes happy-dom/jsdom by **11–39×**.
- **Why:** `parseRaw` ≈ parse5, so html5ever itself is fast — the cost was building the JS tree.
  SoA emits compact typed arrays once and inflates node objects only on access, deleting the
  eager full-tree allocation. This is the Phase-2.5 bet from the plan, now shipped.
- **wasm fallback** (`npm run bench:wasm`): 49–105% of native throughput — acceptable, same SoA
  contract, single boundary copy.

## API

```js
const { parse, parseBuffer, parseFragment } = require('fast-dom-parser');

parseBuffer('<div id=a><span>hi</span></div>');  // → SoA typed arrays (runtime fast path)
parse('<div id=a><span>hi</span></div>');         // → nested tree { nodeType, name, children … }
parseFragment('<li>one</li>', );                   // body context
parseFragment('<rect/>', 'svg path');             // foreign context
```

`parse()` nodes: `{ nodeType, name, value, namespace, publicId, systemId, attrs, children }`
where `attrs` is `{ name, value, prefix }[]`. nodeTypes follow the DOM (`1` element, `3` text,
`8` comment, `9` document, `10` doctype, `11` template-content fragment). The DOM runtime below
uses `parseBuffer()`.

## Test runtime (Layers 2–5)

A lazy, copy-on-write DOM + window assembled over the native parser:

```js
import { createEnvironment } from './src/runtime/index.mjs';

const env = createEnvironment('<!doctype html><body><div id=app></div></body>');
env.document.querySelector('#app');     // nodes inflate lazily from the buffer
env.window.localStorage;                // globals materialize on first touch
env.reset();                            // arena-style per-file reset
```

- **Layer 2** — node handles inflate lazily from the immutable parse buffer; first
  access memoizes the handle, so `===` identity, WeakMap keys, and event targets hold.
  Mutation promotes the node to fully-owned (COW). `childNodes`/`children`/
  `getElementsByTagName`/`ClassName` are **live**. Full event model (capture/target/bubble,
  `composedPath`, `stopImmediatePropagation`, `once`, typed events). Selector engine
  (combinators, `[attr op]`, `:not`, structural pseudos), `innerHTML`/`outerHTML`.
- **Layer 3** — `window` is a Proxy; ~lazy globals self-replace on first `get` and the
  Proxy traces which a test touched. Render-only tests construct **zero** globals.
- **Layer 4** — honest stubs: `getBoundingClientRect()` → zeros, `getComputedStyle` →
  inline-only (empty, never a plausible lie), no reflow.
- **Layer 5** — `reset()` drops the COW overlay + node cache + materialized globals,
  keeping the buffer and classes warm.

Light-test path (construct + one query): **25.6× jsdom, 12.9× happy-dom**
(`npm run bench:construct`).

### Full benchmark report

Numbers from one run on darwin-arm64 (Node 24). Reproduce with `npm run bench:all`.

**Per-file construction + 1 light query** (`bench/construct.mjs`, ops/sec):

| engine | ops/sec | vs jsdom |
|---|---:|---:|
| fast-dom | 6,808 | **25.6×** |
| happy-dom | 526 | 2.0× |
| jsdom | 266 | 1.0× |

Lazy payoff inside fast-dom: touch-1-node **6,928** vs touch-all+globals **934** → the light
path is **7.4×** the full-inflation path (laziness pays when a test touches little).

**Real test-suite wall-clock** — 200 files, fresh env each (`bench/suite.mjs`, lower better):

| engine | total | per file | |
|---|---:|---:|---|
| fast-dom | 25 ms | **0.125 ms** | — |
| happy-dom | 289 ms | 1.447 ms | 9.1× slower |
| jsdom | 671 ms | 3.356 ms | **21.0× slower** |

- lazy vs eager **nodes** (fast-dom): 33 ms vs 38 ms (1.15× — workload touches most nodes here)
- lazy vs eager **window** (construction only): 12,509 vs 10,173 constructs/sec (1.23×)

**WASM vs native** parseBuffer (`bench/wasm.mjs`, ops/sec):

| fixture | native | wasm | wasm/native |
|---|---:|---:|---:|
| small | 71,396 | 71,430 | 100% |
| ssr-large | 520 | 242 | 47% |
| deep-nested | 2,599 | 2,015 | 78% |
| malformed | 62,625 | 54,773 | 87% |
| real-storybook | 4,716 | 2,746 | 58% |

Worst case 47% of native — fallback acceptable, same SoA contract, single boundary copy.

## Testing — every bench & test the plan specifies (§9)

| Phase | Plan-specified item | Where | Result |
|---|---|---|---|
| **1** | eager-vs-lazy window microbench | `bench/suite.mjs` (c) | 1.10× (our globals are cheap stubs) |
| **1** | surface-usage histogram (tracer) | `bench/construct.mjs` | render-only → **0** globals; routing → history+location |
| **1** | real-suite wall-clock, per-file | `bench/suite.mjs` (a) | **18.9× jsdom, 6.9× happy-dom** (0.12 ms/file) |
| **2** | parse throughput vs JS parsers | `bench/parse.mjs` | 11–39× happy-dom/jsdom; SoA beats parse5 on large |
| **2** | boundary-cost isolation (parse vs marshal) | `bench/parse.mjs` (`parseRaw`) | drove the SoA build; now within 1.4–1.65× of floor |
| **2** | WASM-vs-native delta | `bench/wasm.mjs` | 49–105% of native — fallback acceptable |
| **2** | html5lib conformance + delta | `harness/delta.mjs` | **99.72%** vs jsdom 97.03% vs happy-dom 37.35% |
| **3** | differential vs jsdom (oracle) | `test/differential.test.mjs` | matches jsdom exactly across fuzz seeds |
| **3** | real-library gauntlet (RTL + user-event) | `test/gauntlet.test.mjs`, `test/userevent.test.mjs` | both run **unmodified** |
| **3** | liveness/identity property tests | `test/liveness.test.mjs`, `test/runtime.test.mjs` | adoption-agency, foster-parenting, WeakMap, `===` |
| **3** | invariant fallback audit | `test/liveness.test.mjs` | **0** divergences (lazy ≡ eager; no fallback needed) |
| **3** | lazy-vs-eager NODES wall-clock | `bench/suite.mjs` (b) | workload-dependent: 1.05× here, 2.6× when tests touch little |

- **Differential (oracle):** deterministic random op sequences applied to fast-dom, jsdom,
  and happy-dom; fast-dom must match jsdom exactly (it does). happy-dom compared too —
  where it disagrees with jsdom it's happy-dom's bug, and fast-dom sides with jsdom.
- **Gauntlet:** `@testing-library/dom` and `@testing-library/user-event` (click, type,
  keyboard, checkbox toggle via default actions) run **unmodified** — the test happy-dom fails.

85 tests total (7 Rust core + 78 JS), all green. Run all benches: `npm run bench:all`.

## Build & test

```bash
npm install
npm run build           # native addon (.node)
npm run build:wasm      # wasm32 fallback

npm test                # full JS suite (unit, conformance, differential, gauntlet, runtime)
npm run test:rust       # Rust core unit tests
npm run test:all        # build + both suites
npm run conformance     # html5lib gate
npm run conformance:delta   # vs happy-dom / jsdom
npm run bench           # parse throughput
npm run bench:construct # Phase-1 construction + surface histogram
```

Requires Node ≥ 24 (uses `node --test`) and a Rust toolchain (`rustup`, stable). The
wasm build also needs `rustup target add wasm32-unknown-unknown`.

## Layout

```
src/core.rs            shared parser core (html5ever → nested tree). No binding deps.
src/lib.rs             napi-rs + wasm-bindgen front-ends over the core.
src/runtime/           the lazy COW DOM + window (Layers 2–5):
  dom.mjs                nodes, lazy inflation, COW, live collections, identity
  events.mjs             EventTarget/Event + typed events (full propagation model)
  selectors.mjs          CSS selector engine
  collections.mjs        live NodeList / HTMLCollection
  window.mjs             lazy self-replacing window Proxy + tracer
  stubs.mjs              honest layout/CSSOM/storage/observer stubs
  index.mjs              createEnvironment() + reset()
harness/               html5lib conformance tooling (dat parser, serializer, gate, delta)
bench/                 parse throughput + construction benchmarks
test/                  unit, conformance, differential, RTL-gauntlet, runtime suites
vendor/html5lib-tests/ 57 WHATWG tree-construction fixtures
```

## Status

All layers built and tested:

- **Layer 1** native parser — full-marshaling milestone. Faster than happy-dom/jsdom and
  more spec-correct (99.72%).
- **Layers 2–5** lazy COW DOM + lazy window + honest stubs + fast reset — passing
  differential-vs-jsdom fuzzing and the unmodified RTL gauntlet.

Next (optimization, not correctness): the **SoA flat buffer** the benches justify — `parse()`
spends 2.2–8× longer than `parseRaw()` building the JS tree, and lazy nodes already avoid
inflating untouched subtrees; moving the source-of-truth to a typed-array SoA would shrink
both the marshaling cost and per-file reset further. Plus live wasm throughput numbers.
