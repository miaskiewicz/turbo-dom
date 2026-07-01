# turbo-dom → pure-Rust runtime port — migration plan

**Branch:** `rust-port`. **Scope:** rewrite the JS runtime layer (`src/runtime/*.mjs`, ~4.3k LOC)
as a pure-Rust core. JS consumers load a **WASM** package. Parser (`src/core.rs`) is already Rust
and stays. Harness (`harness/`) stays JS for now.

Companion doc: **`RUST_PORT_PERF_HISTORY.md`** — every perf optimization mined per-commit (v0.1.0→v0.2.5),
each tagged PORTS / V8-SPECIFIC / BOUNDARY / ALREADY-RUST. Read it before implementing any module —
it says which of our 20+ shipped wins transfer and which are V8 artifacts that vanish in Rust.

---

## 0. The boundary — what changes, what doesn't (read this first)

**Decision:** the runtime serves **two consumers** — JS test suites (vitest/jest + React/RTL/user-event)
*and* a future in-Rust pipeline (e.g. turbo-crawl in Rust). One pure-Rust core, **two thin front-ends**
(exactly the shape `core.rs` already uses: napi + wasm-bindgen, now extended over the whole runtime).

There are two distinct boundaries; the port helps one and creates the other — be precise:

1. **Internal seam (core.rs Rust ↔ runtime `.mjs`) — DISSOLVES.** Today the native parser marshals a SoA
   buffer over napi to a JS runtime that walks it. A single Rust→WASM blob deletes this seam: the SoA
   *is* the in-linear-memory representation, no marshal. Every PERF_HISTORY entry tagged BOUNDARY that
   refers to the napi seam (camelCase serde, the unpack step, the byte-blob string table) **goes away**.

