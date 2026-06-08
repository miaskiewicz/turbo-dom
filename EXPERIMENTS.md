# turbo-dom performance experiment loop

This is the **exact protocol** for proposing, validating, and shipping a perf change.
Every change ships ONLY if it passes ALL gates below. No exceptions, no "looks faster",
no single-run wall-clock claims. Measure everything. Ditch anything that doesn't clearly win.

The runtime (`src/runtime/*.mjs`) is pure JS — only the parser is the native `.node`.
So a runtime change can be **hot-swapped** into a consuming repo's `node_modules` without
rebuilding: copy the `.mjs` files over.

## KPI (the real goal)

Reliably get the real suites UNDER these wall-clock targets over time, via genuine
turbo-dom improvements — never by sacrificing correctness, coverage, or honesty:
- ../ui-design-components < 50s
- ../payroll-app < 75s

turbo-dom only controls the `environment` + part of the `tests`/`setup` buckets;
`import`/`transform` (vitest module loading) dominate and are out of our hands. So
progress is cumulative small per-file wins. Wall-clock is load-noisy — the KPI is a
TREND target, validated when the machine is quiet.

## Repos to validate against (real suites, turbo-dom vitest env)

| repo | run command | files |
|---|---|---|
| `../ui-design-components` | `npx vitest run --config vitest.config.dev.ts` | ~386 |
| `../payroll-app` | `npx vitest run` | ~1004 |

Hot-swap the local runtime into BOTH before running:
```bash
for R in ../ui-design-components ../payroll-app; do
  cp src/runtime/*.mjs "$R/node_modules/@miaskiewicz/turbo-dom/src/runtime/"
done
```
After an experiment (shipped or ditched), re-sync `node_modules` to the committed HEAD
runtime so the repos aren't left on an uncommitted/garbage state:
```bash
git -C . stash --include-untracked >/dev/null 2>&1 || true   # only if needed
for R in ../ui-design-components ../payroll-app; do
  cp src/runtime/*.mjs "$R/node_modules/@miaskiewicz/turbo-dom/src/runtime/"
done
```

## The gates (ALL must pass to ship)

1. **gr0gdom correctness + coverage gate**: `npm run test:cov` exits 0
   (full suite + conformance + coverage thresholds). New code must keep runtime files
   at 100% line; add tests for every new branch/function before shipping.
2. **Isolated microbench A/B** — the change MUST win here, cleanly:
   - Write a focused microbench in `/tmp` importing `src/runtime/index.mjs`.
   - **Guard against dead-code elimination**: use a non-eliminable sink
     (`if (e.bubbles) sink++`, accumulate `.length`, etc). A loop whose result V8 can
     prove unused gets deleted → garbage numbers (huge/negative ops/s). Always print
     `sink` and sanity-check it.
   - **best-of-N** (N≥6), warm up first (~50k iters), report `best` (min ms / max ops/s).
   - A/B via `git stash`: measure current (change in tree), `git stash`, measure HEAD,
     `git stash pop`. Run each variant ≥2×. **Every change-run must beat every HEAD-run**
     — if they overlap, it's noise, not a win.
3. **Both real suites pass** (correctness): hot-swap runtime, run each. ALL tests green.
   Watch `Duration` / `tests` / `environment` buckets but DO NOT trust wall-clock A/B
   (the shared dev machine is too noisy; ±40s swings from background load are common).
   Wall-clock is a sanity check, not the ship criterion — the microbench is.

If a change is **zero-downside** (strictly fewer allocations / a redundant-op skip with
byte-identical behavior), the isolated microbench + both-suites-pass is sufficient to ship.
If a change has any behavioral surface (new branch in a hot path, reordering), it needs the
microbench win AND a careful read that the non-target path is unchanged.

## Ship procedure (only when ALL gates pass)

1. Add/confirm tests covering 100% of the new code (run `npm run test:cov`, eyeball the
   per-file table; runtime files must stay 100% line).
2. Bump `package.json` patch version (`vX.Y.Z`).
3. `git add -A` and commit with subject `vX.Y.Z: <summary>` + a body giving the microbench
   delta (best-of-N, "every run beat baseline") and "both real suites green". Include the
   tests in the same commit. End with the Co-Authored-By trailer.
