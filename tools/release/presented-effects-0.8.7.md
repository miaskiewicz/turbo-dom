# Presented-effects integration candidate — v0.8.7 / #555

This is a local producer integration checkpoint for an immutable private Surf test
build. It is **not a validated release handoff**: complete workspace, strict gates,
coverage floors and consumer regressions remain required before release integration.
The final Surf handoff batches validated animation, selector-projection and SVG
paint changes in one coherent producer SHA; npm-runtime work is independent.

The API reuses canonical timing/easing/composition and existing native pre-matched
cascade inputs, with explicit Unknown rather than underlying-style guesses. See
[the host contract](../../crates/turbo-dom-browser-env/docs/presented-effects.md).

Focused TDD receipts are in `.evidence/presented-effects/`: enum-only cascade
baseline3failed/1passed →4passed; public host sampler baselineunavailable →15passed,
then new SVG dash-offset and custom-name interpolation controls reproduce failures
and require the canonical typed/discrete corrections. Actual Chromium159.0.8154.6
source-hashed offline oracles verify CSS vs script custom reflection, dash-offset
length units and registration scope. Initial public-reflection test construction
was corrected only after that independent oracle proved CSS custom omission.

Private consumer baseline23cases3passed/20failed improved to22passed/1SVGstroke
unsupported before the latest canonical length fix. Consumer results on the final
source and all other guards are still required; no application success is claimed.
Underlying-dependent/cross-origin/context and unsupported-value certification is
still being audited before shipping. The local checkpoint is not permission to
weaken any consumer refusal, resource limit, coverage floor or assertion.

The simultaneous-isolate consumer control exposed a real ownership-index defect:
a thread-local registry attempted to localize another isolate's weak handle.
The producer regression reproduced an abort before the fix. The registry now
belongs to an isolate slot and retains only weak page references; neither a
thread reset nor another isolate can clear or traverse it. All 18 public host
controls pass, including same-thread nested isolates, cancellation after restoring
the outer isolate, replacement-isolate teardown and existing foreign/adoption
refusals. These focused results do not replace the remaining release gates.

A subsequent actual Chromium159 oracle and producer RED prove a context-dependent
base error: parent font10→30px at midpoint plus child calc(10em * sibling-index())
and additive width10→20px produced a false Certified115px instead of Chrome215px.
Sampling now prepares the complete expanded target set before emitting rows.
Base-dependent entries refuse changed inherited/font/SVG-paint/custom context,
multiple contributing targets, and independently composed transition bases.
Complete absolute frames and standalone additive width stacks keep their existing
certification. All19 host tests pass; the new context case remains explicit Unknown.
