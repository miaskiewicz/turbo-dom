# Canonical SVG dash-array and legacy vendor effects

The canonical animation value model now samples `stroke-dasharray` as nonnegative
length lists, rather than choosing a discrete endpoint or returning an empty effect.
Unitless SVG numbers become px; absolute units normalize through the existing length
parser. Repeatable lists expand to their least common multiple, then use the existing
canonical interpolation/serialization path. Equal percentage units are supported.
Mixed percentage/length components remain explicitly unsupported; the host sampler
returns Unknown before composition. Dash arrays are non-additive: their ordinary
canonical add/accumulate result replaces the underlying list, matching the oracle.
The existing conservative inherited/base-dependent host guard remains in place.

Before expansion, LCM work must fit the sum of the two original canonical source
UTF16 lengths. The division check runs before multiplication or allocation. Coprime
lists that would grow quadratically return Unknown from host preflight; an ordinary
computed-style composition throws a clear RangeError. This introduces a conservative
source-linear bound and does not raise any existing budget.

Lightning CSS admits legacy `-ms-` and `-o-` property names which Chromium ignores.
Canonical property admission now rejects those names together with the existing
`-moz-` rejection, case-insensitively. This fixes declarations, keyframe reflection
and the engine supports seam at their common owner. The sampler's completeness check
is unchanged. Webkit and custom-property controls remain positive; genuinely
unmodeled `float` remains Unknown.

## Reproduction and scoped checks

The baseline host run with the new controls and original production sources had
21 passing / 6 failing tests. Actual captured rotation/ripple sources were Unknown
because their canonical CSSOM retained `-ms-transform`; the circle was Unknown for
`stroke-dasharray`. A separate intermediate control proved `-o-transform` was
incorrectly admitted as supported. The exact immutable Chrome 159.0.8154.6 input
HTML, source hashes and 15 observations are preserved in
`crates/turbo-dom-browser-env/tests/fixtures/captured_mui_animation_chrome159.json`.
The vendor fixture preserves the source-hashed Chromium supports/CSSOM oracle.

Focused validation:

- `presented_effects`: 29 passing, including all 15 captured sample points and a
  three-case control with no pause, seek, finish or effect modification.
- Existing canonical animation unit controls: 19 passing.
- Existing CSSOM parser unit controls: 9 passing.
- Existing animation-presentation / lifecycle / shadow / cross-isolate snapshot
  controls: 5 / 12 / 6 / 1 passing.
- Strict browser-env Clippy over all targets with host-internals; fmt / JS syntax /
  whitespace checks. Changed functions are at most CC9; pre-existing binding-only
  complexity diagnostics are recorded separately and unchanged.

Commands use Rust 1.98.0, the pinned V8 archive, offline/locked dependencies and one
build job. Baseline/green logs and additional actual Chromium unitless/list/percentage
controls are durable under `.evidence/svg-dasharray/` in the implementation worktree.
This is scoped producer validation; final combined workspace/coverage release gates
and actual Surf consumer acceptance are separate receipts.
