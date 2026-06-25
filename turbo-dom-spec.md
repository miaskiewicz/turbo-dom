# gr0gdom Test Runtime — Technical Specification

A lazy, copy-on-write, native-parser-backed DOM implementation for test runners.
Goal: faster than jsdom **and** more compatible than happy-dom — by economizing on
*eagerness* instead of *correctness/breadth*.

---

## 0. Thesis

Two independent cost centers in DOM testing, attacked separately:

1. **Parsing** — bulk, compute-bound, no fine-grained JS interaction → push to native
   (Rust via N-API). One call in, one tree/buffer out.
2. **The DOM runtime** — chatty, synchronous, fine-grained object access → must stay
   in JS/V8. The win is *not building what isn't touched*, not "compiling" it.

> The mistake would be native-ifying the runtime. Keep it in JS, make it lazy.

### Why WASM is wrong for the runtime (and right for the parser)
The DOM's value is live JS objects test code touches synchronously (`el.children`,
`getBoundingClientRect`, event dispatch). If the DOM lived in WASM linear memory, every
access crosses the JS↔WASM boundary and marshals data — that boundary cost would *dwarf*
any internal speedup. WASM is great for compute-bound batch work (parsing) and terrible
for chatty fine-grained object access. So WASM appears in exactly one place: a fallback
build of the **parser**, where it's bulk-in/bulk-out.

