# Presented effects for a native layout host

This host-internals API samples the binding's canonical CSS/Web Animations records.
It does not add an interpolator to a consumer. Call `sample_presented_effects` with
an exact native browsing Document and native CSS-candidate handles. The batch also
discovers canonical script targets without an animation-name. An unavailable sample
is `None`; unsupported per-target presentation is `PresentedTargetState::Unknown`.
An empty Unknown property list is not an inactive-effect certificate.

The snapshot owns native target identities, lifecycle identities, one pinned host
clock sample and separate `animation`/`transition` property pairs. Construct native
`Declaration` records directly from those pairs, with `important:false`, and use
`CascadeOrigin::Animation` / `CascadeOrigin::Transition` in existing `OriginRules`.
Do not serialize pairs back into CSS declaration text. Native cascade resolution
must run before child inheritance and variable substitution; post-cascade overlays
are insufficient. Animation is below important declarations; Transition is above
them, including UA important. UA paragraph provenance remains distinct.

Sample before accepting geometry-cache hits: effect methods need not change the
native tree version. Contributing effects initially prevent geometry reuse unless
the host already has an independent complete query proof. Retain all ordinary
capture/owner/cascade/composition/resource/admission guards. Data is for one
synchronous capture, not a reusable permission after guest reentry or clock/world
changes. Both isolate and separately installed context queues require explicit
microtask checkpoints. No frame advancement, guest getter, used-size/transform
provider recursion, finish/pause/cancel API or event dispatch runs in the host seam.
Ordinary canonical CSS input synchronization remains part of style resolution.

Certification is conservative: unsupported value/basis/timeline/composition,
unmodeled source declarations, potential custom-property registration and repaired
non-scalar payloads preserve strict refusal. Foreign-source effects and adopted
targets that this realm's canonical stack cannot certify return unavailable. A
weak ownership index adds no strong guest/context roots. Independent media, touch,
computed-style, CSS animation/transition and presentation policies remain separate.

Development candidate controls cover default Document0, startup snapshot, closed
shadow, CSS/WAAPI discovery, one-clock batch, important transitions, inherited
absolute fonts, discrete unregistered custom tokens, unsupported/refused paths and
throwing layout providers. CSS model getKeyframes keeps Chromium's custom-token
omission while script keyframes retain their existing reflection behavior.

Underlying-dependent frames are additionally conservative: multiple contributing
targets or animated inherited/font/SVG-paint/custom context prevent certification
of a suppressed underlying value. Base-dependent transitions remain Unknown until
cross-origin composition can certify their base. Complete absolute frames retain
their ordinary path. Opaque CSS imports prevent custom-registration absence proof;
no import I/O is performed by the sampler. Ownership evidence is isolate-owned and
weak, so concurrent isolates on the same thread never localize each other's handles.
