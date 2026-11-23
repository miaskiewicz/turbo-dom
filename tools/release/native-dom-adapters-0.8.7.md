# Native test-window adapters, v0.8.7

## Inventory and ownership

Direct issue and recent-issue APIs on 2026-11-23 identified four new reports,
#542–545, with no fifth related producer issue. All four initially had no milestone
and no comments; each was assigned milestone 9, `v0.8.7`.

| Producer report | Tracking issue | Outcome |
| --- | --- | --- |
| #542, missing window.history | turbo-test #460 | Producer default-window History adapter |
| #543, missing DOMParser.parseFromString | turbo-test #461 | Producer native HTML/XML parser adapter |
| #544, Response(stream).text() stringifies body | turbo-test #463 | Host-runtime defect, requires turbo-test change |
| #545, missing SubtleCrypto methods | turbo-test #462 | Host-runtime defect, requires turbo-test change |

The umbrella is turbo-test #459. Its iframe document.open observation was not
confirmed against the reference implementation and is not a fifth producer fix.
Older #540/#541 were already fixed in the release graph and were not reimplemented.

## Causal regressions

The initial native test failed with
`TypeError: (intermediate value).parseFromString is not a function`.
The history regression then failed with `ReferenceError: history is not defined`.
Further tests cover receiver and argument conversion, UTF-16, independent native
document ownership, script inertness, state cloning, failed-update atomicity,
same-origin URL admission, task-queued traversal and embedder overrides.

`native_dom_adapters_tests.rs` replays the unmodified XML Chrome 156 control through
the public DOMParser. The existing Trusted Types sink and inert-document view
transition controls now include DOMParser instead of excluding it.

`native_dom_adapters_probe.js` was independently recorded on Chromium
159.0.8154.6 using an ephemeral localhost fixture, with the required mock-keychain
and password-store flags. Its source hash is checked against
`native_dom_adapters_chrome159.json`. Chrome 156 was unavailable for a new recording;
the new combined control is a 159 control, not a new 156 parity claim.

## Integration boundary

The native parser uses the existing HTML/XML and Trusted Types APIs; no parser is
copied into a consumer. A preinstalled functional host DOMParser is retained, and
the default interface remains configurable for a later embedder install.

History installs only with the binding-created fallback Location and when no host
History exists. The History global and window.history property remain configurable
so Surf can install its navigation controller. Same-document traversal uses the
existing captured native task scheduler; reload and missing-scheduler traversal
report explicit unsupported capabilities. No network navigation is simulated.

No Surf source, dependency pin, frozen addon, application service or fixture was
changed. Consumers must repin the DOM-family crates coherently to the integrated
producer commit and rebuild before claiming the new behavior.

## Host reproductions

An existing turbo-test binary identifying itself as 0.8.6 reproduced both host
failures on independent local tests:

- Response.text returned `[object Object]` for a byte stream.
- crypto.subtle.importKey was not a function.

Binary SHA-256:
`2d90613b331784626d0526c4330417dbd38f1ec1c2c17165aa9d7b1394a6791e`.
At the initial inspection, the release/0.8.7 host sources independently showed
`String(this._body)` in Response.text and a digest-only lean `__subtleCrypto`.
Neither API is supplied by the producer DOM binding. These are not fixed by a
producer dependency repin.

A subsequent independently copied host candidate, still identifying as 0.8.6,
had SHA-256 `bbaf0b09f1f8eb4af976e7d82538777dfef67dbdac3bdf7f88d3027fc164d37d`.
It passed the basic stream-body reproduction and still failed WebCrypto importKey.
That observation is not a full validation or release claim for the consumer fix;
the turbo-test owner handles its implementation and integration.

Exact issue snapshots, failing logs, host fixtures, commands and gate logs are
retained locally under `.evidence/issues-542-545/`; those local logs are not
distributed package inputs. The committed tests, probe and independent control
are the portable regression artifacts.

## Toolchain

Rust 1.98.0, at most two compilation jobs, one turbo-dom-v8 150.4.2.
V8 archive SHA-256:
`5aeffd8d5a0c1b79ac1d70af83d5b19099655fd9c645a794dc43f101f779838c`.
The build used the existing producer-target cache after checking its cargo lock
had no owner. No execution budget, resource limit, coverage floor or assertion
was weakened.

## Verification

- `cargo +1.98.0 test --workspace --features turbo-dom-browser-env/host-internals,turbo-dom-browser-env/test-support`:
  3,176 passed, 0 failed, 12 existing ignored, across 224 test/doc-test summaries.
  This includes 841 native-binding unit tests and the binding integration tests.
- All 14 focused native adapter tests passed.
- `cargo +1.98.0 clippy --workspace --all-targets --features turbo-dom-browser-env/host-internals,turbo-dom-browser-env/test-support -- -D warnings`: passed.
- `cargo +1.98.0 fmt --all --check`: passed.
- The repository Rust complexity gate and both new Rust modules at `--max 9): passed.
- Pinned oxlint 1.87.0 with the repository complexity configuration, including the
  new JS adapters, probe and control script: passed.
- `git diff --check`: passed.
- Native-binding LLVM tarpaulin, the CI feature set, source filter and 2,400-second
  timeout, unchanged `--fail-under 97.6`: **97.63%**, 22,657/23,206 covered lines.
  It ran with two jobs in an independent local target, offline and locked.
- The isolated core churn allocation gate: **372 allocations/op**, 82,270 bytes/op,
  below the unchanged 380/85,000 ceilings.
- Version agreement and retired selector-dual guards: passed.

The producer implementation checkpoint is
`5672e9fca1c2668f78fbc4d90d4c4090f3e55589`. The two host reports #544/#545 were
subsequently closed as `not_planned` by the host owner after being reclassified
into turbo-test #463/#462; that is an ownership correction, not a producer fix.
