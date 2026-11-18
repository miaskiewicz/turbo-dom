# Native canvas and real-player integration (#523)

The primary Surf NAPI path has been verified with the actual dotLottie player.
The secondary MCP capture/restore path remains separate work under
[turbo-surf#527](https://github.com/miaskiewicz/turbo-surf/issues/527). Constructor
support alone is not the acceptance criterion for either path.

## Pinned primary-path proof — 2026-11-18

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
It is neither a guest global nor a document property. The four integrated methods
are:

| Method | Canonical result |
| --- | --- |
| `readContext(canvas)` | The canvas's actual active 2D context. |
| `readCanvas(context)` | The actual owning canvas. |
| `snapshotJSON(context)` | A nondraining JSON snapshot of the canonical drawing/upload records. |
| `size(canvas)` | The canonical native bitmap dimensions. |

Unknown or foreign-document objects return `null`. These reads ignore overwritten
`__ctx2d`, `_ops`, `canvas`, `width` and `height` diagnostic properties and do not
invoke guest serialization hooks. Hosts should use this seam instead of treating
those mutable JavaScript properties as authoritative. Authored JavaScript
method/property overrides remain observable to page code.

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

## Remaining MCP handoff and final validation

A small host-private pure-RGBA restore API is still pending for the secondary Surf
MCP capture path. Its implementation and the consumer integration tracked in
[turbo-surf#527](https://github.com/miaskiewicz/turbo-surf/issues/527) are not
established by the primary NAPI proof above. Do not use a guest-visible shim or
assume that rerunning a stale addon validates new dependency pins.

The Surf integration owner should repin the coherent turbo-dom crate set, retain
one turbo-dom-v8 dependency, rebuild its native host and run the existing local
real-player harness through the actual consumer path. Retain load events, frame
30, canonical upload dimensions/data, nontransparent alpha counts, errors and
native/Chromium PNGs. A nonempty fallback image or constructor existence is
insufficient. The primary proof's expected alpha count is 544,214.

The final combined gates, 24-comparison component audit and paired performance
run are still pending for the runtime pipeline through `e1516f2a`. The earlier
`5ed0ed7d` CI coverage receipt required fixes and is not a final-green release
receipt. No PR, tag or publication is claimed here.
