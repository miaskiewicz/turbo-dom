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

## Final local validation

Rebuilt native, Node WASM and Web WASM packages pass7 parser tests,301 Rust DOM
contracts, all three strict Clippy variants and the isolated allocation gate.
JavaScript tests pass548/548; vitest passes29/29 and Jest passes2/2. The existing
html5lib report remains1778/1783 evaluated (8 skipped); its5 known failures are
not represented as complete parser conformance. npm pack and Rust crate contents
include matching0.5.5 metadata and the tokenizer license/provenance files.
GitHub CI must pass on the exact release PR head before merge/tag publication.
