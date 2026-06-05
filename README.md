# fast-dom-parser

Native HTML parser for a lazy, copy-on-write DOM test runtime — **Layer 1** of the
[fast-dom design](./fast-dom-spec.md). One Rust core backed by
[`html5ever`](https://github.com/servo/html5ever) (Servo's spec-compliant WHATWG tree
constructor), exposed through two interchangeable front-ends:

| Front-end | Build | Use |
|---|---|---|
| **napi-rs** native addon | `npm run build` | default fast path |
| **wasm-bindgen** `wasm32` | `npm run build:wasm` | StackBlitz / WebContainers / locked-down CI |

Both expose the same API and the boundary is crossed **exactly once per parse** — the
whole tree returns as a single value (the plan's "full marshaling" milestone; the SoA
flat-buffer is a deliberately deferred optimization).

## Thesis, restated

happy-dom traded **correctness** for speed → a permanent compatibility tax, and its
hand-rolled parser will never catch the WHATWG tree-construction edge cases (table
foster-parenting, optional end tags, `<template>` content, SVG/MathML foreign content).
We **inherit** Servo's correctness instead of chasing it.

## Conformance

Gated against [`html5lib-tests`](https://github.com/html5lib/html5lib-tests)
tree-construction (the WHATWG conformance suite, fuzzed against browsers):

```
PASS 1755  FAIL 28  ERROR 0  SKIP 8
Conformance: 98.43%   (49/57 fixture files fully clean, 0 crashes)
```

**Every one of the 28 misses is the same upstream `html5ever` 0.27 divergence** on the
`<select>` insertion mode (`<menuitem>`/`<keygen>`/`<svg>`/`<math>`/nested-select), where
the crate lags a later WHATWG spec change. **Zero are marshaling or serializer bugs** —
a regression test asserts any non-`<select>` failure fails the build (`test/conformance.test.mjs`).

### Delta vs happy-dom / jsdom (same suite, same serializer)

| engine | pass | fail | crash | rate |
|---|---:|---:|---:|---:|
| **fast-dom** | 1755 | 28 | 0 | **98.43%** |
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

Parse throughput, ops/sec (higher is better), `node bench/parse.mjs`:

| fixture | fast-dom `parse()` | fast-dom `parseRaw()` | parse5 | happy-dom | jsdom |
|---|---:|---:|---:|---:|---:|
| small (122 B) | 45.8k | 287k | 329k | 17.0k | 24.4k |
| ssr-large (56 KB) | 111 | 704 | 635 | 50 | 64 |
| deep-nested (4.4 KB) | 1,204 | 2,685 | 1,161 | 237 | 82 |
| malformed (92 B) | 27.6k | 222k | 240k | 12.5k | 2.3k |
| real-storybook (20 KB) | 1,696 | 6,502 | 2,381 | 343 | 153 |

**fast-dom `parse()` beats happy-dom by 2.2–5.1× and jsdom by 1.7–14.7× on every fixture** —
the actual competitors, which (like us) build a usable tree.

The `parse()` vs `parseRaw()` gap (parse-only, no JS tree) is the key signal: **2.2–8× — the
full-marshaling JS-tree build is the dominant cost, not parsing** (raw html5ever ≈ parse5).
This validates the plan's Phase-2.5 bet: the SoA buffer + lazy COW nodes (Layer 2) are worth
building — they recover most of that gap by not allocating nodes the test never touches.

## API

```js
const { parse, parseFragment } = require('fast-dom-parser');

parse('<div id=a><span>hi</span></div>');
// → { nodeType: 9, name: '#document', children: [ <html> … ] }

parseFragment('<li>one</li><li>two</li>');            // body context
parseFragment('<rect/>', 'svg path');                 // foreign context
```

Each node: `{ nodeType, name, value, namespace, publicId, systemId, attrs, children }`
where `attrs` is `{ name, value, prefix }[]`. nodeTypes follow the DOM
(`1` element, `3` text, `8` comment, `9` document, `10` doctype, `11` template-content fragment).

## Build & test

```bash
npm install
npm run build         # native addon (.node)
npm run build:wasm    # wasm32 fallback

npm test              # JS suite (unit + serializer + dat-parser + conformance gate)
npm run test:rust     # Rust core unit tests
npm run test:all      # build + both suites
```

Requires Node ≥ 24 (uses `node --test`) and a Rust toolchain (`rustup`, stable). The
wasm build also needs `rustup target add wasm32-unknown-unknown`.

## Layout

```
src/core.rs            shared parser core (html5ever → nested tree). No binding deps.
src/lib.rs             napi-rs + wasm-bindgen front-ends over the core.
harness/dat.mjs        html5lib-tests .dat fixture parser.
harness/serialize.mjs  tree → html5lib dump-format serializer.
harness/conformance.mjs the conformance gate (CLI + reusable runner).
test/                  JS unit tests (smoke, dat, serialize, conformance).
vendor/html5lib-tests/ 57 WHATWG tree-construction fixtures.
```

## Status

Layer 1 (native parser) — **done at the full-marshaling milestone.** Next per the plan:
boundary-cost isolation bench (parse-only vs parse+marshal) to decide whether the SoA
buffer is worth building, then Layers 2–5 (lazy COW nodes, lazy window, honest stubs,
fast per-file reset).
