import assert from 'node:assert/strict';
import test from 'node:test';
import {createCssWindowEnvironmentReader} from '../src/runtime/css-window-environment.mjs';
import {loadRealCssBackend} from './helpers/css-shared-real.mjs';

function policy(backend) {
  const {preferences, environment} = JSON.parse(backend.cssRuntimePolicyV1());
  return {preferences: () => preferences, environment: () => environment};
}
const windowFacts = {hostPlatform:'macos', width:800, height:600, devicePixelRatio:1, mediaType:'screen'};

for (const mode of ['native', 'node-wasm']) {
  test(mode + ' window reuse exactly matches fresh Rust results across host input changes', async () => {
    const {backend} = await loadRealCssBackend(mode);
    const read = createCssWindowEnvironmentReader(backend, policy(backend), true);
    const fresh = createCssWindowEnvironmentReader(backend, policy(backend), false);
    const facts = {...windowFacts};
    let previous = read(facts);
    assert.equal(read({...facts}), previous);
    for (const [field, value] of [['hostPlatform','other'],['width',400],['height',300],
      ['devicePixelRatio',2],['mediaType','print'],['width',undefined],['height',undefined]]) {
      facts[field] = value;
      const result = read(facts);
      assert.notEqual(result, previous);
      assert.deepEqual(result, fresh(facts));
      assert.equal(read({...facts}), result);
      previous = result;
    }
    const override = {family:[65], sizePx:18};
    facts.fontOverride = override;
    assert.equal(read(facts).preferences.initial.fontSizePx,18);
    override.sizePx = 20;
    assert.equal(read(facts).preferences.initial.fontSizePx,20);
    facts.fontOverride = null;
    assert.deepEqual(read(facts),fresh(facts));
    assert.equal(read(facts),read(facts));
    Object.defineProperty(facts,'fontOverride',{get:()=>null});
    assert.notEqual(read(facts),read(facts));
    const inherited = Object.assign(Object.create({fontOverride:null}), windowFacts);
    assert.notEqual(read(inherited),read(inherited));
  });
}

test('environment failures cannot seed the reusable window result', async () => {
  const {backend} = await loadRealCssBackend('native');
  const limits = policy(backend);
  const read = overrides => createCssWindowEnvironmentReader({...backend,...overrides},limits,true);
  assert.throws(()=>read({})({...windowFacts,hostPlatform:'unknown'}),/host platform unavailable/);
  assert.throws(()=>read({cssComputedPreferencesV1:null})(windowFacts),/missing cssComputedPreferencesV1/);
  assert.throws(()=>read({cssComputedEnvironmentV1:()=> '{"abiVersion":2}'})(windowFacts),/ABI/);
  let mismatch = false;
  const get = read({cssComputedEnvironmentV1(input) {
    const result = JSON.parse(backend.cssComputedEnvironmentV1(input));
    if (mismatch) result.preferenceIdentity = [];
    return JSON.stringify(result);
  }});
  const initial = get(windowFacts);
  mismatch = true;
  const next = {...windowFacts,width:320};
  assert.throws(()=>get(next),/identity changed/);
  assert.throws(()=>get(next),/identity changed/);
  mismatch = false;
  assert.deepEqual(get(windowFacts),initial);
  assert.notDeepEqual(get(next).environmentKey,initial.environmentKey);
});
