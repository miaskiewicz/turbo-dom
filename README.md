# turbo-dom

A faster, more spec-correct DOM for test runners — a drop-in-style alternative to
**jsdom** and **happy-dom** for **vitest** and **jest**.

The HTML parser is native ([html5ever](https://github.com/servo/html5ever), Servo's
WHATWG tree constructor, via Rust/N-API with a WASM fallback). The DOM itself stays in
JavaScript but is **lazy** — nodes inflate from a compact typed-array buffer only when a
test touches them, and `window` globals materialize only on first use.

```bash
npm install -D @miaskiewicz/turbo-dom
```

- ✅ **More compatible than happy-dom** — 99.72% on html5lib-tests vs happy-dom's 37%.
  Runs React Testing Library, `user-event`, downshift, Radix UI, and Headless UI unmodified.
- ⚡ **Faster than both** — ~130× jsdom / ~45× happy-dom on a realistic suite (parse-memoized repeated shells), ~8–44× faster HTML parsing on real pages, and ~2.7× happy-dom on repeated queries while staying 99.7% spec-correct.
- 🎨 **Real computed style** — `getComputedStyle` runs a **partial cascade**: it resolves real
  injected `<style>` rules (emotion/MUI `.css-HASH{…}`) + inline styles with proper specificity/source
  order, **inherits the standard inheritable properties down the tree** (a global `body{color}` reaches
  descendants), and **canonicalizes colors to `rgb()/rgba()`** like a browser (so
  `toHaveStyle({color:'#fff'})` matches a `#ffffff`/`white`/`rgb(255,255,255)` rule). It only ever
  returns values a real rule set — never an invented one.
- 🎯 **Honest, not lying** — no fake layout numbers; `getBoundingClientRect()` is zeros and computed
  style never invents layout. Geometry tests belong in a real browser.

## Quick start

### vitest

```ts
// vitest.config.ts
import { defineConfig } from 'vitest/config';
import { createRequire } from 'node:module';

const envPath = createRequire(import.meta.url).resolve('@miaskiewicz/turbo-dom/environment/vitest');

export default defineConfig({
  test: {
    environment: envPath, // vitest resolves a bare name only for `vitest-environment-*`
                          // packages, so a scoped package is referenced by file path
  },
});
```

Works on vitest 1–4.

### jest

```js
// jest.config.js
module.exports = {
  testEnvironment: '@miaskiewicz/turbo-dom/jest',
};
```

Now `document`, `window`, and friends are global in your tests — write them exactly like
you would against jsdom/happy-dom:

```js
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

test('counter increments', async () => {
  render(<Counter />);
  await userEvent.click(screen.getByRole('button'));
  expect(screen.getByText('count: 1')).toBeInTheDocument();
});
```

### Without a test runner

```js
import { createEnvironment } from '@miaskiewicz/turbo-dom/runtime';

const env = createEnvironment('<!doctype html><body><div id="app"></div></body>');
env.document.querySelector('#app');     // nodes inflate lazily from the parse buffer
env.window.localStorage;                // globals materialize on first touch
env.reset();                            // fast per-file reset (reuses the parse buffer)
```

### Just the parser

```js
const { parse, parseBuffer, parseFragment } = require('@miaskiewicz/turbo-dom');

parse('<div id=a><span>hi</span></div>');         // nested tree
parseBuffer('<div id=a>...</div>');                // compact SoA typed-array buffer
parseFragment('<rect/>', 'svg path');              // fragment in a context element
```

### Choosing the parser backend (native vs WASM)

The runtime resolves a parser lazily on first parse: native N-API addon first, WASM
fallback. Force it per environment or globally:

```js
import { createEnvironment, setParserMode } from '@miaskiewicz/turbo-dom/runtime';

createEnvironment(html, { parser: 'wasm' });   // 'wasm' | 'native' | 'auto' (default)
setParserMode('wasm');                          // process-global; also TURBO_DOM_PARSER=wasm
```

### Embedding in a non-Node runtime (bare V8, no Node builtins)

The native addon and the `--target nodejs` WASM build both need Node. For a fully
node-free host, instantiate the `--target web` WASM yourself and inject the binding —
the runtime then never touches `node:module`/`fs`:

```js
import init, { initSync, parse, parseBuffer, parseFragment } from '@miaskiewicz/turbo-dom/parser-wasm';
import { setParser, createEnvironment } from '@miaskiewicz/turbo-dom/runtime';

initSync({ module: wasmBytes });                // you supply the bytes (sync, no fs)
setParser({ parse, parseBuffer, parseFragment });
createEnvironment('<div id=app/>');             // now runs with zero Node deps
```

`setParser` also reads `globalThis.__TURBO_DOM_PARSER__` if you prefer injection by
global. Use `installGlobals(globalThis, { html })` (from `@miaskiewicz/turbo-dom/install`)
to set up `document`/`window` on any global object.

## Compatibility

| | turbo-dom | happy-dom | jsdom |
|---|---|---|---|
| html5lib-tests conformance | **99.72%** | 37.35% | 97.03% |
| @testing-library/dom + user-event | ✅ | ✅ | ✅ |
| React + Radix / Headless UI / downshift | ✅ | partial | ✅ |
| Real layout | ❌ (honest stub) | partial | partial |
| `getComputedStyle` cascade | partial (real `<style>` + inline + inheritance + `rgb()` colors) | partial | partial |
| Shadow DOM (attach, slots, event retargeting, scoped/`:host` CSS) | ✅ | ✅ | ✅ |

turbo-dom inherits Servo's tree constructor, so the "messy input" cases hand-rolled parsers
get wrong — adoption-agency reparenting (`<a><p></a></p>`), table foster-parenting, optional
end tags, `<template>` content, SVG/MathML — all match the spec. The 5 remaining
conformance misses are bleeding-edge `<select>`-family proposals upstream `html5ever` hasn't
adopted yet.

## Performance

Measured on darwin-arm64, Node 24 (`npm run bench:all`). Higher = faster, except
the suite row (ms/file, lower = faster):

| benchmark | turbo-dom | happy-dom | jsdom |
|---|---:|---:|---:|
| **realistic suite**, 200 files (ms/file) | **~0.025** | 1.08 | 3.20 |
| **cold per-file construct + query** (ops/s) | **~230k** | 575 | 267 |
| **parse 56 KB SSR** (ops/s) | **528** | 48 | 65 |
| **parse 20 KB real page** (ops/s) | **4,353** | 135 | 99 |
| repeated query throughput (iters/s) | **~1.8M** | 661k | 3.3k |
| html5lib conformance | **99.72%** | 37.35% | 97.03% |

On a realistic suite — 200 files of construct + queries + events — turbo-dom is
**~45× happy-dom and ~130× jsdom**, runs repeated queries **~2.7× happy-dom**
(~550× jsdom), and parses real pages/SSR documents **~8–44×** faster, all at 99.7%
conformance.

The per-file setup number is so high because the parser **memoizes the read-only
SoA buffer by HTML string**: a suite calls the env setup with the same document
shell every file, so it's parsed once and the buffer (never mutated — all changes
go to per-Document overlays) is reused. The first parse of a given shell pays full
cost (the parse rows above); every reuse is near-free.

