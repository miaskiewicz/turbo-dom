# Shared CSSOM integration checkpoint

Optional native/WASM CSS payload foundation21c and atomic CSSOM projectionb636
were integrated on the current validated v0.8.7 producer graph. Runtime defaults,
HTML parser defaults and compatibility controls are unchanged. This is an
incremental #296 integration, not its completion or a default cutover.

The integrated pipeline passes default workspace tests, Core computed-style
feature tests, native CSS root tests, strict workspace/native-CSS all-target
Clippy, and browser binding presentation/snapshot/transition controls with the
shared Core feature enabled. Rust1.98 fmt, configured RustCC9 and property
registry generation checks pass. Local exact logs and command receipts:
`.evidence/npm-cssom-integration/{PIPELINE,RESULT}.json`.

The source checkpoint has measured unchanged Core100% floors:
default14307/14307 and optional17460/17460. Actual native and Node/web WASM
15-case replays complete with authored values unchanged; six preserved legacy
CSSOM contract differences are disclosed in the source receipt rather than
claimed as full Chrome parity. See `npm-cssom-projection-0.8.7.md` and its JSON.

Used-size CSSOM parity, trusted runtime wiring and measured performance
acceptance remain separate blockers before switching the npm runtime default.
No publishing, tags or registry packages are created by this integration.
