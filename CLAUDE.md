# CLAUDE.md

Dev guide for this repo. See `turbo-dom-spec.md` for the full design and `README.md` for usage.

## What this is

Layer 1 of turboDom: a native HTML parser (Rust + `html5ever`) with napi-rs (native) and
wasm-bindgen (fallback) front-ends over **one shared core**. Parser produces a complete
nested tree and crosses the JS boundary once ("full marshaling"). The SoA flat-buffer from
the architecture doc is **intentionally not built yet** — it's gated on marshaling proving
to be the cost (a later bench). Don't SoA-ify `core.rs`.

## Build & test

```bash
npm run build         # native addon → turbo-dom-parser.<platform>.node + index.js/.d.ts (napi codegen)
npm run build:wasm    # wasm32 fallback
npm test              # JS: node --test  (MUST glob: 'test/*.mjs' — `node --test test/` is misparsed on Node 24)
npm run test:rust     # cargo test --lib  (core unit tests live in src/core.rs #[cfg(test)])
npm run conformance   # html5lib-tests gate
```

Toolchain: Node ≥ 24, Rust stable via rustup (`source $HOME/.cargo/env` if cargo isn't on PATH).

## Architecture

- `src/core.rs` — the only place html5ever is touched. `parse_html_document`,
  `parse_html_fragment_context`. Returns `core::Node` (plain, no binding deps).
- `src/lib.rs` — two feature-gated front-ends (`napi-bind` default, `wasm-bind`). Each is a
  thin `From<core::Node>` / serde conversion. Keep logic in core, not here.
- `harness/` — JS conformance tooling (dat parser, serializer, runner). All unit-tested.
- `src/runtime/` — the lazy COW DOM + window (Layers 2–5). `index.mjs` exports
  `createEnvironment(html)` → `{ window, document, reset, touched }`. Nodes inflate lazily
  from the parser tree and memoize for identity (`dom.mjs` `Document.__inflate`); the window
  Proxy (`window.mjs`) self-replaces lazy globals and traces touched ones. Selectors read the
  internal `node.__children()` array, NOT the live `childNodes` Proxy. Honest stubs in
  `stubs.mjs` (never invent layout/cascade numbers). Validated by `test/differential.test.mjs`
  (jsdom oracle + happy-dom), `test/gauntlet.test.mjs` (RTL unmodified), `test/runtime.test.mjs`.

## Non-obvious things (read before editing)

- **Field naming:** napi `#[napi(object)]` camelCases fields. Rust `public_id` → JS `publicId`,
  `node_type` → `nodeType`. The serializer reads the **camelCase** JS names.
- **Scripting flag is OFF** (`opts()` sets `scripting_enabled: false`) to match html5lib-tests'
  default, so `<noscript>` content is parsed as markup, not rawtext. Don't flip it.
- **`<template>` content** is NOT in `children` — html5ever puts it in `template_contents`.
  `walk()` appends it as a synthetic `nodeType 11` `content` fragment node. html5lib prints it
  as the literal word `content`.
- **Foreign content:** elements carry `namespace` ("svg"/"math", else ""); foreign attrs carry
  `prefix` ("xlink" etc). The serializer renders `<svg svg>` and `xlink href="…"`.
- **Fragment context:** `parseFragment(html, context)` where context is `"td"` or namespaced
  `"svg path"` / `"math ms"`. Empty → body.
- **Hot paths are allocation-free — keep them that way.** `querySelectorAll`/`querySelector`/
  `getElementsByTagName`/`getElementsByClassName` and selector matching MUST NOT allocate per
  element: no `el.classList` (allocates a ClassList + splits), no `Array.find`/`.some`/`.filter`
  closures, no regex, no per-node filtered child arrays. Use the `hasClass`/`elHasClass`
  whole-word string scan, index loops over `__children()`, and the for-loop `getAttribute`.
  This is what makes query-heavy RTL suites ~6× jsdom; a stray `classList` in a matcher tanks it.
- **Selector parse cache** (`__selectorCache` in selectors.mjs) memoizes selector STRING → AST.
  Pure (DOM-independent) → no mutation invalidation. Never cache *results* without it.

## Benchmarks

```bash
npm run bench           # parse throughput (parseBuffer vs parse5/happy-dom/jsdom)
npm run bench:construct # per-file construction + surface-usage histogram
npm run bench:suite     # 200-file suite wall-clock; lazy-vs-eager nodes/window
npm run bench:query     # query-heavy DOM work (RTL-style) vs jsdom
npm run bench:wasm      # wasm vs native parseBuffer
npm run bench:all       # all of the above
```
Latest (darwin-arm64, Node 24), vs jsdom / happy-dom:
- per-file setup: ~23× jsdom, ~10× happy-dom
- realistic 200-file suite (construct+query+events): ~23× jsdom, ~10× happy-dom
- parse: 18–37× both
- conformance: 99.72% vs jsdom 97.03% vs happy-dom 37.35%
- raw query throughput: ~7× jsdom, but happy-dom is ~25× US here (its selector engine
  trades correctness for speed). We win realistic suites because per-file CONSTRUCTION
  dominates, and lazy construction crushes it — not because raw queries are fastest.
- `getByLabelText` was O(n²) (element.labels → document.getElementsByTagName('label') per
  element); fixed with a version-keyed getElementsBy* cache (Document.__byTag/__byClass,
  invalidated by Document.__version which every mutation bumps). 1342→277µs.
Numbers in README.md — refresh both (bench against jsdom AND happy-dom) if you touch hot paths.

## Conformance gate

`test/conformance.test.mjs` locks the rate ≥ 99.5% AND asserts **zero non-`<select>` failures**.
On `html5ever` 0.39 the 5 known misses are all bleeding-edge `<select>`-family proposals
(`<selectedcontent>`, `<input>`/`<button>`-in-select) the newest html5lib-tests track but the
crate hasn't adopted. If a failure appears whose input has no `<select>`, it's likely a
marshaling/serializer bug in THIS repo and the test will fail loudly. Investigate before
touching the threshold. To improve further: bump `html5ever` (picks up upstream fixes) — never
by loosening the serializer to match.

## Adding a node-type or field

1. Add to `core::Node` (+ all construction sites: walk, fragment root, template content node).
2. Mirror in `src/lib.rs` `JsNode` + the `From` impl (and it flows to wasm via serde).
3. Handle it in `harness/serialize.mjs` `serializeNode`.
4. Add a Rust unit test (`src/core.rs`) and a serializer unit test (`test/serialize.test.mjs`).
