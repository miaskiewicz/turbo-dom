# Mixed solid-border controls

`mixed_border_chrome159.json` records all 256 RGBA pixels of each 16×16,
DPR-1, offline Chrome 159.0.8154.6 screenshot. Regenerate it with
`scripts/mixed-border-chrome-control.mjs`, setting `PLAYWRIGHT_CORE` and
`CHROME_EXECUTABLE` to local installations. No browser runs in the Rust tests.

Square sides use Blink's solid-side order (top, bottom, right, left) within
increasing-opacity groups; the sides paint opaquely and opacity is applied to
nested groups. See the primary [BoxBorderPainter source](https://raw.githubusercontent.com/chromium/chromium/159.0.8154.6/third_party/blink/renderer/core/paint/box_border_painter.cc),
`ComplexBorderInfo`, `PaintOpacityGroup`, and `ComputeMiter`. This avoids applying
translucency twice at a join. Rounded joins accumulate disjoint side coverage
before one ring mask and one source-over operation.

Integer square controls compare every channel of every pixel with a budget of two intensity levels per
channel. Measured differences are at most one intensity level, except the unequal-width
control, which reaches two intensity levels. The native painter uses floating-point coverage,
premultiplied arithmetic and rounding once; Skia also has integer-alpha and
fixed-coverage paths ([SkColorPriv](https://api.skia.org/SkColorPriv_8h_source.html)).
For example, the native common-half red side is `[255,127,127,255]`, while
Chrome's opacity-group path gives `[255,126,126,255]`. The existing component
comparison budget of 16 and the existing exact uniform/color controls are unchanged.

Fractional quads and curved rings have a separate, explicit coverage comparison
mode. They are not claimed pixel-identical to Skia. The fixture retains their
complete Chrome pixels, and the tests check all 256 native premultiplied pixels:
applying half opacity must scale the opaque geometry once (within one intensity level),
never produce alpha above 128, and preserve independently captured interior side
colors (within two intensity levels). The old painter fails these opacity invariants for
both the fractional and rounded controls. This is a semantic regression test,
not a relaxed Chrome pixel comparison.

The fixture's independent `quadDiagnostic` isolates antialiasing from DOM,
layout, border painting and opacity groups. For the standalone SVG polygon
`M0 0 L2 2 L2 6 L0 8 Z`, translated by `(0.25,0.5)`, Chrome reports coverage
`72/255` at pixel `(2,2)`. Exact polygon intersection is `5/32`: a `1/4 × 1/2`
rectangle plus a triangle of side `1/4`. The native area test asserts `5/32`.
The full fractional mixed-border capture consequently differs from Chrome by up
to 16 intensity levels on antialiased joins. The rounded mixed capture differs by up to 32 intensity levels
on inner-curve antialiasing; even Chrome's uniform and mixed rounded paths differ
at that curve because one uses a direct ring draw and the other opacity-group
clipping. The separately captured unchanged uniform rounded control differs
from native by up to 10 there. These diagnostic differences are retained and
reported, rather than relabelled exact parity or encoded as color corrections.
