# Native canvas and real-player integration (#523)

The primary Surf NAPI player and the local source-consumer MCP capture/restore
path have been validated. The consumer changes are not published; full-workspace
and final CI validation remain pending. The integration is tracked under
[turbo-surf#527](https://github.com/miaskiewicz/turbo-surf/issues/527). Constructor
support alone is not the acceptance criterion for either path.

## Historical 8c primary-path proof — 2026-11-18

The unchanged local fixture ran the real dotLottie 0.70.0 JavaScript and WASM in a
rebuilt Surf native NAPI addon against turbo-dom
`8c7f96adf8c55fdabad9b9c0d02fba0e0ff8af05`, with Chromium 156.0.8078.47 as the
independent browser control. Both reached `ready` through a `load` event at frame
30, reported 544,214 nontransparent canvas pixels and had no page errors. The PNG
comparison had zero changed pixels, including at zero color tolerance. The release
owner visually inspected the native result.

Retained artifacts:

- [Proof and source identity](../tools/component-parity/evidence/lottie-native-v0.8.5/proof.json)
- [Native PNG](../tools/component-parity/evidence/lottie-native-v0.8.5/native.png)
- [Chromium PNG](../tools/component-parity/evidence/lottie-native-v0.8.5/chromium.png)

The addon SHA-256 is
`4d0c17d0090bf071d835d899d97480f350c930b4044726b21e80abf65f193b1a`.
The Surf base was `ac0d81fbb8984cb97d836a6caddd1a11dca9801c`; the host changes
used for this integration were uncommitted. The receipt retains 35 hashes of the
harness, fixtures, assets and vendor inputs rather than presenting that checkout
as a published Surf build. Key unchanged asset SHA-256 values are:

- JavaScript: `9a35aed7b41fd97bbdcd1b1a2c9d9d1fbd4150ecfb8a807ba96b9f54ca0a31da`
- WASM: `3169df80d43187939d7d7ab81d27d0b6843694687f8e960e94606986544c7bcf`
- Animation: `51c691ee8cd7e003e0bfe520aaedfbb8e223c5e03495480a97f6e697057d3937`

This supersedes the earlier Chromium-only baseline and the old pre-fix-addon
handoff. It proves this primary native player fixture at the recorded revision,
not arbitrary animations or a fresh rerun of every later release change.

## Canonical host canvas access

Embedders obtain `canvasHost` from the document's private `get_interface_record`.
It is neither a guest global nor a document property. The five integrated methods
are:

| Method | Canonical result |
| --- | --- |
| `readContext(canvas)` | The canvas's actual active 2D context. |
| `readCanvas(context)` | The actual owning canvas. |
| `snapshotJSON(context)` | A nondraining JSON snapshot of the canonical drawing/upload records. |
| `size(canvas)` | The canonical native bitmap dimensions. |
| `restoreJSON(canvas, text)` | Restore one validated full RGBA8/sRGB bitmap seed and return the canonical 2D context. |

Unknown or foreign-document objects return `null`. These reads ignore overwritten
`__ctx2d`, `_ops`, `canvas`, `width` and `height` diagnostic properties and do not
invoke guest serialization hooks. Hosts should use this seam instead of treating
those mutable JavaScript properties as authoritative. Authored JavaScript
method/property overrides remain observable to page code.

`restoreJSON` was integrated in turbo-dom
`019dd016aef1283a5a5b578d02fc1bc981c1eaae`. It accepts exactly one unpositioned
`putImageData` operation covering the entire current native bitmap. The image must
have exactly `width`, `height`, `data`, `colorSpace: "srgb"` and
`pixelFormat: "rgba-unorm8"`; dimensions and dirty bounds must match the canvas,
and `data` must contain exactly width×height×4 integer bytes in 0–255. Positioned
or partial uploads, mixed draw logs and other formats are rejected. Hosts must
flatten earlier drawing into this seed first: the API is not a generic draw-log
replayer.

Validation and pixel allocation finish before commit. A rejected seed or locked
existing log preserves the previous context, pixels and draw records. A successful
restore preserves an existing 2D context's identity, but resets drawing defaults,
paths and the save stack before installing the seed; it transfers neither previous
transforms nor JavaScript state. With no existing 2D context, the prepared context
is published only after validation. Zero-area or over-budget seeds are rejected;
the limits are 268,435,456 pixels and 268,435,456 JSON code units. Unknown or
foreign-document canvases return `null`.

Canvas `width` and `height` reflect null-namespace content attributes, defaulting
to 300×150. Setting either dimension, including its current value, or changing or
removing an existing dimension attribute resets the drawing state, path log and
bitmap. Missing-attribute removals and unrelated namespaces preserve them. Native
attribute and Attr-node routes notify the private reset callback synchronously.

RGBA8/sRGB uploads retain immutable pixel data and dirty rectangles for native
replay. Private readback storage is bounded by bitmap area, including updates
after host upload-log drains. Display-P3/float16 ImageData construction and
inspection are supported; unsupported bare-binding canvas color/pixel-format
conversions throw explicitly rather than silently approximating output.

## Local MCP validation and current acceptance

The integrated restore API passed 822 binding tests and strict checks. The Surf
source-consumer validation reported nine MCP tests passing, including four
previously failing regression cases, plus 39 focused tests, 834 shim tests and
55 end-to-end tests. These are local integration results, not a published Surf
release or a completed full-workspace gate.

A rebuilt real native player run against
`019dd016aef1283a5a5b578d02fc1bc981c1eaae` again reached `ready` through `load`
at frame 30 with no page errors and 544,214 nontransparent canvas pixels. Its
screenshot had zero changed pixels across 810,000 compared pixels. This later
result is separate from the historical 8c artifacts above; its committed consumer
receipt is to be added after the Surf source changes are committed. Neither a
nonempty fallback PNG nor constructor existence was used as acceptance.

Surf's `019dd016` source-consumer validation also has green full Clippy. At the
reported whole-workspace checkpoint, 1,409 cases had passed while render binaries
were still running (160/303 reported). The workspace result remains incomplete.
The earlier `e0a81a80` turbo-dom CI checkpoint had 21 green checks, including all
coverage jobs, and five native jobs pending; it predates the newly integrated
`2ba05e87` typed-edge source, which still needs CI. Local strict/no-default checks
pass; a full all-feature rerun with host GPU access remains pending after sandbox
Metal was unavailable. These facts do not establish final release readiness.

The admitted [combined typed-edge report](benchmarks/cascade-v085-bcfce3b4.md)
measured modest report/card/article improvements against v0.8.4, effectively flat
healthy flex and a small grid result. All 15 benchmark output hashes match
v0.8.4; the 24 component PNG/geometry comparisons preserve the corrected
pre-optimization rendering. The failed [e151 checkpoint](benchmarks/cascade-v085-e1516f2a.md)
and [standalone 8234 experiment](benchmarks/cascade-v085-8234d47f.md) remain
recorded. The standalone experiment was not accepted by itself; its borrow/guard
ideas joined the combined bundle admitted as `2ba05e87`. Cross-session results
cannot isolate the gains of individual changes, and no broad speedup is claimed.
These results do not establish that every final check has passed. No PR, tag or
publication is claimed here.
