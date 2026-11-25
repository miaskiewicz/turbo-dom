# Explicit host termination recovery — v0.8.7

`host_internals::recover_after_termination(scope) -> Result<(), TerminationRecoveryError>`
notifies every live registered native DOM realm in the current isolate after the
HOST has recorded an actual termination request/result, allowed the outer guest
execution to return, joined all watchdogs, and canceled termination. It never
cancels execution itself, pumps microtasks, reads a clock or dispatches events.
The host must preserve the original command result/error and retain its pending
recovery marker if cleanup fails. It must not invoke recovery from an inner native
callback or an unconditional Cancel site.

The entry rejects still-terminating execution, implicit microtask policy and any
active JavaScript stack before cleanup. Separately attached context queues must
also be explicit. It activates the isolate-owned existing native world before
localizing weak contexts, deduplicates inert document aliases by exact Context,
validates canonical owner and captured callback creation context, and includes
retained/nonbrowsing realms. An uninstalled isolate is refused before touching a
previous TLS world. Captured private rehydrated callbacks introduce no guest-global
lookup or new strong context roots.

Only interrupted inspection/CSS-update and computed-cache/build state is reset.
Live animations, effects, targets, current times, promises and scheduled events
remain intact. Partial committed DOM/CSS changes are reconciled on the next fresh
read; recovery does not pretend to roll them back. The per-batch cache uses only
weak capabilities/local maps, so an aborted batch introduces no strong retained
reader or cross-call cache reuse.

Three native TerminateOnce routes were RED after ordinary HOST cancellation: the
same document's next sampler returned None. The exact callbacks were reached; no
sleep or larger budget was used. Genuine nested inspection refusal passed. After
explicit recovery, the final 73-test presented/recovery binary passes, including
public getAnimations interruption, retained/inert contexts, isolate separation,
active-JS/uncanceled/implicit-microtask refusal, snapshot rehydration and preservation
of paused WAAPI identity/time/ready/finished promises. A pending Promise remains
unexecuted until the HOST explicitly performs its later microtask checkpoint.
Strict Rust1.98 all-target Clippy, fmt and scoped checks passed.

Implementation commits:706e9678d20634e9db1c6fccf786205bd11312d3 and
5037e641df526a2ecc83acb975d150f3ecaec61c; coherent batch release
410045a001a80ee0098fd30259e11b3604a629a6. Local RED/GREEN/source hashes:
`.worktrees/effect-inspection-termination-087/.evidence/`. The final combined batch
also passes seven batch controls, seven transition-creation and five transform
provider controls. Actual optimized consumer cost/auth acceptance remains pending;
the recovery defect is not asserted as the initial budget cause.
