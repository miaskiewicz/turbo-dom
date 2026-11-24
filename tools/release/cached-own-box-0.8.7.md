# Cache-only own-box admission, v0.8.7 (#549)

## Contract

With `turbo-dom-browser-env/host-internals`, the public Rust API is:

```rust
host_internals::cached_own_box(scope, element) -> host_internals::CachedOwnBox
// CachedOwnBox::{Unknown, DisplayNone}
```

`DisplayNone` proves only an element's own invariant cached `display:none`.
Embedders may return empty bounding/client rectangles before their normal visual
geometry capture. `Unknown` preserves that path and its errors. Do not use this
proof for used-size, client, offset, scroll or viewport metrics. Consume it
synchronously without guest reentry; both the isolate and any attached context
queue must use explicit microtask checkpoints.

The probe enters the genuine wrapper's active native owner and reads a private
stable cache record. It performs no cascade, media evaluation, lifecycle
inspection, layout/provider call or guest getter. Cold/stale/volatile/animated,
unsupported or uncertain entries return `Unknown`, as do detached/non-HTML,
inert/adopted/frame/shadow ownership, contents and ancestor-only suppression.
Independent active realms are entered and the caller's document is restored.

## Why cached display alone is insufficient

An unsupported container rule can reveal an element while an ordinary snapshot
still contains `none`. Capability metadata therefore examines all potentially
applicable declarations during normal cache creation/revalidation, including
inactive media/supports branches. Only literal `none` can preserve the proof;
`all`, other display values, animation/transition declarations, uncertain groups,
opaque sheets and swallowed cascade/provider failures reject it. Necessary native
subject-type and class/id keys may prove a rule irrelevant. They are conservative
candidate filters, not selector matches or a tag exemption. Inline, base-cache
reuse and style-sharing paths retain the same admission checks. Metadata is
feature-gated; the default binding does not perform this added analysis.

## Regression evidence

TDD controls first failed for missing warm-cache admission, the conservative
subject-type helper, and swallowed provider errors. Nine native API contracts
cover warm/cold/stale records, mutation/CSSOM/base reuse, inactive branches,
container/starting-style uncertainty, explicit block/contents/important values,
animations, brands, owner/world restoration and explicit microtask policy.
Throwing geometry/media providers and poisoned guest accessors prove warm probes
invoke neither; a direct provider control confirms the trap is active.
Public integration tests also cover snapshot rehydration.

The source-hashed `cached_own_box_probe.js` and
`cached_own_box_chrome159.json` compare sixteen offline geometry cases with
Chromium 159.0.8154.6. Every admitted case has computed display `none`, zero
bounding dimensions and an empty client-rect list. This is a new Chromium 159
control, not a Chromium 156 claim. Recreate it with
`scripts/cached-own-box-chrome-control.mjs`; consumers can access both files through
`test_support::{CACHED_OWN_BOX_PROBE_JS,CACHED_OWN_BOX_CHROME159_JSON}`.

The public `cached_own_box_capture` example replays captured HTML/CSS offline,
without application scripts or network services. After ordinary position reads,
both native settings captures admit 334/334 STYLE elements; the Chromium-derived
captures admit 575/575 and 558/558. All cold probes return `Unknown`. These are
eligibility controls, not an actual Surf timing or application-success claim.

```sh
cargo run -p turbo-dom-browser-env --features host-internals \
  --example cached_own_box_capture -- /absolute/path/to/capture.json
cargo test -p turbo-dom-browser-env --features host-internals,test-support
cargo test -p turbo-dom --lib ordinary_subject_type
```

Private application captures and complete red/green/gate logs remain in the
repository-local `.evidence/cached-no-box-plan/`; no application payload is
published with this receipt. Surf owns consumer wiring, coherent dependency
repinning and rebuilding/retesting its actual addon.

## Release gates

The full workspace with `host-internals,test-support` passed 3,201 tests, zero
failures (13 existing ignored). Default-feature binding compatibility passed
838 tests, zero failures (four existing ignored). A final Rust-only helper
extraction passed strict all-target workspace clippy and all nine focused
contracts plus both external API/snapshot tests again. Formatting, Rust/JS
complexity limits, version synchronization, the single-V8 graph, churn and sibling
allocation controls passed. Core LLVM line coverage is 100% (13,447/13,447).