**turbo-dom wins across the board on what test suites actually do**: per-file
construction (~45× happy-dom, ~130× jsdom on a repeated-shell suite), parsing,
spec-correctness (99.7% vs 37%), **and** repeated queries.

How the query speed holds up against happy-dom (whose whole design trades correctness
for query speed): the selector/match engine is allocation-free on the hot paths (no
per-element `classList`/`split`/regex), and `querySelectorAll`/`getElementsBy*`/
`getElementById` results are **cached per (selector, DOM-version)** — a static
`querySelectorAll` list is safe to reuse until the next mutation. So the repeated
queries RTL/`findBy`/`waitFor` run against an unchanged tree are near-free, and
`getByLabelText` went from O(n²) (1.3 ms) to ~270 µs.

## How it works

```
test code (RTL, user-event)
   └─ lazy window (Proxy, self-replacing globals)        ← JS
   └─ lazy copy-on-write node tree (memoized identity)   ← JS
   └─ immutable Structure-of-Arrays parse buffer          ← shared
        └─ Rust: html5ever → flat typed-array buffer       ← native (N-API / WASM)
```

The parser runs in Rust (compute-bound, one boundary crossing per parse). The DOM stays in
JS (chatty, fine-grained) but pays only for what a test touches. Full design notes:
[turbo-dom-spec.md](./turbo-dom-spec.md).

## Rust-native DOM runtime (`rtdom`) — for Rust consumers

Everything above is the **JS-consumer** path: the DOM is JS objects, so React/RTL touch it with
zero boundary — that's why it's fast for vitest/jest. A **Rust** consumer (crawler, extractor,
server-side scraper) wants the opposite: the DOM in-process in Rust, no JS at all. For that there's
**`rtdom`** — a pure-Rust port of the runtime (lazy copy-on-write tree over the same SoA buffer,
version-cached queries, partial `getComputedStyle`, events, shadow DOM, serialize).

Why a separate runtime instead of exposing this one to Rust via WASM? Measured: a Rust-DOM-in-WASM
called *from JS* is **~0.55×** the JS runtime (the boundary crossing dominates — exactly what the
[spec §3](./turbo-dom-spec.md) predicted), while `rtdom` run **in-process from Rust** is **~2.7×**
the JS runtime on the same chatty workload (zero boundary). So: JS consumers keep the JS runtime,
Rust consumers use `rtdom`.

Add it to a Rust project from crates.io (the `turbo-dom` crate is the Rust-native
runtime; the npm `@miaskiewicz/turbo-dom` is the JS path):

```bash
cargo add turbo-dom
```

```rust
use turbo_dom::{Dom, DocumentExt};
use turbo_dom::rtdom::cascade;

let mut dom = Dom::parse("<main class=grid><div class=card id=hero>hi</div></main>");
let cards = dom.tree.query_selector_all("div.card");        // version-cached, in-process
let id = dom.tree.get_attribute(cards[0], "id");            // Some("hero") — plain Rust call
let style = cascade::computed_style(&dom.tree, cards[0]);   // partial honest cascade
```

