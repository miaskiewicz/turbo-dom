# Computed CSS output foundation

Admitted background/text-decoration shorthands can request their measured context-independent reset defaults without materializing an unrelated UA map. Background position axes are derived from the same measured initial shorthand through the existing canonical expansion. The first actual run failed on the missing X axis; the corrected source passes all focused controls.

Declared text-decoration provenance is retained separately from computed longhands, using the existing candidate rank and preserving raw UTF-16 pending values, origin, importance and order. It does not publish a marker or change a public compatibility mode.

Full feature Core tests: 796 passed, 9 ignored; default Core: 574 passed, 9 ignored. Strict feature Clippy, formatting, CC9 and the generated metadata check pass. Actual source-hashed Chrome159 controls are included for background/decoration values; declared-marker controls also preserve the separately named jsdom27 fixture.

No npm runtime default changes. No newer native/WASM payload is claimed. Coverage on this source remains unmeasured; the prior safety floor is still red. The runtime CSSOM projection, portable Buffer provenance, measured production performance and final acceptance gates remain required before closing #296.
