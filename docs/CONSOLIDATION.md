# turbo-dom consolidation (0.6.0)

turbo-html2pdf, turbo-test and turbo-surf used to carry their own copies of the browser
engine pieces they share: a rusty_v8 fork (`turbo-surf-v8`, vendored twice), turbo-test's
rtdom <-> V8 binding (vendored into turbo-surf by a copy script), html2pdf's layout engine
(re-implemented as a copier in turbo-surf), turbo-surf's raster tier and two selector
engines. Since 0.6.0 they live once, here, as five published crates with library APIs;
the consumers depend on them through crates.io.

Governing rule: every step was behaviour-preserving (consumer corpora byte-identical,
proven by the parity harness below) with exactly two approved exceptions, both "match
what Chrome does": the one Chrome 156-correct selector engine (S3), and turbo-surf static
rendering of `t:*` as unknown inline elements (S2/S4).

## Crate map

| crate | version | MSRV | contents | origin |
|---|---|---|---|---|
| [`turbo-dom`](../crates/turbo-dom) | 0.6.0 | 1.89 | pure-Rust DOM (`rtdom::Tree`), the ONE selector engine (`query*.rs`, `parse_for_cascade`/`match_for_cascade`/`parse_for_supports` over a `Navigator`), CSSOM, `rtdom::cascade`, `rtdom::css_values`, `Tree::parse_with` | this repository |
| [`turbo-dom-v8`](../crates/turbo-dom-v8) | 150.4.0 | 1.91 | rusty_v8 150.4.0 + interceptor `ShouldThrowOnError` and isolate weak-teardown fixes, `FORK_SHOULD_THROW_FIX`; generated, never committed | upstream `v8-150.4.0.crate` + turbo-surf's reviewed patch |
| [`turbo-dom-layout`](../crates/turbo-dom-layout) | 0.6.0 | 1.89 | style cascade, box generation, block/inline/flex/grid/table layout, fonts, image sizing, layout snapshots, `layout_html`; `style::TreeView` (html2pdf mode), `live` (surf mode), `cssom_view` | turbo-html2pdf-core 0.3.9 layout half (S1), turbo-surf CSSOM-View modules (S4) |
| [`turbo-dom-raster`](../crates/turbo-dom-raster) | 0.6.0 | 1.89 | synthetic screenshots (PNG/SVG painters), canvas op-log replay, `gpu-metal` WebGL replay | turbo-surf-raster 0.5.5 (S1) |
| [`turbo-dom-browser-env`](../crates/turbo-dom-browser-env) | 0.6.0 | 1.91 | the rtdom <-> V8 browser binding (window, document, CSSOM, forms, events, …) + JS bootstrap + production JSON; features `host-internals`, `test-support`, `native-sdk-frame-read`; Chrome controls and their collectors | turbo-test 0.4.8 `src/` (S0), collectors from turbo-test (S5) |
| npm `@miaskiewicz/turbo-dom` + `turbo-dom-parser` (npm only, `publish = false`) | 0.6.0 | — | JS DOM runtime + native/WASM parser addon | this repository |

## Dependency graph (acyclic)

```
turbo-dom-v8 ──────────────────────────────► turbo-dom-browser-env ◄── turbo-test
                                                 ▲          ▲
turbo-dom ───────────────────────────────────────┘          │
  └──► turbo-dom-layout ──► turbo-dom-raster                 │
          ▲     ▲                 ▲                          │
          │     └── turbo-html2pdf-core                      │
turbo-dom-v8 ──► turbo-surf-deno-v8 0.4.1 ──► deno family ──► turbo-surf
turbo-dom, -layout, -raster, -browser-env ─────────────────► turbo-surf
```

Repository edges: turbo-dom <- {turbo-html2pdf, turbo-test, turbo-surf}; turbo-dom
depends on no consumer (the V8 generator reads only the upstream crates.io archive and
its committed patches). One `turbo-dom-v8` per graph (`links = "rusty_v8"`); consumers
require `~150.4`.

## Publish order

One annotated `v0.6.0` tag on the merge commit runs `.github/workflows/publish.yml`
(see [PUBLISHING.md](../PUBLISHING.md)): the full CI workflow, then

1. `turbo-dom-v8` 150.4.0 — generated from the pins, published by
   `crates/turbo-dom-v8/scripts/release.py --publish` (an existing 150.4.0 must match the
   fresh package byte-for-byte);
