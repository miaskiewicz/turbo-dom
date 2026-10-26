# turbo-dom-v8

`turbo-dom-v8` 150.4.0 is turbo-dom's fork of [rusty_v8](https://github.com/denoland/rusty_v8)
(crates.io `v8` 150.4.0, upstream revision `5c15a6995c9bb4bacd3e341b59fff32c909c80bf`).
It links the **official prebuilt** rusty_v8 v150.4.0 static library and generated
bindings unchanged; only Rust sources differ. The library is still named `v8`
(`use v8::...`), and the package declares `links = "rusty_v8"`, so a graph can hold only
one copy. It supersedes the `turbo-surf-v8` crates (149.3.0 in turbo-test, 150.4.0 in
turbo-surf), which are left untouched on crates.io. Consumers require `~150.4`.

## Fork delta

| change | where | why |
|---|---|---|
| Interceptor `ShouldThrowOnError` | `src/function.rs` | Upstream `PropertyCallbackArguments::should_throw_on_error` reads the mode through a `PropertyCallbackInfo<Value>` wrapper that V8 folds to `false`. The fork records whether the callback type carries the slot (setter/definer: `()`, deleter: `Boolean`), reads the Smi after the holder slot and resolves the inferred mode through V8's exported `v8::internal::ShouldThrowOnError(Isolate*)`. Strict-mode `[[Set]]`/`[[Delete]]` and `Object.defineProperty` now report `true`, as in Chrome. |
| Isolate weak teardown | `src/isolate.rs` | Upstream invalidates its `IsolateHandle` before dropping native isolate/context slots, so a `Weak` destructor skips V8's global-handle reset while its callback data can already be freed. The handle now stays live through slot and finalizer cleanup and is invalidated immediately before V8 disposal. |
| `UPSTREAM_V8_VERSION` | `build.rs` | The prebuilt archive URL and the `RUSTY_V8_MIRROR` binding URL use `const UPSTREAM_V8_VERSION: &str = "150.4.0"` instead of `CARGO_PKG_VERSION`, so fork patch releases (150.4.x) keep downloading the upstream v150.4.0 SDK. A `build.rs` unit test (`cargo test --test build`) asserts it equals the manifest's pinned `upstream-version`. |
| Fork marker | `src/lib.rs` | `pub const FORK_SHOULD_THROW_FIX: bool = true;` Bindings that rely on the accessor write `const _: () = assert!(v8::FORK_SHOULD_THROW_FIX);` so a stock `v8` cannot be substituted silently. |
| Lint allows | `src/lib.rs` | `#![allow(warnings, clippy::all, clippy::pedantic)]`: upstream code is kept as-is, so consumers' lint gates do not apply to it. |
| Release teardown tests | `tests/test_api.rs` | turbo-surf's reviewed `v8-teardown-tests.patch`: corrects upstream's contradictory slot-liveness assertion and adds weak-slot retirement controls. |
| Lib test init | `src/array_buffer.rs` (test only) | Upstream's `test_default_allocator` initialized V8 itself, so whichever other lib test initialized second panicked; it now shares the crate's `Once`. |
| Manifest | `Cargo.toml` | Renamed `turbo-dom-v8`; `links = "rusty_v8"`; `rust-version = "1.91"`; `LICENSE` kept in the package; repository/description name the fork; `[package.metadata.turbo-dom-v8]` records the upstream package, version, revision and repository; `doctest = false` (upstream doc examples do not compile); the `test_api`, `test_cppgc` (need `third_party/icu/common/icudtl.dat`, not shipped) and `test_ui` (trybuild) targets and their only dev-dependencies (`align-data`, `rustversion`, `trybuild`) removed; explicit `[[test]]` targets for our tests (upstream sets `autotests = false`). `Cargo.toml.orig`, `Cargo.lock`, `.cargo_vcs_info.json` and `rust-toolchain.toml` are dropped. |
| Regression tests | `tests/` | `interceptor_should_throw.rs` (4 tests: setters, definers, deleters, getters), `release_v8_teardown.rs` (2), `weak_teardown.rs` (1), `fork_marker.rs` (1). On stock function.rs three interceptor tests fail. |

## What is in this directory

Only the inputs: no V8 or rusty_v8 source is committed.

