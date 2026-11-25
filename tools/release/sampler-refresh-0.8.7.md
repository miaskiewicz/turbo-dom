# #579: keep lazy inherited defaults in their current inspection read

Issue: https://github.com/miaskiewicz/turbo-dom/issues/579

A real Surf full-session CPU profile motivated this reduction. Its sampler paths
included lazy CSS-wide/inherited-default reads that entered `computeStyleFresh`
without the private inspection batch. Sampled ancestry is overlapping time, not
invocation counts; this receipt does not claim the live auth timeout is resolved.

The independent public sampler control varies N=16/64 selected elements and
D=4/16 ancestors. Both measured HOST calls have the same native tree/selector/
CSSOM/live world and canonical clock. Adding only `letter-spacing:inherit` to the
explicit-font control exposed repeated fresh ancestor cascades:

| D | N | Fresh cascades before | Fresh cascades after | Contextless fresh before/after | Guard validations before/after |
|---|---|---|---|---|---|
| 4 | 16 | 262 | 22 | 240 / 0 | 90 / 555 |
| 4 | 64 | 1030 | 70 | 960 / 0 | 378 / 2283 |
| 16 | 16 | 2482 | 34 | 2448 / 0 | 270 / 5013 |
| 16 | 64 | 9874 | 82 | 9792 / 0 | 1134 / 20565 |

The results hold independently on the first and second measured call. The
explicit control remains N+D+2 cascades. Every selected root still refreshes;
there is no cross-call cache, checkpoint substitution or elapsed-time floor.
Full guard validation has an explicit cost; it was not memoized or omitted to
obtain the fresh-cascade reduction.

`computedColor` now passes its current inspection snapshots through the private
CSS-wide/default-value read chain. A privately registered map is eligible only
when its node is not the selected root, an existing local snapshot (if present)
is the exact current batch entry, and the original entry validator accepts owner,
world, native probe, media, dependency identities, mode and volatility/box-failure
state, including its final recheck. The helper returns that map; any missing local
snapshot is imported by the original `computeStyleSnapshot` path. It does not
copy the import algorithm. Refusal uses the original fresh-map fallback. Ordinary
computed-style calls and the existing lifecycle fallback retain their behavior.

No map is retained on declaration records or on a global active-reader variable.
The optional parameter is confined to synchronous private reads. Counter hooks
are captured and deleted, exist natively only under cfg(test), and store primitive
isolate-owned numbers/function pointers rather than V8 references.

The counter matrix separately covers paused ancestors and seeking, inherited
custom-property changes, and symbolic percentage transforms followed by parent
resizing. Actual native callbacks change world, font probe, or media answers
inside the original root read; captured underlying values remain current and the
old entry is refused. Between-root mutation, volatile ancestors and a genuinely
throwing media callback preserve fresh fallback and failed-box metadata. Complete
owned sampler payloads are compared with the existing no-token fresh-reference
path. Provider calls remain zero, each HOST sample reads the canonical clock once,
and queued Promise work remains zero until the host explicitly drains it.

Seven focused tests also cover private/isolate-owned counters and actual startup
snapshot rehydration. Raw RED, GREEN, reason vectors and world/certification
controls remain in `.evidence/sampler-refresh/` in the producer worktree. Final
full host/default/strict gate results are recorded separately; actual optimized
Surf artifact and original application acceptance must follow integration.

Final committed-source gates: the complete `host-internals,test-support` binding
profile passes **1107 tests, zero failures, four existing ignored** across 32
suites, including the seven new counter/guard tests and all 73 presented/recovery
controls. Default-feature computed-default controls pass **20/20**. Strict binding
all-target Clippy with `-D warnings`, formatting, diff checks, Rust CC9 on the new
test module and JS CC9 on the changed inspection helpers pass. Coverage floors
remain unchanged and their combined-release remeasurement is separate from these
scoped gates. No native SDK, runtime budget or application assertion was changed.
