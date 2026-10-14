# Native storage collection

`Tree::collect_unreachable` is an opt-in public Rust API. It reclaims native data
only when the embedder supplies every independently retained handle. It does not
discover external Rust references or install JavaScript garbage collection.

The document, composed components containing supplied pins, and undrained native
mutation events are roots. Holding a detached descendant retains its ancestors,
siblings, shadow trees and attached Attr nodes. Retained identity is stable;
reclaimed IDs are never reused. `handles()` enumerates live identities and
`node_count()` reports their count after collection introduces gaps.

Node and edge budgets bound scans. The payload cap applies to expanded retained
node strings, namespace metadata and pending-record strings. Admission occurs
before materializing parser payloads. A large unreachable source string can be
freed with a small retained-payload budget. Invalid pins, observed inconsistent
composed edges, cycles and budget failures leave storage unchanged. Collection
emits no DOM mutation and preserves `Tree::version`. Diagnostic bytes are logical
string payload, not allocator overhead or a complete process heap measurement.

Two adversarial source reviews preceded the release checks. The first identified
post-copy admission, omitted namespace/pending-record payloads and an overwritten
shadow-host link that could lose an explicitly pinned old root. The corrected
source rejects inconsistent links and checks every required root/pin is marked;
it admits expanded bytes before copying. The second found no additional blocker
in this bounded opt-in seam. Consumer pin discovery remains a separate obligation.

The 14 offline storage contracts include megabyte parsed-payload reclamation,
composed detached retention, Attr ownership, namespace/doctype/template/text
preservation, pending mutation records, 10,000 allocations with non-reused IDs,
atomic budget/invalid-pin/cycle failures, namespace/pending-string limits and an
allocation-counter proof that 10,000 shared 1 KiB values reject a 16 KiB cap before
copying them. The existing independent Chrome mutation-event oracle also runs in
the native crate gate. Additional V8 consumer tests are maintained in turbo-surf.

For release validation use `PUBLISHING.md`; the collector does not change the
ordinary dense parse/mutation path until explicitly requested. The allocation
churn gate must still run alone because its allocator is process-global.
