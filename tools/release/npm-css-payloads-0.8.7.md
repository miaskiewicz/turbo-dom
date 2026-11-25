# Lazy npm CSS payload packaging, v0.8.7

Packaging-only integration based on actual component
`21c4aab10d6a122ab50906b89941c22301663dea`. Root Cargo/class glue stays unchanged.
The frozen earlier source patch remains available separately; this topic adapts
its packaging to the committed component and fixes Cargo-config-only target dirs.
No default runtime cutover, release publication or completed #296 parity claim.

## Artifact contract

- Existing parser build commands/default features and generated parser index/types
  stay unchanged. CSS builds into `native-css/turbo-dom-css.<platform>.node` and
  `pkg-css` / `pkg-css-web` with `turbo_dom_css` names; no parser codegen overwrite.
- Native copy paths come from **post-build locked Cargo metadata**, honoring both
  env and Cargo-config-only target dirs. The print-only plan invokes no tools and
  labels its pre-build source path as a hint. Caller jobs/cache/flags and the
  existing musl dynamic-CRT policy are preserved.
- The artifact exports CompiledStylesV1, ABI1 and compiled package0.8.7 getters.
  Private WASM packages use explicit commonjs/module types; emitted snippets and
  existing font/copy-code notices are included. No required ParserBinding method
  or public runtime-selection flag is added.
- Both explicit build-tool env paths select a no-install route:
  `TURBO_DOM_WASM_BINDGEN` must match Cargo.lock exactly (currently0.2.122),
  `TURBO_DOM_WASM_OPT` must be reviewed wasm-opt117. It runs offline locked Cargo,
  bindgen nodejs/web, then `-Oz`; Binaryen defaults to one core. Tool/optimization
  errors propagate without installer, unoptimized payload or runtime fallback.
  Scratch outputs replace working packages only after every stage succeeds.
  Without overrides, CI's existing wasm-pack route remains available.

## Real tests and provenance

No Rust, WASM or optimizer compilation was started by this packaging task.

- Offline Python helper/guard tests:73passed/0failed.
- Node packaging helper tests:6passed/0failed/0skipped; CC9, syntax, versions and
  whitespace checks pass.
- Real Cargo-config-only target_directory metadata probe passes (metadata only).
- Real native CSS c808daf506c0ac1a380dded2d8a20aa9eac2ab73c9f7ecb735d79a90120d9787
  and Node/web CSS WASM5c4af4da0bd4c14315fc82d98b6422e2b1355a6a1f035404fcc66a33640fa33b
  expose ABI1/version0.8.7. The real native load passes with a throwing global
  WebAssembly getter; packaged ESM web initialization passes too.
- Actual payload copies came from the producer owner's immutable artifact worktree,
  with before/after SHA checks. Original files were not modified. Normal private
  package metadata was generated from source0.8.7; this is packaging metadata,
  **not** a new binary build. Raw receipts are in `.evidence/css-packaging-v2/`.
- Full npm coverage gate:884passed/0failed/0skipped, runtime lines100%, functions100%,
  branches97.62% (floor97.3 unchanged). This is packaging regression coverage,
  **not** proof of the pending new runtime adapter's parity/default integration.

## Release gates and measurements

Full npm/test:cov/Vitest/Jest gates require actual matching CSS ABI/version first;
optional capability absence cannot silently run only legacy tests. Complete release
bundling requires exactly five parser and five CSS native platform binaries, and
all four WASM packages with coherent versions/nonempty files/valid headers. It
records exact SHA/byte receipts and applies GLIBC2.35 to parser+CSS GNU binaries.
A host-only payload correctly fails that complete-matrix guard; no host binary was
renamed into an unbuilt platform, no full matrix success is claimed.

The actual **host-only** pack dry run is23,895,431compressed /55,812,021unpacked
bytes, including51,717,238CSS-payload bytes. The full multi-platform tarball is
unmeasured. Lazy loading does not make parser-only download cost unchanged in the
current single bundled npm distribution. Independent parser startup comparisons,
actual cold-load performance, runtime native/WASM/container/shadow/invalidation
acceptance and complete-platform builds remain integration/release gates.
`retainedBytes` is a shared CSS-payload lower bound, not a heap/RSS measurement.
No coverage floors, runtime limits, timeouts or assertions were weakened.
