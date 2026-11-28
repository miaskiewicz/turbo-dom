# turbo-dom

Release **0.8.7** (local unreleased candidate). turbo-dom is the shared browser-engine base of three projects:

- **[turbo-test](https://github.com/miaskiewicz/turbo-test)**, a vitest-compatible test runner. It runs tests in V8 over this repository's DOM.
- **[turbo-html2pdf](https://github.com/miaskiewicz/turbo-html2pdf)**, which renders HTML to PDF and uses this repository's layout engine.
- **[turbo-surf](https://github.com/miaskiewicz/turbo-surf)**, a browserless crawler and MCP server that uses the DOM, the V8 binding, the layout engine and the screenshot painter.

> Agents: start at [docs/AGENTS.md](docs/AGENTS.md) for the one-command workflows in `scripts/agent/`.

turbo-dom is also a standalone npm package: a fast DOM environment for
**vitest** and **jest**, usable in place of jsdom or happy-dom.

Up to 0.5.x, each consumer kept its own copy of these pieces. Since 0.6.0 each piece lives
in this repository once and is published from here. The consumers depend on the
published crates and keep no copies.
[docs/CONSOLIDATION.md](docs/CONSOLIDATION.md) has the architecture, the dependency graph
and the publish order.

## Packages

The DOM packages share version 0.8.7. Before registry publication, local
consumers use `[patch.crates-io]` overrides for a verified release-branch
revision. After publication, consumers can resolve 0.8.7 from the registry. The independently versioned V8 fork remains published at 150.4.2
and uses the official V8 150.4.0 archive; the AVIF decoder fork `turbo-dom-rav1d`
remains 1.1.0.

| package | registry | version | what it is | used by |
|---|---|---|---|---|
| [`@miaskiewicz/turbo-dom`](https://www.npmjs.com/package/@miaskiewicz/turbo-dom) | npm | 0.8.7 | lazy JS DOM runtime + native html5ever parser (N-API, WASM fallback) | vitest / jest users |
| [`turbo-dom`](crates/turbo-dom) | crates.io | 0.8.7 | pure-Rust DOM (`rtdom`), the one selector engine, CSSOM, partial cascade | all three consumers |
| [`turbo-dom-layout`](crates/turbo-dom-layout) | crates.io | 0.8.7 | HTML/CSS layout (block / inline / flex / grid / table), fonts, images, CSSOM-View geometry | turbo-html2pdf, turbo-surf |
| [`turbo-dom-raster`](crates/turbo-dom-raster) | crates.io | 0.8.7 | synthetic screenshots (PNG / SVG), canvas op-log replay, WebGL readback | turbo-surf |
| [`turbo-dom-browser-env`](crates/turbo-dom-browser-env) | crates.io | 0.8.7 | the `rtdom` <-> V8 browser environment (window, document, CSSOM, forms, events) | turbo-test, turbo-surf |
| [`turbo-dom-v8`](crates/turbo-dom-v8) | crates.io | 150.4.2 | turbo-dom's rusty_v8 fork, generated from the upstream crate plus reviewed patches | turbo-dom-browser-env, turbo-surf |

The root `turbo-dom-parser` crate is `publish = false`. It ships inside the npm package.

```
turbo-dom ──► turbo-dom-layout ──► turbo-dom-raster
    │               │                    │
    │               ├──► turbo-html2pdf  │
    │               └────────────────────┴──► turbo-surf
    └──► turbo-dom-browser-env ◄── turbo-dom-v8
               ├──► turbo-test
               └──► turbo-surf
```

## Install

### npm (vitest / jest)

```bash
npm install -D @miaskiewicz/turbo-dom
```

Prebuilt native binaries are bundled for linux x64 (glibc and musl), linux arm64,
macOS arm64 and windows x64. Other platforms use the WASM parser.

**vitest** (1–4):

```ts
// vitest.config.ts
import { defineConfig } from 'vitest/config';
import { createRequire } from 'node:module';

const envPath = createRequire(import.meta.url).resolve('@miaskiewicz/turbo-dom/environment/vitest');

export default defineConfig({
  test: { environment: envPath }, // a scoped package is referenced by file path
});
```

**jest**:

```js
// jest.config.js
module.exports = { testEnvironment: '@miaskiewicz/turbo-dom/jest' };
```

`document` and `window` are then global, as with jsdom. React Testing Library,
`user-event`, downshift, Radix UI and Headless UI run unmodified.

**Without a test runner:**

```js
import { createEnvironment } from '@miaskiewicz/turbo-dom/runtime';

const env = createEnvironment('<!doctype html><body><div id="app"></div></body>');
env.document.querySelector('#app');     // nodes inflate lazily from the parse buffer
env.reset();                            // fast per-file reset (reuses the parse buffer)
```

### DOM backend selection

Each environment accepts `domBackend`. The default, `'js'`,
uses the existing npm DOM. The `'rust'` value is reserved for the Node integration
of `turbo-dom-browser-env`. In this build it raises a setup error before creating
a document. Parser selection is independent, so the JS DOM can still use the native
Rust parser or WASM.

| Entry point | Explicit JS configuration |
| --- | --- |
| npm runtime | `createEnvironment(html, { domBackend: 'js' })` |
| Vitest config | `test.environmentOptions.turboDom.domBackend = 'js'` |
| Jest config | `testEnvironmentOptions.turboDom.domBackend = 'js'` |

### Experimental CSS backend selection (v0.8.8 development)

The [initial packed Vitest measurements](docs/benchmarks/css-opt-in-2026-11-27.json)
compare selector reuse and shared-addon loading. Later experiments reduced
[equality and native request overhead](docs/benchmarks/css-equality-native-hit-2026-11-27.json).
The [singleton cascade experiment](docs/benchmarks/css-singleton-selection-2026-11-27.json)
avoids sorting single-candidate declarations. The [latest keyword and scope-root
experiments](docs/benchmarks/css-keyword-scope-roots-2026-11-28.json) avoid tokenizer
setup for plain ASCII identifiers and heap allocation for unscoped roots, while
preserving validation and fallback behavior. Combined native-workload CPU fell
7.99% (7/7 ABBA block wins). Packed Vitest measured 2.55% lower matched median wall
time (5/7 wins); adding keywords to scope-root alone was effectively flat in the
packed suite but reduced native-workload CPU by 2.00% (6/7 wins).
The [bounded native success-cache experiment](docs/benchmarks/css-exact-success-cache-2026-11-28.json)
retains up to eight exact successful requests under the existing byte limits and
owner/token checks. Authentic native-workload CPU fell 11.55% (7/7 ABBA wins),
with 50 cache hits instead of 13. Packed Vitest was effectively flat: 0.36% lower
matched median wall time (4/7 wins). A fresh same-package comparison measured
Rust 7.79% slower than legacy JS; its final JS run had a retained load outlier.
These separate-window results do not establish performance parity or a stable
whole-suite speedup. The CSS default remains JavaScript.

`cssBackend: 'rust'` opts into the shared Rust computed-style engine while keeping
`domBackend: 'js'`. The CSS default remains `'js'`; this is independent of the
full Rust DOM integration described above. Missing or incompatible CSS artifacts
produce an explicit error rather than silently changing the selected backend.

| Entry point | Rust CSS opt-in |
| --- | --- |
| npm runtime | `createEnvironment(html, { cssBackend: 'rust' })` |
| Vitest config | `test.environmentOptions.turboDom.cssBackend = 'rust'` |
| Jest config | `testEnvironmentOptions.turboDom.cssBackend = 'rust'` |

Rust CSS also accepts the explicit `cssResultProtocol: 'native-state-v1'` option.
It retains resolved styles natively and projects CSSOM without transferring the
full result back through JavaScript. The default `wire-v1` protocol preserves the
existing resolve/serialize callbacks. This is one Rust CSS engine with two result
protocols, not another backend; see [the protocol and callback contract](docs/native-css-result-protocol.md).
Missing native-state capabilities fail explicitly. The isolated native replay
used 5.12% less CPU (7/7 ABBA wins); packed suite wall/CPU were effectively flat,
so this does not establish Rust-versus-JavaScript performance parity.

Repeated Rust CSS reads reuse selector inputs only after freshly capturing and
validating the complete matching input. DOM, stylesheet, state, shadow-tree and
owner changes retain their invalidation checks. With an unresolved explicitly
native parser, Rust CSS can load one combined parser/CSS addon; existing parser
selections and injected bindings are left intact. A combined addon initialization
error can therefore surface during parsing instead of the first style read.

**Just the parser:**

```js
const { parse, parseBuffer, parseFragment } = require('@miaskiewicz/turbo-dom');
parse('<div id=a><span>hi</span></div>');   // nested tree
parseBuffer('<div id=a>...</div>');          // compact SoA typed-array buffer
parseFragment('<rect/>', 'svg path');        // fragment in a context element
```

**Parser backend.** The default is the native addon, with WASM as the fallback. To force
one backend, use `createEnvironment(html, { parser: 'wasm' | 'native' | 'auto' })`,
`setParserMode('wasm')` or `TURBO_DOM_PARSER=wasm`.

To run without Node builtins (bare V8): instantiate the `--target web` build from
`@miaskiewicz/turbo-dom/parser-wasm` yourself. Pass the binding to `setParser({ parse,
parseBuffer, parseFragment })` or set it as `globalThis.__TURBO_DOM_PARSER__`. Then call
`installGlobals(globalThis, { html })` from `@miaskiewicz/turbo-dom/install`.

### Rust crates

```toml
[dependencies]
turbo-dom = "0.8.7"
turbo-dom-layout = "0.8.7"                 # bundled fonts on by default
turbo-dom-raster = "0.8.7"
turbo-dom-browser-env = "0.8.7"            # needs a V8 archive, see its README
```

```rust
use turbo_dom::Dom;
use turbo_dom::rtdom::cascade;

let dom = Dom::parse("<main class=grid><div class=card id=hero>hi</div></main>");
let cards = dom.tree.query_selector_all("div.card");      // version-cached
let id = dom.tree.get_attribute(cards[0], "id");          // Some("hero")
let style = cascade::computed_style(&dom.tree, cards[0]); // partial, honest cascade
```

Each crate's README covers its API: [turbo-dom](crates/turbo-dom/README.md),
[turbo-dom-layout](crates/turbo-dom-layout/README.md),
[turbo-dom-raster](crates/turbo-dom-raster/README.md),
[turbo-dom-browser-env](crates/turbo-dom-browser-env/README.md),
[turbo-dom-v8](crates/turbo-dom-v8/README.md). MSRV: Rust 1.89 for turbo-dom, -layout and
-raster; Rust 1.91 for turbo-dom-v8 and -browser-env.

## Feature flags

| crate | feature | default | effect |
|---|---|---|---|
| `turbo-dom-layout` | `bundled-fonts` | on | embeds OFL faces (Inter/Roboto, Liberation Serif/PT Serif, Fira Code/IBM Plex Mono) so a document lays out with no caller fonts |
| | `hyphenation` | on | `hyphens: auto` for English (`en` / `en-*`, not en-GB) with the embedded en-US TeX patterns (~90 KB); off, `auto` acts as `manual` |
| | `hyphenation-languages` | on | `hyphens: auto` in every other language Chrome 156 on macOS hyphenates (en-GB, ca, cs, da, de, el, es, fi, fr, hr, hu, it, nb / no, nl, pl, pt, ro, ru, sk, sv, uk) with the TeX patterns (~3 MB, all of the `hyphenation` crate's); implies `hyphenation` |
| | `svg` | off | routes `image/svg+xml` through the SVG rasterizer in the image pipeline (implies `svg-rasterize`) |
| | `svg-rasterize` | off | the shared resvg rasterizer only (`svg::rasterize`) |
| | `serde` | off | `Serialize` for the CSSOM-View metrics records |
| | `perf` | off | hot-path instrumentation counters |
| | `test-support` | off | font fixtures and test literals for downstream tests; never in production |
| `turbo-dom-raster` | `avif` | off | AVIF pixels (pure-Rust rav1d, via the `turbo-dom-rav1d` fork) |
| | `jxl` | off | JPEG XL pixels (jxl-oxide) |
| | `gpu-metal` | off | wgpu (Metal) canvas and WebGL rasterization, falling back to the CPU path on any GPU failure |
| | `coretext` | off | macOS only: canvas text through CoreText (inert elsewhere) |
| `turbo-dom-browser-env` | `host-internals` | off | doc-hidden host access to native documents (turbo-surf) |
| | `test-support` | off | probe, control and case constants for embedders' Chrome-parity tests |
| | `native-sdk-frame-read` | off | reads the interceptor's should-throw mode from the V8 frame, for native SDKs without the fork fix |
| `turbo-dom-parser` (npm) | `napi-bind` / `wasm-bind` / `wasm-runtime` | `napi-bind` | the native addon, the WASM parser and the WASM DOM-runtime spike |

`turbo-dom` offers opt-in `computed-style-grammar` for the shared CSS property
grammar used by the optional npm Rust CSS backend. Its DOM/parser defaults keep
that dependency disabled. `turbo-dom-v8` forwards upstream rusty_v8's features
(for example `simdutf`).

## Status

- **0.8.7:** canonical animation sampling and recovery for native
  embedders, shadow-inclusive document checkpoints, SVG paint and metric fixes,
  and an optional shared Rust CSS engine. The npm DOM now includes corrected
  history events, textarea defaults and selection, disabled/inert focus, nested
  shadow focus and DOMRect branding. [Full release notes](CHANGELOG.md#087--2026-11-27).

- **0.8.6:** the `parseBuffer` parser builds its flat buffer from an
  index-arena tree sink instead of a reference-counted tree, and gates
  `<selectedcontent>` with a SIMD scan: Rust-level `parse_html_soa` is 9-25% faster
  across the bench fixtures. Grid baseline items lay out on their natural lines and
  align on their first text line again (#537).

- **0.8.5:** reduce cascade declaration cloning, winner-map lookups
  and inherited-value allocations. See the changelog and the reproducible cascade
  performance harness.

- **0.8.4:** bounded shaping-cache retention, shared immutable styles
  and font-family storage, and inline text paths that avoid unused box parsing and
  disabled wrapping work. See the changelog and turbo-dom#508.
  The host-only auxiliary browsing-context boundary (#504) adds genuine native
  WindowProxy admission and lifecycle APIs; a consumer must supply its real Page,
  network and policy integration. See the browser-env auxiliary-context guide.

- **0.8.3:** browser binding fixes include branded Touch events, hidden-control geometry,
  and document replacement that preserves Window/document identity. Screenshot APIs add
  reusable prepared scenes, resource planning, masks and fractional device-scale handling.
  Embedders can independently opt into jsdom-compatible TouchEvent init lists and declared
  computed `text-decoration` serialization; Chrome behavior remains the default. See the
  [browser-env compatibility switches](crates/turbo-dom-browser-env/README.md#api)
  and [CHANGELOG.md](CHANGELOG.md) for scope and limitations.
- **Parser:** html5lib-tests conformance is 99.94% through the npm parser (happy-dom
  37.35%, jsdom 97.03%). The remaining misses are `<select>`-family proposals that
  upstream html5ever has not adopted yet. The Rust `rtdom` tree passes its direct
  html5lib-tests gate at 100% (`npm run conformance:rtdom`).
- **Chrome parity:** behaviour fixes are backed by Chrome 156 controls. These are
  fixtures recorded from headless Chrome and asserted in tests. Every corpus output change
  is declared in [`tools/parity/expected/`](tools/parity/expected) (checkpoint `b-0.7.0`).
  0.7.0 adds vertical writing modes, UAX #9 bidi, Chrome's line box model on the PDF path,
  the full `border-radius` model with `overflow` clipping, background tiling and gradient
  length stops, native form-control appearance, size / layout containment, replaced media
  boxes, XML documents (with DTD ATTLIST defaults), processing instructions, CSS
  animations in `getAnimations()`, a browser-env cascade with specificity, `@layer`,
  `@supports` and `@scope`, real canvas `measureText` and an opt-in V8 startup snapshot
  install (see [CHANGELOG.md](CHANGELOG.md)).
- **Open gaps (tracked as issues):**
  - glyph stems in raster screenshots lack Chrome's stem darkening, so glyph edges are
    up to ~0.5px thinner per side (#266).
  - Known difference, not tracked: `hyphens: auto` breaks at the TeX patterns' points
    (what Chrome uses on Linux, Windows and Android); Chrome on macOS hyphenates with the
    system dictionary, whose points differ for a few words (en `international`, pl
    `szczególnie`; `tests/fixtures/hyphens/words-chrome156.json` lists them).
- **The npm runtime has no layout.** Geometry is synthetic: `getBoundingClientRect()`,
  `offsetWidth/Height`, `clientWidth/Height` and `scrollWidth/Height` return a non-zero
  size that is stable for a given DOM state (blocks fill the parent's width, the root
  takes `innerWidth` or 1024; heights stack children or 18px per line), while every
  position (`top` / `left`, `offsetTop/Left`, `scrollTop/Left`) is 0.
  `getComputedStyle` is a partial cascade: top-level `<style>` rules (and rules injected
  with `insertRule`) plus inline styles, ordered by Selectors 4 specificity and source
  order, with a fixed set of inherited properties. Colours read back as `rgb()` /
  `rgba()` and a bare `0` as `0px`; a property no rule sets reads `''` (never an initial
  value). Not resolved: rules inside any at-rule block (`@media`, `@supports`, `@layer`,
  `@container`, `@scope`) are skipped, rules with state pseudo-classes (`:hover`, ...)
  never match, a pseudo-element argument is ignored (the element's own style comes
  back) and `::before`-style rules never apply. Values come back as written: no unit
  conversion (`2em` stays `2em`), and `var(...)` and `inherit` / `initial` / `unset` are
  returned verbatim, not resolved. Shadow DOM is supported: `attachShadow`, declarative
  shadow roots, slots and `assignedNodes()`, event retargeting, scoped styles with
  `:host` / `::slotted()`, and inheritance across the host boundary. Layout belongs to
  `turbo-dom-layout`, and through it to turbo-html2pdf and turbo-surf.

## Benchmarks

### Single-compound selector matching (2026-11-26)

The held N-API CSS build with direct single-compound matching took 2.23% less
wall time than the preceding Rust build in the packed Vitest suite (four of
seven ABBA blocks faster). Repeated DOM capture used 4.06% less CPU. In a
separate comparison with legacy JS the candidate took 10.69% more wall time;
legacy JS won all seven blocks. All 110 observed style results matched the
previous Rust build byte for byte.

[Individual runs and capture measurements](tools/acceptance/packed-vitest-296/results-2026-11-26-selector/RESULT.json)

### Packed N-API selector-answer reuse (2026-11-26)

The held Rust CSS build retained selector answers on the prepared model for 97
of 110 style reads. Request traffic fell from 1,857,987 to 370,456 bytes,
including token arguments. All 110 style results matched byte for byte. Across
seven ABBA blocks the combined capture and transport changes took 0.31% less
wall time than the previous Rust build (four blocks faster). The candidate took
10.71% more wall time than legacy JS, which won all seven blocks.

[Artifact hashes, checks and timings](tools/acceptance/packed-vitest-296/results-2026-11-26-retained/BENCHMARK.json)

### Packed N-API empty-child capture (2026-11-26)

The Rust CSS validation build can skip copying an empty child array during DOM
capture. The 28-test packed Vitest workload hit that case 657 times in 110 style
reads. Median process wall time fell 0.45% across seven ABBA blocks, with four
blocks faster. Summed test callback time rose 0.57%. A separate experiment that
preclassified selector guard descriptors took 0.20% more wall time at the median.
The integrated capture patch adds an intrinsic guard. That revision passed
1,144 tests with 100% line coverage and 97.34% branch coverage. The timings
above refer to the pilot before the final guard refinement.

[Per-block timings and package file hashes](tools/acceptance/packed-vitest-296/results-2026-11-26-empty-children/BENCHMARK.json)

### Packed N-API after inheritance-chain assembly (2026-11-26)

Across seven ABBA blocks, the packed Rust CSS candidate with the `aa69a733`
adapter took 13.69% more wall time than the preserved legacy JavaScript
CSS build at the median. Median process times were 1.537 s for Rust and
1.396 s for legacy JS. Rust won 0 of seven blocks. Both packages passed
the same 28 tests in every timed process. The Rust CSS preflight passed separately.

[Raw timings, fixture and package hashes](tools/acceptance/packed-vitest-296/results-2026-11-26-chain/LEGACY.json)

### Rust CSS capture CPU (2026-11-26)

On the packed N-API candidate, 100 repeated DOM captures used 14.1% less CPU
than the previous Rust build at the median. 7 of seven ABBA blocks improved by over 0.5%.
The fast path checks that an ordinary element has the original prototype chain
and lacks four optional selector-state fields. Elements with any of those fields
still go through the full descriptor checks on every capture.

Before a final helper extraction, the 28-test Vitest benchmark took 4.43% more wall time
than the previous Rust build and 14.12% more than legacy JavaScript
at the median. System load varied substantially during the process timings.
Separate unfiltered acceptance passed all 29 compatibility tests
and the Rust CSS check on both N-API and WASM. The default switch is pending.

A second seven-block run used the final packed N-API candidate throughout.
For 100 captures it used 15.08% less CPU (7/7 blocks improved by over 0.5%).
Across the 28 Vitest tests it took 1.14% less wall time than the previous
Rust package and 9.71% more than legacy JS at the median.
[Final-build rerun timings](tools/acceptance/packed-vitest-296/results-2026-11-26-capture/RETRY.json)

[Timings and package hashes](tools/acceptance/packed-vitest-296/results-2026-11-26-capture/RESULT.json) · [Capture workload](tools/acceptance/packed-vitest-296/results-2026-11-26-capture/capture.mjs) · [Candidate delta against the previous Rust package](tools/acceptance/packed-vitest-296/results-2026-11-26-capture/candidate.patch)

### Packed Vitest after selector collection and encoding changes (2026-11-26)

The packed Rust CSS candidate includes the matching changes from `8137bae4`.
The previous Rust package includes the routed-request changes from `06a799d4`.
The legacy package uses JavaScript for CSS. Each comparison ran five ABBA
blocks with fresh single-worker Vitest processes on the same 28 tests. The
time columns give medians across ten processes per package. The last column
gives the median within-block change in process wall time.

| Mode | Comparison package | Comparison (s) | Rust candidate (s) | Change |
|---|---|---:|---:|---:|
| N-API | Previous Rust | 1.737 | 1.714 | +0.29% |
| N-API | Legacy JS | 1.502 | 1.688 | +12.04% |
| WASM | Previous Rust | 1.909 | 1.914 | -0.26% |
| WASM | Legacy JS | 1.518 | 1.951 | +28.59% |

On N-API, isolated measurements across the 13 selector rebuilds showed 9.8%
less time collecting unscoped matches and 12.0% less time encoding answers.
Each stage improved in seven of seven comparisons. Separate packed acceptance
runs passed all 29 compatibility tests and the Rust CSS check on each backend.
The release keeps the existing CSS default.

[Benchmark timings and package identities](tools/acceptance/packed-vitest-296/results-2026-11-26-matching/RESULT.json)

### Packed Vitest before and after the Rust CSS switch (2026-11-26)

The [packed consumer fixture](tools/acceptance/packed-vitest-296/README.md)
runs the same 28 React, Testing Library and forms tests in each environment.
Each measurement uses fresh Vitest processes, one worker, one warmup per
environment and five rotated rounds. Times are total process wall times.
Lower is faster.

| Environment | Before median (s) | Before min–max (s) | After median (s) | After min–max (s) | Median change |
|---|---:|---:|---:|---:|---:|
| happy-dom | 1.774 | 1.740–1.807 | 1.909 | 1.873–1.923 | +7.6% |
| jsdom | 2.806 | 2.728–2.966 | 2.961 | 2.908–3.189 | +5.5% |
| turbo-dom (native) | 1.382 | 1.337–1.488 | 1.892 | 1.841–1.928 | +36.9% |
| turbo-dom (WASM) | 1.392 | 1.368–1.402 | 2.191 | 2.143–2.228 | +57.4% |

The turbo-dom baseline uses the legacy JavaScript CSS engine. The measured candidate
used the shared Rust CSS engine by default and passed the public CSS regression
and all 29 compatibility tests in both modes. The release branch retains the
JavaScript CSS default. Both control environments were
slower in the second window too. The measured changes compare the packed
candidates end to end and include startup, parser and runtime costs.

[Before results](tools/acceptance/packed-vitest-296/results-2026-11-26/RESULT.json)
and [after results](tools/acceptance/packed-vitest-296/results-2026-11-26-after/RESULT.json)
include per-process reports, machine and dependency versions, and artifact
identities. See the [packed candidate receipt](tools/acceptance/packed-vitest-296/results-2026-11-26-after/PACKED-ACCEPTANCE.json)
for the public Vitest results.

### Matched Vitest rerun (2026-11-26)

The [paired runner](tools/acceptance/packed-vitest-296/benchmark-paired.mjs)
alternated the legacy package and the optimized Rust candidate in the same
window, with the exact same fixtures and resolved dependencies. Each mode had
five adjacent pairs and passed the same 28 tests in every timed process.

| Mode | Legacy median (s) | Optimized Rust median (s) | Median paired change |
|---|---:|---:|---:|
| N-API | 1.767 | 2.024 | +20.9% |
| WASM | 1.741 | 2.184 | +35.1% |

The Rust candidate was slower in every pair. The paired change is the median
of the five within-pair ratios. See the [matched receipt](tools/acceptance/packed-vitest-296/results-2026-11-26-matched/RESULT.json)
for exact package manifests, all timings, and the two passing CSS preflight
checks.

### Rust CSS profiling and reuse pilot (2026-11-26)

Vitest worker profiles attributed 256 ms of sampled time to Rust-backed
`getComputedStyle` on N-API and 541 ms on WASM. Selector matching accounted
for 66 and 77 ms, respectively. The cascade request, including transport,
accounted for 56 and 128 ms. WASM also spent 107 ms on compile and preparation
and 57 ms loading the CSS backend. These are sampled times across seven
profiled test workers per mode.

An unpublished prototype reused matching data for identical selector inputs.
It passed the CSS preflight and all 28 timed tests. The five-pair median
was 2.1% faster than the previous Rust candidate on N-API and 1.3% slower on
WASM. Compared with the legacy JavaScript package, its five-pair medians
were still 11.3% slower on N-API and 29.7% slower on WASM.

See the [worker profile summary](tools/acceptance/packed-vitest-296/results-2026-11-26-hotspots/PROFILE-SUMMARY.json),
[pilot versus legacy](tools/acceptance/packed-vitest-296/results-2026-11-26-hotspots/PILOT-VS-LEGACY.json),
and [pilot versus the previous Rust build](tools/acceptance/packed-vitest-296/results-2026-11-26-hotspots/PILOT-VS-RUST.json).

v0.8.2 re-measurement — darwin-arm64, Node 26.11, 2026-11-10. Higher is faster except
the suite row (ms/file, lower is faster).

| benchmark | turbo-dom | happy-dom | jsdom |
|---|---:|---:|---:|
| realistic suite, 200 files (ms/file) | 0.015 | 1.108 | 3.091 |
| cold per-file construct + query (ops/s) | 172,562 | 675 | 294 |
| parse 56 KB SSR (ops/s, parseBuffer) | 736 | 63 | 70 |
| parse 20 KB real page (ops/s, parseBuffer) | 4,893 | 546 | 469 |
| repeated query throughput (iters/s) | 2,059,250 | 740,421 | 3,909 |

Ratios: ~26x happy-dom / ~72x jsdom on the suite, ~2.8x happy-dom / ~527x jsdom on
repeated queries, ~9–12x on realistic parses. WASM fallback runs at 42–67% of native
parseBuffer. Full results, methodology, load averages and caveats (machine was not
quiet; fixed per-fixture contestant order) in
[docs/BENCHMARKS-history.md](docs/BENCHMARKS-history.md), which also keeps the v0.1.55
baseline for comparison. 0.7.0's browser-env performance work (measured on a real
React/MUI suite) is listed in [CHANGELOG.md](CHANGELOG.md).

## Development

Requires Node ≥ 24 and Rust 1.98.0 (CI's toolchain; `mise.toml` pins Node and Rust).
`npm install` also points git at `.githooks`, whose pre-commit hook runs `npm run test:cov`
whenever `src/`, `test/` or `harness/` changes.

```bash
npm run build              # native addon + both WASM packages
npm test                   # JS suite (unit, conformance, differential, gauntlets)
npm run test:cov           # same + coverage gate (lines 100 / functions 100 / branches 97.3)
npm run test:vitest && npm run test:jest
npm run test:rust          # cargo test -p turbo-dom
npm run conformance        # html5lib-tests report (parser)
npm run conformance:rtdom  # html5lib-tests through the rtdom tree
cargo test -p turbo-dom-layout          # + --features svg / perf / serde,test-support
cargo test -p turbo-dom-raster          # + --features avif / jxl
python3 crates/turbo-dom-v8/scripts/generate.py --download --write   # generate the V8 fork
cargo test -p turbo-dom-browser-env --features host-internals,test-support   # needs RUSTY_V8_ARCHIVE
npm run complexity:rust    # cc-check --max 9 over tracked Rust except browser-env (install: CLAUDE.md)
npm run complexity:js      # oxlint complexity <= 9 over src/ (npm runtime)
```

[CLAUDE.md](CLAUDE.md) has the complete command list and the design rules.

**Test fonts come from fixtures.** Text-geometry tests load the repository's own font
files (`turbo_dom_layout::test_support::fixture_fonts()`, from
`crates/turbo-dom-layout/assets/fonts`). Their expected values come from Chrome 156 with
the same files loaded through `@font-face` (`scripts/fixture-font-chrome-controls.mjs`).
Results therefore do not depend on the fonts installed on the host. Only the tests of
the host-font policy itself read system fonts.

### Gates

| gate | where | status |
|---|---|---|
| line coverage (tarpaulin, LLVM) | `ci.yml` `coverage` | **blocking**. Floors: turbo-dom 100, turbo-dom-parser `src/core.rs` 100, turbo-dom-layout 99.8, turbo-dom-raster 100 (default / avif / jxl), turbo-dom-browser-env 97.6 |
| JS runtime coverage | `npm run test:cov` (pre-commit hook) | lines 100 / functions 100 / branches 97.3 |
| cyclomatic complexity ≤ 9 (Rust cc-check, JS oxlint) | `ci.yml` `complexity-rust`, `complexity-js` | **blocking** for tracked Rust outside browser-env and the npm runtime (`src/`); browser-env remains excluded from both scans |
| clippy pedantic `-D warnings`, rustfmt, MSRV 1.89, version agreement, churn allocation gate, verify-copied reviews | `ci.yml` | blocking |

**CI triggers.** Branch CI runs automatically when a PR is opened, reopened, or
updated, regardless of its target branch. Branch pushes without an open PR do not
start CI. Without a PR, use **Actions → CI → Run workflow**, select the branch, or run:

```sh
gh workflow run ci.yml --repo miaskiewicz/turbo-dom --ref main
gh run list --repo miaskiewicz/turbo-dom --workflow ci.yml --branch main --event workflow_dispatch
```

Replace `main` with your branch. It must contain the `workflow_dispatch` trigger;
merge the latest `main` into older branches first. Manual CI on a branch does not
publish packages; `v*` tags still invoke the separate publishing workflow.

**CI runners.** Pull requests and manual CI run on Linux (x64 and arm64) and on
Windows x64: the napi build, and the V8 / browser-env matrix with the default and
simdutf archives. macOS arm64 builds and jobs run only in the release workflow.
`publish.yml` calls `ci.yml` with `darwin: true`. Linux binaries build on ubuntu-22.04 and a
guard (`scripts/check-glibc-ceiling.sh`) rejects any addon that needs glibc above 2.35. The
guard also fails when an expected addon is missing or unreadable. A newer push to a pull
request cancels its superseded CI run. Manual and publish runs each get their own
concurrency group, so they are never cancelled or dropped while pending.

## Releasing

One annotated `vX.Y.Z` tag on the release merge commit publishes everything in dependency
order. The order is turbo-dom-v8 → turbo-dom → turbo-dom-layout → turbo-dom-raster →
turbo-dom-browser-env → npm. See [PUBLISHING.md](PUBLISHING.md) and the history in
[CHANGELOG.md](CHANGELOG.md). This repository publishes to npm and crates.io; it has
no PyPI package or Python release tag.

## License

[MIT](LICENSE).
