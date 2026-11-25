# Surf integration batch, v0.8.7

Code checkpoint: `483092a468177d3368203eec2b0a8c35c51583c4`.

Integrated producer fixes cover canonical presented effects and native animation/transition cascade (#555), embedded SVG raster decoding (#556), MIME signature aliases (#557), strict canonical selector-state projection (#558), static SVG variable paint (#559), certified inner-SVG client/scroll metrics (#560), and finite-singular local scroll provenance (#561). Shared owned CSS syntax, bounded variable computation and explicit rollback requests are reused by the SVG and pending npm adapters. Existing independent compatibility controls remain separate.

## Verified behavior

- Earlier combined workspace checkpoint:3,149 passed,0 failed,13 existing ignored. Full host/test-support checkpoint:986 passed,0 failed,4 existing ignored; strict workspace all-target Clippy passed. Later animation/scroll/SVG changes have their focused and scoped receipts below; a final combined workspace refresh remains required.
- Canonical actual-MUI controls:29 producer host tests pass, including the exact captured rules/15 Chromium159 observations and unmodified live effects. Canonical legacy vendor rejection preserves webkit/custom properties and the unmodeled-source refusal.
- Surf's actual private consumer replay on parent `bdd09774a0d560726f9919518466b2638ff81b75`:5/5 pass, including rotation/ripple/circle, unmodified live effects and scale0 rectangle0x0/client20x10/scroll20x10. Consumer receipt: `.recovery/presented-effects-consumer-draft/captured-mui-bdd.log` in its owning repository. This is not an application-suite result.
- Singular scroll provenance:15 source-hashed Chromium controls and default layout library539 tests pass; strict Clippy/fmt/CC9 pass. Extra private OWN metadata is48bytes/fragment plus vector header.
- SVG variable paint:66 Chromium159 controls (51 pixel parity,15 explicit unsupported contexts); actual24-asset replay and FAQ bytes match the explicit-color control. Scoped layout all-features1,315/default1,257/raster476 pass with0 failures/ignores; strict all-target/all-feature Clippy passes.
- Shared core coverage:13,880/13,880=100%. Updated binding coverage:22,949/23,506=97.63%, above the unchanged97.6% floor.

## Coverage work remains open

The combined all-feature layout floor is **RED**:20,736/20,804=99.67%, below the unchanged99.8% requirement. Raw LLVM report and mapped uncovered lines are retained in `.evidence/presented-effects/coverage-layout-all/` in the producer integration worktree. Default-layout coverage and final combined gate refresh remain pending. No floor, assertion, execution budget or resource limit was relaxed.

The user explicitly authorized pushing this integrated code to `release/0.8.7` while coverage work continues. This push is not a green-gate release, registry publication, tag or main merge. Surf must keep its exact source/artifact receipts and complete its native gates, actual addon rebuild, auth46 and previously failed cohort before a wider suite.

## Independent npm work

Issue#296 is still open: real native/WASM computed-style classes, runtime adapter/default cutover, payload builds and full parity/performance gates are in progress. Shared kernels are integrated, but this batch does not claim the published npm computed-style behavior is fixed. That independent runtime work does not block Surf's producer integration.

## Coverage follow-up after the current-name integration

Production acceptance remains `2cf66aa5e8ed37dd609e6abc377be599a425f504`.
The test-only integration `b4e90485a118fddf4ce88c94272dea182b7cc3c3` adds
four meaningful native provenance controls without changing production behavior.
They cover missing border/used-box refusal, finite coefficients with an overflowing
transform determinant, and imported SVG text-path refusal. Their focused tests,
strict all-target layout Clippy, fmt and CC9 passed.

The Rust1.98 all-feature layout LLVM coverage measurement now passes the unchanged
99.8% floor: **20,929 / 20,966 lines, 99.82%**, exit0. This supersedes the earlier
99.67% all-feature result above; the earlier failed receipt remains preserved.
Command: `cargo +1.98.0 tarpaulin --offline --locked --engine llvm -p turbo-dom-layout
--all-features --include-files 'crates/turbo-dom-layout/src/*' --timeout 600
--out Json --out Stdout --fail-under 99.8` with one build job.
The exact local log and JSON are under `.evidence/presented-effects/coverage-layout-post-controls*`.

The separate default-feature layout measurement and latest binding/Core coverage
checks are not implied by this all-feature receipt. No floor, timeout, resource
budget, production guard or assertion was weakened. No tag or publication occurred.
