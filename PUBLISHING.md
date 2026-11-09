# Publishing turbo-dom

One `vX.Y.Z` tag releases every package of this repository:

| package | registry | version |
|---|---|---|
| `turbo-dom-v8` (generated rusty_v8 fork) | crates.io | upstream-derived, 150.4.x |
| `turbo-dom` | crates.io | X.Y.Z |
| `turbo-dom-layout` | crates.io | X.Y.Z |
| `turbo-dom-raster` | crates.io | X.Y.Z |
| `turbo-dom-browser-env` | crates.io | X.Y.Z |
| `@miaskiewicz/turbo-dom` (JS runtime + native/WASM `turbo-dom-parser`) | npm | X.Y.Z |

The root `turbo-dom-parser` crate is `publish = false` (it ships inside the npm package).
The source of truth is [`.github/workflows/publish.yml`](./.github/workflows/publish.yml),
which first runs the whole [`ci.yml`](./.github/workflows/ci.yml) on the tagged commit.
The repository has no Python package or PyPI publish workflow. Architecture, dependency
graph and consumer setup: [docs/CONSOLIDATION.md](docs/CONSOLIDATION.md).

## Prepare and verify the release PR

1. Verify the current npm and crates.io versions, then choose the next version.
2. Update `package.json`, both root entries in `package-lock.json`, the root parser
   manifest and the four `crates/*/Cargo.toml` versions, the `version = "X.Y.Z"` of every
   intra-workspace path dependency (turbo-dom-layout -> turbo-dom, turbo-dom-raster ->
   turbo-dom-layout, turbo-dom-browser-env -> turbo-dom), `Cargo.lock`, the README release
   line and `CHANGELOG.md`. `python3 scripts/check-versions.py` must pass. Rebuild both
   WASM packages so their generated `pkg/package.json` and `pkg-web/package.json`
   versions match. `turbo-dom-v8` keeps its upstream-derived version unless the fork
   itself changes (then bump the patch in `crates/turbo-dom-v8/pins.json`, see its README).
3. Use the CI toolchain (Rust 1.98.0 with Clippy, rustfmt, `wasm32-unknown-unknown`;
   Rust 1.89 for the MSRV check) and install npm development dependencies. Point
   `RUSTY_V8_ARCHIVE` at a local prebuilt rusty_v8 150.4.0 static library for the V8 crates.
4. Run the local gate (the CI jobs in `ci.yml` are the complete list):

   ```sh
   npm run build
   cargo test -p turbo-dom-parser --lib
   cargo test -p turbo-dom
   cargo test -p turbo-dom churn_alloc_gate -- --ignored --nocapture   # alone: process-global allocator
   cargo clippy -p turbo-dom -p turbo-dom-parser --all-targets -- -D warnings
   cargo clippy -p turbo-dom-parser --all-targets --no-default-features --features wasm-bind -- -D warnings
   cargo clippy -p turbo-dom-parser --all-targets --no-default-features --features wasm-runtime -- -D warnings
   cargo test -p turbo-dom-layout && cargo test -p turbo-dom-layout --features svg && cargo test -p turbo-dom-layout --features perf
   python3 scripts/check-baseline-red.py scripts/baseline-red/turbo-dom-layout-no-default-features.tsv -- -p turbo-dom-layout --no-default-features
   cargo test -p turbo-dom-raster
   python3 crates/turbo-dom-v8/scripts/generate.py --download --write
   (cd crates/turbo-dom-v8/crate && cargo test)
   cargo test -p turbo-dom-browser-env --features host-internals,test-support
   sh scripts/check-no-selector-dual.sh && python3 scripts/check-versions.py
   npm test && npm run test:vitest && npm run test:jest && npm run conformance
   ```

