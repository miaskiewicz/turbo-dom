# turbo-dom performance experiment loop

This is the **exact protocol** for proposing, validating, and shipping a perf change.
Every change ships ONLY if it passes ALL gates below. No exceptions, no "looks faster",
no single-run wall-clock claims. Measure everything. Ditch anything that doesn't clearly win.

The runtime (`src/runtime/*.mjs`) is pure JS — only the parser is the native `.node`.
So a runtime change can be **hot-swapped** into a consuming repo's `node_modules` without
rebuilding: copy the `.mjs` files over.

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
