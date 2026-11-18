# Component layout/raster parity against Chromium

Dev-only, test-only reproduction corpus for the Flux homepage layout regressions
targeted at **v0.8.5**. This does not fix engine behavior, change release settings,
or add known-red assertions to the default CI gate. It contains **no Lottie player
or animation assets**. See [HANDOFF.md](HANDOFF.md) for diagnosis and fix acceptance.

## Build and reproduce

From the turbo-dom repository root:

```sh
cargo build -p turbo-dom-raster --example component_parity

# Fully offline: compare the source build's raster geometry with frozen Chromium.
node tools/component-parity/run.mjs \
  --native target/debug/examples/component_parity --native-only --assert

# Fresh real Chromium geometry, screenshots and pixel diffs of the same inputs.
PLAYWRIGHT_CORE=/absolute/path/to/playwright-core \
node tools/component-parity/run.mjs \
  --native target/debug/examples/component_parity \
  --chrome /absolute/path/to/chrome-headless-shell --assert

# Focus a fix, without hiding the independent remaining failures.
node tools/component-parity/run.mjs \
  --native target/debug/examples/component_parity --native-only --assert \
  --case nested-flex-padding --widths 1440

# Harness and fixture-provenance tests.
node --test tools/component-parity/parity.test.mjs
```

Honor `CARGO_TARGET_DIR` when locating the example executable. Once dependencies
are cached, Cargo can build with `--offline`. Browser mode needs an installed
Chromium and Playwright Core; the default Core lookup is the adjacent
`../turbo-surf/node_modules/playwright-core`. Native-only mode needs neither Node
packages, an installed browser, a turbo-surf addon, V8, nor a localhost listener.

Default widths are **1440,768,390**, viewport height **1000**, device scale **1**.
Use `--case NAME`, `--widths 1440`, and `--out /absolute/output/directory` to focus
the run. The output defaults to `/tmp/turbo-dom-component-parity-results`.

**Current assertion runs are expected to exit 1.** Recording mode (omit
`--assert`) exits successfully after writing results even when engines disagree.
Setup/build errors throw and are not passing or skipped cases. No selector match,
missing fixture, unknown case, stale reference hash, or missing native executable
should be interpreted as parity. A fix is successful only when the relevant
assertions become green without changing the oracle to match the broken engine.

## Corpus

| Case | Scope |
| --- | --- |
| `feature-cards` | Actual responsive MUI Grid with nested column-flex cards |
| `product-cards` | Actual two-column product composition |
| `partner-logos` | Actual nested MUI Stack, wrap, raster/SVG intrinsic sizing |
| `resources` | Actual responsive CSS grid inside centered column-flex stacks |
| `mui-width-expression` | MUI width expression with variables, multiplication/division, parentheses |
| `mui-width-additive` | Equivalent additive `% - px` width isolates dimension conversion |
| `nested-flex-padding` | Literal width isolates nested flex height/padding allocation |
| `nested-grid-tracks` | Text/image/JS-free reduction of resource-card track sizing |

The first four fixtures preserve deployed component markup and matching author
CSS from **https://fluxpayroll.ai/**, captured **2026-11-18**. They retain container
ancestors, responsive rules, custom properties and MUI classes. Scripts, unrelated
rules and inline style nodes were removed. Public external images are frozen in
`fixtures/assets` under content-derived names; SVG data URLs remain inline.
Remote font loading is removed and text uses Arial to isolate component geometry.
The four reductions preserve responsible declarations and nesting with solid
colors. Their fixed widths/track counts are intentional even at narrow viewports;
the full component fixtures supply the responsive-breakpoint controls.

Source pointers are the adjacent website-global repository's
`apps/web/src/components/Blocks/FeatureVariantBlock.tsx` and
ui-design-components' `src/marketing-website/components/FeatureBlock/FeatureCard.tsx`,
`Article/Article.tsx`, and the deployed partner/resource section composition.
Fixture hashes, rather than mutable source repository heads, identify exact inputs.

## Measurement and evidence

`component_parity.rs` uses only public **turbo-dom-layout/turbo-dom-raster** APIs.
It gets selector indices from the parsed tree, resolves `screenshot_element_rects`
with the exact HTML, assets, viewport and options supplied to
`screenshot_png_with_options`, then emits PNG and geometry JSON. Both use system
fonts. These are **static raster geometry and pixels**, with no turbo-surf shim
or live browser binding in the native measurement path.

Chromium runs the same HTML/CSS on localhost with only same-origin requests
allowed, awaits fonts/load completion, measures the declared selectors and takes
a viewport PNG. Native-only mode uses `chromium-reference.json` and checks each
fixture SHA256 before comparing geometry. Running the harness never rewrites
that independent reference. Fresh Chrome capture supports additional widths;
native-only mode requires a corresponding frozen reference entry.

Every run saves `results.json` with measured boxes, missing/extra boxes, geometry
differences, source binary/Cargo.lock hashes, browser version and pixel metrics
(fresh-browser mode). Per-case outputs include the native request JSON and
native PNG/geometry, plus Chromium and pink/white difference PNGs in browser mode.
Geometry tolerance is **1 CSS pixel**. Pixel comparison ignores per-channel
deltas of at most **16**, then counts every changed pixel; assertion mode requires
zero changed pixels as well as matching geometry. Text antialiasing can therefore
keep full-component pixel checks red after geometry is fixed. The text-free solid
reductions provide the stronger pixel controls; document any genuine residual,
do not silently widen the budget.

[evidence/observed-2026-11-18.json](evidence/observed-2026-11-18.json) is the measured
current-main 0.8.4 source build, compared with fresh Chromium 159.0.8154.6.
Four desktop reduction PNG triplets are committed beside it so an agent can inspect
the failure without rerunning the browser. Historical observations are evidence,
not expectations to assert as correct behavior. Keep the independent Chromium
oracle; a future fix may intentionally change native output.

## Validation for changes to this harness

```sh
node --test tools/component-parity/parity.test.mjs
cargo fmt --all --check
cargo clippy -p turbo-dom-raster --example component_parity -- -D warnings
```

Run the relevant native-only case and fresh Chromium case after a fix, then all
24 cases and the engine's existing gates. Do not use `--no-verify`, weaken
coverage/complexity floors, or bypass the normal release process.
