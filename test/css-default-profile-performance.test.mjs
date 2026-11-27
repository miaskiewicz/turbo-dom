import {loadNativeCss} from './opt-in-helpers/native.mjs';
import assert from 'node:assert/strict';
import test from 'node:test';
import { createEnvironment } from './opt-in-helpers/runtime.mjs';
import { createDefaultCssCapture } from '../src/runtime/css-default-capture.mjs';
import { createDefaultCssProfile } from '../src/runtime/css-default-profile.mjs';
import { createDefaultSharedCssAdapter } from '../src/runtime/css-shared-adapter.mjs';

test('stable host inputs reuse the Rust environment and changed viewports refresh it', () => {
  const native = loadNativeCss();
  const policy = JSON.parse(native.cssRuntimePolicyV1());
  let calls = 0;
  const backend = {
    ...native,
    cssComputedEnvironmentV1(request) {
      calls++;
      return native.cssComputedEnvironmentV1(request);
    },
  };
  let width = 800;
  const profile = createDefaultCssProfile({
    backend,
    capture: createDefaultCssCapture(),
    readWindowContext: () => ({ hostPlatform: 'other', width, height: 600, devicePixelRatio: 1, mediaType: 'screen' }),
    resourcePolicy: {
      preferences: () => policy.preferences,
      environment: () => policy.environment,
      snapshot: () => policy.snapshot,
      selectorFacts: () => policy.selectorFacts,
      scene: () => policy.scene,
      composition: () => policy.composition,
      adapter: Object.fromEntries(Object.entries(policy.adapter).map(([name, value]) => [name, () => value])),
    },
  });
  const { document } = createEnvironment('<body><p>one</p><p>two</p>');
  const [first, second] = document.querySelectorAll('p');
  const original = profile.readContext(first);
  assert.deepEqual(profile.readContext(second).environment, original.environment);
  assert.equal(calls, 1);
  width = 640;
  const resized = profile.readContext(second);
  assert.notDeepEqual(resized.environment.features, original.environment.features);
  assert.equal(calls, 2);
  assert.deepEqual(profile.readContext(first).environment, resized.environment);
  assert.equal(calls, 2);
});

test('ordinary CSS resolves the inheritance chain and keeps complete scene inputs for used width', () => {
  const native = loadNativeCss();
  const targetCounts = [];
  let compileCount = 0;
  class MeasuredStyles {
    constructor(request) {
      this.compiled = new native.CompiledStylesV1(request);
      compileCount++;
    }
    metadata() { return this.compiled.metadata(); }
    dispose() { this.compiled.dispose(); }
    prepare(request) {
      const prepared = this.compiled.prepare(request);
      return {
        metadata: () => prepared.metadata(),
        dispose: () => prepared.dispose(),
        resolveBatch(request) {
          targetCounts.push(JSON.parse(request).targets.length);
          return prepared.resolveBatch(request);
        },
      };
    }
  }
  const backend = { ...native, CompiledStylesV1: MeasuredStyles };
  const policy = JSON.parse(native.cssRuntimePolicyV1());
  const profile = createDefaultCssProfile({
    backend,
    capture: createDefaultCssCapture(),
    readWindowContext: () => ({
      hostPlatform: 'other', width: 800, height: 600, devicePixelRatio: 1, mediaType: 'screen',
    }),
    resourcePolicy: {
      preferences: () => policy.preferences,
      environment: () => policy.environment,
      snapshot: () => policy.snapshot,
      selectorFacts: () => policy.selectorFacts,
      scene: () => policy.scene,
      composition: () => policy.composition,
      adapter: Object.fromEntries(Object.entries(policy.adapter).map(([name, value]) => [name, () => value])),
      fonts: policy.fonts,
    },
  });
  const { document } = createEnvironment(
    '<style>main{--tone:blue} main:has(aside) .item{color:var(--tone)}</style>' +
    '<main><b class="item">one</b><aside>visible to selector matching</aside></main>',
  );
  const target = document.querySelector('.item');
  const adapter = createDefaultSharedCssAdapter(profile, true, true);
  const style = adapter.get(target);
  assert.equal(style.color, 'rgb(0, 0, 255)');
  assert.deepEqual(targetCounts, [4]);
  targetCounts.length = 0;
  assert.equal(adapter.get(target).color, style.color);
  assert.deepEqual(targetCounts, [4]);
  assert.equal(compileCount, 1);
  targetCounts.length = 0;
  document.querySelector('style').textContent =
    'main{--tone:red;width:246px} .item{display:block;color:var(--tone);width:50%}';
  const sized = adapter.get(target);
  assert.equal(sized.color, 'rgb(255, 0, 0)');
  assert.equal(sized.width, '123px');
  assert.equal(style.color, 'rgb(0, 0, 255)');
  assert.equal(compileCount, 2);
  assert.equal(targetCounts[0], 4);
  assert.equal(targetCounts[1], document.querySelectorAll('*').length);
  assert.equal(targetCounts.length, 2);
  adapter.dispose();
});