5. Inspect the packages: `npm pack --dry-run --ignore-scripts --json`;
   `cargo package -p turbo-dom -p turbo-dom-layout -p turbo-dom-raster` (packages and
   verifies the three together through Cargo's local overlay of unpublished siblings);
   `cargo package -p turbo-dom-browser-env --no-verify --exclude-lockfile` (it cannot
   resolve until turbo-dom-v8 is on crates.io); `python3 crates/turbo-dom-v8/scripts/release.py
   --out <dir>` for the fork. Every crate must stay under crates.io's 10 MB compressed limit.
   Record the release's public Rust API diff against the previous release in
   `tools/release/public-api-X.Y.Z.txt` (`cargo public-api -sss`, CI's features; the
   header names the tool version and both revisions) and make the CHANGELOG's
   "Breaking changes" match it. 0.7.0 also keeps its review-round mutation audit in
   `tools/release/mutation-audit-0.7.0.md`.
6. Commit without bypassing the configured pre-commit coverage/conformance gate, push
   the release PR, and wait for every CI job to pass on that exact head revision.

## Merge, tag and monitor

Merge the green release PR with a **merge commit**. Resolve its actual merge SHA from
GitHub, fetch it locally and verify both merge parents. Create an annotated `vX.Y.Z` tag
pointing at that merge commit, then push the tag. Do not tag the PR's branch commit.
Pushing a `v*` tag is the publish trigger and requires explicit release authorization.

`publish.yml` then runs, in order:

1. **ci** — the full CI workflow on the tag (napi binaries for linux x64 glibc/musl,
   linux arm64, macOS arm64 and windows x64 are uploaded by its build matrix). Linux
   binaries build on `ubuntu-22.04` / `ubuntu-22.04-arm` (glibc 2.35), never
   `ubuntu-latest`: `scripts/check-glibc-ceiling.sh` fails any linux-gnu `.node` that needs
   a `GLIBC_` version above 2.35 (0.6.0 was built on 26.04 / glibc 2.43 and broke 24.04
   consumers). The guard names the expected addons and fails on a missing file, an
   `readelf` failure or a binary with no `GLIBC_` symbols, so it never passes vacuously.
   It uses `readelf -W --dyn-syms -V`, which reads the arm64 addon on the x64 npm runner
   (`objdump -T` fails there with "file format not recognized").
2. **preflight** — `check-versions.py --tag vX.Y.Z` (the tag equals every manifest
   version) and `check-no-selector-dual.sh`.
3. **crates** (sequential, `CARGO_REGISTRY_TOKEN`):
   - generate `turbo-dom-v8`, then `crates/turbo-dom-v8/scripts/release.py --publish`
     (only on `refs/tags/v<crates/turbo-dom version>` in miaskiewicz/turbo-dom; an
     already-published 150.4.x must match the fresh package byte-for-byte apart from
     Cargo's generated lock/VCS records, and nothing is uploaded);
   - wait until the crates.io index lists it, drop the root manifest's pre-publish
     `[patch.crates-io]` (the candidate branch) and re-lock `turbo-dom-v8` from crates.io;
   - `scripts/publish-crates.py publish`: turbo-dom -> turbo-dom-layout ->
     turbo-dom-raster -> turbo-dom-browser-env. A version already on crates.io is
     skipped; each published crate is awaited in the index before the next starts, so
     the next one's packaged-consumer verification resolves it.
4. **npm** (`NPM_TOKEN`, after crates) — bundles the ci job's napi artifacts into the
   one scoped package (no per-platform subpackages). It fails unless all five addons are
   there (linux x64 gnu/musl, linux arm64 gnu, darwin arm64, win32 x64) and no other
   `.node` is in the package root. It builds no native addon: it runs only
   `build:wasm:pkg` and `build:wasm:web`, never `prepublishOnly`, which would rebuild the
   linux-x64 addon over the CI-built one. It reruns the glibc guard on the two linux-gnu
   addons that ship, then runs `npm publish --ignore-scripts`. The job is skipped when that
   version is already on npm.
5. **napi-prebuilts** (`contents: write`, after ci + preflight; turbo-dom#479) — downloads
   the same `bindings-*` artifacts the ci job uploaded and attaches
   `turbo-dom-parser.<platform>.node` for `darwin-arm64`, `linux-x64-gnu`,
   `linux-arm64-gnu` and `win32-x64-msvc` to the GitHub release for the tag. turbo-surf's
   release pipeline downloads these prebuilts so its playwright-shim's napi path does not
   need a local Rust build or `TURBO_SURF_NAPI_PATH`.

A failed run is resumed by re-running the failed jobs: every publish step is guarded
against an existing version. Verify npm's `latest` version and tarball contents (platform
binaries, WASM metadata) and that every crate version is listed on crates.io. Preserve
failed job logs and repair a concrete failure before retrying. Registry versions cannot
be overwritten.

After the tag: consumers drop their pre-publish `[patch.crates-io]` sections and re-lock
(docs/CONSOLIDATION.md, "How consumers depend on the crates"), in the order turbo-html2pdf
/ turbo-test / turbo-surf release-deps, then turbo-surf.

## turbo-dom-v8

`turbo-dom-v8` is not part of the workspace: it is generated from pinned inputs into the
gitignored `crates/turbo-dom-v8/crate/` and published first by `release.py --publish`
from the release workflow. See [crates/turbo-dom-v8/README.md](crates/turbo-dom-v8/README.md).