- **crates.io:** [`turbo-dom`](https://crates.io/crates/turbo-dom) — the Rust DOM engine lives
  here and only here (the workspace-member crate `crates/turbo-dom/`; there is no in-repo
  `src/rtdom` copy and no `rust-runtime` cargo feature — an earlier feature gated a duplicate copy
  that was consolidated away). Minimal deps (`html5ever` + `rustc-hash`, no napi/wasm), a runnable
  `examples/crawl.rs`, 227 tests, 100% line coverage, and a direct html5lib-tests gate at
  **99.75%**. From the repo: `npm run build:rtdom` (= `cargo build -p turbo-dom --release`),
  `npm run test:rust` (= `cargo test -p turbo-dom`), `npm run conformance:rtdom`.

## Limitations (by design)

- **No layout.** `getBoundingClientRect()` returns zeros; `getClientRects()` is empty.
- **`getComputedStyle` is a partial cascade** — it resolves REAL injected `<style>` rules
  (emotion/MUI `.css-HASH{…}`) plus inline `style`, applying specificity and source order
  (inline wins). It **inherits the standard inheritable properties** (`color`, `font*`,
  `line-height`, `text-align`, `visibility`, …) down the flattened tree — a global rule on
  `body`/`:root` reaches descendants, and inheritance crosses the shadow host boundary. It
  **canonicalizes `<color>` values to `rgb()/rgba()`** (`#fff`/`white`/`hsl(...)` → `rgb(255, 255, 255)`)
  exactly as browsers serialize computed style, so `@testing-library/jest-dom`'s `toHaveStyle`
  color assertions compare equal regardless of how the rule authored the color. It expands common
  shorthands to longhands (`margin`/`padding`/`border`/single-token `background`), serializes bare
  `0` as `0px` for length props, and normalizes `font-family` comma spacing. Out of scope (returns
  `''`): `@media`/`@supports`/`@keyframes`, `:hover`/state pseudo-classes, pseudo-elements, the
  `inherit`/`initial`/`unset` keywords, CSS custom properties, and length-unit conversion
  (`em`/`rem`→`px`). Only ever returns values from a matched, inline, or inherited declaration —
  never an invented initial value. Style/geometry assertions belong in a real browser
  (Playwright/WebDriver).
- **`<style>.textContent` reflects rules injected via `sheet.insertRule()`** — CSS-in-JS engines in
  "speedy" mode (emotion/styled-components) inject rules straight into the CSSOM without writing the
  node's text; turbo-dom serializes them back into `textContent` (as browsers/jsdom do), so tests
  that scrape `querySelectorAll('style')` text see the injected CSS.
- Canvas, `<select>` rendering, and similar visual APIs are honest no-op stubs.
- **Shadow DOM** is supported and pay-for-what-you-use — every event/query/cascade hot path
  is unchanged until the first `attachShadow` flips a per-document flag. Covered: `attachShadow`
  (open/closed), encapsulated `querySelector`/`getElementById`, `getRootNode({composed})`,
  full event propagation with `target`/`relatedTarget` retargeting and `composed` boundary
  crossing, `<slot>` `assignedNodes`/`assignedElements`/`assignedSlot`, scoped `getComputedStyle`
  with `:host`/`:host(...)`/`::slotted(...)` and inheritance across the boundary, and declarative
  `<template shadowrootmode>` promotion. Out of scope (honest): flattened-tree layout, `slotchange`
  events, and the cascade caveats above (`@media`/state/pseudo-elements) inside shadow trees.

## Development

Requires Node ≥ 24 and a Rust toolchain (`rustup`, stable). `mise.toml` pins both.

```bash
npm install
npm run build           # native addon (.node) + wasm — the JS-consumer artifacts
npm test                # JS suite (unit, conformance, differential, gauntlets)
npm run test:rust       # Rust tests: parser core + the turbo-dom crate (cargo test -p turbo-dom)
npm run conformance     # html5lib-tests report (parser)
npm run conformance:rtdom  # html5lib-tests gate run through the rtdom tree
npm run bench:all       # JS-runtime benchmarks
npm run build:wasm      # wasm32 parser fallback
npm run build:rtdom     # pure-Rust DOM runtime (no napi/wasm) — the Rust-consumer build
```

The JS runtime (`src/runtime/*.mjs`) and the Rust runtime (the `crates/turbo-dom` crate) are
independent — touching one never affects the other. See
[RUST_PORT_PLAN.md](./RUST_PORT_PLAN.md) for the dual-runtime architecture and
[RUST_PORT_PERF_HISTORY.md](./RUST_PORT_PERF_HISTORY.md) for how each JS perf win maps to Rust.

Contributions welcome — issues and PRs at
[github.com/miaskiewicz/turbo-dom](https://github.com/miaskiewicz/turbo-dom).

## License

[MIT](./LICENSE).
