# Packed Vitest acceptance (Step 4)

This fixture tests the published environment export from a locally packed and extracted package, never an absolute source import. Step 4 remains **pending** until the switched default passes the strict CSS assertion in both parser modes.

Baseline source: `0fa17f575df68a1f9869bc7d1c4e061d35391e68`.

Local preparation used `npm pack --ignore-scripts --offline --pack-destination .evidence/step4/pack --json`, with an own `npm_config_cache`. The actual native parser and node/web WASM packages were copied from the release worktree, hashes recorded in BASELINE.json. No compilation, install, network or publication occurred. The tarball was extracted as a real `consumer/node_modules/turbo-dom` directory. Other existing dependencies were linked individually read-only, not installed. Package name is `@miaskiewicz/turbo-dom`; the local alias exercises its real exports unchanged.

Copy this directory's config, mode audit, strict CSS file and `shared` directory into the isolated consumer, then run from that consumer:

```
CONSUMER_ENV=turbo-dom TURBO_DOM_PARSER=native node node_modules/vitest/vitest.mjs run --config vitest.config.mjs
CONSUMER_ENV=turbo-dom TURBO_DOM_PARSER=wasm node node_modules/vitest/vitest.mjs run --config vitest.config.mjs
CONSUMER_ENV=happy-dom node node_modules/vitest/vitest.mjs run --config vitest.config.mjs
CONSUMER_ENV=jsdom node node_modules/vitest/vitest.mjs run --config vitest.config.mjs
CSS_ACCEPTANCE=1 CONSUMER_ENV=turbo-dom TURBO_DOM_PARSER=native node node_modules/vitest/vitest.mjs run --config vitest.config.mjs
CSS_ACCEPTANCE=1 CONSUMER_ENV=turbo-dom TURBO_DOM_PARSER=wasm node node_modules/vitest/vitest.mjs run --config vitest.config.mjs
```

The same 29 existing React/forms/RTL assertions are unchanged under all environments. Turbo native, Turbo WASM and jsdom pass29/29; happy-dom passes28/29 and differs on the wrapping label's first-control assertion. jsdom also prints its existing scrollTo unsupported message. No expectations were adjusted to erase these differences.

Strict public CSS is intentionally RED on the old default: both Turbo modes return color/width empty. Happy-dom also returns empty; jsdom returns `rgb(0, 0, 0)`/`auto`. The required assertion stays `rgb(12, 34, 56)` and `100px` for connected @layer/@media, variable/inherit and percentage used width. There is no private kernel call, adapter hook, mode-specific assertion, skip or synthetic provider.

The mode audit imports only the public runtime `getParser` export and rejects any native module loaded in the forced-WASM process. Resolution JSON proves the actual environment path inside the extracted package. Detailed raw logs, mode JSON, tarball and extracted bytes remain under this worktree's `.evidence/step4`.

## Serial Vitest timing protocol (prepared 2026-11-26)

`benchmark.mjs` ran in the coordinated quiet window; raw results are in `results-2026-11-26`. It runs one untimed warmup per row, then five rotated serial rounds, each in a fresh Vitest process with one worker. Four rows are happy-dom, jsdom, Turbo native parser + **legacy JavaScript CSS**, and Turbo WASM parser + **legacy JavaScript CSS**. Exactly the same 28 assertions run in every row: the single excluded full title is `getByLabelText: wrapping <label> labels only its FIRST control`. Every successful process must report28 passes and exactly that one filtered test; a failure, extra skip or changed title aborts the comparison. The unchanged29-case compatibility matrix and strict CSS sentinel remain separate.

Ready command from the worktree root:

```
node tools/acceptance/packed-vitest-296/benchmark.mjs .evidence/step4/consumer .evidence/step4/benchmark-2026-11-26 5
```

The baseline four compatibility runs take roughly1.4–2.8s each. Four warmups plus20 measured serial runs therefore represent approximately35–70s of observed baseline process time, excluding scheduling noise; this is an estimate, not a deadline. The runner records Node/machine/installed dependency versions, source/package/artifact baseline hashes, per-run wall times/load averages, JSON reports/logs, medians and min/max spreads. The measured results are recorded in the dated directory and the main README, separately from the legacy microbenchmarks.
