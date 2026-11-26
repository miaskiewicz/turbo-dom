import test from 'node:test';
import assert from 'node:assert/strict';
import { createCssBackendResolver, cssCapabilityDescriptor } from '../src/runtime/css-backend.mjs';
import { normalizeCssArtifact, createCssArtifactResolver } from '../src/runtime/css-artifact.mjs';
const names = ['serializeCssomV1', 'projectCssomPropertyV1', 'cssComputedPreferencesV1'];
const CompiledStylesV1 = class {};
const raw = () => ({ cssAbiVersion: () => 1, cssBackendVersion: () => '0.8.7', CompiledStylesV1 });
for (const name of names) {
  test('optional CSS operation forwarded without identity or receiver wrapper: ' + name, () => {
    const binding = raw();
    let calls = 0;
    const operation = function () {
      calls++;
      return this;
    };
    binding[name] = operation;
    const descriptor = normalizeCssArtifact(binding);
    assert.equal(descriptor[name], operation);
    assert.equal(descriptor[name](), descriptor);
    assert.equal(calls, 1);
    const resolved = createCssBackendResolver().resolve({ mode: 'native', binding: { ...descriptor }, revision: 0 });
    assert.equal(resolved[name], operation);
    assert.equal(Object.isFrozen(resolved), true);
  });
  test('optional CSS operation getter is read once and original failures propagate: ' + name, () => {
    const binding = raw(),
      error = new Error(name);
    let reads = 0;
    const operation = () => {};
    Object.defineProperty(binding, name, {
      get() {
        reads++;
        return operation;
      },
    });
    assert.equal(normalizeCssArtifact(binding)[name], operation);
    assert.equal(reads, 1);
    const failed = raw();
    Object.defineProperty(failed, name, {
      get() {
        throw error;
      },
    });
    assert.throws(
      () => normalizeCssArtifact(failed),
      (value) => value === error,
    );
    assert.throws(
      () =>
        createCssBackendResolver().resolve({
          mode: 'auto',
          binding: {
            cssAbiVersion: 1,
            CompiledStylesV1,
            get [name]() {
              throw error;
            },
          },
          revision: 0,
        }),
      (value) => value === error,
    );
  });
  test('optional CSS operation malformed or partial capability never falls back: ' + name, () => {
    let native = 0,
      wasm = 0;
    const resolver = createCssBackendResolver({
      loadNative() {
        native++;
        return null;
      },
      loadWasm() {
        wasm++;
        return null;
      },
    });
    assert.throws(
      () =>
        resolver.resolve({ mode: 'auto', binding: { cssAbiVersion: 1, CompiledStylesV1, [name]: null }, revision: 0 }),
      new RegExp(name + ' must be a function'),
    );
    assert.equal(native + wasm, 0);
    assert.throws(() => normalizeCssArtifact({ ...raw(), [name]: 17 }), new RegExp(name + ' must be a function'));
    assert.throws(
      () => resolver.resolve({ mode: 'auto', binding: { [name]: () => {} }, revision: 1 }),
      /incompatible CSS backend ABI/,
    );
    assert.equal(native + wasm, 0);
    assert.throws(() => normalizeCssArtifact({ [name]: () => {} }), /incompatible CSS artifact/);
  });
}
test('absent optional operations preserve descriptor own keys and parse-only injection', () => {
  assert.deepEqual(Object.keys(cssCapabilityDescriptor(1, CompiledStylesV1)), ['cssAbiVersion', 'CompiledStylesV1']);
  const minimal = {
    parse() {
      return 'parser';
    },
  };
  assert.equal(createCssBackendResolver().resolve({ mode: 'auto', binding: minimal, revision: 0 }), null);
  assert.equal(
    createCssArtifactResolver({ require: null }).resolve({ mode: 'wasm', binding: minimal, revision: 0 }),
    null,
  );
  assert.equal(minimal.parse(), 'parser');
});
test('artifact metadata ignores public call getters and preserves native receiver/errors', () => {
  const binding = raw();
  let getters = 0,
    bodies = 0;
  for (const [name, result] of [
    ['cssAbiVersion', 1],
    ['cssBackendVersion', '0.8.7'],
  ]) {
    const fn = function () {
      assert.equal(this, binding);
      bodies++;
      return result;
    };
    Object.defineProperty(fn, 'call', {
      get() {
        getters++;
        throw new Error('unused call getter');
      },
    });
    binding[name] = fn;
  }
  assert.equal(normalizeCssArtifact(binding).CompiledStylesV1, CompiledStylesV1);
  assert.equal(getters, 0);
  assert.equal(bodies, 2);
  const error = new Error('metadata body');
  binding.cssBackendVersion = function () {
    assert.equal(this, binding);
    throw error;
  };
  assert.throws(
    () => normalizeCssArtifact(binding),
    (value) => value === error,
  );
});
test('optional descriptor definitions do not invoke inherited operation setters', () => {
  const binding = raw(),
    operation = () => {};
  binding.serializeCssomV1 = operation;
  let calls = 0;
  const prior = Object.getOwnPropertyDescriptor(Object.prototype, 'serializeCssomV1');
  let result;
  try {
    Object.defineProperty(Object.prototype, 'serializeCssomV1', {
      configurable: true,
      set() {
        calls++;
        throw new Error('unused operation setter');
      },
    });
    result = normalizeCssArtifact(binding);
  } finally {
    if (prior) Object.defineProperty(Object.prototype, 'serializeCssomV1', prior);
    else delete Object.prototype.serializeCssomV1;
  }
  assert.equal(calls, 0);
  assert.equal(result.serializeCssomV1, operation);
});