4. `git tag vX.Y.Z` (lightweight) and `git push origin main && git push origin vX.Y.Z`.
   The pre-commit hook re-runs `test:cov`; CI publishes to npm on the tag.
5. **Refresh benchmarks vs jsdom/happy-dom**: `npm run bench:all`. If any headline number
   moved materially, update the tables/prose in `README.md` AND the benchmark block in
   `CLAUDE.md` (keep them consistent; conservative rounding). Commit doc-only refreshes as
   `docs: refresh benchmarks after vX.Y.Z` (no tag — don't trigger a publish for docs).

## Ditch procedure (any gate fails)

- `git checkout -- <files>` (and revert any version bump). Re-sync node_modules to HEAD.
- Record the result in the "Ledger" below (what, measured outcome, why ditched). A measured
  rejection is a success — it protects the suites from real regressions.

## Hard-won lessons (don't relearn these)

- **Monomorphism helps MANY instances, never singletons.** Predeclaring lazy fields on
  `Element` (many) won ~7-8% (v0.1.34); doing it on `Document` (singleton) REGRESSED query
  ~9% — extra slots, nothing to unify. Text/Comment have no lazy fields → already mono.
- **Measure the premise before writing code.** `toLowerCase()` on an already-lowercase
  string is free in V8 (returns same ref); a manual "skip if lowercase" scan was *slower*.
- **Microbench win ≠ suite win.** Event-path pooling was +6% isolated but the suite was
  flat/negative (the 2nd-walk cost on listener-ful events offset it). The reverse also holds:
  a real win can hide under suite noise — that's why the microbench is the ship criterion and
  the suites are the regression guard.
- **Batch `Object.defineProperties` is SLOWER** than N× `defineProperty` here (~25%). Don't.
- **Dead-code elimination wrecks microbenches** — always use an observable sink + print it.
- **Result caching gates the matchers.** `querySelectorAll`/`querySelector`/`getElementsBy*`
  results are cached per `(selector, Document.__version)` (`cachedQSA`/`cachedQS`/`__byTag`…).
  The selector MATCHER (`simpleMatcher`/`matchComplex`/`matchCompound`/`matchAttr`) therefore
  runs only ONCE per selector per version — on a cache MISS — not per query. RTL's repeated
  queries against an unchanged tree are cache hits. So optimizing the matcher (backlog #1, #3,
  #6) has near-zero real-suite impact; a cache-busting microbench might show a delta but it
  won't translate. Don't spend the loop on matcher micro-opts — target NON-cached hot paths
  (event dispatch listener lookup, getAttribute, inflation, construction, mutation).

## Candidate backlog (grounded in the code; pick the next untried one)

Roughly ordered by expected value. Each must go through the full protocol.

1. **Selector matcher specialization / compiled matchers** — `matchComplex` walks the AST
   per element. Compile a selector to a closure once (cached) so per-element matching is a
   straight-line call. Watch the allocation-free hot-path rule (no per-element closures *inside* the match).
2. **`getAttribute` for-loop vs Map for elements with many attrs** — linear scan is fine for
   few attrs; profile whether MUI/emotion elements (many `data-*`/`aria-*`) benefit from a
   name→value Map built lazily on first attr access. (Watch: most elements have few attrs.)
3. **`splitClasses` / class scan reuse in matchers** — `elHasClass` rescans the class string
   per candidate class; for multi-class selectors (`.a.b.c`) this is O(classes × len). Profile.
4. **`classList` token caching** when an element's `class` attr is unchanged (version-guarded).
5. **Event listener Map → array for ≤N listeners** — `__listeners` is a Map keyed by type;
   most nodes have 1-2 types. A small array/inline could beat Map.get for the common case.
6. **`querySelectorAll` simple-matcher coverage** — extend `simpleMatcher` (allocation-free
   fast path) to more selector shapes (`tag.class`, `[attr]`) so they skip `matchComplex`.
7. **Lazy `Element.content`/`shadowRoot` predeclare audit** — already predeclared; check the
   set is minimal (no wasted slots) and ordered for the hottest reads first.
8. **`__nodeAt` / inflation fast path** — the buffer→handle switch in `__nodeAt`; profile the
   element-create branch (the hottest) and see if a type-dispatch table beats the switch.
9. **`notifyMutation` version bump inline** — `__touch()` and notifyMutation both do
   `(doc.__version||0)+1`; with `__version` predeclared 0 on Document, drop the `||0`. (Tiny;
   verify Document still predeclares __version=… or this is unsafe.)
10. **`childNodes`/`children` Proxy → cached array with version guard** — risky (liveness);
    only if a microbench shows the live `getArray()` re-read dominates AND liveness tests pass.
11. **SoA buffer column packing / dict attr values** (parser side, `core.rs` + buffer.mjs) —
    bigger lift, native; gate on marshaling being the proven cost (a separate bench). Likely
    out of scope for the pure-JS hot-swap loop.

## Are we actually faster? — two signals

1. **Microbench scorecard (the reliable signal).** Deterministic, low-noise. Run
   `node bench/scorecard.mjs` on any version — it prints ops/s for the hot paths the
   loop targets (createEnvironment, inflation, dispatch listener-less + single, mutation,
   textContent, addEventListener). Compare across versions to KNOW if a change helped.
   This — not suite wall-clock — is what proves a speedup.

   Baseline @ v0.1.43 (darwin-arm64, Node 24, best-of-6 ops/s):
   - createEnvironment (empty shell): ~768k
   - inflate+traverse (~1200 els, fresh env): ~9.7k
   - dispatch listener-less (bubbles): ~3.22M
   - dispatch single-listener (bubbles): ~2.15M
   - mutation append+setAttr+remove (no observer): ~15.7M
   - textContent read (single text child): ~201M
   - addEventListener (3 / fresh elem): ~14.4M
2. **Suite wall-clock by version (coarse trend, NOT precision).** Single-run `Duration`
   on each real suite at the SHIP run. The shared machine swings ±10-40s with load, so
   read it as a trend, never a per-change verdict. Append a row on every ship.

| version | ui-design-components (386f/6188t) | payroll-app (982f/9670t) | change |
|---|---:|---:|---|
| v0.1.41 | 49.6s | 94.9s | addEventListener inline |
| v0.1.42 | (passed, untimed) | (passed, untimed) | single-listener slice skip |
| v0.1.43 | (passed, untimed) | (passed, untimed) | tagName read once |
| v0.1.44 | 70.5s | 106.0s | nodeType read once (NOTE: machine loaded this run — wall higher than v0.1.41) |
| v0.1.44 (quiet machine) | **48.1s** ✓<50 | 92.1s | clean re-run — uidc under target; payroll wall dominated by vitest import/setup (not turbo-dom) |

## Ledger (append one line per experiment; newest at bottom)

- v0.1.34 — monomorphic Element shape — SHIPPED (~7-8% suite, +clean microbench).
- v0.1.36 — lazy Document.__mo — SHIPPED (+6% createEnvironment).
- v0.1.37 — zero-alloc mutations when no observer — SHIPPED (+17% mutation throughput).
- v0.1.38 — lazy Event._path — SHIPPED (~2× Event construct).
- v0.1.39 — textContent single-text fast-path — SHIPPED (~28% textContent reads).
- v0.1.40 — parse-cache LRU skip on MRU — SHIPPED (+2.3% createEnvironment).
- DITCHED — event-path pooling (suite flat/neg), monomorphic Document (-9% query),
  installGlobals batch (-25%), getAttribute lowercase (V8 free), getElementById index
  (loses early-exit), window-proxy reorder (risky), Text/Comment shapes (already mono),
  classList memoize (not hot), collections (already memoized, liveness-risky).
- DITCHED — simpleMatcher attr fast-path (#6): microbench flat (2ms==2ms) — cachedQSA
  memoizes results per version, so the matcher runs once per (selector,version), not per
  query. Matcher micro-opts (#1/#3/#6) don't move real suites. Lesson recorded above.
- v0.1.41 — inline addEventListener option parsing — SHIPPED (+33% listener attach; suites 6188 + 9670 green). Non-headline path → README numbers unchanged.
- DITCHED — move dispatch invoke-closure inside hasListener: microbench overlapped (3.08-3.24M vs 3.15-3.18M). V8 escape-analysis already elides the non-escaping closure. No win.
- v0.1.42 — skip listener-snapshot slice for single-listener dispatch — SHIPPED (+13% single-listener dispatch; suites 6188 + 9670 green). Snapshot semantics regression-tested.
- v0.1.43 — read tagName once in __nodeAt inflation — SHIPPED (+5.5% inflation/traverse; suites 6188 + 9670 green). Non-material to headline ratios → README unchanged.
- DITCHED — setAttribute/removeAttribute .find/.filter → index-loop+splice: microbench SLOWER (8.9M vs 9.3-9.45M renders/s, every run). V8 optimizes .find/.filter on small attr arrays better than manual loops (+ splice shift cost). Closure!=slow here — opposite of addEventListener .some(); always measure, never assume.
- DITCHED — classList memoize (this.__classList): microbench overlapped (1.95-1.98M vs 1.91-2.00M elems/s). Transient ClassList is cheap/escape-analyzed; memoize slot-write offsets it. (style/dataset memoize stays — those proxies are heavier.)
- v0.1.44 — read nodeType once during child inflation — SHIPPED (+5% inflation A/B; suites 6188 + 9670 green, 70.5s/106.0s under load). Scorecard absolutes are session-relative (load-dependent) — trust back-to-back A/B, not vs stale baseline.
- DITCHED — textContent cache c.nodeType in a local: SLOWER/overlap (109 vs 105-107ms). node.nodeType is a constant-returning getter V8 INLINES free — caching it adds nothing. (Contrast buf.tagName/buf.nodeType wins: those are real SoA array reads.) Lesson: redundant-read wins are for real lookups, not inlined constant getters.
- DITCHED — installGlobals value-descriptors for base+STATIC_BASE (vs lazy getters): SLOWER (19.5-19.9k vs 22.3-22.6k ops/s). Value descriptors must read window[name] for all ~80 globals AT INSTALL, forcing resolution of globals tests never touch; lazy getters defer to actual access (tests use ~10) → lazy wins despite closure allocs. The lazy-window deferral is optimal for the access pattern.
| v0.1.45 | 47.9s ✓<50 | 93.6s | lazy customElements (+5% createEnvironment) |
- v0.1.45 — lazy customElements registry (eager makeCustomElements → SHARED_LAZY) — SHIPPED (+5% createEnvironment; uidc 47.9s ✓, payroll 93.6s; both green). Env-bucket win, env-independent factory fresh-per-env on access.
| v0.1.46 | 49.9s (loaded) | 101.0s (loaded) | lazy window.origin (+18% createEnvironment) |
- v0.1.46 — lazy window.origin (eager new URL(url) → per-env lazy) — SHIPPED (+18% createEnvironment, biggest env win; suites green, wall load-noisy this run). URL parse is heavy; deferring it for the common non-origin-reading test is a big per-file env-bucket cut.
- DITCHED — Image/Audio/getSelection base closures → lazy: createEnvironment overlap (982-984k vs 980-987k). Closures are cheap to build; the v0.1.46 origin win was the heavy URL PARSE specifically, not closure-deferral. Lesson: lazy-defer pays only for genuinely heavy eager work (URL parse, Maps), not cheap closures.
| v0.1.47 | 48.1s ✓<50 | 87.6s | unpack SoA once per cached HTML (+27% createEnvironment) — payroll lowest yet |
- v0.1.47 — unpack SoA blob once per cached HTML (was re-unpacked per Document) — SHIPPED (+27% createEnvironment, biggest env win; uidc 48.1s ✓, payroll 87.6s lowest yet; both green). Parse cache now memoizes UNPACKED soa; read-only views shared across Documents.
- DITCHED — __load reuse __cache array + null __mo (vs realloc []): createEnvironment overlap (1201-1208 vs 1189-1224k). After unpack-once, 1-2 small array allocs per createEnvironment are below the measurement floor.
- AUDIT (no ship) — makeLocation/makeHistory: single URL parse in makeLocation (lazy), makeHistory reads location.href (no re-parse). No double-parse. createEnvironment path now well-optimized (~1.2M ops/s, +60% since v0.1.43); remaining per-env costs (closures, small arrays, the one needed URL parse) are at/below the measurement floor.
| v0.1.48 | 48.0s ✓<50 | 87.0s | version-cache children filter (+58% children access) — payroll lowest |
- v0.1.48 — version-cache children element-filter (was re-filtered per access) — SHIPPED (+58% children access; uidc 48.0s, payroll 87.0s lowest; both green). First tests-bucket win; live via Document.__version key.
| v0.1.49 | 47.5s ✓<50 | 91.0s (loaded) | shared element-child version-cache (childElementCount/first/last ~18×) |
- v0.1.49 — share version-cached element-child array across children/childElementCount/first/last (was re-filter per access) — SHIPPED (~18× on count/first/last microbench; uidc 47.5s best, payroll 91.0s loaded; both green). Extends v0.1.48 pattern.
| v0.1.50 | 48.9s ✓<50 | 95.3s (loaded) | version-cache Element/ShadowRoot getElementsBy* (~2.7× reused) |
- v0.1.50 — version-cache Element/ShadowRoot getElementsByTagName/ClassName subtree walks (closure-local, per collection) — SHIPPED (~2.7× reused-collection microbench; both suites green). Document versions were already cached; this extends to scoped collections.
| v0.1.51 | 48.1s ✓<50 | 95.4s (loaded) | nextElementSibling/prev O(n²)→O(n) (~2×) |
- v0.1.51 — nextElementSibling/previousElementSibling one-indexOf+scan (was nextSibling-per-step, each O(n) indexOf → O(n²)) — SHIPPED (~2× text-interspersed walk; both suites green). Algorithmic, no cache.
- NOTE v0.1.51 — rewrite dropped dom.mjs to 99.88% line (return-null edges); aggregate gate still passed; fixed forward with sibling-edge test → dom.mjs 100% line restored. Lesson: new branches in a hot getter need their edge (null/empty) cases tested even when the gate is aggregate.
- DITCHED — select.value index-loop over getElementsByTagName Proxy (vs Array.from+find): SLOWER (1.37M vs 1.84M). liveHTMLCollection Proxy per-index get-trap costs more than Array.from (one iterator pass→plain array) + .find (V8 array builtin). Lesson: do NOT index-loop a live-collection Proxy in a hot path — copy to array first.
- AUDIT (no ship) — __children() already memoized (__kids, O(1) after first build). Remaining hot-getter candidates rejected on risk/net: raw nextSibling/previousSibling index-cache would add a per-child write to the hot __children build (risks regressing inflation v0.1.44) for a less-hot getter; closest()/matches() result-cache needs per-(element,selector,version) keying (complex, risky). No clear low-risk win this round.
- AUDIT (no ship) — notifyMutation already lean: no-observer path is ownerDocument read + version bump + mo.length check + return, ZERO alloc (record/addedNodes built only after early return). Mutation hot path (React renders) confirmed optimal. Next candidate to check: does Element.style memoize (like dataset/__childNodesList) or build a fresh CSSStyleDeclaration per access?
- AUDIT (no ship) — Element.style already memoized (__style), like dataset/childNodes/children. All hot wrapper-getters memoize; __children memoized; notifyMutation zero-alloc. Runtime hot paths confirmed well-optimized after v0.1.34-51 (16 ships). Remaining per-op costs are inherent (closest/matches per-element selector match, isConnected/contains ancestor walks) or below the measurement floor. Plateau of low-risk wins reached; loop now steady-state audit (cheap — no gate dump) until a genuine candidate surfaces.
- AUDIT (no ship) — matchCompound (selectors.mjs, uncached path for matches/closest) already reads class/id/tag ONCE each, no-alloc hasClass scan (intentional, per comment). No double-read. closest/matches cost is inherent per-element selector matching. Confirmed optimal.
- AUDIT (no ship) — collectByTag clean (nodeType inlined, localName field, single read/node). collectByClass re-reads class per class ONLY for multi-class selectors, but runs in the cached path (Document __byClass / Element version-cache v0.1.50) → once per version, not per-access. Cache-gated → no win (matcher-opt lesson). Walks confirmed optimal.
| v0.1.52 | 48.5s ✓<50 | 88.5s | deep cloneNode direct __kids build (~2×) |
- v0.1.52 — deep cloneNode builds __kids directly (skips per-child appendChild notifyMutation+reparent on detached clone) — SHIPPED (~2× on ~120-node clone; both suites green). Detached-tree-construction pattern: skip mutation bookkeeping when nothing observes it.
| v0.1.53 | 48.3s ✓<50 | **75.7s** (quiet, ~target) | DocumentFragment.cloneNode direct build (+8%) |
- v0.1.53 — DocumentFragment.cloneNode direct __kids build (completes v0.1.52 detached-clone pattern) — SHIPPED (+8%; uidc 48.3s, payroll 75.7s LOWEST/~at 75s KPI target on quiet machine; both green).
- AUDIT (no ship) — Range.cloneContents/extractContents DO use frag.appendChild(node.cloneNode(true)) per node (same detached-build pattern as v0.1.52/53, could be direct-built). BUT Range ops are COLD (rich-text/selection editors only, ~never in React component suites) → no KPI/suite impact; not worth the ship cycle. Known low-priority consistency item. innerHTML-set + __inflateNested already direct-build. KPI HIT (uidc<50, payroll 75.7s at target) — major wins banked.
- AUDIT (no ship) — setAttribute lean: lowercase name, build-attrs-once, .find (V8 builtin — faster than manual loop, proven), update-or-push, notifyMutation (zero-alloc no-observer). React-render hot paths all confirmed optimal: createElement (ctor monomorphism v0.1.34), append/insertBefore (notifyMutation lean), setAttribute, dispatch (single-walk), textContent (fast-path). KPI HIT (uidc<50, payroll 75.7s). Deep plateau post-KPI — remaining candidates cold (Range) or below-floor.
- AUDIT (no ship) — buffer.attrGet lean: linear scan over typed arrays (attrNameId→attrNames compare), no alloc/double-read. For typical <5 attrs/element a scan beats a name→value Map (hash + build overhead > tiny scan); backlog #2 (getAttribute Map) stays dead. getByRole reads many attrs/element but each scan is O(small C). Optimal.
- AUDIT (no ship) — createElement minimal: new Element(this, String(tag).toLowerCase(), ""), NO per-call tag→class lookup (generic Element; tag-specific HTML*Element via prototype/instanceof, not construction). String(tag).toLowerCase() free for React lowercase tags + ctor monomorphism (v0.1.34). Optimal. All React-render hot paths now exhaustively audited optimal; loop in confirmatory-audit steady-state post-KPI.
| v0.1.53 (re-run) | **46.9s** ✓ best | 80.8s | KPI data point (no code change) — payroll varies 75.7-80.8s by load |
- KPI DATA POINT @v0.1.53 (no code change): uidc 46.9s (best yet, solidly <50 ✓), payroll 80.8s. payroll varies ~75.7-80.8s with machine load; hits the 75s target only on quietest runs. Its wall is vitest-import-bound (import ~358s vs turbo-dom env ~39s cumulative) — reliably-sub-75 needs vitest-side change (pool/shard), not turbo-dom micro-opts. uidc target met reliably.
- AUDIT (no ship) — dispatch optimal: lazy _path (v0.1.38), single-walk hasListener flag gating capture/bubble (listener-less React events skip all phases), single-listener slice-skip snap (v0.1.42). Multi-listener slice necessary (safe iteration during mutation). LAST hot spot confirmed — full hot-path audit complete, all optimal.
- AUDIT (no ship) — (1) base Node.cloneNode (line ~236) still appendChild-per-child but COLD (Element/DocumentFragment/Text/Comment all override; base hit only by Document/rare clone) → deprioritized like Range. (2) Considered granular cache invalidation (attr mutations skip __byTag structure-cache) — REJECTED: tests run queries AFTER render, not interleaved with mutations, so caches rebuild once regardless = no win. Well of low-risk turbo-dom wins is dry; KPI hit (uidc<50 reliably, payroll 75.7-80.8 vitest-bound).
- AUDIT (no ship) — getElementsByName re-walks collectByTag("*")+filter per access (NOT version-cached like v0.1.50 scoped getElementsBy*). Same closure-local cache would fix it cheaply BUT getElementsByName is not clearly hot (radio-group lookups, rare in React/RTL) → candidate-if-it-shows-hot, not shipping speculatively. element.labels already cached (getByLabelText fix per CLAUDE.md).
- SCORECARD @v0.1.53 vs v0.1.43 baseline (quiet): createEnvironment 768k→1,235k (+61%), inflate+traverse 9.7k→11.0k (+14%); dispatch-listenerless 3.22M→3.26M, dispatch-single 2.15M→2.16M, mutation 15.7M→16.1M, textContent ~200M, addEventListener ~14.3M — all STABLE (already optimal), NO REGRESSION across 18 ships. Scorecard does not cover v0.1.48-53 children/childElementCount/getElementsBy*/clone/nextElementSibling wins → understates total gains.
- AUDIT (no ship) — ClassList add/remove/toggle use split(/\\s+/)+includes+join (allocs/op) but are MUTATION path (infrequent vs reads; React sets className→setAttribute, not classList.add) → not read-hot, not worth. PLATEAU: turbo-dom thoroughly optimized, KPI met, well dry. Loop switching to ~25min cadence for idle phase (was 90s) to conserve context; stays armed to 20:05 deadline.
| v0.1.53 (re-run 2) | 46.6s ✓ | **74.9s** ✓<75 | KPI point — BOTH targets met (quiet) |
- KPI DATA POINT @v0.1.53 (no code change): uidc 46.6s ✓, payroll 74.9s ✓ — BOTH under target on quiet machine. payroll confirmed dips <75 on quiet runs (74.9-75.7); 80.8 was a loaded run. KPI met when machine quiet; load-variance is vitest-import-bound, not turbo-dom.
- AUDIT (no ship) — Text.splitText/wholeText: cold (text-manipulation, ~never in React/RTL render-assert flow); not hot-path. No further low-risk turbo-dom wins identified. KPI both met (uidc 46.6 / payroll 74.9 quiet). Idle plateau confirmatory.
| v0.1.53 (re-run 3) | 46.1s ✓ | **73.5s** ✓<75 best | KPI point — both met, payroll best |

## Rust layer (post-JS-plateau)
Baseline @HEAD (parse-ab.mjs best-of-6): REAL 150-cards ~1,617 ops/s, ATTR 120-inputs ~2,271 ops/s. Pack is small tail of parse (html5ever+walk+intern dominate); suites are parse-CACHED so Rust wins show in parse microbench, suites = no-regression gate.
- DITCHED idea1 — bulk-copy pack columns (unsafe from_raw_parts LE-cast vs per-value to_le_bytes loop): back-to-back A/B REAL +0.7% but ATTR overlaps (HEAD 2291 > new 2279), not every-run-beats. Pack too small a fraction of total parse → negligible. Reverted (no dep, no unsafe added).
- SHIPPED idea2 (v0.1.54) — FxHashMap (rustc-hash) intern: +6-7% parse both fixtures (REAL 1600→1710, ATTR 2260→2425, every-run-beats back-to-back), byte-identical, both suites green.
- DITCHED idea3 (analysis, no build) — dedupe double to_string in intern: fires only on MISS (2 owned copies genuinely needed: Vec + map key). get-first is already alloc-free on HITS (common case); std entry API allocs key on every call incl hits = regression; Rc<str> would be 1 shared alloc but breaks napi Vec<String> boundary. No simple win.
- DITCHED idea4 — pre-size SoA column Vecs (reserve_nodes html.len()/10): back-to-back A/B REAL idea4 [1673,1715,1733] vs HEAD [1681,1684], ATTR idea4 [2420,2437,2459] vs HEAD [2385,2422] — overlapping ranges (idea4 mins dip below HEAD maxes), NOT every-run-beats. Realloc savings (few per column) below the measurement noise floor. Reverted.
- DITCHED idea5 — codegen-units=1 + strip: clean best-of A/B mixed — REAL cgu1 1655 vs cgu16 1562 (+6%) but ATTR cgu1 2176 vs cgu16 2215 (cgu16 wins), high variance. Not every-run-beats. Reason: html5ever (bulk of parse) is a SEPARATE crate already optimized + lto=true inlines cross-crate; cgu=1 only affects our small core crate → negligible parse effect, and doubles compile time. Reverted.
- DITCHED R1 — kill per-attr prefix String alloc: WRONG premise. Empty String / String::default() does NOT heap-allocate, so the common no-prefix case (unwrap_or_default → empty String) was ALREADY alloc-free; to_string only allocs for rare non-empty foreign-content prefixes (xlink/xml, ~never in HTML). A/B confirms no win (REAL/ATTR overlap, load-drift). Reverted. Lesson: empty-String is alloc-free in Rust.
- SHIPPED R5 (v0.1.55) — pre-reserve FxHashMap capacities (tag32/name64/prefix8/value128): REAL every-run-beats +6% (1589-1690 vs HEAD 1501-1556), ATTR flat-to-slightly-up, byte-identical output, both suites green. Reserve avoids rehash during build — same family as the FxHashMap win (hashing is the real SoA-side hot spot).
- DITCHED R2 (analysis) — reuse builder FxHashMaps via thread_local: clear() drops String keys (re-alloc per parse on misses anyway), and R5 already RESERVED capacity so maps no longer rehash/grow. R2-on-top-of-R5 saves only ~4 bucket-array allocs/parse → below noise. Medium-risk thread_local state not justified.
- DITCHED R3 (analysis) — drop pub_id/sys_id per-node columns: buffer-size is the pack TAIL (~1% of parse, per idea1 bulk-pack result); removing 8N of ~48N bytes ≈ <1% total → below noise. Plus format-change risk (Rust pack + JS unpack must mirror). Not worth it.
- DITCHED R4 (analysis) — custom TreeSink to skip RcDom: html5ever adoption-agency/foster-parenting (reparent_children, append_based_on_parent_node) MUTATE the tree mid-construction; a flat append-only SoA cannot support that → you would reimplement RcDom mutable tree. High risk, uncertain win, NOT simple-to-test. Rejected.

## Rust round-2 summary: 1 ship (R5 reserve +6% REAL), 4 ditched (R1 empty-string-noop; R2/R3/R4 analysis). Both Rust wins (v0.1.54 FxHashMap +7%, v0.1.55 reserve +6%) are in the INTERNING/HASHING path — the real SoA-side hot spot. Everything else (buffer pack/columns/allocs, RcDom-fuse) is below noise (parse dominated by html5ever tokenize + RcDom build) or too risky.
- DITCHED N1 (lazy per-id string decode): render-only distinct-big-text A/B TIED (lazy 2847-2902ms vs eager 2850ms). JS TextDecoder is cheap — NOT the bottleneck; HR2 won by killing the napi UTF-8→UTF-16 CONVERSION (native side), not the JS decode. Lazy only adds a .get() branch to the hot text path for zero gain. Reverted. LESSON: JS-side string handling is cheap; only the native/boundary conversion + hashing are real levers.
- DITCHED N4 (skip attr_value interning / no-dedup): SLOWER both fixtures (REAL 1461 vs ~1700, ATTR 2137 vs ~2450). Real docs repeat attr values heavily (type/class/aria/boolean ×N per doc); interning dedups to 1 entry, no-dedup pushes N copies → bloated attr_values table + string blob + N×to_string allocs. Dedup hash cost << the bloat it prevents. Interning is correct. Reverted. LESSON: dedup wins even on "unique-heavy" docs — enough values repeat (boolean/enum attrs) that the hash pays for itself.
- DITCHED N5 (static tag-id match): TIED vs HEAD at same load (N5-restored 1499-1539 = HEAD 1497 at load 5.6; the earlier 1848-1913 was just lower load, not N5 faster). FxHash of a short tag name is already ~1ns; a compiled match doesnt beat it. As predicted ~0.4%, below noise. Reverted.
- DITCHED N2 (analysis) — merge packed+strBlob+strMeta into one Uint8Array: saves ~2 napi wrapper objects/parse. HR2 won on per-STRING conversion (N conversions), NOT array-count (already 3). 3→1 arrays ≈ idea1-tier (below noise). Not worth.
- DITCHED N3 (analysis) — drop attr_prefix_id column + prefixes table: buffer-byte reduction (idea1 bulk-pack = ~1%, already ditched) + one tiny table; below noise, AND high effort (Rust Soa + pack + JS unpack + out-of-band prefix store). Not worth.

## Rust N-round (5 ideas based on HR1/HR2/HR3 learnings): 0 shipped, 5 ditched (N1/N4/N5 RUN+measured, N2/N3 analysis). The noise floor is conclusive across ~14 experiments: ONLY boundary-conversion (HR2 string blob +9%) and broad-hashing (FxHashMap +7%, reserve +6%) beat it. Node-alloc, Vec/map pooling, JS-decode, buffer-bytes, array-count, single-table-hash, no-dedup ALL below noise or worse. The parser is html5ever-tokenizer-bound; turbo-dom-side wins are exhausted. Shipped parser wins this session: v0.1.54 FxHashMap, v0.1.55 reserve, v0.1.56/57 string-blob.
