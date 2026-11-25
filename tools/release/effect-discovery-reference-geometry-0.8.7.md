# Effect discovery and reference geometry — #577

Issue: https://github.com/miaskiewicz/turbo-dom/issues/577

Public `document.getAnimations()` and `element.getAnimations()` flushed ordinary
computed style, including used transform normalization. A paused opacity animation
on a 100×20 element with `translateX(50%)` therefore attempted the used-size provider
**twice during discovery**. A none/absolute-matrix inverse made zero attempts.
Later effect-model inspection made two additional attempts; those are separate.

The public control was RED in all four document/element × returning/throwing
provider cases. It now makes **zero discovery provider calls**, preserving the
same animation, frames, paused state and time. Explicit `getComputedStyle().transform`
still consults the real current box: width 100→200 yields translation 50→100.

The fix adds a private composed-lifecycle mode to the existing weak-capability
input batch. It composes canonical effects and inheritance while skipping only
used-value normalization; underlying host snapshots remain a separate mode. Lazy
parent defaults propagate primitive mode provenance into fresh private maps.
Roots remain fresh, volatile ancestors unshared, validation mode-keyed, and normal
cross-call caches unmodified. Public flush preserves the caller's microtask policy,
holds one canonical clock, restores outer availability state and propagates original
exceptions. Existing termination recovery clears its reused inspection state.

Validation (Rust 1.98, one job): **13/13** provider/lifecycle controls,
**98/98** presented-effects controls and **880/880** binding unit tests pass
(four existing ignored). Strict all-target Clippy (-Dwarnings), fmt, syntax and
scoped changed JS/Rust CC≤9 pass. Frozen Chrome 156 animation assertions are
unchanged. Controls cover composed font/weight/custom tokens/lazy defaults,
underlying snapshots before/after, public computed-style basis, Auto/Explicit
microtasks, one clock, nested refusal, original errors and native termination
recovery, quoted names, shadow own-display suppression and private iframe media.
The standalone bridged iframe document is not a genuine native Document; its
public brand refusal/detached-element behavior remains unchanged.

Chromium 159.0.8154.6 oracle files under `tests/fixtures/` of browser-env:

| File | SHA-256 |
| --- | --- |
| `reference_discovery_oracle.mjs` | `ff0f4dd67802b8f17558e7c1024393864450c8fa6d3fbf3ca0e7fe1a28c04da4` |
| `reference_discovery_chrome159.json` | `31c68db1454893ac8d9524e9002cda76b9c0c9f1bdd06777b1dcb372a8fbc1b9` |

A separate baseline defect is retained: CSS animation under a hidden closed-shadow
HOST remains discoverable on unmodified ea299, versus zero in Chrome. A hidden
shadow element's own display is handled; script WAAPI effects under a hidden host
legitimately remain discoverable in Chrome. This patch does not fix or waive the
host-suppression defect. The independent baseline source/log is retained in
`.worktrees/discovery-shadow-baseline-087/`; #577 raw receipts are in
`.worktrees/discovery-reference-transform-087/.evidence/reference-discovery/`.

Named-profile ancestry placed used-size BEFORE sampler entry during public style
flushing. This public reduction proves unnecessary geometry work, not resolution
of the real application's modal waits or an optimized live timing guarantee.