2. **External seam (consumer ↔ DOM) — depends on the consumer:**
   - **Rust consumer → ZERO boundary, pure win.** turbo-crawl-in-Rust calls the DOM in-process. The chatty
     hot paths (`children`, `getAttribute`, `dispatchEvent`, `matches`) are plain Rust method calls — free.
     Everything PERF_HISTORY tags PORTS transfers and runs at native speed. This path is strictly better
     than anything achievable in JS.
   - **JS consumer → boundary is real and NEW.** React-dom/RTL/the app-under-test are JS (you don't own
     them, can't compile them to Rust). Today the DOM is *also* JS objects, so `el.appendChild(child)` is
     a free in-process V8 call. Move the DOM into WASM and that same call becomes a JS→WASM crossing — on
     every one of the thousands of DOM touches per render. This is the cost the spec's §3 warned about.

**So:** the port is unambiguously good for the Rust consumer, and a *measured bet* for the JS consumer.
The V8-SPECIFIC wins (monomorphic Element shape, read-once locals, lazy-global hoist) vanish as problems
in Rust but also stop paying — what they bought (cheap chatty JS access) is now taxed by the JS↔WASM seam.

**Net for the JS path:** wins come from moving *bulk* work into Rust (parsing already is; selector
matching/`querySelectorAll`/cascade run entirely Rust-side and cross **once** with a packed result, vs N
per-node JS calls today), not from per-node chatter. Whether that nets positive on chatty React/RTL is the
**Phase-1 kill/continue gate** (§3), measured on the KPI suites (`../ui-design-components` <50s,
`../payroll-app` <75s) **before** porting all 14 modules — same discipline as `EXPERIMENTS.md`.

---

## 1. What exists today (module map)

| Module | LOC | Role | Port difficulty |
|---|---:|---|---|
| `dom.mjs` | 1850 | Node/Element/Text/Comment/Document/DocumentFragment/ShadowRoot/Range/TreeWalker, ClassList, style/dataset proxies, synthetic geometry, MutationObserver, notifyMutation | **Hardest** — the chatty core |
| `window.mjs` | 457 | lazy window Proxy, globals, clock, scheduler, MessageChannel polyfill, FormData, btoa/atob | Hard (Proxy + host interop) |
| `selectors.mjs` | 366 | selector parse + AST cache, matchers (alloc-free), querySelectorAll/All | Medium (algorithmic, ports well) |
| `events.mjs` | 332 | Event/CustomEvent/typed events, EventTarget, dispatch path walk, retargeting | Medium |
| `cascade.mjs` | 288 | partial getComputedStyle cascade, specificity, inheritance, :host/::slotted | Medium |
| `stubs.mjs` | 188 | matchMedia, ResizeObserver, IntersectionObserver, Storage, FileReader, canvas | Easy |
| `color.mjs` | 160 | named/hex/hsl → rgb() canonicalization | Easy (pure fn) |
| `buffer.mjs` | 115 | unpack SoA blob, Buffer view over typed arrays, utf8 decode | Easy (already index-based) |
| `index.mjs` | 110 | createEnvironment, parse cache, declarative shadow promote | Medium (orchestration) |
| `cssom.mjs` | 109 | CSSStyleRule/Sheet, insertRule/deleteRule, styleSheets | Easy |
| `collections.mjs` | 80 | live NodeList/HTMLCollection/NamedNodeMap (Proxy, index fast path) | Medium (Proxy → ?) |
| `svg.mjs` | 70 | SVGElement wrappers, SVGAnimatedLength/String/Rect | Easy |
| `html-serialize.mjs` | 75 | outerHTML/innerHTML serialize | Easy (pure) |
| `parser.mjs` | 68 | lazy parser binding registry | **Delete** — WASM core owns parsing directly |

**Load-bearing data model (must be preserved in Rust):**
- Immutable SoA parse buffer, shared read-only across many Documents (`parseBufferCached`, unpack-once).
- COW overlay: buffer-backed nodes promote to owned on first mutation; reads identical across the boundary.
- Identity memoization: `nodeAt(idx)` returns the *same* handle (===-stable) — libraries WeakMap nodes.
- `Document.__version` counter: bumped on **every** mutation; the invalidation key for ALL caches
  (query results, children element-filter, getElementsBy*, computed style, synthetic geometry).
- Lazy everything: attrs (`__attrIdx`→build on first touch), listeners (`null`→Map on first add),
  Event `_path`, window globals, customElements, `__mo`, `.sheet`.

---

## 2. Architecture decision (validate before full port)

**One core, two front-ends** (mirrors today's napi+wasm split, extended over the whole runtime):
- **Rust front-end** — `pub` API exposing `Document`/`Node`/`Element` as real Rust types. The in-Rust
  consumer uses this directly: zero boundary, native speed. This front-end is *free* once the core exists.
- **WASM/JS front-end** — wasm-bindgen exports + a thin JS shim reconstructing `{ window, document }` so
  vitest/RTL keep working unchanged. This is where the boundary cost lives and where §0's mitigations apply.

The Rust consumer makes the core's API design the priority (it's the win path); the JS front-end is a
boundary-minimizing adapter over the same core. JS consumers must keep working unchanged:
`import { createEnvironment } from '@miaskiewicz/turbo-dom'` → `{ window, document }`, live + identity-stable.
Three shapes for the **JS front-end**, ordered by recommendation:

### Option A (recommended) — Rust core owns the tree; JS holds **thin handle objects**; fields read through WASM
- Rust holds the SoA buffer + COW tree in linear memory. Each JS `Node` is a tiny class wrapping a
  `u32` handle; getters call exported wasm fns (`tdom_first_child(h)`, `tdom_get_attr(h, nameId)`).
- Identity map lives **in JS** (`Map<u32, Node>`) so `===` and WeakMap keys hold — handle→wrapper memoized,
  exactly like today's `nodeAt`. This is the spec's identity rule, just keyed on a Rust handle.
- **Boundary-cost mitigations (these decide whether A wins or loses):**
  - *Batch the chatty reads*: `querySelectorAll`/`getElementsBy*` return a packed `Uint32Array` of handles
    in one call (not N calls). Selector matching runs entirely in Rust — this is a **net win** vs JS.
  - *Cache hot scalar fields on the JS wrapper*, invalidated by the `__version` counter (also mirrored
    to JS as a single number read per access, or pushed on mutation). `tagName`, `nodeType`, `localName`
    are immutable for buffer-backed nodes → read once, cache forever on the wrapper.
  - *Strings*: return interned string **ids** (u32) across the boundary, resolve to JS strings via a
    JS-side interned table, so repeated `getAttribute('class')` doesn't re-marshal UTF-8.
- **Risk:** event dispatch and `el.style`/dataset proxies are very chatty; may need to stay JS-side
  reading cached fields. Measure early (Phase 1 spike).

### Option B — Rust core, but **eagerly inflate** a JS object tree once per parse (no per-access boundary)
- WASM parses + builds the COW tree, then marshals a full JS object graph across once (like the original
  "full marshaling" the spec started with). After that, the DOM is plain JS — chatty access is free.
- **Loses** the lazy-inflation win (build-what-isn't-touched) — pays for every node up front. Contradicts
  the v0.1.23/47 unpack-once + lazy-attrs wins. Only viable if marshaling proves cheap (it wasn't — that's
  why SoA exists). Effectively reverts to "JS runtime, Rust parser" = today minus the laziness. **Not recommended.**

### Option C — full Rust, JS gets a coarse façade (no live per-node DOM)
- Expose only high-level ops (query, serialize, snapshot) to JS; no live `el.children` mutation chatter.
- **Breaks** RTL/user-event/React, which mutate and re-read fine-grained. Out of scope per the spec's whole
  thesis. **Rejected** unless the consumer is rewritten in Rust too (explicitly out of scope).

**Plan of record: Option A**, gated on a Phase-1 spike proving boundary cost is survivable on the KPI suites.
Option A is the only one of the three that **preserves laziness** (see next section) — B inflates everything
eagerly, C exposes no live DOM. Laziness is why A is recommended, not just boundary cost.

---

## 2.5 Laziness is the load-bearing invariant — DO NOT REGRESS IT (applies to both layers)

The spec's entire thesis (`turbo-dom-spec.md` §0): *"trade eagerness for speed, not correctness — an
interface that's never touched is 100% correct AND free because you didn't run it."* Every speed number we
have (createEnvironment ~1.2M ops/s, suite ~45× happy-dom, free per-file reset) comes from **not building
what the test doesn't touch.** The port must keep this on **both** layers. The easy way to destroy it: have
the WASM front-end eagerly walk the tree and build a JS wrapper per node on parse — that pays for every node
*and* marshals it, i.e. strictly worse than today. **Forbidden.** The rule per layer:

### Rust core — lazy = don't allocate owned state until mutated/queried
The parsed SoA buffer already *is* the tree (read-only, in linear memory) — reading structure allocates
nothing. Lazy in Rust means deferring **owned/derived** state:

| Today (JS) | Rust core mechanism | Built when |
|---|---|---|
| buffer-backed node → owned on first mutation (COW) | node = `enum { Buffer(u32), Owned(Box<OwnedNode>) }`; promote on mutate | first `appendChild`/`setAttribute`/`textContent=` |
| lazy `__attrs` (build on first attr touch) | `attrs: OnceCell<Vec<Attr>>` — buffer-backed nodes store only the SoA attr slice `(start,len)` | first `getAttribute`/`setAttribute` on that node |
| lazy `__listeners` (null→Map) | `listeners: Option<Box<ListenerMap>>` | first `addEventListener` |
| lazy Event `_path` | `path: OnceCell<Vec<u32>>`; built only if a listener is on the dispatch walk | first `composedPath()` / dispatch with listener |
| version-cached children/getElementsBy*/computed-style/geometry | `Cell<Option<(version, Box<[u32]>)>>` per node; recompute on version miss | first access at a given `__version` |
| `getComputedStyle` partial cascade / synthetic geometry | computed lazily, memoized per `__version` | first `getComputedStyle`/`getBoundingClientRect` |
| free reset (drop overlay, keep buffer) | clear the owned-overlay arena + caches; SoA buffer stays shared | per-file `reset()` |

Single-accessor discipline (lesson from 23ab9ea): every lazy field reads through **one** `fn attrs(&self)`
/ `fn listeners(&self)` accessor that builds-if-empty, so no call site can observe a `None`/half-built state.

### JS/WASM front-end — lazy = don't materialize a wrapper or a global until JS touches it
This layer is where laziness is *most* at risk and *most* valuable (it's the boundary):

- **Lazy node wrappers, memoized for identity.** A JS `Node` wrapper is created **only** when JS first
  reaches that node (`document.body`, a query result, `parentNode`), then cached in a `Map<u32, Node>` so
  `===`/WeakMap hold. Never pre-walk the tree to build wrappers. This is today's `nodeAt` memoization,
  re-keyed on the Rust handle — the single most important laziness rule to keep.
- **Lazy `window` stays a JS self-replacing Proxy.** `window` is inherently a JS concept and full of
  host-interop globals — keep it as the JS shim's Proxy (per `window.mjs`): each global is a factory that
  materializes on first `get` and self-replaces. `localStorage`/`IntersectionObserver`/`matchMedia`/`Range`
  never construct unless touched. customElements/origin/`__mo`/`.sheet` stay lazy (v0.1.36/45/46). The Rust
  core does **not** own `window`; it owns the `document` tree.
- **Lazy scalar caching on wrappers, not eager mirroring.** Immutable fields (`tagName`, `nodeType`,
  `localName`) cache on the wrapper on first read; mutable reads re-fetch and self-cache per `__version`.
  Don't eagerly copy a node's fields across at wrapper-creation — defer each to its first read.
- **`el.style`/`dataset`/`classList` proxies stay lazy + memoized** JS-side (built on first access), reading
  cached Rust fields — same as today's `__style`/`__dataset` memo slots.

### Validation hook for the Phase-1 spike (laziness must be *measured*, not assumed)
Carry forward the spec's tracing-Proxy idea: instrument the front-end to count (a) JS wrappers materialized
and (b) Rust owned-promotions + attr/listener builds, per test file. A correct port touches ~the same small
fraction of the tree the JS runtime does today. A blow-up in either count = a laziness regression and a
kill-signal, independent of wall-clock.

---

## 3. Phased migration (each phase killable, mirrors spec §9 + EXPERIMENTS discipline)

### Phase 0 — Scaffolding (no behavior change)
- New crate `turbo-dom-runtime` (or a `runtime` feature/module beside `core.rs`), `wasm-bind` target via
  wasm-bindgen, `wasm32-unknown-unknown`. Reuse `core::Node` from the parser.
- Stand up the SoA buffer + COW tree + identity-handle plumbing in Rust. Export `create_environment(html)`.
- Thin JS shim (`index.mjs`) that calls the WASM exports and reconstructs `{ window, document }` wrappers.
- Keep the existing JS runtime in place behind a flag so we can A/B and diff.

### Phase 1 — Spike: prove the boundary (THE kill/continue gate)
Port the **single chattiest path end-to-end** through Option A and benchmark vs the JS runtime:
- Candidate: `querySelectorAll` + per-result `getAttribute`/`textContent` (RTL's bread and butter), plus
  `dispatchEvent` on a listener-less tree (React fires thousands).
- Run `bench/scorecard.mjs`-equivalent + hot-swap into BOTH KPI suites.
- **Continue** if within ~10% of JS runtime (bulk query wins offset per-node boundary cost) OR clearly faster.
- **Kill/pivot to Option B-eager or stay-JS** if the boundary tax blows past the JS runtime on chatty paths.
  Better to learn this in a 1-week spike than after porting 14 modules.

### Phase 2 — Pure / algorithmic modules (low risk, port cleanly)
Port the modules whose wins are algorithmic, not V8-tricks (see PERF_HISTORY PORTS tags):
`color.mjs`, `html-serialize.mjs`, `buffer.mjs` (already index-based), `selectors.mjs` (parse+AST cache+
alloc-free matchers → Rust slices/&str, no closures), `cssom.mjs`, `cascade.mjs`, `svg.mjs`, `stubs.mjs`.
These are mostly leaf computations; running them in Rust over the in-memory tree is a strict win (no
boundary in the inner loop).

### Phase 3 — The chatty core (`dom.mjs`, `events.mjs`, `collections.mjs`, `window.mjs`)
Only after Phase 1 proves the model. Port with the boundary-mitigation toolkit from §2 and §4. This is
where the risk lives; differential-test hard (jsdom oracle + happy-dom + the RTL gauntlet, all already in
`test/`). Reuse the existing JS test suite as the conformance oracle against the WASM build.

### Phase 4 — Validate + bench + cut over
- All existing tests green against the WASM runtime (port `test/*.mjs` to drive the WASM build, or keep
  them and point `createEnvironment` at WASM).
- `npm run conformance` ≥ 99.5%, zero non-`<select>` failures.
- KPI suites: hold or beat `../ui-design-components` <50s, `../payroll-app` <75s.
- Refresh README/CLAUDE benchmark numbers.

---

## 4. How each perf technique maps to Rust (summary — details per-commit in PERF_HISTORY)

| Technique (JS) | Commits | Rust/WASM port |
|---|---|---|
| SoA flat buffer, index-as-id | b9dad9a, 354b966, 9091fbc | **Native home.** Stays in Rust linear memory; this is what we marshal handles over. |
| Lazy node inflation + identity memoize | f49a4bf | Handle (`u32`) → JS wrapper memoized in a JS `Map`; Rust never allocates a "node object". |
| COW overlay (buffer-backed → owned on mutate) | f49a4bf | Rust enum per node: `Buffer(idx)` vs `Owned{..}`; promote on first mutation. |
| `Document.__version` cache key | ac134db, d2208e1, bb45ea9, 0e246b6 | A single `u32` version counter on the Rust tree; all Rust-side caches key on it; mirror to JS for wrapper-field cache invalidation. |
| Alloc-free matchers (no classList/closures/regex) | 066302f, e1c9170 | **Free in Rust** — iterate `&[u32]` children, `&str` class scan, no heap. Matching moves fully into Rust (net win: no per-node boundary). |
| Result caching (selector,version) | ac134db | `HashMap<(SelId, u32), Box<[u32]>>` in Rust; return packed handle array once. |
| Version-cached children/siblings/clone | bb45ea9, 5335164, 09439a9, 6cd36c2 | Cached `Vec<u32>` invalidated by version; sibling O(n) scan same algorithm. |
| Zero-alloc mutation when no observer | b9b23f6, e2afd29 | Same branch: skip building MutationRecord unless an observer is registered. |
| Lazy attrs / listeners / _path / globals | e3f6c2c, e2afd29, 61c1b8b, f26d5c6 | `Option<...>`/`OnceCell` lazy fields; build on first touch. |
| Monomorphic Element shape | 87278bb | **N/A** — Rust structs are monomorphic by definition; no win, no cost. |
| Read-field-once locals | 3137d91, 3c6629f | **N/A / free** — Rust field reads are free; compiler already does this. |
| FxHashMap intern, reserve, byte-blob strings | 496e13d, 3ed3512, 9091fbc | **ALREADY RUST** in `core.rs` — preserve verbatim; extend interning to the runtime side. |
| Partial getComputedStyle cascade | e20c892, fb3e644, 3cfb602 | Port the cascade algorithm; memoize per version. Pure compute → Rust win. |
| Synthetic geometry (stable, memoized) | 6ce3629 | Port the box-model heuristics; memoize per version (must stay STABLE per DOM state). |
| Injectable clock + virtual scheduler | 6f3a4f2 | Clock fn injected from JS; scheduler must still route through JS `setTimeout`/MessageChannel (host interop) — likely stays partly JS-side. |
| Lazy window Proxy, self-replacing globals | f49a4bf, fbeccb1 | JS `Proxy` is a JS concept; window likely stays a **JS shim** over Rust document. Globals lazy in JS. |

**Two things that must stay JS-side** (boundary/host reality): the `window` Proxy + global self-replacement,
and the scheduler/clock host interop (`setTimeout`/`MessageChannel`/`performance.now`). Everything tree-shaped
and compute-shaped moves to Rust.

---

## 5. Open questions to settle before Phase 2

1. wasm-bindgen handle ergonomics vs raw `u32` + manual exports — measure call overhead both ways in Phase 1.
2. String marshaling: interned-id table on the JS side, or `wasm-bindgen` string returns? (Phase 1 bench.)
3. Do `el.style`/`dataset` proxies stay JS (reading cached Rust fields) or become Rust? (chatty — likely JS.)
4. Test strategy: re-point existing `test/*.mjs` at the WASM `createEnvironment`, or port tests to Rust?
   (Recommend: keep JS tests as the oracle; they already encode every correctness corner.)
5. Build/distribution: one `pkg-web/` WASM artifact replaces the native `.node` for the runtime; the parser's
   existing native addon can stay for parse-only embedders, or fold into the one WASM module.

---

## 6. Deliverables of this planning step
- [x] `RUST_PORT_PERF_HISTORY.md` — per-commit perf optimization ledger with Rust portability tags (mined
      from real diffs by 6 batch agents).
- [x] `RUST_PORT_PLAN.md` — this document.
- [x] `../payroll-app` switched to `staging` + pulled latest — Phase-1 KPI validation target ready.
- [x] Go given — Phase 0+1 building (see §7).

---

## 8. Build status (rtdom — the Rust-native runtime)

Published crate `crates/turbo-dom` (pure Rust, native API, no wasm/napi — its own workspace
member, not an in-parser-crate module). Verified by `cargo test -p turbo-dom`.

| Module | Status | Notes |
|---|---|---|
| `tree.rs` | ✅ | COW tree over `core::Soa`, `version` counter, lazy attrs, reads/mutations, `set_inner_html` (fragment import). |
| `query.rs` | ✅ + pseudo-classes (in progress) | selectors (tag/class/id/attr, descendant/child, comma), version-cached qSA, getElementById/ByTag, `matches`. Pseudo-classes added by agent. |
| `color.rs` | ✅ | named/hex/hsl → rgb()/rgba(), byte-identical to color.mjs. 14 tests. |
| `serialize.rs` | ✅ | inner/outerHTML, void + raw-text, exact escaping. |
| `cascade.rs` | ✅ | partial getComputedStyle (inline + `<style>` rules via `tree.matches`, specificity, inheritance, color canon, px/shorthand). Shadow scoping = TODO. |
| `events.rs` | ✅ | `Dom` (owns Tree + listeners), capture/target/bubble, listener-less skip, stop/preventDefault, once, tree-mutating handlers. |
| `gauntlet.rs` | ✅ | Phase-4 end-to-end: parse→query→cascade→event-mutate→re-query→serialize. |

**Phase mapping vs the original plan:** Phase 0 (scaffold) ✅, Phase 1 (spike → PIVOT) ✅, Phase 2
(pure/algorithmic: color/serialize/cascade/selectors) ✅, Phase 3 (chatty core: tree/events; query) ✅
for the Rust-native API, Phase 4 (gauntlet + native bench) ✅.

**Native throughput (Phase-4 bench, `rtdom::gauntlet::native_workload_throughput`, release, 300-card
chatty workload identical to `bench/spike.mjs`):**

| path | ops/s | vs JS runtime |
|---|---:|---:|
| **rtdom native (Rust, zero boundary)** | **107,387** | **~2.7×** |
| JS runtime (in-process V8) | 39,869 | 1.00× |
| WASM-from-JS DOM, best case | 22,471 | 0.56× |

Native Rust is ~2.7× the JS runtime and ~4.8× the WASM-from-JS path on the same workload — the
dual-runtime thesis, measured: the Rust-consumer path WINS (zero boundary), the JS-loads-WASM path
LOSES. (Cross-runtime comparison — directional signal, not a controlled in-process A/B.)

**Gaps resolved (all four):**
- ✅ **Shadow-DOM scoping** — `tree.rs`: `attach_shadow`/`shadow_root`/`shadow_host`/`shadow_root_of`/
  `descendants`/`assigned_nodes` (encapsulation: light queries skip shadow; named/default slot assignment).
  `cascade.rs`: `:host`/`:host(sel)`, `::slotted(sel)`, shadow-scoped `<style>`, and `flattened_parent`
  hops shadow-root→host so inheritance crosses the boundary.
- ✅ **Attr operators** — `query.rs`: `^=` `$=` `*=` `~=` `|=` plus exact/presence (`AttrOp` enum).
- ✅ **CSSOM / SVG / stubs / NodeRef** — `cssom.rs` (CssStyleSheet/Rule + insert/deleteRule + parse),
  `svg.rs` (SVG length/string/viewBox accessors), `stubs.rs` (Storage, media-query eval, observer entries),
  `node_ref.rs` (ergonomic `NodeRef` + `DocumentExt` façade, descendant-scoped query).
- ✅ **Controlled in-process A/B** — `gauntlet::lazy_vs_eager_ab` (same runtime/harness/fixture, isolates
  the laziness variable): lazy buffer reads **88,651 ops/s** vs eager full-overlay **52,883 ops/s** →
  **lazy 1.68× eager** + 0.38ms up-front inflate avoided. The laziness design wins measurably in-process.

- ✅ **"JS-host" surfaces ported** (they were portable — only async-event delivery + JS-class ctors are
  genuinely JS, and a Rust consumer replaces those with sync/trait APIs): `file.rs` (Blob/File + sync
  FileReader incl. hand-rolled base64), `canvas.rs` (honest 2D-context stub), `custom_elements.rs`
  (name→definition registry), `location.rs` (Location URL-parse + History stack), `mutations.rs`
  (MutationObserver — gated record buffer on `Tree`, sync `take_records`, type/target/subtree/old-value
  filtering; no-observer path stays zero-alloc).
- ✅ **Hotspot harness + optimizations** (`bench.rs`): getComputedStyle memo (283×), serialize (4.3×),
  getElementById cache (~740× repeat). Remaining slowest ops are parse-bound (html5ever).

**100% line coverage on all 15 rtdom modules**, 184 unit tests (+3 ignored benches). The only things
NOT ported are inherently the JS event-loop / JS-class layer (async FileReader event firing, the
whenDefined Promise, microtask MutationObserver delivery) — a Rust consumer doesn't want those.
The JS runtime (`src/runtime/*.mjs`) stays untouched — the path for JS/vitest consumers.

---

## 7. Phase-1 spike — concrete spec (the kill/continue gate)

**Goal:** answer ONE question with a number — *does a Rust-core-in-WASM DOM, accessed from JS, beat (or stay
within ~10% of) the current pure-JS runtime on the chattiest hot paths?* Build the thinnest possible vertical
slice that exercises the boundary honestly. Do **not** port full modules.

### Crate / file layout
- Reuse `src/core.rs` (parser → `core::Node` tree) unchanged.
- New `src/runtime/` Rust module (Rust side), behind a `wasm-runtime` cargo feature so the existing
  `napi-bind`/`wasm-bind` parser builds are untouched:
  - `src/runtime/tree.rs` — owns the document: `core::Node` tree in a flat `Vec<NodeRec>` arena (SoA-style
    columns: `parent/first_child/next_sib/node_type/tag_id` + attr slices + a string-intern table). Index = handle.
  - `src/runtime/query.rs` — a minimal selector matcher (tag, `.class`, `#id`, descendant) running entirely
    Rust-side; `query_selector_all(doc, sel) -> packed Vec<u32>`. Alloc-free inner loop (slices, no per-node heap).
  - `src/runtime/wasm.rs` — `#[wasm_bindgen]` exports: `create(html)->u32 docHandle`, `qsa(doc,sel)->Uint32Array`,
    `get_attr(h, name_id)->i32 strId` (interned-id out, NOT a string), `tag_name(h)->u32 strId`,
    `first_child/next_sib/parent(h)->i32`, `dispatch(h, type_id)->u32` (listener-less path: walk ancestors,
    return path length — exercises the dispatch walk without JS callbacks), `intern_str(s)->u32` + `str_of(id)`.
- JS shim `bench/spike-shim.mjs` — `Map<u32,Node>` identity memo, lazy wrapper creation, JS-side interned
  string table, wrappers cache immutable scalars (`tagName`). Mirrors Option A + §2.5 laziness rules.

### Spike workload (mirror RTL/React chatter)
1. Parse a real fixture (one of the bench fixtures / a payroll-app shell).
2. `qsa('div.card')` → for each result read `getAttribute('class')` + `tagName` + walk `parentNode` to root.
3. `dispatch` a listener-less event on N nodes (React fires thousands).
Compare against `src/runtime/index.mjs` (current JS) doing the identical sequence, best-of-6, observable sink.

### Definition of done (what makes the gate decidable)
- Spike compiles to `wasm32-unknown-unknown`, loads in Node, runs the workload, produces identical results
  to the JS runtime for the same fixture (correctness first).
- A bench script prints: ops/s (Rust+WASM vs JS), **boundary-crossing count** per workload iteration, and the
  laziness counters from §2.5 (wrappers materialized, attr builds) for both impls.
- Written verdict appended here: **CONTINUE** (within ~10% or faster → proceed to Phase 2) or **PIVOT**
  (boundary tax dominates → reconsider Option B-eager / keep JS front-end for the JS consumer, Rust API for
  the Rust consumer only).

### Phase-1 verdict (measured 2026-06-17) — **PIVOT**

Spike: `src/spike.rs` (feature `wasm-runtime`) + `bench/spike.mjs`, 300-card RTL-style fixture, chatty
workload (qsa-once → per-node getAttribute×2 + tagName + parent-walk + listener-less dispatch walk),
best-of-6, observable sink. Results (darwin-arm64, Node 24):

| variant | crossings/node | ops/s | vs JS |
|---|---:|---:|---:|
| **JS runtime** (DOM is JS objects, zero boundary) | 0 | 39,869 | **1.00×** |
| WASM prefetch-record + cache (Option A, best case; 1 crossing/node warm) | 1.0 | 22,471 | 0.56× |
| WASM per-call ids (handles + interned ids) | 9.0 | 13,471 | 0.34× |
| WASM naive (per-call String marshal) | 9.0 | 5,549 | 0.14× |

**Finding:** even at the theoretical floor — whole node record prefetched in ONE crossing, cached on the
wrapper, all reads served from cache, only the single dispatch walk crossing remaining — a WASM-backed DOM
**accessed from JS is ~1.8× slower** than today's pure-JS runtime, and the naive shape is ~7× slower. The
JS↔WASM crossing is a hard floor that zero-crossing in-process JS access always beats. This confirms
`turbo-dom-spec.md` §3 ("WASM is wrong for the runtime") empirically.

**The pivot (architecture of record going forward):**
- **JS consumer (vitest/React/RTL) → KEEP the existing JS runtime (`src/runtime/*.mjs`) unchanged.** It is
  already optimal and has zero boundary. A WASM port is a 1.8–7× regression for this path. Do not port it.
- **Rust consumer (e.g. turbo-crawl in Rust) → build a pure-Rust runtime with a native Rust API.** That path
  has NO boundary — `el.children`/`getAttribute`/`dispatch` are in-process Rust calls, faster than V8. Every
  PERF_HISTORY PORTS technique applies and runs native. This is the real, unambiguous win.
- Shared `core.rs` parser stays as-is (already Rust, already ALREADY-RUST-tagged wins).

So it is a **dual-runtime** (one Rust-native, one JS), NOT a single Rust core behind a WASM front-end for
JS. Phases 2–4 below retarget accordingly: build the Rust-native runtime + its Rust API and a Rust test
gauntlet; the JS runtime is untouched. The "JS loads WASM for the DOM" idea is retired by the data — kept
only as a fallback for a JS environment that has *no* JS runtime option (not our case).
