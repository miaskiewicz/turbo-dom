# Absolute flex padding-box regression (#546)

Reported from Surf's payroll E2E work; producer base
`448d05e89240ab8ce20c2be50723500d2c4a2917`.
Issue: <https://github.com/miaskiewicz/turbo-dom/issues/546>, milestone v0.8.7.

## Causal reduction

A positioned, centered flex span has 9px padding and a 24x24 SVG child. Its
transparent absolute `input[type=checkbox]` declares `left:0;top:0;width:100%;height:100%`.
The parent's border box is 42x42.

| Implementation | Checkbox rect relative to parent |
| --- | --- |
| Unmodified producer release base | [9, 9, 24, 0] |
| Intermediate frame-only change | [0, 0, 42, 0] |
| Final candidate / Chromium 159 | [0, 0, 42, 42] |

The same tests exercise both public `layout_boxes` and
`snapshot_html_live_tree_with_native_text`. A default-stretch variant hides the
zero-height defect and instead yields 24x24 on the old source; the centered,
SVG-backed checkbox is therefore retained explicitly.

## Final ownership and behavior

The native flex adapter's first solve continues to use its content-box root,
including its existing in-flow percentage-height definiteness and min/max rules.
After that solve, a positioned-only root receives the actual padding/border frame
and the finalized content size. It measures only out-of-flow leaves and preserves
the source order when native fragments are placed. Normal-flow children are
neither copied nor re-solved in that second root.

Native placement retains an absolute child's solved height even under center
alignment. The adapter's automatic start inset remains a content-edge static
anchor, distinct from an authored zero inset against the padding box.

Applying the frame to the original in-flow root exposed regressions in the
existing signed-column and nested-min-height controls. That intermediate approach
was replaced; the controls and their assertions were not changed.

## Independent controls

`scripts/absolute-padding-box-chrome-control.mjs` records 48 plain HTML/CSS cases
on Chromium 159.0.8154.6, using the required mock-keychain/password-store flags.
Its source hash is pinned in
`tests/fixtures/absolute_padding_box_chrome159.json`. Chrome 156 was unavailable;
the new recording is explicitly a 159 control.

The matrix covers flex/inline-flex, rows/columns, actual SVG-backed centered
checkbox inputs, auto/fixed/minimum heights, asymmetric padding, borders,
border-box sizing, percentage padding/insets, end insets, auto static starts,
cross-axis centering and ordinary-box inset stretch/normal-flow percentages.
Both public pipelines compare every parent, checkbox/box and flow-child rectangle
against the oracle with a 0.02px tolerance. The dimensions are chosen so static
integer quantization does not require weakening the assertions.

## Evidence and integration

Local red/intermediate/fixed logs and reproducible commands live under
`.evidence/absolute-padding-box/`. No React/MUI bundle is vendored into the
producer. Surf's source, Git pins, actual addon, services and leased fixtures were
not changed. The consumer must coherently repin its DOM-family dependencies and
rebuild its actual addon before claiming its application checkbox check passes.

All compilation uses Rust 1.98.0 with at most two jobs. No coverage floor,
complexity limit, resource budget or existing assertion is relaxed.

## Local validation

- Layout default: 1,247 passed, zero failed.
- Layout all features: 1,291 passed, zero failed.
- No-default-feature baseline: exactly the same 43 listed failures; no new skips.
- Workspace with host-internals/test-support: 3,179 passed, zero failed, 12 existing
  ignored tests, across 225 test/doc-test summaries.
- Strict workspace/all-target clippy and layout all-feature/all-target clippy:
  passed with `-D warnings`. Explicit imports and a borrowed placement context
  were used; no lint suppressions were added.
- Final all-feature focused tests replay the 48 oracle cases on both public
  paths and the unchanged signed-column and nested-height controls.
- Rustfmt, Rust complexity at max 9 (including the new module/test), pinned
  oxlint 1.87.0 complexity, version agreement and diff whitespace checks: passed.
- The graph retains one turbo-dom-v8 150.4.2.
- LLVM tarpaulin default: **99.86%**, 19,617/19,644 covered lines.
- LLVM tarpaulin all-feature CI variant: **99.85%**, 19,796/19,826 covered lines.
  Both passed the unchanged `--fail-under 99.8` and 600-second binary timeout,
  offline/locked, in an independent repository-local target with two jobs.
  The new positioned adapter has 53/53 lines covered in the default run.

The implementation checkpoint is
`14ab69e5bf90373174aba7d8f23f126360aad53c`. The validation receipt is a subsequent
documentation-only commit; no implementation changed after the passing coverage
runs. No tag, publication or main merge is authorized by this work.
