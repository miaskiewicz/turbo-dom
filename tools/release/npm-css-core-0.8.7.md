# Shared CSS kernel checkpoint

This additive optional Rust kernel keeps immutable CSS models and accepts bounded matching/environment facts. It retains no DOM mirror. The legacy core cascade now reuses the shared structural parser under its existing raw recovery policy. Public npm getComputedStyle remains unchanged; #296 is open. Native CSS artifacts and runtime orchestration are separate increments.

Pinned backend-depth preflight preserves caller-stricter limits and refuses excessive nesting before recursive parsing. UTF-16 payloads, layers/scopes/container provenance, typed missing-capability errors, computed inheritance and used-size separation have deterministic controls. Unknown definitions and layout-dependent container effects refuse geometry projection.

Validation: clean committed-source optional-feature Core774/0, default574/0, nine existing ignored tests in each configuration; strict feature all-target Clippy, fmt and CC9 pass. All Cargo gates used --locked in a clean detached checkout, with empty git status before and after. The last measured optional-Core coverage is **99.70%, below the unchanged100% gate**; this checkpoint does not represent the issue or coverage work as complete. Exact source and gate metadata are in the adjacent JSON.

Actual old native artifact passes16/18 unchanged Chrome159 acceptance cases. Source controls cover the remaining content/container behavior; rebuilt native/WASM replay, ownership/performance and compatibility validation remain before default cutover. No Chrome158 migration or publication is claimed.

Committed source: `ecf46101ebc54f0b2cb32b54900ff3ea89fe36dd`. Native binding delegation, Root CSS classes/artifacts and the optional Core/Root CSSOM serializer remain separate.

This is **not zero default Core code change**. Shared default cascade parsing and custom-property detailed rollback/post-substitution CSS-wide behavior change in this snapshot; the latter carries the SVG shared-foundation provenance `069d8f1`/`015d94e`/`17c4df3` across this branch's older base. Unchanged npm GCS selection and unchanged dependency feature defaults are narrower claims. The clean default574 suite and source-hashed post-var controls provide local evidence; canonical Badge and native legacy variable controls remain required on future integration. Current Surf acceptance pins are untouched.
