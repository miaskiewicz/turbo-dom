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

`turbo-dom-rav1d` (the generated rav1d fork behind turbo-dom-raster's `avif` feature) is
not released by the tag. It is published by hand, only when its pins change, before any
turbo-dom-raster that needs it. See [turbo-dom-rav1d](#turbo-dom-rav1d) below.

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
   parser and lazy CSS WASM packages so generated `pkg/package.json`,
   `pkg-web/package.json`, `pkg-css/package.json` and `pkg-css-web/package.json`
   versions match. `turbo-dom-v8` keeps its upstream-derived version unless the fork
   itself changes (then bump the patch in `crates/turbo-dom-v8/pins.json`, see its README).
3. Use the CI toolchain (Rust 1.98.0 with Clippy, rustfmt, `wasm32-unknown-unknown`;
   Rust 1.89 for the MSRV check), Python 3.11 or newer for the artifact checker,
   and install npm development dependencies. Point
   `RUSTY_V8_ARCHIVE` at a local prebuilt rusty_v8 150.4.0 static library for the V8 crates.
4. Run the local gate (the CI jobs in `ci.yml` are the complete list):

   ```sh
   npm run build                  # parser-only build, unchanged default features
   npm run build:css              # separate native-css / pkg-css / pkg-css-web
   npm run check:css:host          # real native + Node WASM ABI; no fallback
   cargo test -p turbo-dom-parser --lib
   cargo test -p turbo-dom
   cargo test -p turbo-dom churn_alloc_gate -- --ignored --nocapture   # alone: process-global allocator
   cargo clippy -p turbo-dom -p turbo-dom-parser --all-targets -- -D warnings
   cargo clippy -p turbo-dom-parser --all-targets --no-default-features --features wasm-bind -- -D warnings
   cargo clippy -p turbo-dom-parser --all-targets --no-default-features --features wasm-runtime -- -D warnings
   cargo clippy -p turbo-dom-parser --all-targets --no-default-features --features napi-bind,css-backend -- -D warnings
   cargo clippy -p turbo-dom-parser --all-targets --no-default-features --features wasm-bind,css-backend -- -D warnings
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

## 0.8.7 consumer preparation

The candidate remains on `release/0.8.7` until its release PR is approved.
Consumers pin one immutable producer revision for every DOM-family patch and
refresh their Cargo lockfiles together. A successful build against one family
member does not establish a coherent dependency graph.

Review exhaustive `CascadeOrigin` matches for `Animation` and `Transition`,
and prefer `Fragment::new` when constructing layout fragments. The new public
`establishes_stacking_context` field must be supplied by struct literals.
See the 0.8.7 changelog and its public API report for the full migration surface.

The npm `domBackend` flag defaults to `'js'`. This candidate reports a setup error
for `'rust'` because the Node browser-env host port is unavailable. Packed
Rust CSS experiments in the README are explicitly identified by artifact and
source revision; they do not change the release branch's default CSS resolver.

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
4. **npm** (`NPM_TOKEN`, after crates) — bundles the ci job's parser and lazy CSS
   napi artifacts into one scoped package. It requires exactly five parser addons
   in the package root and five matching `native-css/turbo-dom-css.<platform>.node`
   addons (linux x64 gnu/musl, linux arm64 gnu, darwin arm64, win32 x64). Unexpected,
   duplicate, missing or empty native artifacts fail before copying. It builds no
   native addon: only `build:wasm:pkg`, `build:wasm:web`, `build:css:wasm:pkg` and
   `build:css:wasm:web`, never `prepublishOnly`. `check:artifacts` then verifies the
   complete native/WASM file set, all four WASM package versions and WASM headers,
   and records exact byte sizes/SHA256s. The glibc guard covers parser and CSS GNU
   addons. `npm publish --ignore-scripts` ships those reviewed files; the job is
   skipped when that version already exists on npm.
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

## turbo-dom-rav1d

`turbo-dom-rav1d` is rav1d 1.1.0 with two changes: upstream's fix for
memorysafety/rav1d#1497 (`patches/on-error-frame-hdr.patch`), and every `extern "C"`
turned into `extern "C-unwind"` (159 sites). With both, turbo-dom-raster can catch a
decoder panic and paint a broken image instead of the process aborting (turbo-dom#535).
[crates/turbo-dom-rav1d/README.md](crates/turbo-dom-rav1d/README.md) has the details.
No rav1d source is committed. `scripts/generate.py` rebuilds the crate from the
checksum-pinned upstream `rav1d-1.1.0.crate`, and CI (`guards`) checks that it reproduces
the pinned tree (`generate.py --download --check`).

It is outside the workspace and outside the tag release. turbo-dom-raster depends on the
published version (`rav1d = { package = "turbo-dom-rav1d", version = "=X" }`), so a new
version must be on crates.io before the turbo-dom-raster change that uses it is pushed.
1.1.0 was published on 2026-11-19 by this procedure.

1. Change the patches or the transform, add a test to `scripts/test_generate.py`, and bump
   `version` in `crates/turbo-dom-rav1d/pins.json`. A published version is immutable, so
   every change needs a new version.
2. Accept the reviewed tree and run the self-tests:
   ```bash
   python3 crates/turbo-dom-rav1d/scripts/generate.py --write --update-pins
   python3 -m unittest discover -s crates/turbo-dom-rav1d/scripts -p 'test_*.py'
   ```
3. Generate a copy **outside the repository**. Inside it, and above all inside a git
   worktree, cargo walks up into the turbo-dom workspace and refuses to build the crate.
   Then dry-run the publish:
   ```bash
   out=$(mktemp -d)/turbo-dom-rav1d
   python3 crates/turbo-dom-rav1d/scripts/generate.py --write --output "$out"
   (cd "$out" && cargo publish --dry-run --allow-dirty --no-default-features --features bitdepth_8,bitdepth_16)
   ```
4. Point turbo-dom-raster at the copy with a temporary, uncommitted root `Cargo.toml`
   `[patch.crates-io] turbo-dom-rav1d = { path = "<out>" }`. Then run the AVIF gates:
   `cargo test -p turbo-dom-raster --features avif --lib` (the corrupt-AVIF fuzz child takes
   about 40 minutes in a debug build) and clippy for the default, `avif` and `jxl` builds.
5. Publish with a crates.io token that can publish new crates. Keep the token out of shell
   history and logs:
   ```bash
   (cd "$out" && CARGO_REGISTRY_TOKEN="$(tr -d '[:space:]' < <token file>)" \
     cargo publish --allow-dirty --no-default-features --features bitdepth_8,bitdepth_16)
   ```
6. Remove the temporary `[patch]`. Bump the requirement in `crates/turbo-dom-raster/Cargo.toml`,
   run `cargo update -p turbo-dom-rav1d`, and check that `cargo metadata --locked` passes.
   Commit `pins.json`, the patches and the lockfile together, then push.

To retire the fork, switch turbo-dom-raster back to `rav1d` once upstream publishes a release
that has the #1497 fix and an unwinding C ABI (memorysafety/rav1d#1521). Keep the
single-threaded `catch_unwind` decode in `src/avif.rs`. Then remove `crates/turbo-dom-rav1d`
and its CI guard steps.

## Lazy npm CSS payloads

The plain `build`, `build:wasm:pkg` and `build:wasm:web` parser commands retain their
existing feature sets and outputs. The package carries the layout engine's existing
bundled-font attribution/license files and copied-code notices alongside the CSS
binaries; font programs stay inside the payload rather than being duplicated as
standalone package files. CSS uses an explicit optional `css-backend`
feature with separate output names; it never overwrites parser `index.js`, types,
`.node` or WASM files. Native builds preserve caller cache/jobs/flags and the
existing napi-rs dynamic-CRT policy for musl. `./css-wasm` exposes the optional web
initialization module without adding a runtime selection mode.

`npm test`, `test:cov`, `test:vitest` and `test:jest` require real matching CSS
payloads before their behavior suites. Optional backend absence can remain a
runtime compatibility path for parser-only injection, but cannot make full
integration/release tests silently exercise only the old partial cascade.
`check:css:host -- --native-only` loads only native code and does not inspect or
load WebAssembly. Manual `prepublishOnly` requires the complete release matrix;
a host-only build cannot publish an incomplete package.

For restricted local builds, provide explicit cached build tools:

```sh
export TURBO_DOM_WASM_BINDGEN=/path/to/cached/wasm-bindgen
export TURBO_DOM_WASM_OPT=/path/to/cached/wasm-opt
npm run build:css
```

These are build-tool paths, not runtime modes. Both must be present. This route
runs locked **offline** Cargo, requires wasm-bindgen to match `Cargo.lock` exactly,
then uses the reviewed wasm-opt117 with `-Oz` (`BINARYEN_CORES` defaults to1).
It never invokes wasm-pack's installer or falls back to another/unoptimized
payload. Node/web packages are assembled in scratch space and replace their prior
working output only after every stage succeeds. With neither override, the
existing wasm-pack build route remains available for CI. `plan --target <triple>`
prints commands without invoking compilers/tools; its source path is a hint,
while actual copy paths come from post-build locked Cargo metadata, including a
Cargo-config-only `target-dir`.

Lazy refers to loading: the existing single npm bundle now includes additional
CSS bytes, so parser-only download size is **not** claimed unchanged. Measure the
full multi-platform tarball separately. Actual cold-load/retained-byte metrics and
independent parser startup comparisons remain integration gates; logical
`retainedBytes` is a shared CSS-payload lower bound, not a heap/RSS measurement.

### Shared CSS WASM assembly

`npm run build:css` builds both raw generated CSS WASM targets and then runs
`npm run assemble:css`. Individual `build:css:wasm:*` commands are intermediate
builds; run assembly after both before packing. The assembler preserves existing
`pkg-css`/`pkg-css-web` loader and exact declaration paths while sharing one
byte-identical WASM file under `css-wasm`. It changes only each generated loader's
leading type-reference comment, records original/output/body hashes, and rejects
unreviewed imports/snippets, mismatched binary bytes or declaration drift.
`check:artifacts` requires this assembly and the complete native matrix. Raw
private `pkg-css*/turbo_dom_css_bg.wasm` paths are no longer distributed; the
exported web initializer and explicit bytes/module injection remain supported.
See `tools/release/npm-css-dedup-review-0.8.7.md` for measured host-only receipts
and the separate, unimplemented optional-platform-package proposal.
