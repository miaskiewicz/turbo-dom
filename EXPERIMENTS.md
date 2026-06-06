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
