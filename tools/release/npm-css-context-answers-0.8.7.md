# Additive Core CSS context answers

Based on `c018bd56b1de677044e8b4722664321415814cc4`, this Core-only increment adds exact tree-context selector and scope-boundary maps. It does not wire the npm adapter, change defaults or claim #296 resolved.

`ContextSelectorAnswers` uses `(tree_scope, selector, item, scopeRoot)`. `ContextBoundaryAnswers` uses `(tree_scope, group, boundary, enclosingRoot)`. `ContextAnswers` borrows both maps. `matching_rules_with_context_answers` and `compute_style_with_context_answers` use the exact prepared binding's tree scope, with no fallback to legacy unkeyed maps. Each answer still requires a valid exhaustive MechanicalAnswer domain. Existing TargetFacts literals, public aliases and entrypoints retain their prior behavior. Cascade selection, physical route validation, stylesheet implicit owners, budgets and the canonical computed-value pipeline are shared.

Controls prove shared-source document/host answer isolation, absent/wrong-context/outside-domain refusal, zero decision budget refusal, contextual boundary-start/end lookup differences, and computed custom/color/rem-unit values. The boundary tests prove lookup contracts; they do not fake shadow physical ancestry or prove the entire authored Chrome scope corpus.

Known limitation: genuine shadow descendant paths omit their host. Actual Chrome controls show host-boundary scope behavior and equal implicit-ShadowRoot/explicit-host proximity that require a separate audited scope-context representation. No virtual host path was added, and the thirty-case authored scope inverse remains pending.

Validation details and source hashes are in the adjacent JSON receipt. Optional Core 100% coverage on this exact increment has NOT YET BEEN MEASURED; it remains a combined-integration gate, with no lowered floor or coverage exclusion. No Root binding, JS, manifests or lockfile changed. No native/WASM artifact or performance/default acceptance is claimed.
