# Canonical symbolic length-percentage effects (#563)

The captured auth label transitions use `max-width: calc(100% - 24px)` to
`calc(133% - 32px)`; the legend uses `.01px` to `100%`. Their real color and
matrix transitions were already supported, but these length-percentage values
made the entire canonical target Unknown.

The private `animationLengthPercentage` bridge reuses the existing CSS math
parser, with no font or containing-block basis. It exposes only finite px and
percentage coefficients in a null-prototype record. The animation model keeps
those coefficients symbolic through interpolation, additive composition, and
normal `calc()` serialization. There is one canonical interpolation path for
ordinary computed style and the host snapshot; Surf needs no second parser.

`auto`, `none`, nonlinear functions, unresolved font-relative values, and
reference-dependent transforms remain unproved. No layout/provider call is
introduced. Current native owner, clock, effect, and cascade guards are unchanged.

## Evidence

The minimized public fixture is
`crates/turbo-dom-browser-env/tests/fixtures/auth_label_transitions.html`, SHA256
`3dacc3c1ce2fa21e0de817253454a3732d25c631611770ea54346b07c15ec23b`.
Its source-hashed Chrome 159.0.8154.6 oracle is adjacent as
`auth_label_transitions_chrome159.json`, with five samples at 0/50/100/150/200ms.
The fixture retains the captured easing, duration, delay, color, and matrix values.
The producer host test failed before the fix (0/1) and passes afterward.

Focused host controls: 62 passed, no failures/ignores. They include unmodified
transitions, Chrome coefficient comparisons, additive WAAPI plus ordinary
computed style, nonlinear/nonfinite rejection, null-prototype metadata, and
provider-not-called checks. The proposed WAAPI `max-width:none` negative was
discarded because existing keyframe admission drops it; the genuine canonical
CSS `none` to `100px` transition remains Unknown and is tested instead.

Local causal and oracle logs are under `.evidence/transitions/`; no app services,
fixtures, budgets, assertions, or Surf source were changed.
