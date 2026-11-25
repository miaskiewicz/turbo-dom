# Shared CSS source adapters for SVG and npm, v0.8.7

Dependencies are the validated shared variable kernel `a047a2fa` and owned raw stylesheet/UTF16 selector structure `8e10d4c8` (local cherry-picks `a95cb9f9`, `a80b2958`). This change adds source adapters without changing a runtime default or duplicating a parser.

- `check_component_limits` guards unchanged UTF16 input units and every component block, including all `var()` arguments/unused fallbacks. It deliberately does not decide CSS/BAD-token/variable-argument validity. Existing strict substitution validation remains unchanged. Explicit caller limits are required; there are no hidden embedding defaults.
- `has_variable_references` and `has_function_reference` inspect canonical CSS tokens; escaped/case-insensitive function names work, quoted strings/comments/URL payloads remain inert. These probes are not full-stream resource validators because matched function arguments are skipped after detection.
- `CustomProperties::as_map` borrows an immutable computed snapshot for child inheritance without copying its token payloads.
- `CssDeclaration::serialized` escapes the normalized property name through CSS Syntax and retains the raw UTF16 value, including unpaired units.
- `CascadeSelector::certifies_structural_matching` is a conservative metadata certificate for root/scope/empty and logical combinations; dynamic/other pseudo classes and every pseudo element remain uncertain. It does not change selector matching.

Validation: Rust1.98.0, one own compiler job; new7 public adapter controls and complete core suite pass; strict core all-target Clippy, fmt and CC≤9 pass. Original LLVM/Tarpaulin core floor100 passed with13,847/13,847 coverable lines. No floor, nesting/output budget or assertion was weakened. Durable raw logs are in the isolated SVG worktree's `.evidence/CORE-ADAPTER-*`; the instrumented report is `.evidence/core-coverage/tarpaulin-report.json`.

These are shared prerequisites for #559 and #296, not a claim that either complete adapter is finished by this commit.
