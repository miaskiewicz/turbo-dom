# Publishing turbo-dom

The release version is shared by the scoped npm package `@miaskiewicz/turbo-dom`,
its native/WASM parser (`turbo-dom-parser`) and the Rust runtime crate `turbo-dom`.
The source of truth is [`.github/workflows/ci.yml`](./.github/workflows/ci.yml).
The repository has no Python package or PyPI publish workflow.

## Prepare and verify the release PR

1. Verify the current npm and crates.io versions, then choose the next patch.
2. Update `package.json`, both root entries in `package-lock.json`, the root parser
   and `crates/turbo-dom` Cargo manifests, their entries in `Cargo.lock`, the README
   release line and `CHANGELOG.md`. Rebuild both WASM packages so their generated
   `pkg/package.json` and `pkg-web/package.json` versions match.
3. Use the same stable Rust release as CI, including Clippy, rustfmt and the
   `wasm32-unknown-unknown` target. Install npm development dependencies.
4. Build and run the complete local CI gate:

   ```sh
   npm run build
   cargo test --lib
   cargo test -p turbo-dom
   cargo clippy --workspace --all-targets -- -D warnings
   cargo clippy -p turbo-dom-parser --all-targets --no-default-features --features wasm-bind -- -D warnings
   cargo clippy -p turbo-dom-parser --all-targets --no-default-features --features wasm-runtime -- -D warnings
   cargo test -p turbo-dom churn_alloc_gate -- --ignored --nocapture
   npm test
   npm run test:vitest
   npm run test:jest
   npm run conformance
   ```

   Check formatting on changed Rust files with rustfmt. The workflow does not
   currently gate repository-wide formatting. Run the allocation gate alone:
   its counting allocator is process-global.
   Inspect `npm pack --dry-run --ignore-scripts --json` and
   `cargo package -p turbo-dom --list` before publishing.
5. Commit without bypassing the configured pre-commit coverage/conformance gate,
   push the release PR, and wait for all build, test and WASM CI jobs to pass on
   that exact head revision. Release jobs are skipped for an untagged PR.

## turbo-dom-v8

`turbo-dom-v8` (150.4.x, upstream-derived version) is not part of the workspace: it is
generated from pinned inputs into `crates/turbo-dom-v8/crate/` and published first by
`crates/turbo-dom-v8/scripts/release.py --publish`, which only runs on this repository's
`v<crates/turbo-dom version>` tag and is idempotent by crate name and version (an
already-published 150.4.0 must match the fresh package byte-for-byte apart from Cargo's
generated lock/VCS records). See [crates/turbo-dom-v8/README.md](crates/turbo-dom-v8/README.md).
The root `turbo-dom-parser` package is `publish = false` (npm only).

## Merge, tag and monitor

Merge the green release PR with a **merge commit**. Resolve its actual merge SHA
from GitHub, fetch it locally and verify both merge parents. Create an annotated
`vX.Y.Z` tag pointing at that merge commit, then push the tag. Do not tag the PR's
branch commit. Pushing a `v*` tag is the publish trigger and requires explicit
release authorization.

The workflow builds Linux x64 (glibc/musl), Linux ARM64, macOS ARM64 and Windows
x64 native addons. The `publish` job downloads and bundles these binaries, rebuilds
both WASM targets through `prepublishOnly`, and publishes **one scoped npm bundle**
using the repository's `NPM_TOKEN`. The `publish-crate` job publishes the pure-Rust
`turbo-dom` crate using `CARGO_REGISTRY_TOKEN`. It does not publish the parser crate
or platform-specific npm subpackages. `turbo-dom-browser-env` (workspace member since
the 0.6.0 consolidation) is not yet in the publish chain: it publishes after
`turbo-dom-v8` and `turbo-dom` once that chain lands, and its `turbo-dom` path
dependency gains a version requirement then. Both publish jobs depend on native builds
and the full Linux test job.

Monitor the tag's workflow until both publish jobs succeed. Verify npm's `latest`
version and the published tarball's version, platform binaries and WASM metadata;
verify the same version is available from crates.io. Preserve failed job logs and
repair a concrete failure before retrying. Registry versions cannot be overwritten.
