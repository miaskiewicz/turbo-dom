# Shared CSS source adapters for SVG and npm, v0.8.7

Dependencies are the validated shared variable kernel `a047a2fa` and owned raw stylesheet/UTF16 selector structure `8e10d4c8` (local cherry-picks `a95cb9f9`, `a80b2958`). This change adds source adapters without changing a runtime default or duplicating a parser.

- `check_component_limits` guards unchanged UTF16 input units and every component block, including all `var()` arguments/unused fallbacks. It deliberately does not decide CSS/BAD-token/variable-argument validity. Existing strict substitution validation remains unchanged. Explicit caller limits are required; there are no hidden embedding defaults.
- `has_variable_references` and `has_function_reference` inspect canonical CSS tokens; escaped/case-insensitive function names work, quoted strings/comments/URL payloads remain inert. These probes are not full-stream resource validators because matched function arguments are skipped after detection.
- `CustomProperties::as_map` borrows an immutable computed snapshot for child inheritance without copying its token payloads.
- `CssDeclaration::serialized` escapes the normalized property name through CSS Syntax and retains the raw UTF16 value, including unpaired units.
- `CascadeSelector::certifies_structural_matching` is a conservative metadata certificate for root/scope/empty and logical combinations; dynamic/other pseudo classes and every pseudo element remain uncertain. It does not change selector matching.

Validation: Rust1.98.0, one own compiler job; new7 public adapter controls and complete core suite pass; strict core all-target Clippy, fmt and CC≤9 pass. Original LLVM/Tarpaulin core floor100 passed with13,847/13,847 coverable lines. No floor, nesting/output budget or assertion was weakened. Durable raw logs are in the isolated SVG worktree's `.evidence/CORE-ADAPTER-*`; the instrumented report is `.evidence/core-coverage/tarpaulin-report.json`.

These are shared prerequisites for #559 and #296, not a claim that either complete adapter is finished by this commit.

## Strict declaration admission follow-up

`valid_custom_value_syntax` reuses the existing strict component/variable grammar and returns false for malformed syntax before cascade selection. It does not substitute or decide whether a valid unresolved/cyclic variable later becomes guaranteed-invalid. Typed input/nesting failures propagate separately. The generic condition/resource guard remains limits-only.

Independent SVG-as-image Chromium inverses distinguish malformed `var()`/malformed unused fallback declarations (discard before selection, preserving earlier red) from valid cycles (selected then unset, inheriting green). The three malformed-declaration cases were causally red before this accessor/admission stage. Core full strict Clippy and original LLVM floor100 pass at13,855/13,855 coverable lines; the new public adapter suite has8 passing tests. No embedding default or limit changed.

## Post-substitution custom CSS-wide values

The shared resolver now applies the same keyword handler before and after substitution, before memoizing each property. `inherit`/`unset` use that property's computed parent; `initial` is guaranteed-invalid. A dependency therefore observes the correctly resolved inherited value, not a literal keyword that could later inherit from the wrong property. Cyclic custom values remain invalid even if their fallback spells `revert`.

`compute_custom_properties_detailed` adds `CustomComputation::{Computed,RequiresCascade{property,keyword}}` and `RollbackKeyword::{Revert,RevertLayer}`. A request identifies the actual dependency property that produced rollback. Each retry creates a fresh memo/cycle set. The legacy API maps the request to its unchanged `ComputedCssError::RequiresCascade`; it never guesses a lower candidate.

The legacy causal baseline had3failed tests and1quoted-word inverse passing; all6 new controls now pass, including dependency identity, cyclic rollback and fresh retry. Independent Chrome159 SVG controls confirm post-var inheritance/initial and dependency behavior. Full original core100 is13,879/13,879 lines with strict core all-target Clippy and CC9 passing. Neither the generic limits-only condition guard nor any runtime default/limit changed.

## Visibility admission

The canonical longhand keyword table now admits `visible`/`hidden`/`collapse` and rejects invalid visibility values, rather than leaving them Unchecked. Escaped/case-insensitive keywords and pending `var()` spelling are covered. This resolves the independent literal-hidden SVG inverse through shared CSSOM admission, without a renderer-specific value parser. Direct admission RED→GREEN, strict core Clippy and original core100 pass at13,880/13,880 lines.
