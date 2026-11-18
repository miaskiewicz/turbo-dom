# Real-player validation handoff

The constructor/API fix is local commit 0ae53ced99e20d84a6242d1b86f6b4d88184e949; native upload replay is 91829eec24eb3f3716f999b92b5f7dd8d01d68da. The combined release branch includes binding integration 79efb5d2 and native replay integration 9bbc2422. Pin Root's final pushed green combined revision, not the older 0.8.4 packages.

## What was actually run

The unchanged local fixture, player JS and WASM were served by turbo-surf's existing raster-parity server and run in cached Chromium 156.0.8078.47. Result:load event, frame 30, 544214 nontransparent canvas pixels, no page errors. JSON+PNG are retained as player-chrome-baseline.*. This is a Chromium baseline only; the fixed native engine was not run.

Asset SHA256:
- JS 9a35aed7b41fd97bbdcd1b1a2c9d9d1fbd4150ecfb8a807ba96b9f54ca0a31da
- WASM 3169df80d43187939d7d7ab81d27d0b6843694687f8e960e94606986544c7bcf
- .lottie 51c691ee8cd7e003e0bfe520aaedfbb8e223c5e03495480a97f6e697057d3937

## Why the existing native artifact is insufficient

The existing Surf shim hard-loads rust/crates/turbo-surf-napi/index.js, which loads its compiled .node/cdylib. Its Cargo.lock still resolves turbo-dom-browser-env and turbo-dom-raster to registry 0.8.4, and the available addon predates this fix. Merely invoking the harness again would retest the old engine. A bare browser-env V8 test is not a complete page host: the player also requires ES modules, fetch, WASM loading and a task loop, supplied here by Surf's render runtime.

No Surf source, manifests, lockfiles or addon artifacts were changed by this task.

## Native run by the Surf integration owner

1. In a Surf-owned isolated integration checkout, patch the four turbo-dom crates (base/layout/raster/browser-env) to Root's final combined 0.8.5 revision, re-lock coherently, and retain one turbo-dom-v8 dependency (scripts/check-single-v8.sh). Build the addon in that checkout using its normal procedure.
2. From that checkout, run the existing harness:

```sh
npm run build:addon
TURBO_SURF_CHROME_PATH="$CHROME_PATH" \
DOTLOTTIE_DIST="$DOTLOTTIE_DIST" \
PARITY_OUTPUT=/private/tmp/turbo-surf-lottie-v085 \
node harness/raster-parity/lottie.mjs --assert
```

3. Inspect retained lottie.json and screenshot, requiring a genuine load event, frame 30 and actual nontransparent uploaded canvas pixels. Before screenshot/readback drains the op log, inspect putImageData snapshot dimensions/data and uploaded alpha count (not just a nonempty fallback PNG or constructor existence). The expected Chromium alpha count is 544214; retain any subsequent loadError/unsupported capability exactly.

Complete Lottie support and issue closure remain contingent on this fixed-native run. Display-P3/float16 ImageData construction is supported; bare canvas color/pixel-format conversion is explicitly unsupported rather than silently approximated.
