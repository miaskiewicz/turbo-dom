# Canonical suppression and empty presentation records

CSS animation-name remains declared when display:none on an element or an ancestor
suppresses its CSS animations. Canonical CSS synchronization intentionally owns no
animation in that state. The presented-effects seam previously treated that legitimate
name/count mismatch as Unknown, making unrelated native geometry fail.

The sampler now accepts suppression only from the canonical synchronization's hidden
state, an empty owned CSS list, a supported private stylesheet graph, and a native
ordinary-document ancestor path. The native proof compares ordinary parent identity
to the existing private composed-parent projection at every ancestor and ends at the
genuine document. Shadow-root/closed-slot participation remains Unknown; an unrelated
shadow sibling does not invalidate an ordinary path. Imports, container/starting-style
and unknown grouping rules remain unproven. Contributing display/custom-property
context prevents an underlying hidden state becoming a presented-suppression proof.
Ordinary valid nonempty presentation remains supported.

Automatic CSS cancellation now tracks the animation without labeling it as guest
script modification. A genuine earlier pause/seek/edit flag remains intact.

The mandatory sampler also previously rejected a page with no animation targets when
public Array/Event/timer or Object descriptor intrinsics were poisoned. Discovery now
uses captured private records first; only a proven empty target set bypasses prototype
inspection. Private list descriptors and target arrays have null prototypes. Existing
native document/realm/owner, finite clock, no-reentry and explicit-microtask guards
remain. Active/candidate paths retain strict prototype checks. WeakMap and descriptor
poison counters prove that discovery does not invoke public methods or getters.

## Evidence

The unchanged source-owned auth fixture is committed at
`crates/turbo-dom-browser-env/tests/fixtures/auth_menu_animations.html`, SHA256
`e2134c7ad68f7eb63ca92e173985715c029c45be96e87a2440431fe6ae00d0d3`.
Actual Chrome159.0.8154.6 shows declared typing-pulse names with zero dot animations
in none/message phases, three animations in typing, and one message animation in
message. All geometry reads remain valid. Exact source-hashed browser output and
producer baseline/green logs live under `.evidence/auth-menu/` in the worktree.

The final causal baseline had 34 passing / 8 failing host tests. The implementation
has 43 passing / 0 failing / 0 ignored host controls, including repeated phase changes,
menu transition/ripple creation, genuine script-pause preservation, unmodeled/reveal
controls, shadow/slot/container refusal and both existing runtime poison contracts.
The independent presentation-policy control was already green before its explicit
policy guard; it is a retained control, not another claimed baseline defect.

Existing canonical animation controls:19 passing; presentation/lifecycle/shadow/
cross-isolate snapshot controls:5/12/6/1 passing. Strict Rust1.98 all-target browser-env
Clippy with host-internals, fmt, JS syntax and whitespace checks pass. New helpers
are CC9 or lower; unrelated pre-existing binding-only complexity findings remain.
All compilation used one job and the pinned V8 archive with offline/locked dependencies.

This is scoped producer validation. Combined release floors/workspace gates and actual
Surf geometry/scroll acceptance have their own integration receipts. No empty Unknown
was waived, and no application-success claim is inferred from these host controls.
