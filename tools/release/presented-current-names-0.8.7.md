# Current CSS animation names and definition lookup

This candidate adds required `PresentedTarget.css_animation_names: Vec<String>` from
the same canonical computed snapshot used to sample values. It preserves source order
and duplicates, omits only the unquoted none keyword, and retains quoted none as a name.
Historical canceled/restarted lifecycle entries remain unchanged. Missing wire data
returns unavailable; no serde default or idle filtering reconstructs the current list.

CSS declaration lists and keyframe definitions now share cssparser token decoding.
Escaped spaces/commas, quoted strings and case-sensitive names resolve consistently.
CSSOM keyframe serialization escapes decoded identifier names. An exact-case missing
name can be proven inactive only with a complete private stylesheet scope and native
ordinary owner path; imports/container/unknown groups and missing hooks still refuse.
Unresolved env/var-like names and non-scalar bridge inputs preserve cannot-prove errors.
String allocation failure uses the existing DOM RangeError path instead of becoming
an unquoted-none marker.

Causal producer baseline:44 passing/3 failing host controls (quoted lookup, unchanged
unused-definition inputs, and unresolved-name error preservation). The additive field
also had a compile-red external API check. Focused final host controls:53 passing,
0 failing/ignored, including all prior auth/poison/owner guards, 11 actual Chrome159
name cases, all10 CSS inputs extracted from the unchanged source-hashed resource probe,
exact clipped spinner cancel/restart/ordinary-GCS inventory, and the seven retained
restart/reorder/duplicate/script-cancel states. A missing-field control refuses the
wire payload. No application geometry result is inferred from producer admission tests.

Evidence is durable under `.evidence/keyframe-names/`; frozen oracle inputs and source
hashes are committed under browser-env tests/fixtures. This immutable local candidate
is ready for consumer compilation. Scoped strict/CSSOM regression checks and combined
release coverage/workspace gates have separate receipts; they were not prerequisites
for the explicitly requested private candidate handoff.