2. `turbo-dom` 0.6.0;
3. `turbo-dom-layout` 0.6.0 (needs turbo-dom);
4. `turbo-dom-raster` 0.6.0 (needs turbo-dom-layout);
5. `turbo-dom-browser-env` 0.6.0 (needs turbo-dom-v8 + turbo-dom; the root manifest's
   pre-publish `[patch]` is dropped first so it verifies against the published fork);
6. npm `@miaskiewicz/turbo-dom` 0.6.0.

Each crate step skips a version crates.io already has and waits for the crates.io index
to list it before the next one (`scripts/publish-crates.py`). Then, in parallel, the
consumers release (turbo-html2pdf `v0.6.0`/`pyv0.6.0`, turbo-test `v0.6.0`, turbo-surf's
deno-family release-deps), and finally turbo-surf `v0.6.0`.

`cargo package` contents (measured for the release prep): turbo-dom 82 files / 256 KiB
compressed; turbo-dom-layout 182 files / 3.9 MiB (the bundled OFL fonts); turbo-dom-raster
18 files / 89 KiB; turbo-dom-browser-env 430 files / 1.2 MiB (all `src/` probes, controls
and cases that `test_support` exposes, the collectors in `scripts/`); turbo-dom-v8 575 KiB.
turbo-dom, -layout and -raster package and verify together today
(`cargo package -p turbo-dom -p turbo-dom-layout -p turbo-dom-raster`, Cargo's local
overlay of unpublished siblings); turbo-dom-browser-env can only be packaged
(`--no-verify --exclude-lockfile`) until turbo-dom-v8 is on crates.io, which is why the
fork publishes first.

## How consumers depend on the crates

Manifests always name the registry version (`turbo-dom = "0.6"`,
`turbo-dom-layout = { version = "0.6.0", default-features = false }`,
`turbo-dom-browser-env = { version = "0.6.0", features = ["host-internals"] }`, dev
`features = ["test-support"]`, `v8 = { package = "turbo-dom-v8", version = "~150.4" }`).

- **Before publication (PR time)**: each consumer adds a `[patch.crates-io]` section that
  resolves the turbo-dom crates by git `rev` of this repository's consolidation branch,
  and `turbo-dom-v8` by `rev` of the orphan branch `consolidation/turbo-dom-v8-candidate`
  (pushed by `crates/turbo-dom-v8/scripts/push-candidate.sh`; the generated crate sits at
  its root). Re-lock with a targeted `cargo update -p …` after each turbo-dom push. No
  generated or copied source lives in a consumer tree. This repository's own root
  manifest patches only `turbo-dom-v8` (its CI's v8 job builds against the freshly
  generated crate instead, via `--config patch.crates-io.turbo-dom-v8.path=…`).
- **After publication**: drop the `[patch.crates-io]` section and re-lock only the
  turbo-dom crates (`cargo update -p turbo-dom -p turbo-dom-layout -p turbo-dom-raster
  -p turbo-dom-browser-env -p turbo-dom-v8`, whichever apply). Guards: one turbo-dom
  and one turbo-dom-v8 in `cargo tree`; no `v8`/`v8x`/`turbo-surf-v8`.

## CI

`.github/workflows/ci.yml` (toolchain 1.98.0; plan §7.4):

| job | what |
|---|---|
| base | napi addon matrix (linux x64 glibc/musl, linux arm64, macOS arm64, windows x64), WASM build, `cargo test -p turbo-dom-parser --lib`, `cargo test -p turbo-dom` + churn allocation gate, clippy `-p turbo-dom -p turbo-dom-parser` (+ wasm front-ends), JS / vitest / jest / html5lib conformance |
| layout (linux + macOS) | clippy (default / `svg,perf,serde,test-support` / no-default); tests default, `--no-default-features` (exact red list via `scripts/check-baseline-red.py`), `svg`, `perf`, `serde,test-support`; wasm32 check (no default features); musl check |
| raster (linux + macOS) | clippy + tests, default, `--features avif` and `--features jxl`; `coretext` / `gpu-metal` clippy and checks on macOS, `gpu-metal` check on Linux |
| v8 + browser-env | linux x64/arm64, macOS arm64, windows x64 × default/simdutf: generate turbo-dom-v8, prebuilt rusty_v8 150.4.0 archive (cached) as `RUSTY_V8_ARCHIVE`, fork tests (no ignored interceptor tests), browser-env tests default and `host-internals,test-support`, separate target dirs per crate × variant |
| msrv | `cargo +1.89 check --all-targets` for turbo-dom, -layout, -raster |
| guards | `check-versions.py`, `check-no-selector-dual.sh`, turbo-dom-v8 generator/release self-tests + `generate.py --check`, `scripts/` and `tools/parity` self-tests, `verify-copied.sh` (verbatim + current), `verify-copied-collectors.sh`, `verify-copied-layout.sh` |
| coverage | tarpaulin (LLVM, macOS) per crate with `--fail-under` floors |
| complexity (rust, js) | cyclomatic complexity <= 15 per function: `cc-check` (turbo-html2pdf, pinned rev) over every tracked `.rs` file, oxlint `complexity` over `src/`; expected red until the post-coverage refactor (below) |

