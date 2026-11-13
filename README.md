# turbo-dom

Release **0.8.4** (local unreleased candidate). turbo-dom is the shared browser-engine base of three projects:

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

## Packages in this candidate

Version 0.8.4 is under local development; the published registry release remains
0.8.3. Local consumers override these candidate requirements with `[patch.crates-io]`
until the release is published.
The V8 fork's published version remains 150.4.1; the 150.4.2 candidate below is
consumed through the exact pre-release git pin.

| package | registry | version | what it is | used by |
|---|---|---|---|---|
| [`@miaskiewicz/turbo-dom`](https://www.npmjs.com/package/@miaskiewicz/turbo-dom) | npm | 0.8.4 | lazy JS DOM runtime + native html5ever parser (N-API, WASM fallback) | vitest / jest users |
| [`turbo-dom`](crates/turbo-dom) | crates.io | 0.8.4 | pure-Rust DOM (`rtdom`), the one selector engine, CSSOM, partial cascade | all three consumers |
| [`turbo-dom-layout`](crates/turbo-dom-layout) | crates.io | 0.8.4 | HTML/CSS layout (block / inline / flex / grid / table), fonts, images, CSSOM-View geometry | turbo-html2pdf, turbo-surf |
| [`turbo-dom-raster`](crates/turbo-dom-raster) | crates.io | 0.8.4 | synthetic screenshots (PNG / SVG), canvas op-log replay, WebGL readback | turbo-surf |
| [`turbo-dom-browser-env`](crates/turbo-dom-browser-env) | crates.io | 0.8.4 | the `rtdom` <-> V8 browser environment (window, document, CSSOM, forms, events) | turbo-test, turbo-surf |
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
turbo-dom = "0.8.4"
turbo-dom-layout = "0.8.4"                 # bundled fonts on by default
turbo-dom-raster = "0.8.4"
turbo-dom-browser-env = "0.8.4"            # needs a V8 archive, see its README
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
| `turbo-dom-raster` | `avif` | off | AVIF pixels (pure-Rust rav1d) |
| | `jxl` | off | JPEG XL pixels (jxl-oxide) |
| | `gpu-metal` | off | wgpu (Metal) canvas and WebGL rasterization, falling back to the CPU path on any GPU failure |
| | `coretext` | off | macOS only: canvas text through CoreText (inert elsewhere) |
| `turbo-dom-browser-env` | `host-internals` | off | doc-hidden host access to native documents (turbo-surf) |
| | `test-support` | off | probe, control and case constants for embedders' Chrome-parity tests |
| | `native-sdk-frame-read` | off | reads the interceptor's should-throw mode from the V8 frame, for native SDKs without the fork fix |
| `turbo-dom-parser` (npm) | `napi-bind` / `wasm-bind` / `wasm-runtime` | `napi-bind` | the native addon, the WASM parser and the WASM DOM-runtime spike |

`turbo-dom` declares no feature flags. `turbo-dom-v8` forwards upstream rusty_v8's
features (for example `simdutf`).

## Status

- **0.8.4 (unreleased):** bounded shaping-cache retention, shared immutable styles
  and font-family storage, and inline text paths that avoid unused box parsing and
  disabled wrapping work. See the unreleased changelog and turbo-dom#508.
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

**CI runners.** Pull requests and pushes to `main` run on Linux (x64 and arm64) and on
Windows x64: the napi build, and the V8 / browser-env matrix with the default and
simdutf archives. macOS arm64 builds and jobs run only in the release workflow.
`publish.yml` calls `ci.yml` with `darwin: true`. Linux binaries build on ubuntu-22.04 and a
guard (`scripts/check-glibc-ceiling.sh`) rejects any addon that needs glibc above 2.35. The
guard also fails when an expected addon is missing or unreadable. A newer push to a pull
request cancels its superseded CI run. `main`, tag and publish runs each get their own
concurrency group, so they are never cancelled or dropped while pending.

## Releasing

One annotated `vX.Y.Z` tag on the release merge commit publishes everything in dependency
order. The order is turbo-dom-v8 → turbo-dom → turbo-dom-layout → turbo-dom-raster →
turbo-dom-browser-env → npm. See [PUBLISHING.md](PUBLISHING.md) and the history in
[CHANGELOG.md](CHANGELOG.md). This repository publishes to npm and crates.io; it has
no PyPI package or Python release tag.

## License

[MIT](LICENSE).