### The core insight vs. happy-dom
happy-dom traded **correctness for speed** → permanent compatibility tax.
This design trades **eagerness for speed** → no tax, because an interface that's never
touched can be 100% correct *and* free (you just didn't run it).

---

## 1. Why happy-dom's "doesn't work" issues exist (structural autopsy)

Every happy-dom failure is the same shape: *a global decision to be cheap, applied
uniformly, that breaks whatever happened to be load-bearing for a given library.*

| Failure genre | Root cause (architectural) | This design's answer |
|---|---|---|
| New platform features missing (`:has()`, `dialog`, `popover`, `ElementInternals`, `AbortSignal` plumbing) | subset chosen for speed; "subset" is a permanent treadmill as the platform grows | Speed comes from laziness, not subsetting — implement full surface, pay only on access. Breadth no longer in tension with speed. |
| `getComputedStyle` lies (returns *something*, not browser-correct) | half-implemented CSS/layout; "kinda" is worse than absent because it lies plausibly | Honest stub by default, **explicit** about absence; opt-in faithful layout as a separate module. Never "kinda." |
| Live-collection / identity bugs (`childNodes` stale after mutation, `node === node` fails, WeakMap breakage) | liveness cut for speed | COW overlay preserves full live semantics + memoized identity; laziness touches allocation *timing*, never correctness. |
| Event gaps (focus order, `composedPath`, capture/bubble, `relatedTarget`, default actions) | event model subset | Event system implemented **fully** — it's small and load-bearing; laziness saves nothing here so there's no temptation to cut it. |
| Parser divergence on messy input (table foster-parenting, optional end tags, `<template>` content doc, SVG/MathML foreign content) | hand-rolled parser will never catch the WHATWG tree-construction edge cases | Use `html5ever` (Servo's parser) — it *is* the spec algorithm, fuzzed against html5lib-tests. Inherit correctness instead of chasing it. |

**Meta-pattern:** they cut breadth uniformly and correctness uniformly; the cuts land on
whatever a given library depended on. The fix is to change the *axis* you economize on:
from breadth/correctness → to eagerness. Be fully correct and fully broad on everything
implemented; economize purely on *when* you build it.

---

## 2. Architecture overview

```
┌─────────────────────────────────────────────────┐
│  Test code (RTL, user-event, your assertions)    │
└───────────────┬─────────────────────────────────┘
                │  touches ~5% of DOM surface
┌───────────────▼─────────────────────────────────┐
│  Lazy window (Proxy, self-replacing globals)     │  ← Layer 3
├──────────────────────────────────────────────────┤
│  Lazy Node handles + COW overlay (JS, memoized)  │  ← Layer 2
│  + stubbed layout/CSSOM                           │  ← Layer 4
├──────────────────────────────────────────────────┤
│  Immutable parse buffer (SoA typed arrays)        │  ← shared, Layer 5 reset
└───────────────┬─────────────────────────────────┘
                │  one boundary crossing per parse
┌───────────────▼─────────────────────────────────┐
│  Rust N-API: html5ever → flat buffer              │  ← Layer 1
└──────────────────────────────────────────────────┘
```

Each layer is independently shippable and independently killable.

---

## 3. Layer 1 — Native parser (N-API, not WASM)

### Library choice
- **`html5ever`** (Servo). Spec-compliant WHATWG tree construction, fuzzed against
  `html5lib-tests`. Alternative for streaming/rewriting: **`lol-html`** (Cloudflare),
  but `html5ever` is the right fit for full-document tree construction.
- Binding: **`napi-rs`** (handles prebuilt binaries per platform/arch, N-API ABI stability).

### Critical design decision — don't marshal a node tree per node
Marshaling a full JS node tree across the boundary re-introduces chatty boundary cost.
Instead, Rust parses into a **flat, compact Structure-of-Arrays (SoA)** in a contiguous
buffer and crosses the boundary **once**.

### Buffer layout (SoA)
```
{
  tagIds:     Uint16Array,   // index into a tag-name table
  parent:     Int32Array,    // node index of parent, -1 for root
  firstChild: Int32Array,    // -1 if none
  nextSib:    Int32Array,    // -1 if none
  nodeType:   Uint8Array,    // element / text / comment / doctype / ...
  attrIndex:  Int32Array,    // offset into attr table (start), -1 if none
  attrCount:  Uint16Array,   // number of attrs for this node
  textIndex:  Int32Array,    // offset into string table for text/comment nodes
  // String tables:
  tagNames:   string[],      // interned, deduped
  attrNames:  string[],      // interned
  attrValues: <packed UTF-8 + offsets> | string[]
  textData:   <packed UTF-8 + offsets> | string[]
}
```

Design notes:
- **Struct-of-arrays, not array-of-structs** → cache-friendly, cheap typed-array copy,
  and lets JS read structure without allocating node objects.
- **Intern tag/attr names** (small finite set) → `Uint16` ids, fast comparison.
- Node `0` = document, node `1` = `<html>` or first parsed node, etc. Index *is* the id.
- One `postMessage`-free copy of typed arrays into JS per `parse()` / `innerHTML=`.

### API
```
parse("<div id=a><span>hi</span></div>")
  → Rust(html5ever) → SoA buffer (above)
  → JS holds buffer as immutable source-of-truth; inflates nodes lazily (Layer 2)
```

Boundary crossed **exactly once** per document parse or `innerHTML` set.
Even parsing then pays (in JS-object terms) only for nodes the test touches.

### WASM fallback
Ship a `wasm32` build of the *same* Rust parser for environments that can't load native
addons (StackBlitz, WebContainers, locked-down CI). Same SoA output contract. This is the
one correct use of WASM in the project: bulk-in/bulk-out, no chatty access.

### Distribution
- `napi-rs` prebuilds: `darwin-arm64`, `darwin-x64`, `linux-x64-gnu`, `linux-arm64-gnu`,
  `linux-x64-musl` (Alpine/CI), `win32-x64-msvc`. Optional-deps pattern per platform.
- WASM build published as a separate entry; loader picks native, falls back to wasm.

---

## 4. Layer 2 — Lazy node inflation + copy-on-write tree

Nodes don't exist as JS objects until accessed. Each `Node` is a thin handle over a
buffer index.

### Handle
```js
class Node {
  #doc; #idx;                       // owning document + buffer index
  constructor(doc, idx) { this.#doc = doc; this.#idx = idx; }

  get parentNode() { return this.#doc.nodeAt(this.#parentIdx()); }
  get firstChild() { return this.#doc.nodeAt(this.#firstChildIdx()); }
  get nextSibling(){ return this.#doc.nodeAt(this.#nextSibIdx()); }
  // structure read straight from typed arrays — no allocation on traversal
}
```

### Identity memoization (non-negotiable)
`doc.nodeAt(idx)` **memoizes**: first access allocates + caches the JS object; later
accesses return the *same* instance. You MUST preserve `===` identity — libraries rely on
it for event targets, WeakMap keys, `node === node` checks.

```js
nodeAt(idx) {
  if (idx < 0) return null;
  let n = this.#cache[idx];
  if (n === undefined) { n = makeNode(this, idx); this.#cache[idx] = n; }
  return n;
}
```

### Copy-on-write promotion
A node starts **buffer-backed** (read-only, structure in typed arrays). The first mutation
(`appendChild`, `removeChild`, `setAttribute`, `textContent=`, …) **promotes** the affected
node (and the minimal necessary neighbors) into a fully-owned normal-JS-object state
("overlay"). Pristine parsed structure stays cheap; only mutated regions cost full DOM.

Promotion boundary rules (the fiddly part):
- Mutating a node's children promotes that node to owned (its child-list becomes a real
  linked structure, not buffer indices).
- Reading is transparent across the boundary: a buffer-backed read and an owned read MUST
  be byte-for-byte indistinguishable.
- **Governing invariant:** *when in doubt, fall back to eager inflation of the subtree —
  slower, never wrong.* Never return a wrong/stale answer to save work.

### Live collections
`childNodes`, `children`, `getElementsByTagName`, etc. must be **live**. Implement as views
that compute over current structure on access (or with a mutation-version counter to
invalidate cached snapshots). This is exactly where happy-dom bleeds issues — treat as
core correctness, not an optimization target.

### Free-ish reset (feeds Layer 5)
Because pristine structure lives in the immutable buffer, per-test reset = drop the owned
overlay + node cache, keep the buffer. Closer to "bump allocator reset to zero" than
teardown.

---

## 5. Layer 3 — Lazy `window` (lowest risk, likely biggest win)

`window` is a `Proxy`. Each lazy global is a factory that materializes on first `get` and
**self-replaces** with the concrete value (one-time Proxy cost per property).

```js
const lazyGlobals = {
  localStorage:         () => new Storage(),
  sessionStorage:       () => new Storage(),
  matchMedia:           () => stubMatchMedia(),
  getComputedStyle:     () => makeGetComputedStyle(),     // honest stub by default
  IntersectionObserver: () => makeIO(),
  ResizeObserver:       () => makeRO(),
  MutationObserver:     () => makeMutationObserver(),
  Range:                () => makeRange(),
  requestAnimationFrame:() => makeRAF(),
  // ~200 entries, none constructed until touched
};

const window = new Proxy(baseWindow, {
  get(t, k) {
    if (k in t) return t[k];
    const factory = lazyGlobals[k];
    if (factory) { const v = factory(); t[k] = v; return v; }   // self-replacing
    return undefined;
  }
});
```

Notes:
- **Subsystem grouping:** related globals co-materialize and share state
  (`history`/`location`; `document.cookie`/storage if coupled).
- **Universal globals** (touched by ~every test, per Phase-1 histogram) can be eager —
  no point lazifying what's always used.
- **Free instrumentation:** the Proxy logs which globals each test actually touches →
  a "DOM surface used" report, useful to flag tests that could drop to `node` env.

A test using only `document.querySelector` never constructs `localStorage`,
`IntersectionObserver`, `matchMedia`, `Range`, CSSOM, etc. jsdom builds ~all of it eagerly,
per file. This is the construction-cost win and it compounds per test file.

---

## 6. Layer 4 — Aggressively, *honestly* stub the unobservable

Headless runners have no layout, so stop pretending (and stop lying):

- `getBoundingClientRect()` → zeros by default. Opt-in fake layout model only if a test
  asks (separate module).
- `getComputedStyle` is a **partial cascade** (since v0.1.29): it resolves REAL injected
  `<style>` rules + inline styles with specificity/source order, shadow-root scoping with
  `:host`/`::slotted`, and inheritance into shadow content — but never invents a value
  (unmatched props read `''`). Still no layout/`@media`/state/pseudo-element realism.
- No reflow.
- **Honest absence over plausible lie:** prefer a `getComputedStyle` that clearly signals
  "layout not available; opt in with X" over one that returns 30%-correct values that pass
  in CI and fail in reality. Half-implemented generates *more* bug reports than absent.

Tests asserting real geometry are integration tests that belong in a real browser
(Playwright/WebDriver), not here.

---

## 7. Layer 5 — Fast per-file reset (the per-suite killer)

Dominant cost in real suites is `new JSDOM()` **per file**. Architecture makes reset cheap:

- Pristine parse buffer is **immutable and shareable** across files.
- Per-test state = COW overlay (mutated nodes) + node cache + materialized globals.
- Reset = discard overlay + node cache + self-replaced global slots; keep the buffer and
  class machinery warm. Arena-style "reset pointer," not reconstruct.

---

## 8. Risk map (honest)

| Risk | Where | Mitigation |
|---|---|---|
| Identity / liveness regressions | Layer 2 COW overlay | Differential fuzzing vs jsdom; fall back to eager inflation when invariant uncertain — slower, never wrong. |
| Event-model fidelity | Layer 2/3 events | Implement fully (it's small + load-bearing). Heavy `user-event` test coverage. |
| Long-tail platform surface | everywhere | Lazy ⇒ breadth is free until touched; implement faithfully or stub honestly, never "kinda." |
| Native build/distribution | Layer 1 | `napi-rs` prebuilds + WASM fallback. |
| Parser edge cases | Layer 1 | Inherit `html5ever`; gate on `html5lib-tests`. |

**Worst-case posture:** "slower in the worst case, never wrong" — the exact opposite of
happy-dom's "fast but wrong on your library."

---

## 9. Testing & validation plan (3 phases)

Sequencing principle: **each phase must be killable.** Validate the cheapest
highest-leverage claim first, the riskiest claim last. Pick kill/continue numbers *before*
seeing results. Phases 1 and 2 each ship standalone, better-than-happy-dom value.

```
Phase 1: lazy window      → tests "is construction the cost?"      → days,   zero risk
Phase 2: native parser    → tests "faster + more spec-correct?"    → weeks,  low risk
Phase 3: COW lazy nodes   → tests "speed without incompatibility?" → months, the real bet
         each phase can be the stopping point
```

### Phase 1 — Lazy `window`, zero correctness risk (1–2 weeks)
**Hypothesis:** per-file construction cost dominates real suites; lazy globals recover most
of it. If false, kills the whole project — so test first, cheaply.

- **Build:** fork happy-dom (not jsdom — isolate the *laziness* variable, not also fight
  breadth). Replace eager `window` with self-replacing Proxy. Touch nothing else.
- **Instrument:** Proxy doubles as tracer — log every global materialized per test file →
  empirical surface-usage histogram (drives all later decisions: universal vs rare globals).
- **Benchmarks:**
  - Microbench: cold construction time, eager vs lazy, isolated.
  - Real suite: actual RTL/React suite (yours or a large OSS component library), wall-clock
    + per-file setup overhead, both modes.
  - **Correctness gate:** entire existing happy-dom test suite must still pass (lazy window
    is observably identical — any break is a Proxy bug, not a design problem).
- **Kill / continue bar:**
  - **Continue** if ≥30% off per-file setup on a real suite (tune to your suites; commit now).
  - **Kill/pivot** if <10% — construction isn't your bottleneck; jump straight to profiling
    parse/runtime instead of building the stack.
- **Deliverable:** benchmark report + surface-usage histogram.

### Phase 2 — Native parser, isolated (2–4 weeks)
**Hypothesis:** `html5ever` via N-API (a) beats the JS parser on real fixtures **and**
(b) is more spec-correct, eliminating the parser-divergence bug class. Two measurable wins.

- **Build:** `napi-rs` + `html5ever`. **Start with full marshaling** (simple/slow: produce
  a complete JS tree). Do NOT build the SoA buffer yet — that's an optimization gated on
  marshaling proving to be the cost. Ship WASM build of the same parser in parallel.
- **Benchmarks:**
  - Parse throughput vs happy-dom JS parser across fixtures: small components, large SSR
    output, deeply nested, deliberately malformed (tables, optional end tags, foreign content).
  - Boundary-cost isolation: parse-only vs parse+marshal separately → decides if SoA buffer
    (Phase 2.5) is worth it.
  - WASM-vs-native delta: confirm fallback is acceptable, not catastrophic.
- **Correctness gate (the real prize):**
  - Run **`html5lib-tests`** (WHATWG tree-construction conformance) against your parser AND
    happy-dom's. Count divergences. Thesis "we inherit Servo's correctness" proven with a
    number → headline compatibility argument.
- **Kill / continue bar:**
  - **Continue** if native parse meaningfully faster on parse-heavy fixtures AND html5lib
    pass rate materially higher than happy-dom's.
  - **Drop parser, keep Phase 1** if parsing is a rounding error in your suites (DOM-light
    React parses tiny fragments). Phase-1 histogram already told you if you're parse-bound.
- **Deliverable:** parser as drop-in behind a flag + conformance-delta report.

### Phase 3 — COW lazy-node tree, the make-or-break (6–12 weeks; only if 1 & 2 cleared)
**Hypothesis:** nodes can materialize lazily from the buffer with COW mutation while
preserving live semantics + identity *perfectly* — speed without happy-dom incompatibility.
Highest-risk, highest-effort; goes last because cheaper phases must justify it.

- **Build:** SoA buffer as source-of-truth, thin `Node` handles, memoized `nodeAt()` for
  identity, COW promotion on mutation. Invariant: buffer-backed read ≡ owned read,
  byte-for-byte; when uncertain, fall back to eager inflation — slower, never wrong.
- **Testing strategy (needs more than benchmarks — adversarial correctness):**
  1. **Differential testing vs jsdom (oracle).** Generate random DOM op sequences (build,
     query, mutate, re-query, dispatch events, read collections); run against jsdom and your
     impl; any divergence in output/identity/liveness is a bug. Fuzz hard — catches the COW
     liveness corners you can't enumerate by hand.
  2. **Real-library gauntlet.** Run RTL, `@testing-library/user-event`, and 3–5 popular
     component libraries **unmodified**. This is the test happy-dom fails — passing it is the
     entire point. Pass rate = headline metric.
  3. **Liveness-specific property tests.** Read `childNodes` → mutate → assert reflected.
     Read node twice → assert `===`. WeakMap a node → mutate around it → assert still keyed.
     Turn each known happy-dom failure class into a regression test.
  4. **Invariant fallback audit.** Count eager-inflation fallbacks. High rate ⇒ COW isn't
     earning its complexity, reconsider. Near-zero in real suites ⇒ design holds.
- **Benchmarks:** full-suite wall-clock vs Phase-1+2-with-*eager*-nodes. Question Phase 3
  must answer: does lazy COW beat eager-but-otherwise-optimized by enough to justify the
  correctness risk? (Compare to your own simpler build, not to happy-dom.)
- **Kill / continue bar:**
  - **Ship it** if gauntlet pass rate ≥ jsdom's AND COW beats eager-nodes by a worthwhile
    margin AND differential fuzzing is clean.
  - **Ship Phase 1+2 with eager nodes** if COW can't clear correctness or speed delta is
    marginal — still a faster, more spec-correct DOM (lazy window + html5ever) without the
    riskiest layer. Don't let sunk cost force the COW.
- **Deliverable:** differential-test harness (permanent compat guarantee) + library-gauntlet
  pass-rate dashboard + final benchmark.

---

## 10. Where to start (best effort-to-information ratio)

Phase 1: scaffold the lazy-`window` happy-dom fork with the tracing Proxy + a benchmark
harness pointed at a real suite. Days of work, proves or kills the central bottleneck
assumption, zero correctness risk. Then Phase 2 parser only if histogram shows parse cost;
Phase 3 COW only if Phase-1+2 wins justify the months of risk.

---

## Appendix A — Adjacent quick wins (orthogonal to building a new DOM)

If the goal is just "tests faster now," independent of this project:

- **Bun test runner** — JavaScriptCore + native Zig transpiler, skips Node module machinery.
  Often 2–10x vs Jest; largely Jest-API-compatible. Biggest single lever for logic/unit tests.
- **Swap transpiler:** `esbuild` / `@swc/jest` instead of `babel-jest` / `ts-jest`. Move
  type-checking to a separate `tsc --noEmit` CI step — type-checking *in* the runner is a
  silent killer.
- **Vitest config:** `pool: 'threads'` (or `'vmThreads'`) over process forking;
  `isolate: false` if tests don't leak state (big win); `deps.optimizer` to pre-bundle heavy deps.
- **Ditch jsdom where unneeded:** `environment: 'node'` per-file. jsdom setup/teardown
  dominates many suites — this whole project is moot for tests that never needed a DOM.

Impact hierarchy: drop jsdom where unneeded → drop type-checking from runner → swc/esbuild
→ threads + `isolate:false` → switch to Bun.

## Appendix B — Open design questions
- Mutation-version counter vs recompute-on-access for live collections — benchmark both.
- SoA string storage: `string[]` interned vs packed UTF-8 + offsets — measure GC pressure.
- COW promotion granularity: node vs subtree vs region — tune via fallback-audit data.
- Shadow DOM / `<template>` content document handling under COW — likely eager-only v1.
- Selector engine: reuse existing (e.g. nwsapi-style) over buffer vs custom — correctness first.

## Appendix C — Rust-native runtime (`rtdom`), and the §3 thesis confirmed

§0/§3 argue the runtime must stay in JS for a JS consumer, because a WASM DOM would pay a boundary
crossing on every chatty access. That was **measured** (Phase-1 spike, `src/spike.rs` + `bench/spike.mjs`):
a Rust DOM in WASM called from JS is **~0.55×** the JS runtime (best case: whole node-record prefetch +
cache; naive per-call is ~0.14×). The boundary is a hard floor that zero-crossing in-process JS access
beats. So the JS-consumer runtime stays in JS, exactly as specified.

The corollary the spec didn't spell out: a **Rust** consumer (crawler/extractor/SSR) has the *opposite*
boundary situation — in-process Rust, no JS. For that there's now **`rtdom`** (the published `crates/turbo-dom` crate), a pure-Rust
port of the runtime that applies this spec's same load-bearing ideas (lazy COW tree over the SoA,
identity by handle, version-keyed caches, partial honest `getComputedStyle`, full event model) natively.
In-process from Rust it measured **~2.7×** the JS runtime on the chatty workload. So the rule generalizes:
**put the DOM on the same side of the boundary as its consumer** — JS DOM for JS, Rust DOM for Rust;
never cross the boundary per-node. See `RUST_PORT_PLAN.md` (architecture) and `RUST_PORT_PERF_HISTORY.md`
(how each JS perf win maps to Rust: PORTS / V8-SPECIFIC / BOUNDARY / ALREADY-RUST).
