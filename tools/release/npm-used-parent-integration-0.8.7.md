# Incremental used-value integration

The shared Core/requested-property used-value projection from `c018bd56` is
integrated without switching the npm runtime default or installing the private
proxy/orchestration. Parent integration passes Core6/6, native CSS51/51 and
strict native-CSS all-target Clippy. Source checkpoint floors are unchanged100%:
default14307/14307 and optional17526/17526; committed-source verification and
exact native/WASM artifact receipts are in `npm-css-used-values-0.8.7.json`.

Refreshed original18, compatibility15 and used-inheritance4 cases pass on each
of native, Node WASM and web ESM (111checks), preserving authored values and
zero Scene calls for color, one for first width, zero repeated-width calls.
The integrated working log is `.evidence/npm-used-integration/PIPELINE.json`.
This checkpoint does not complete#296: runtime assembly, compatibility and
performance acceptance remain held. It requires no Surf consumer repin.
