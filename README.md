# turbo-dom

Release **0.6.0**. turbo-dom is the shared browser-engine base of three projects:

- **[turbo-test](https://github.com/miaskiewicz/turbo-test)**, a vitest-compatible test runner. It runs tests in V8 over this repository's DOM.
- **[turbo-html2pdf](https://github.com/miaskiewicz/turbo-html2pdf)**, which renders HTML to PDF and uses this repository's layout engine.
- **[turbo-surf](https://github.com/miaskiewicz/turbo-surf)**, a headless browser that uses the DOM, the V8 binding, the layout engine and the screenshot painter.

turbo-dom is also a standalone npm package: a fast, spec-correct DOM environment for
**vitest** and **jest**, usable in place of jsdom or happy-dom.

Up to 0.5.x, each consumer kept its own copy of these pieces. In 0.6.0 each piece lives
in this repository once and is published from here. The consumers depend on the
published crates and keep no copies.
[docs/CONSOLIDATION.md](docs/CONSOLIDATION.md) has the architecture, the dependency graph
and the publish order.

## What is published

| package | registry | version | what it is | used by |
|---|---|---|---|---|
| [`@miaskiewicz/turbo-dom`](https://www.npmjs.com/package/@miaskiewicz/turbo-dom) | npm | 0.6.0 | lazy JS DOM runtime + native html5ever parser (N-API, WASM fallback) | vitest / jest users |
| [`turbo-dom`](crates/turbo-dom) | crates.io | 0.6.0 | pure-Rust DOM (`rtdom`), the one selector engine, CSSOM, partial cascade | all three consumers |
| [`turbo-dom-layout`](crates/turbo-dom-layout) | crates.io | 0.6.0 | HTML/CSS layout (block / inline / flex / grid / table), fonts, images, CSSOM-View geometry | turbo-html2pdf, turbo-surf |
| [`turbo-dom-raster`](crates/turbo-dom-raster) | crates.io | 0.6.0 | synthetic screenshots (PNG / SVG), canvas op-log replay, WebGL readback | turbo-surf |
| [`turbo-dom-browser-env`](crates/turbo-dom-browser-env) | crates.io | 0.6.0 | the `rtdom` <-> V8 browser environment (window, document, CSSOM, forms, events) | turbo-test, turbo-surf |
| [`turbo-dom-v8`](crates/turbo-dom-v8) | crates.io | 150.4.0 | turbo-dom's rusty_v8 fork, generated from the upstream crate plus reviewed patches | turbo-dom-browser-env, turbo-surf |

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
turbo-dom = "0.6"
turbo-dom-layout = "0.6"                 # bundled fonts on by default
turbo-dom-raster = "0.6"
turbo-dom-browser-env = "0.6"            # needs a V8 archive, see its README
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

`turbo-dom` and `turbo-dom-v8` have no features of their own. `turbo-dom-v8` forwards
upstream rusty_v8's features (for example `simdutf`).

## Status

- **Parser:** html5lib-tests conformance is 99.94% through the npm parser (happy-dom
  37.35%, jsdom 97.03%). The remaining misses are `<select>`-family proposals that
  upstream html5ever has not adopted yet. The Rust `rtdom` tree passes its direct
  html5lib-tests gate at 100% (`npm run conformance:rtdom`).
- **Chrome parity:** behaviour fixes are backed by Chrome 156 controls. These are
  fixtures recorded from headless Chrome and asserted in tests. At the 0.6.0 parity
  remeasure (2026-10-29), turbo-surf's API inventory against Chrome 156 has 2 missing
  members, down from 142. Every corpus output change is declared in
  [`tools/parity/expected/`](tools/parity/expected) (checkpoint `b-0.6.0`).
- **Open gaps (tracked as issues):**
  - XPath fragment contexts (turbo-dom#185)
  - XML parsing (#186)
  - CSS animations in `getAnimations` (#187)
  - focus / dialog details (#188)
  - FileList and selector error text (#189)
  - RTL scroll and vertical writing (#190)
  - shaping and font metrics (#191)
  - inset `box-shadow` (#192)
  - block and table layout residuals (#53–#102, #128, #178)
- **The npm runtime has no layout:** `getBoundingClientRect()` returns zeros, and
  `getComputedStyle` is a partial cascade. It resolves real `<style>` rules and inline
  styles, applies inheritance, and serializes colours as `rgb()`. It never invents a
  value. `@media`, state pseudo-classes, pseudo-elements and unit conversion return `''`.
  Shadow DOM is supported: slots, retargeting, `:host` / `::slotted`, and declarative
  shadow roots. Layout belongs to `turbo-dom-layout`, and through it to turbo-html2pdf and
  turbo-surf.

## Benchmarks — pending re-run

0.6.0 has not been benchmarked yet. All four repositories will be re-run together on a
quiet machine after the release. Until then, the last measured numbers (v0.1.55,
2026-06-07, darwin-arm64, JS runtime vs happy-dom and jsdom) are kept unchanged and
labelled in [docs/BENCHMARKS-history.md](docs/BENCHMARKS-history.md). They predate the
consolidation and say nothing about 0.6.0.

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
npm run complexity:rust    # cc-check --max 15 (install: CLAUDE.md)
npm run complexity:js      # oxlint complexity <= 15 over src/ + crates/turbo-dom-browser-env/src
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
| cyclomatic complexity ≤ 15 (Rust cc-check, JS oxlint) | `ci.yml` `complexity-rust`, `complexity-js` | **report-only** (`continue-on-error`) until the post-release refactor clears the backlog ([docs/CONSOLIDATION.md](docs/CONSOLIDATION.md#complexity)) |
| clippy pedantic `-D warnings`, rustfmt, MSRV 1.89, version agreement, churn allocation gate, verify-copied reviews | `ci.yml` | blocking |

**CI runners.** Pull requests and pushes to `main` run on Linux (x64 and arm64) and on
Windows x64: the napi build, and the V8 / browser-env matrix with the default and
simdutf archives. macOS arm64 builds and jobs run only in the release workflow.
`publish.yml` calls `ci.yml` with `darwin: true`.

## Releasing

One annotated `vX.Y.Z` tag on the release merge commit publishes everything in dependency
order. The order is turbo-dom-v8 → turbo-dom → turbo-dom-layout → turbo-dom-raster →
turbo-dom-browser-env → npm. See [PUBLISHING.md](PUBLISHING.md) and the history in
[CHANGELOG.md](CHANGELOG.md).

## License

[MIT](LICENSE).
