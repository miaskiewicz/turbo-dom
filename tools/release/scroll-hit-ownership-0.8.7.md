# Scroll and hit-clip ownership, v0.8.7 (#550, #551)

## Separate causal defects

At release `0823d5c756293d07309f8a0b6fcd7182faf3b406`, BODY height 40px with
`overflow:hidden` and 130px of normal-flow content incorrectly propagated its
overflow to the viewport even when HTML or BODY had `contain:paint`. The public
live-snapshot regression failed in `scroll::propagated_body`; Chromium 159 retained
BODY's independent 90px range. The fix checkpoint is
`e5f3e4b2918f0e9904038a262256ac6173bd74bb` (#550).

A separate public `hit::elements_at_point` regression failed after that fix:
a control below a propagated BODY and a viewport-fixed descendant escaping an
ordinary overflow clip both had correct public geometry, but were absent from
the hit results. Neither reproduction uses a Surf click helper, retargeting,
application code or live services (#551).

## Ownership changes

BODY propagation requires a direct HTML document parent, visible root overflow in
both computed axes, and displayed root/BODY elements without active containment.
Containment includes the relevant `contain` keywords, content visibility and
size query containers. Root nonvisible axes retain BODY's own scrolling; body
`overflow:clip` remains unscrollable. Hidden own boxes are not element scrollers.

Hit overflow clips now follow the native source/containing-block chain. Overflow
propagated from BODY and root overflow belong to the viewport, which retains its
actual boundary. Fixed/absolute descendants may escape intermediate clips whose
containing blocks lie outside those clips. Clips within their positioned subtree
and clips from real containing blocks still apply. Existing `clip:rect()` handling
and explicit asymmetric-clip errors remain on their ordinary paths.

The two containment decisions deliberately differ: Chromium's size query
containers disable BODY propagation, but do **not** establish fixed containing
blocks. A causal counterexample failed the initial hit candidate before this
was corrected. Paint/layout/content/strict containment and active content
visibility retain their real fixed-containing-block clips. Native box provenance
distinguishes ordinary inline ancestors from atomic/blockified/flex-item boxes;
missing foreign containment provenance remains an error. Paint containment keeps
its own padding-box clip even when its overflow is visible. This latter boundary
was independently red on a blockified inline before the final refinement.

## Independent controls

`body_overflow_propagation_probe.js` and its source-hashed Chromium
`body_overflow_propagation_chrome159.json` record 57 independent cases: both
root/BODY containment, combined query-container values, inactive values,
display:none/contents, root axes, BODY clip, quirks and nested frame documents.
The producer replays their HTML/CSS through its public native layout/CSSOM seams,
including scroll ranges, exposure and source scroll translation. Probe helper
decomposition preserved all 57 recorded results exactly.

`hit_clip_ownership_probe.js` and its source-hashed
`hit_clip_ownership_chrome159.json` record 35 cases. Native hit tests cover the
34 supported cases, including propagated/independent/scrolled BODY, fixed and
absolute escapes, nested real clips, contents, inline/blockified/flex-item boundaries and the containment counterexample.
Positive hit controls also compare public geometry. The transformed-fixed case
remains explicitly unsupported; the existing public snapshot regression
`transformed_fixed_admission_is_identity_independent_and_shared_by_both_entry_points`
retains that admission error. This work does not claim transformed-fixed layout
support or an eight-case consumer click success.

Both portable collectors use an independent ephemeral localhost oracle:

```sh
node scripts/body-overflow-propagation-chrome-control.mjs --chrome /path/to/chrome
node scripts/hit-clip-ownership-chrome-control.mjs --chrome /path/to/chrome
cargo test -p turbo-dom-layout --lib
```

The recorded browser is Chromium 159.0.8154.6; this is not a new Chromium 156
claim. Producer regression logs and exact original public Surf receipt identities
are retained under `.evidence/{body-overflow-containment,hit-clip-ownership}/`.
No frontend/backend, fixture, artifact or dependency pin in Surf was changed.
Surf owns coherent repinning, rebuilding the actual addon and consumer retests.

The standards basis is [CSS Overflow viewport propagation](https://www.w3.org/TR/css-overflow/#overflow-propagation)
and [CSS Containment](https://www.w3.org/TR/css-contain-2/#contain-property).

## Validation

Final implementation: `f51031a3675125b2a182696ca61e96fe6ab18223` (includes
#550's separate checkpoint and its final control refinement). The frozen graph
passed 3,209 workspace tests with zero failures (13 existing ignored), and all
1,299 all-features layout tests with zero failures/ignores. The default layout
library passed 530 tests. Strict all-target workspace and all-features layout
clippy, formatting, Rust/JS complexity (maximum nine), version and single-V8
checks passed. Public transformed-fixed rejection and asymmetric-clip error
controls remain green. No coverage floor, execution/resource budget or assertion
was relaxed.
Default LLVM line coverage passed at 99.86% (19,710/19,737), above the unchanged
99.8% floor.
All-features LLVM line coverage passed at 99.85% (19,889/19,919), also above
99.8%. Tarpaulin uses the repository's crate-qualified feature names; an initial
unqualified metadata invocation failed before tests and was corrected without
changing feature coverage or limits. Negative contained fixed controls establish
clip rejection; their fixed-position placement is a separate contract.
