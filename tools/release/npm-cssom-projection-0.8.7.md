# Shared CSSOM projection foundation — v0.8.7

The optional shared CSS backend now projects canonical computed longhands through CSSOM aliases and complete authored shorthands. It preserves the existing npm partial public view and declared text-decoration profile. Authored declarations remain raw. The npm runtime default is unchanged.

Font weights normalize before inheritance. Unitless line height remains computed until presentation uses the actual own font scalar. Source-hashed Chrome 159 controls establish the font-size clamp before inheritance/em bases; this changes no execution or resource budget.

The new stateless native/WASM `serializeCssomV1` endpoint accepts explicit limits and selected decoration value/origin. Cascade ranking stays inside the resolver: its usize maximum cannot safely round-trip through JavaScript numbers and is unnecessary for presentation. Original failing and corrected payload receipts remain repository-local.

Both original Core floors pass: default 14,307/14,307 and optional grammar 17,460/17,460. Strict Core/native/WASM checks, formatting and CC9 pass. All 15 compatibility cases execute in native, Node WASM and web ESM modes with identical results and unchanged authored styles. Actual Chrome also loads and exercises the web payload; native loading works with WebAssembly unavailable. Six explicit property differences preserve existing partial/default and declared-decoration behavior.

Quota preflight now reuses the stylesheet source guard, and measured container values retain their validated standard feature identity. Defensive syntax/resource paths remain tested; no floor or exclusion changed. A mixed host/ordinary selector-list control proves per-item tree routing. The equivalent Option mapping avoids a standalone zero-hit else-keyword trace whose None return was already exercised.

`#296` remains open. Canonical Scene already measures percentage inheritance correctly, but the runtime still needs requested-property used-size finalization and the remaining default, compatibility and performance gates. The separate trusted-parser/performance prototypes are outside this increment.
