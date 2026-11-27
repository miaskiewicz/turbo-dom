import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const mode = process.argv[2],
  artifact = process.argv[3];
assert.ok(mode === 'native' || mode === 'wasm');
assert.ok(artifact);
if (mode === 'native') {
  // Node's lazy HTTP globals initialize a WASM parser. This CSS-only probe
  // removes those unrelated host capabilities before importing the runtime.
  for (const name of ['fetch', 'Headers', 'Request', 'Response', 'FormData', 'WebSocket', 'EventSource'])
    delete globalThis[name];
  globalThis.WebAssembly = undefined;
}
const { createCssArtifactResolver } = await import('../../../src/runtime/css-artifact.mjs');
const { createRealCssFixture } = await import('../../helpers/css-shared-real.mjs');
const require = createRequire(import.meta.url),
  paths = [];
function load(path) {
  return require(path);
}
load.resolve = (spec) => {
  paths.push(spec);
  if (mode === 'wasm') assert.ok(spec.endsWith('/pkg-css/turbo_dom_css.js'));
  else assert.ok(spec.endsWith('darwin-arm64.node'));
  return artifact;
};
const resolver = createCssArtifactResolver({ require: load, platform: 'darwin', arch: 'arm64' });
const backend = resolver.resolve({ mode, binding: { parse() {} }, revision: 0 });
const raw = require(artifact);
for (const name of ['serializeCssomV1', 'projectCssomPropertyV1']) assert.equal(backend[name], raw[name]);
if (process.argv[4] === 'preferences') {
  assert.equal(backend.cssComputedPreferencesV1, raw.cssComputedPreferencesV1);
  const ua = JSON.parse(backend.cssUserAgentSourceV1());
  // Explicit test-only bound includes the actual canonical UA payload, not a runtime default.
  const fixtureUnits = ua.text.length + ua.documentText.length + ua.fullscreenText.length + 1024;
  const response = JSON.parse(
    backend.cssComputedPreferencesV1(
      JSON.stringify({
        abiVersion: 1,
        hostPlatform: 'macos',
        fontOverride: { family: [115, 101, 114, 105, 102], sizePx: 18.25 },
        limits: {
          inputUnits: 1024,
          retainedUnits: fixtureUnits,
          computation: { maxNesting: 32, maxOutputUnits: fixtureUnits, maxTotalUnits: fixtureUnits * 2 },
        },
      }),
    ),
  );
  assert.equal(response.abiVersion, 1);
  assert.deepEqual(response.initial.fontFamily, [115, 101, 114, 105, 102]);
  assert.equal(response.initial.fontSizePx, 18.25);
} else if (process.argv[4] === 'legacy') {
  assert.equal(
    backend.cssComputedPreferencesV1,
    undefined,
    'older c018 payload remains valid without optional preferences',
  );
} else {
  assert.equal(backend.cssComputedPreferencesV1, raw.cssComputedPreferencesV1);
}
const real = { backend, mode: mode === 'native' ? 'native' : 'node-wasm', artifact };
const fixture = createRealCssFixture(
  real,
  '<!doctype html><style>.host{width:800px;--ink:red}.x{width:50%;color:var(--ink)}</style><main class="host"><div class="x">sample</div></main>',
);
try {
  const style = fixture.adapter.get(fixture.doc.querySelector('.x'));
  assert.equal(style.color, 'rgb(255, 0, 0)');
  assert.equal(style.width, '400px');
  assert.equal(style.getPropertyValue('--ink'), 'red');
} finally {
  fixture.dispose();
}
assert.equal(paths.length, 1);
if (mode === 'wasm')
  assert.deepEqual(
    Object.keys(require.cache).filter((path) => path.endsWith('.node')),
    [],
  );
else assert.equal(globalThis.WebAssembly, undefined);
console.log(
  JSON.stringify({
    mode,
    adapterValues: true,
    paths,
    nativeModules: Object.keys(require.cache).filter((path) => path.endsWith('.node')).length,
  }),
);
