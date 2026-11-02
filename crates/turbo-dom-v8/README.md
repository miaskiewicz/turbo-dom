# turbo-dom-v8

turbo-dom's fork of [rusty_v8](https://github.com/denoland/rusty_v8): crates.io `v8` 150.4.0
(upstream revision `5c15a6995c9bb4bacd3e341b59fff32c909c80bf`) plus a small set of reviewed
Rust-only fixes. It links the **official prebuilt** rusty_v8 v150.4.0 static library and
generated bindings unchanged. The library name stays `v8` (`use v8::...`).

The crate is **generated, not vendored**: this directory holds only pins, patches, tests and
scripts; the crate is written to the gitignored `crate/` and published from the release
workflow.

## Where it sits

```
turbo-dom-v8 ──► turbo-dom-browser-env ──► turbo-test, turbo-surf
      └────────► turbo-surf (its Deno family links the same fork)
```

`links = "rusty_v8"`, so a dependency graph holds at most one `turbo-dom-v8`; consumers require
`~150.4`. Stock `v8` declares no `links` key, so Cargo does not stop it from coexisting with the
fork (both would link a `librusty_v8`): keep stock `v8` out of the graph (`cargo tree -i v8`).
It replaces the `turbo-surf-v8` crates (149.3.0 in turbo-test, 150.4.0 in turbo-surf), which
stay on crates.io untouched. Part of the [turbo-dom](https://github.com/miaskiewicz/turbo-dom)
0.6.0 release (see [docs/CONSOLIDATION.md](../../docs/CONSOLIDATION.md)).

## Install

```toml
[dependencies]
v8 = { package = "turbo-dom-v8", version = "~150.4" }
```

MSRV 1.91. Set `RUSTY_V8_ARCHIVE` to a prebuilt `librusty_v8[_simdutf]_release_<target>.a`
(`.lib` on Windows) to skip the download; without it build.rs downloads the upstream
v150.4.0 archive. `V8_FROM_SOURCE` is unsupported (no C++ sources ship).

## Features

Upstream rusty_v8's features, unchanged (CI tests the default build and `simdutf`). The fork
adds no features.

## API

Upstream rusty_v8 150.4.0's API, plus one item: `pub const FORK_SHOULD_THROW_FIX: bool = true`.
Bindings that rely on the fixed interceptor accessor assert it at compile time
(`const _: () = assert!(v8::FORK_SHOULD_THROW_FIX);`), so stock `v8` cannot be substituted
silently.

### Fork delta

| change | where | why |
|---|---|---|
| Interceptor `ShouldThrowOnError` | `src/function.rs` | Upstream `PropertyCallbackArguments::should_throw_on_error` reads the mode through a `PropertyCallbackInfo<Value>` wrapper that V8 folds to `false`. The fork records whether the callback type carries the slot (setter/definer: `()`, deleter: `Boolean`), reads the Smi after the holder slot and resolves the inferred mode through V8's exported `v8::internal::ShouldThrowOnError(Isolate*)`. Strict-mode `[[Set]]`/`[[Delete]]` and `Object.defineProperty` now report `true`, as in Chrome. |
| Isolate weak teardown | `src/isolate.rs` | Upstream invalidates its `IsolateHandle` before dropping native isolate/context slots, so a `Weak` destructor skips V8's global-handle reset while its callback data can already be freed. The handle now stays live through slot and finalizer cleanup and is invalidated immediately before V8 disposal. |
| `UPSTREAM_V8_VERSION` | `build.rs` | The prebuilt archive URL and the `RUSTY_V8_MIRROR` binding URL use `const UPSTREAM_V8_VERSION: &str = "150.4.0"` instead of `CARGO_PKG_VERSION`, so fork patch releases (150.4.x) keep downloading the upstream v150.4.0 SDK. A `build.rs` unit test (`cargo test --test build`) asserts it equals the manifest's pinned `upstream-version`. |
| Fork marker | `src/lib.rs` | `pub const FORK_SHOULD_THROW_FIX: bool = true;` Bindings that rely on the accessor write `const _: () = assert!(v8::FORK_SHOULD_THROW_FIX);` so a stock `v8` cannot be substituted silently. |
| Lint allows | `src/lib.rs` | `#![allow(warnings, clippy::all, clippy::pedantic)]`: upstream code is kept as-is, so consumers' lint gates do not apply to it. |
| Release teardown tests | `tests/test_api.rs` | turbo-surf's reviewed `v8-teardown-tests.patch`: corrects upstream's contradictory slot-liveness assertion and adds weak-slot retirement controls. `test_api` is not a test target here (it needs `icudtl.dat`), so these run only in turbo-surf; `release_v8_teardown.rs` carries the same retirement controls. |
| Lib test init | `src/array_buffer.rs` (test only) | Upstream's `test_default_allocator` initialized V8 itself, so whichever other lib test initialized second panicked; it now shares the crate's `Once`. |
| Doc examples | `src/scope.rs` (docs only) | `doctests.patch` (turbo-dom#182): upstream never runs its doctests and 6 of the 12 in `scope.rs` do not compile (duplicate `Object` import, no `isolate` in scope, `Number::new(scope, 42)`, a `;` dropping a return value). They now set up V8 (hidden `#` lines) and run; `cargo test --doc` is a CI step. |
| Manifest | `Cargo.toml` | Renamed `turbo-dom-v8`; `links = "rusty_v8"`; `rust-version = "1.91"`; `LICENSE` kept in the package; repository/description name the fork; `[package.metadata.turbo-dom-v8]` records the upstream package, version, revision and repository; the `test_api`, `test_cppgc` (need `third_party/icu/common/icudtl.dat`, not shipped) and `test_ui` (trybuild) targets and their only dev-dependencies (`align-data`, `rustversion`, `trybuild`) removed; explicit `[[test]]` targets for our tests (upstream sets `autotests = false`). `Cargo.toml.orig`, `Cargo.lock`, `.cargo_vcs_info.json` and `rust-toolchain.toml` are dropped. |
| Regression tests | `tests/` | `interceptor_should_throw.rs` (4 tests: setters, definers, deleters, getters), `release_v8_teardown.rs` (2), `weak_teardown.rs` (1), `fork_marker.rs` (1). On stock function.rs three interceptor tests fail. |

## Generate

Generation has three checked stages:

1. **Upstream archive**: `v8-150.4.0.crate` from static.crates.io (sha256 `42a978ff…`),
   taken from `--archive PATH`, else Cargo's registry cache, else fetched with
   `--download` into `.download/`. Only its Rust subset is extracted (root files plus
   `src gen tools tests benches examples`; 186 files, no C++ trees); the stock function.rs
   and isolate.rs hashes are verified.
2. **Reviewed fixes**: `patches/rusty-v8-150.patch`, moved verbatim from turbo-surf's
   `rust/vendor/rusty-v8-150.patch` (b1de4eb). The resulting 187-file tree, including
   function.rs `b12f0237…`, isolate.rs `4eb4214b…` and LICENSE, is pinned; it is
   byte-identical to the tree turbo-surf vendored at that commit (`--compare DIR` checks this
   against such a tree).
3. **turbo-dom deltas**: the remaining patches, the manifest transform, reserved-file
   removal, our tests and rustfmt.toml; the 188-file result is pinned.

```sh
python3 crates/turbo-dom-v8/scripts/generate.py --download --write   # -> crates/turbo-dom-v8/crate/
python3 crates/turbo-dom-v8/scripts/generate.py --download --check   # regenerate; compare with pins
python3 crates/turbo-dom-v8/scripts/generate.py --verify DIR         # hash-check a generated crate
python3 -m unittest discover -s crates/turbo-dom-v8/scripts -p 'test_*.py'
```

After reviewing a change to `patches/`, `tests/`, this README (shipped as the package README,
relative links made absolute) or the generator, accept it with
`generate.py --write --update-pins`.

## Testing

The generated crate is standalone (the root workspace excludes `crates/turbo-dom-v8`). Use one
target directory per archive variant:

```sh
cd crates/turbo-dom-v8/crate
RUSTY_V8_ARCHIVE=.../librusty_v8_release_<target>.a CARGO_TARGET_DIR=target-default \
  cargo nextest run --all-targets
RUSTY_V8_ARCHIVE=.../librusty_v8_simdutf_release_<target>.a CARGO_TARGET_DIR=target-simdutf \
  cargo nextest run --all-targets --features simdutf
cargo test --doc                                                   # doctests (nextest skips them)
cargo test --test interceptor_should_throw -- --list --ignored   # 0 tests
nm <target>/debug/deps/interceptor_should_throw-* | grep _ZN2v88internal18ShouldThrowOnErrorEPNS0_7IsolateE
```

Run the suite with cargo-nextest, as upstream rusty_v8's CI does: each test gets its own
process. Upstream's tests (slots.rs and others) are not safe to share one process, and a
plain parallel `cargo test` aborts now and then (libc++ hardening `vector[] index out of
bounds`, SIGSEGV) on the unpatched upstream 150.4.0 crate as well.

Without `RUSTY_V8_ARCHIVE`, build.rs downloads the upstream v150.4.0 archive as upstream
does. `V8_FROM_SOURCE` is unsupported: the crate ships no C++ sources.

## Gates

- CI job `v8 + browser-env` (`.github/workflows/ci.yml`): regenerates the crate, checks it
  against `pins.json`, runs the suite with cargo-nextest plus `cargo test --doc`, on linux
  x64/arm64 and windows x64 × default/simdutf (macOS arm64 added in the release run), then
  builds and tests turbo-dom-browser-env against it.
- `guards`: generator and release-script self-tests; `generate.py --check`.
- No tarpaulin floor: the fork delta is covered by its regression tests above (the upstream
  code is not ours). Complexity checks never scan the generated `crate/` (not tracked by git).

## Pre-publish candidates and release

`scripts/push-candidate.sh` commits the verified `crate/` to the orphan branch
`consolidation/turbo-dom-v8-candidate`; consumer PRs use
`turbo-dom-v8 = { git = "https://github.com/miaskiewicz/turbo-dom", rev = "<sha>" }` under
`[patch.crates-io]` until the crate is on crates.io.

`scripts/release.py --out DIR` packages and checks the crate; with `--publish` (only on a
`v<turbo-dom version>` tag of miaskiewicz/turbo-dom) it publishes `turbo-dom-v8 150.4.0` once.
An existing 150.4.0 must match the fresh package apart from Cargo's lock/VCS records, and then
nothing is uploaded. It publishes first in the chain ([PUBLISHING.md](../../PUBLISHING.md)).
A fork-only change bumps the patch version in `pins.json`; `UPSTREAM_V8_VERSION` keeps the
SDK download at 150.4.0.

## Links

- [turbo-dom-browser-env](../turbo-dom-browser-env/README.md) — the main consumer
- [docs/CONSOLIDATION.md](../../docs/CONSOLIDATION.md) — crate map, dependency graph, publish order
- [PUBLISHING.md](../../PUBLISHING.md) · [CHANGELOG.md](../../CHANGELOG.md)

License: MIT (upstream rusty_v8 license kept in the package).
