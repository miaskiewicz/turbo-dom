# Root computed preferences and document UA for #296

The native/WASM CSS ABI now exposes explicit host font preferences and an additive
computed-document UA bundle. The old no-argument UA export remains byte-identical.
Raw validated preferences and exact identity stay separate from the normalized
font basis and internal shortest-round-trip dimension input. Private renderer
font/color defaults do not materialize public unmatched CSSOM properties.

A bounded numeric audit found nineteen one-ULP changes among 138 finite values in
the old JSON decoder. `serde_json/float_roundtrip` is enabled only by `css-backend`;
the plain native/WASM parser graphs and lockfile remain unchanged. Typed errors
and caller quotas propagate; identity/output copies are preflighted before retention.

The adjacent receipt records exact source/artifact hashes, integer sizes, tests,
strict checks and the preserved old UA. Actual native and Node/web WASM payloads
passed raw identity, normalization, quota and existing adapter controls. This does
not compile or validate the separate Native browser-env wrapper, switch npm's
runtime default, or add general per-property computed-value laziness. #296 stays open.
