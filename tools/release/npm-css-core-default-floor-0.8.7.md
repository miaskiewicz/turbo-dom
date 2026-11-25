# Default Core coverage and module identity

Default Core coverage passes the unchanged 100% floor: **14265/14265 lines**. Meaningful default-feature controls cover font descriptor modes, strict CSS whitespace, value families, selector UTF-16 payload accounting and typed length errors. Declared-marker expansion also has a short hexadecimal input/per-output limit inverse.

The pinned Tarpaulin0.37.5 correctly evaluates feature cfg, but its module path discovery selects the existing `grammar/` directory instead of sibling `grammar.rs`. Its generic-function analyzer then synthesizes 58 zero-hit traces from uncompiled code. Moving identical source bytes to the normal `grammar/mod.rs` layout makes module identity unambiguous. No cfg, exclusions, floor or tool version changed. The optional module remains measured separately when enabled.

The private length resolver now uses the already-checked canonical source nesting budget once. Its admitted IR preserves/reduces wrapper depth; tree-counting forms that could add grouping are refused first. Public calc/min/max/clamp boundary controls1–5 preserve the original typed refusal, numeric output and finite-clamp policy.

All default/feature tests, strict feature Clippy, fmt and CC9 pass; exact counts and source hashes are adjacent. Optional-feature100 coverage remains pending; its preceding source was still red at99.90%. This is not completion/default cutover of #296 and does not certify newer artifacts.
