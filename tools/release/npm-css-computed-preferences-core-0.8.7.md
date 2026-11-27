# Shared computed preferences: Core foundation for #296

This increment shares the native binding's existing raw host-font preference
validation and platform defaults without importing V8 or the optional CSS grammar
into default Core. The tiny generated font/color seed reads the existing measured
property registry; the full generated registry is unchanged.

The optional context builder keeps raw host preferences and exact identity inputs
separate from computed CSS values. Internal `font_size_input` preserves the
canonical f32 dimension basis with a shortest round-trip spelling. Public
`computed_font_size` keeps the existing six-significant CSSOM serializer. Fractional,
subnormal, finite-huge, exact-quota and one-under controls exercise the distinction.
Renderer-only initial font-family/color projection leaves public/ancestor maps sparse.

Combined with dependency `3051a51549b009b6025e7050cd5b9d7684aa39e7` (local equivalent
`03bd864923b709e7109dcf0ffabe637c61150004`), Core's default and optional-grammar
suites passed, strict all-target Clippy/format/CC9 passed, and both unchanged 100%
line floors passed. Exact source hashes and counts are in the adjacent JSON receipt.
The stock macOS Bash3.2 coverage helper stops on its empty feature array; its exact
Cargo arguments were run directly with JSON reporting added, preserving the LLVM
engine, include-files filter, timeout and 100% floor. No helper/floor was changed.

Root endpoints, the native wrapper, layout UA, Node profile assembly and the npm
default switch are excluded. This increment does not close #296 or claim complete
npm/native/WASM runtime parity. Clean committed-source checks follow the normal commit.