### Coverage

The gates are the line coverage measured at the release prep (tarpaulin 0.37.5, LLVM
engine, macOS arm64, `--include-files 'crates/<crate>/src/*'`), rounded down:

| crate | measured | gate |
|---|---|---|
| turbo-dom | 88.33% (6327/7163) | 88 |
| turbo-dom-layout | 90.63% (8450/9324) | 90 |
| turbo-dom-raster | 77.23% (1184/1533) | 77 |
| turbo-dom-browser-env (`host-internals,test-support`) | 88.59% (14538/16411) | 88 |

The coverage phase (P5-cov) adds behaviour-asserting tests and raises every gate to
>= 99, then 100 per crate as it gets there (turbo-dom-v8: 100% of the fork delta through
a coverage filter). A gate is never lowered to pass a change. (turbo-dom#22's 47.5% was
measured without the `--include-files` filter, i.e. counting other packages' files.)

### Complexity

One limit for the whole repo: every function's cyclomatic complexity is <= 15, with no
allow-list and no baseline (CLAUDE.md "Complexity gates" has the tools and commands;
turbo-html2pdf's own crates stay at cc < 6). The limit was chosen from the distribution
measured when the gate landed (functions above each threshold):

| scope | functions | > 5 | > 10 | > 15 | > 20 | > 25 | > 30 | > 40 | max |
|---|---|---|---|---|---|---|---|---|---|
| Rust, all tracked `.rs` | 10137 | 380 | 113 | 48 | 21 | 11 | 8 | 4 | 145 |
| JS, `src/` | 1136 | 141 | 50 | 24 | 10 | 7 | 4 | 2 | 50 |

At 15 the backlog is 72 functions (0.6% of the code's functions); 10 would triple it (163)
and reach into ordinary dispatch code, while 20 would leave a 16-20 band whose branch sets
are already too large to test exhaustively. Both jobs are red (with `continue-on-error`,
so the workflow and `publish.yml` stay green; not required checks yet) until the backlog is
refactored, which happens only after the coverage phase (refactoring before full tests could
break behaviour silently). The backlog (`function:line=cc`):

Rust (48):
- `crates/turbo-dom-browser-env/src/browser_env.rs`: validate_namespace_name:2453=21, el_dispatch_event:4763=36, form_controls_native:5400=18
- `crates/turbo-dom-browser-env/src/css_selector_syntax.rs`: attribute:448=18, pseudo:594=28, class_arguments:672=18, element_arguments:773=20
- `crates/turbo-dom-browser-env/src/document_xpath.rs`: snapshot:31=22
- `crates/turbo-dom-browser-env/src/dom_implementation.rs`: invoke:111=61
- `crates/turbo-dom-browser-env/src/dom_mutations.rs`: plan:12=17, incoming_plan:398=21
- `crates/turbo-dom-browser-env/src/html_dialog.rs`: focusable:151=24
- `crates/turbo-dom-browser-env/src/html_text.rs`: collect:146=21
- `crates/turbo-dom-browser-env/src/sanitizer.rs`: sanitize:299=18, filter_attributes:413=19
- `crates/turbo-dom-browser-env/src/script_state.rs`: (closure):194=20
- `crates/turbo-dom-browser-env/src/selection.rs`: plain_text:640=29
- `crates/turbo-dom-browser-env/src/shadow_slots.rs`: assign:397=17
- `crates/turbo-dom-browser-env/src/template_contents.rs`: serialize_with:472=18
- `crates/turbo-dom-browser-env/src/trusted_types.rs`: attribute_units:199=25
- `crates/turbo-dom-browser-env/src/view_transitions.rs`: (closure):254=17
- `crates/turbo-dom-layout/src/cssom_view/hit.rs`: elements_at_point:39=30
- `crates/turbo-dom-layout/src/cssom_view/metrics.rs`: element_geometry:27=16, offset_position:226=33
- `crates/turbo-dom-layout/src/cssom_view/range.rs`: select:93=18
- `crates/turbo-dom-layout/src/cssom_view/scroll.rs`: scroll_metrics:86=34, viewport_metrics:412=16
- `crates/turbo-dom-layout/src/cssom_view/text.rs`: caret:43=16
- `crates/turbo-dom-layout/src/cssom_view/visibility.rs`: check:48=18
- `crates/turbo-dom-raster/src/canvas_ops.rs`: parse_hex:197=16
- `crates/turbo-dom-raster/src/webgl.rs`: apply:166=16, rewrite:597=16
- `crates/turbo-dom-raster/tests/image_formats_chrome.rs`: class:15=19
- `crates/turbo-dom-raster/tests/inline_svg_chrome.rs`: class:17=19
- `crates/turbo-dom-raster/tests/paint_chrome.rs`: class:21=19
- `crates/turbo-dom-raster/tests/raster_img_chrome.rs`: class:20=19
- `crates/turbo-dom/src/rtdom/cascade.rs`: parse_stylesheet:338=16, computed_style_with_policy:565=31
- `crates/turbo-dom/src/rtdom/color.rs`: parse_float_js:302=20
- `crates/turbo-dom/src/rtdom/events.rs`: dispatch_event:139=22
- `crates/turbo-dom/src/rtdom/query.rs`: lex_strict:489=41, matches_compound:1625=21
- `crates/turbo-dom/src/rtdom/query_utf16.rs`: decoded:93=16, state_argument:730=16
- `crates/turbo-dom/src/rtdom/storage.rs`: collect_unreachable:121=47
- `crates/turbo-dom/src/rtdom/svg.rs`: parse_leading_float:61=22
- `crates/turbo-dom/src/rtdom/unit_tokenizer/mod.rs`: step:737=145
- `crates/turbo-dom/tests/cascade_selectors_chrome.rs`: selectors_match_chrome156:145=21

JS (24; `function:<line>` is an anonymous function):
- `src/runtime/cascade.mjs`: setProp:45=17, parseStylesheet:81=17, collectMatched:147=17, function:232=21
- `src/runtime/custom-elements.mjs`: createCustomElement:276=16
- `src/runtime/dom.mjs`: validLocalName:107=26, insertBefore:206=16, sanitizeInputValue:1662=18, queueRecords:1765=23, __inflateNested:1908=17, createElementNS:1985=18
- `src/runtime/events.mjs`: dispatchEvent:181=41, function:231=17
- `src/runtime/html-serialize.mjs`: serializeNode:18=19
- `src/runtime/selectors.mjs`: parseAttr:186=20, parseAnB:285=26, tokenize:334=36, parsePseudo:400=28, parseComplexTokens:459=36, disabledState:707=17, matchPseudo:861=50, matchCompound:924=21, matchChain:960=16
- `src/runtime/stubs.mjs`: evalMediaQuery:58=17

`crates/turbo-dom-layout`'s old cc <= 5 allow-list for the surf-origin `cssom_view` modules
is superseded by this gate; its functions above 15 are in the list above.

## Parity harness

`tools/parity/` renders pinned corpora through built napi addons (turbo-html2pdf PDFs and
`layoutBoxes`, turbo-surf static screenshots and live-render dumps) and compares sha256
manifests against recorded checkpoints (`b-main` = the released mains, `b2-surf`):

```sh
tools/parity/fetch-inputs.sh                                  # pinned upstream inputs (git archive)
python3 tools/parity/parity.py run OUT --html2pdf h2p.node --html2pdf-svg h2p-svg.node --surf surf.node
python3 tools/parity/parity.py compare OUT b-main --expect tools/parity/expected/<change>.json
python3 tools/parity/parity.py record OUT <checkpoint>        # after a reviewed set of intended changes
python3 -m unittest discover -s tools/parity -p 'test_*.py'   # self-test, no addons (CI)
```

Every intended output change is an expected-diff file (`tools/parity/expected/`); a
listed output that did not change fails as stale. See [tools/parity/README.md](../tools/parity/README.md).

## Review guide

Each step is one merged branch; inside it, commits follow the same structure so a
reviewer can skip the mechanical ones: **verbatim copy** (message records the source repo
+ SHA; checked by a committed `scripts/verify-copied*.sh`) -> **scripted rewrites**
(allow-listed in that script) -> **mechanical fmt/lint** -> **small semantic commits**
-> **docs**. The per-test move manifest is [CONSOLIDATION-TESTS.md](CONSOLIDATION-TESTS.md).

| step (merge) | commits | review focus |
|---|---|---|
| S0 V8 (`d305f5e`) | 6ceaa16 workspace exclude + parser `publish = false`; 1f66290 V8 regression tests ported from turbo-test; 7f31adc generator + delta patches + pins; 7f33b46 release + candidate scripts; 5450319 docs; 0cec64e generate from the upstream crates.io archive | `crates/turbo-dom-v8/patches/*`, `pins.json`; nothing generated is committed |
| S0 binding (`813db5d`, `c7f92a4`) | 308f73a verbatim move (turbo-test@cb5c64e, `verify-copied.sh verbatim`); 6ced937 scripted crate wiring; fb8b36a lint allow; eb7c216 rustfmt; 4a5df1f `rtdom::css_values`; f2df98a public API (crate surface, host-internals, test-support); f88ce54 `native-sdk-frame-read`; c3cd707 docs; c7f92a4 link turbo-dom-v8, fork accessor | `verify-copied.sh current` lists the 7 semantic edits; `crate_surface.rs`, `host_internals.rs`, `test_support.rs` |
| S1 relocation (`2c3bcef`) | 7f9f02c / c762a72 / c95a888 verbatim copies (turbo-html2pdf@e2d7c4d, turbo-surf@b1de4eb); fedd864 `verify-copied-layout.sh`; 85aadc2 / 6b0b7d6 / 4ec26b4 scripted renames; 68e9bcf fmt; 8ec8eb2 manifests + small manual fixes; f82a5a1 docs; 58c1e04 tarpaulin config | `scripts/verify-copied-layout.{map,rules}`; zero output diffs |
| S3a selectors (`d83ef0d`) | 0fda519 Chrome fixes; 7c52d4b Chrome 156 control; 70fa2d5 / aa07681 frozen engine + dual gate (later deleted); f83dc2d / 4d5a63d docs; 3751f34 edge tests | `crates/turbo-dom/tests/fixtures/cascade_selectors_chrome156.json`, [selector-engine.md](selector-engine.md) |
| parity harness (`7dfc1d9`) | corpora, pinned inputs, checkpoints, expected diffs | `tools/parity/` |
| S2 DOM swap (`4463654`) | 33df5f6 `Tree::parse_with`; ff02985 TreeView + markers + runtime options; f3b0e1d equivalence gate + adapted tests; 562bad9 legacy twins; af25a7e `Navigator`; 49b29ab / abdcb3c docs; 2676799 tarpaulin config | `tests/tree_input_equivalence.rs` |
| S3b unify (`ba96d9e`, `6b58d7b`) | 645a520 re-freeze; 0d20a26 Navigator-generic matching + cascade API; ce77776 layout on the engine; 35da6bd … a53ccaa gate tooling; 62d721f expected-diff files; 5006123 docs; e1d0304 delete old engine + dual gate (adds `check-no-selector-dual.sh`); f8e05b4 edge tests | Gate A/B results in the S3b report; `tools/parity/expected/s3b-*.json` (all outputs unchanged) |
| S4 surf mode (`9cac873`) | 9dceb39 verbatim CSSOM-View modules (turbo-surf); c31844e expected diff for the `t:` exception; baaa40c `live` + `cssom_view`; 6af9f78 cc split; cb2387f `parse_for_supports`; b057664 / 5f5e927 / 152415f docs | budgets/errors in `live`; `tools/parity/expected/s2-surf-static-t-elements.json` |
| S5b release readiness | e7f24ed 18 `test_support` consts; ee611bd verbatim collectors + `live-css-host.md` (turbo-test@39b5911); cec985a repoint to `src/`; 2491647 `verify-copied-collectors.sh`; 712fca5 versions 0.6.0; 1f4b1f8 baseline-red checker; 0fcfa27 parity self-test; 9de23cc guard tolerates its own name; 1616229 `check-versions.py`; 6c69541 CI; 4470d06 publish workflow; 6cf7866 crate LICENSEs; docs | no behaviour change: cargo results identical to 9cac873 plus the new tests |
