# Shared CSS source foundation — v0.8.7 / #296

Implementation: `f9cac906694eb5d9d4d98ba64dd21cc899032db0`; integrated with the shadow checkpoint release in `eafc43efbc96c09ed48469fcdae50e27c6d2655e`.

The additive `rtdom::computed_css` source/declaration primitives retain raw UTF-16 token payloads, including lone units, separately from CSS-parser-normalized identifiers. Both legacy UTF-8 declaration values and new source ranges use the existing cssparser declaration scanner through one generic collector. Legacy Unicode `.trim()` behavior remains unchanged; no intermediate declaration vector is introduced. The new owned source map rejects ranges inside scalar characters and does not retain DOM objects or a mutable tree mirror.

This is a shared-kernel foundation, not the complete npm getComputedStyle fix. Npm loader/backend/default behavior, published DOM/Vitest/Jest APIs and existing native compatibility controls remain unchanged. #296 stays open: variable resolution, cascade features, backend adapters and actual runtime cutover require their own validation.

Validated pinned Rust1.98 native/WASM controls: **16 passed**, including raw/lone UTF-16, identifier normalization, Unicode whitespace, quoted delimiters/important, and legacy declaration equivalence. Rebuilt native plus Node/Web WASM npm suite: **757 passed / 0 failed / 0 skipped**; coverage **100% lines / 100% functions / 97.46% branches**, above the unchanged97.3% branch floor. Original18 npm parity-gap outputs remain unchanged, proving this commit does not claim a default runtime fix.

Merged release gates: workspace **3233 passed / 0 failed / 13 existing ignored**; strict workspace all-target Clippy, fmt and diff checks pass. Combined core coverage is **13,541/13,541 (100%)**. The isolated branch additionally passed core/parser controls, native/WASM feature Clippy, CC9, churn and core100% before integration. No floors, assertions, budgets or compatibility flags changed.

Full local evidence: `.evidence/npm-css-foundation/FINAL-GATES.json`; branch artifacts, source hashes and ABI/npm controls: `.worktrees/npm-cascade-296/.evidence/issue-296/FOUNDATION-VALIDATION.json`. No tag/publication is implied.
