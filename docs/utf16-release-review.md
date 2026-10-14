# UTF-16 release review

The Rust DOM stores original code units through additive public methods. Scalar
getters remain compatibility views; this release does not claim that every npm
runtime string API or browser lifecycle implements the same native interface.

## Review of storage and parsing

The first full release gate caught a real mutation-churn memory regression:
93,798 bytes per operation exceeded the existing85,000-byte ceiling. Compact
native payload variants now measure81,606 bytes and367 allocations, below the
unchanged85,000/380 limits. Native identity and pending mutation payloads are
included in collection accounting. Original units distinguish surrogate names,
namespace and attribute identities from replacement-character scalar views.

## Review of depth and source transport

Recursive parser intermediates, native imports, text aggregation and serializers
were replaced with explicit stacks. Tests run8192 nested document/fragment
levels on a512KiB host stack. The deeper controls also caught and repaired lost
template fragments and interior BOMs. Internal tokenizer transport escapes are
injective, including literal and numeric-reference markers; native nodes decode
before they are exposed. Source hashes and both upstream licenses ship with the
adaptation. The scalar API remains available and both scalar and unit serializers
are exercised independently.

## Independent adversarial round and repairs (2026-10-14)

An independent review of PR11 head `a7279b5` reproduced four correctness bugs:
namespace collisions in ID/CSS lookup, omitted original-unit doctype/PI-target
payload admission, HTML-only foreign fragment contexts, and a split CRLF CSS
hex-escape terminator. All four are repaired. The namespace getter borrows native
records so scalar query hot paths avoid per-element allocation. Collection
regressions check every doctype/PI field, atomic failure and exact-budget success.

The source-pinned `tests/fixtures/utf16-adversarial-probe.js` control was collected
in fresh Chrome155.0.8059.79 / V815.5.35.23 with zero exceptions. It covers both
attribute insertion orders, escaped-colon namespace selection and140 fragment
cases across SVG, foreignObject, MathML text and annotation-xml integration
points, scalar/BOM/high/low payloads and five insertion modes. Offline native
tests compare actual node namespaces, original-unit values and structure.

Post-repair local checks pass 305 backend contracts, seven parser contracts, all
three strict Clippy variants with CI's `RUSTFLAGS=-D warnings`, and the unchanged
367-allocation/81,606-byte churn gate. Rebuilt native, Node WASM and Web WASM
packages pass 548 JS tests, 29 vitest tests and two Jest tests. The html5lib report
retains five known failures (1778/1783 evaluated; eight skipped). npm pack retains
matching 0.5.5 metadata. Exact-head GitHub CI must pass before merge/publication.
The prior validation below describes the pre-review candidate.

## Pre-review local validation

Rebuilt native, Node WASM and Web WASM packages pass7 parser tests,301 Rust DOM
contracts, all three strict Clippy variants and the isolated allocation gate.
JavaScript tests pass548/548; vitest passes29/29 and Jest passes2/2. The existing
html5lib report remains1778/1783 evaluated (8 skipped); its5 known failures are
not represented as complete parser conformance. npm pack and Rust crate contents
include matching0.5.5 metadata and the tokenizer license/provenance files.
GitHub CI must pass on the exact release PR head before merge/tag publication.
