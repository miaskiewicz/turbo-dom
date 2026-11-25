# Native SVG custom-property paint, v0.8.7 (#559)

This static SVG-as-image adapter uses the shared Rust CSS source parser, raw UTF16 selector metadata, native selector engine, existing cascade rank and shared custom-token computation. It adds no CSS parser/matcher/substitution engine and changes no npm/backend compatibility default.

## Causal controls

The original public `svg::rasterize` returns opaque black for fallback and defined custom-property fill. Independent Chromium159.0.8154.6 observes red/green. The source-hashed probe and66-case fixture live in `scripts/svg-custom-paint-chrome-control.mjs` and `tests/fixtures/svg_custom_paint_{cases,chrome159}.json` under the layout crate. Of these,51 compare actual pixels and15 assert explicit checked Unsupported for uncertified paths. The initial public baseline and failed overlay attempt remain in `.evidence/VARIABLE-{RED,XML}.log`.

Actual original FAQ raw pixels now match the independent explicit-color version byte-for-byte; defined/fallback assets match green/red controls. Every unrelated SVG/raster/MIME/nav/security-badge asset stays unchanged. The producer replay uses the exact workspace dependency versions and records raw-pixel hashes in `.evidence/VARIABLE-FINAL-REPLAY-RESULT.json`; this does not claim an actual rebuilt Surf addon or application run.

The final original FAQ replay exposed a conservative display:block refusal. Its exact independent Chrome positive was red before normalized block admission, then green; this source-hashed case stays in the fixture.

Malformed `var()`/malformed unused fallbacks must be discarded before cascade, preserving an earlier valid declaration. Valid unresolved/cyclic variables instead unset after selection and inherit appropriately. Three new duplicate-declaration controls were red before strict syntax admission, then green; the valid-cycle inverse remains green. The public shared strict accessor is separate from the limits-only condition guard.

## Canonical static materialization

A temporary immutable native Tree is constructed with namespace-aware elements from already-validated XML. Its connected document has only the actual SVG root: no HTML wrapper participates in `:root`, selectors or inheritance. XML namespace/local-name/case/CDATA, class matching and prefixed root controls are independently recorded.

All matching happens before any XML edits. Shared ranked declarations retain invalid-value fallback candidates and shorthand/longhand order. Known-invalid nonpaint values are rejected by the existing canonical CSSOM grammar. Certified CSS nonpaint values are deliberately limited to numeric opacity, display none/inline/block, visibility visible/hidden and literal font-family. Context-dependent geometry, calc and inheritance remain explicit Unsupported. Original nonpaint presentation attributes remain unchanged. Original stylesheet contents are removed; per-element nonpaint bundles are emitted in canonical order without importance markers, and resolved own paint becomes presentation attributes. This avoids usvg's first-important-wins store and prevents rewritten `[style]`/`:empty` inputs from changing subsequent matches. Dedicated selector-freeze, invalid-declaration fallback and XML-escaping inverses cover that boundary.

Literal SVGs without actual variable references bypass rewriting. Quoted/comment/URL `var` strings remain inert under canonical token inspection. Nested SVG data images use the same adapter while retaining the existing no-images-inside-nested-SVG/no-external-I/O resolver boundary.

## Policy and honest limits

Variable computation keeps the existing16-level nesting policy. Per-output and cumulative retained custom-token units derive from original SVG UTF16 source units. XML replacement growth is charged incrementally against one original-source worth of additional units before retention/output construction. Inherited paint payloads share immutable storage. Structural reference inspection uses the SVG renderer's existing depth bound; it does not evaluate variables or raise their16-level computation budget.

Typed nesting/resource failures never select a CSS fallback. Checked APIs return explicit Unsupported for uncertified use/instance inheritance, conditional/import/descriptor cascades, dynamic/other uncertified pseudos, SMIL/animation/transition, foreign XML/DTD rewriting and nonpaint var/env/revert/all semantics. The initial certificate handles solid-color paint; other unresolved paint grammars remain uncertain. Literal DTD/use/GIF/clip/nested-SVG inverses preserve existing behavior. Existing embedded-raster dimensions/cumulative decode-work limits and external string resolver denial remain intact.

Legacy Option APIs preserve None on refusal; typed errors require checked API adoption by the consumer. No source-specific asset exception, budget increase, assertion weakening or coverage exclusion is used.

## Validation

Shared prerequisites069d8f1c/015d94e7 expose canonical source guards and strict declaration admission.17c4df31 resolves post-substitution custom CSS-wide keywords before dependency memoization and provides typed rollback requests;78120087 adds canonical visibility keyword admission. Final original core100 floor:13,880/13,880 lines, strict all-target Clippy green.

Complete scoped gates: layout all-features1,315 passed; layout default1,257 passed; raster476 passed; zero failures/ignored. Strict all-target/all-feature layout+raster Clippy, workspace fmt, changed-source CC9 and probe lint/format pass. Logs live in `.evidence/SVG-FROZEN-*`; Clippy fixture-only allocation/raw-string corrections are recorded separately. The final exact public all-assets replay records FAQ equality, defined/fallback colors and unrelated byte identity. Combined release/consumer floors remain owned by the release coordinator; no actual Surf rebuild or application success is claimed here.