```
pins.json               upstream identity, the pinned input tree, patch hashes, generated-tree hash
patches/*.patch         the turbo-dom deltas above (build.rs, lib.rs, test_api.rs, array_buffer.rs)
tests/*.rs              our regression tests (copied into the generated crate)
rustfmt.toml            upstream's 2-space style, also copied into the crate
scripts/generate.py     generator (+ test_generate.py)
scripts/release.py      package / idempotent publish (+ test_release.py)
scripts/push-candidate.sh  push the generated crate to the PR candidate branch
crate/                  generated output (gitignored)
```

The input is turbo-surf's already-reviewed rusty_v8 150.4.0 source:
`rust/vendor/rusty_v8` in miaskiewicz/turbo-surf at `b1de4eb` (the published
`v8-150.4.0.crate` Rust subset plus turbo-surf's `rusty-v8-150.patch`, which carries the
function.rs and isolate.rs fixes and the MIT `LICENSE`). `pins.json` pins every input
file (count + tree hash) and the individual function.rs (`b12f0237…`), isolate.rs and
LICENSE hashes.

## Generate and check

```sh
export TURBO_DOM_V8_INPUT=/path/to/turbo-surf/rust/vendor/rusty_v8
python3 crates/turbo-dom-v8/scripts/generate.py --write    # -> crates/turbo-dom-v8/crate/
python3 crates/turbo-dom-v8/scripts/generate.py --check    # regenerate + compare with crate/ and pins
python3 crates/turbo-dom-v8/scripts/generate.py --verify DIR  # hash-check a generated crate (no input needed)
python3 -m unittest discover -s crates/turbo-dom-v8/scripts -p 'test_*.py'
```

After reviewing a change to `patches/`, `tests/` or the generator, accept it with
`generate.py --write --update-pins` (input pins never change this way). Nothing here
downloads V8 or builds it from source.

## Build and test

The generated crate is a standalone package: the root workspace excludes
`crates/turbo-dom-v8`, so a fresh checkout's workspace (and its `-p turbo-dom
-p turbo-dom-parser` gates) resolves without generating V8 first. Point
`RUSTY_V8_ARCHIVE` at the matching prebuilt static library and use one target directory
per variant:

```sh
cd crates/turbo-dom-v8/crate
RUSTY_V8_ARCHIVE=.../librusty_v8_release_<target>.a CARGO_TARGET_DIR=target-default \
  cargo test
RUSTY_V8_ARCHIVE=.../librusty_v8_simdutf_release_<target>.a CARGO_TARGET_DIR=target-simdutf \
  cargo test --features simdutf
cargo test --test interceptor_should_throw -- --list --ignored   # 0 tests
nm <target>/debug/deps/interceptor_should_throw-* | grep _ZN2v88internal18ShouldThrowOnErrorEPNS0_7IsolateE
```

Without `RUSTY_V8_ARCHIVE`, build.rs downloads the upstream v150.4.0 archive as upstream
does. `V8_FROM_SOURCE` is unsupported: the crate ships no C++ sources.

## Pull-request candidates

`scripts/push-candidate.sh` verifies `crate/` against `pins.json` and commits it to the
branch `consolidation/turbo-dom-v8-candidate` of miaskiewicz/turbo-dom (orphan history,
crate at the branch root, one commit per change, never force-pushed). Consumer PRs patch
it by revision before the crate is on crates.io:

```toml
[patch.crates-io]
turbo-dom-v8 = { git = "https://github.com/miaskiewicz/turbo-dom", rev = "<printed sha>" }
```

## Release

`scripts/release.py --out DIR` verifies `crate/` against the pins, copies it, runs
`cargo package --no-verify --exclude-lockfile` and checks the archive identity, `LICENSE`
and reviewed sources. With `--publish` (only on `refs/tags/v<crates/turbo-dom version>` in
miaskiewicz/turbo-dom, using the generated `crate/`) it publishes `turbo-dom-v8 150.4.0`
once: if that version already exists, the published archive must match the fresh package
(only Cargo's `Cargo.lock`/`.cargo_vcs_info.json` may differ) and nothing is uploaded.
turbo-dom-v8 publishes first in the turbo-dom release chain.
