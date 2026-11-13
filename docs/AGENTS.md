# AGENTS.md

Field guide for AI agents (and humans) working on turbo-dom. Read this first.
The full design is in [turbo-dom-spec.md](../turbo-dom-spec.md); the dev notes
are in [CLAUDE.md](../CLAUDE.md); publishing is in [PUBLISHING.md](../PUBLISHING.md).

## One-command workflows

All in `scripts/agent/`. Each supports `--help`.

| script | what it does |
|---|---|
| [`build-napi.sh`](../scripts/agent/build-napi.sh) | Build the native html5ever addon with the shared cache env. Needed before any `npm test` / commit that touches `src/` or `test/`. |
| [`test-crate.sh CRATE [TEST]`](../scripts/agent/test-crate.sh) | `cargo test -p CRATE` with the right features and `CARGO_TARGET_DIR`. |
| [`coverage-crate.sh CRATE [VARIANT]`](../scripts/agent/coverage-crate.sh) | Tarpaulin against the ci.yml floor for that crate. |
| [`record-chrome-control.sh PROBE [OUT]`](../scripts/agent/record-chrome-control.sh) | Biome-format, then run a `scripts/*-chrome-control.mjs` probe against the cached Chrome 156. |
| [`check-versions.sh`](../scripts/agent/check-versions.sh) | The version-sync guard (every manifest + lockfile + WASM pkg). |
| [`verify-copied.sh`](../scripts/agent/verify-copied.sh) | Run all three S1/S5 "verify copied" guards. |
| [`release-prep.sh X.Y.Z`](../scripts/agent/release-prep.sh) | Bump every manifest, re-lock, rebuild WASM, run the guards. Does **not** tag or publish. |

## Shared cache locations

The helpers read these via `scripts/agent/lib.sh`; override any one with an
env var of the same name.

| name | default | what |
|---|---|---|
| `CACHE_ROOT` | `~/.cache/turbo-consolidation` | parent dir |
| `CHROME156` | `$CACHE_ROOT/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell` | Chrome for Testing 156 (oracle for every control) |
| `V8_ARCHIVE` → `RUSTY_V8_ARCHIVE` | `$CACHE_ROOT/v8cache/librusty_v8_simdutf_release_aarch64-apple-darwin.a.gz` | prebuilt rusty_v8 150.4 static lib (for turbo-dom-browser-env / turbo-dom-v8) |
| `TARGET_DIR` → `CARGO_TARGET_DIR` | `$CACHE_ROOT/target/turbo-dom` | one shared cargo target across every helper |
| scratchpad target | your agent session's `/private/tmp/.../scratchpad/target` | use this only if you need an isolated target for one experiment; the default shared one caches across sessions |

Run `bash scripts/agent/lib.sh --help` to verify each path exists on the host.

## Chrome 156 control probes

All probes live at `scripts/*-chrome-control.mjs` and share one contract:

```sh
node scripts/FOO-chrome-control.mjs [--chrome PATH] [--out FILE]
```

Required chromium.launch args (locked in every probe, don't change them):

```
headless: true, args: ["--use-mock-keychain", "--password-store=basic"]
```

A new probe copies the top of an existing one (e.g. `scripts/paint-chrome-control.mjs`)
for the chromePath / playwright-core resolution. Then record with:

```bash
scripts/agent/record-chrome-control.sh my-new-probe
```

The wrapper biome-formats the probe first, so the commit stays lint-clean.

## Coverage floors (ci.yml, blocking)

| crate / variant | floor |
|---|---|
| `turbo-dom` | 100 |
| `turbo-dom-parser` (`src/core.rs`) | 100 |
| `turbo-dom-layout` (default) | 99.8 |
| `turbo-dom-layout` (`all-features`: svg+perf+serde+test-support) | 99.8 |
| `turbo-dom-raster` (default / `avif` / `jxl`) | 100 each |
| `turbo-dom-browser-env` (`host-internals,test-support`) | 97.6 |

Don't lower one to pass a change. Add or restructure the code so the test covers
the real behaviour; a provably dead defensive line is `unreachable!()` with its
proof in a comment.

## Pre-publish pin and tagging

turbo-dom is the leaf: consumers (turbo-test, turbo-html2pdf, turbo-surf) pin a
pre-publish `turbo-dom-*` through `[patch.crates-io]` against this repo's
release branch while the release PR is open. The order is:

1. Open and merge this repo's release PR into `main` with a merge commit.
2. Create an **annotated** `vX.Y.Z` tag on the actual merge commit
   (not the PR branch tip). Pushing it triggers `publish.yml`.
3. Once crates.io lists every crate, consumers drop their `[patch.crates-io]`
   sections and re-lock (see
   [docs/CONSOLIDATION.md](CONSOLIDATION.md) "How consumers depend on the
   crates" for the exact section names).

Never tag without the release PR merged, and never force-push the tag.

## Pre-commit hook gotchas

The hook is `.githooks/pre-commit`, zero-dep, wired by the `prepare` script at
`npm install`. It runs `npm run test:cov` **only** when staged paths match
`src/` / `test/` / `harness/` / `index.js` / `package.json`.

- A src/test change needs the native addon present. Run
  [`scripts/agent/build-napi.sh`](../scripts/agent/build-napi.sh) first.
- Don't commit with `--no-verify`. If the hook fails, fix the test, don't
  bypass it.
- Commit subjects: short, lower-case / sentence-case, scope in parens
  (`fix(runtime): ...`, `chore(release): ...`). Match the style of
  `git log --oneline` on `main`.
- A Chrome-control probe you hand-edit must be biome-formatted before it lands
  (the record helper does this for you).

## Agent etiquette

- Branches follow `chore/<area>-<topic>` or `fix(<scope>)/<topic>`. One PR per
  logical change.
- Never touch `CLAUDE.md`, `CHANGELOG.md` or `.github/workflows/` from an
  agent-tooling PR — those are the release agents' territory.
- Large hand-off notes live in the group's shared notes folder (outside the
  repo), not in the PR.
- Long-running builds (`npm run build`, tarpaulin, publish-crate-dry-run) run
  in the background.
