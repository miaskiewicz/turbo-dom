'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { hostPlatform, checkBinding, checkHost } = require('./check-css-host.cjs');

class CompiledStylesV1 {
  metadata() {}
  prepare() {}
  dispose() {}
  retainedBytes() { return 0; }
}
const binding = { CompiledStylesV1, cssAbiVersion: () => 1, cssBackendVersion: () => '0.8.7' };
const mac = { platform: 'darwin', arch: 'arm64' };

test('platform mapping matches the reviewed native matrix and rejects absent platforms', () => {
  assert.equal(hostPlatform(mac), 'darwin-arm64');
  assert.equal(hostPlatform({ platform: 'win32', arch: 'x64' }), 'win32-x64-msvc');
  for (const arch of ['x64', 'arm64']) {
    const report = { getReport: () => ({ header: { glibcVersionRuntime: '2.35' } }) };
    assert.equal(hostPlatform({ platform: 'linux', arch, report }), `linux-${arch}-gnu`);
  }
  assert.equal(hostPlatform({ platform: 'linux', arch: 'x64' }), 'linux-x64-musl');
  assert.throws(() => hostPlatform({ platform: 'linux', arch: 'arm64' }), /no linux-arm64-musl/);
  assert.throws(() => hostPlatform({ platform: 'darwin', arch: 'x64' }), /no payload/);
});

test('actual export guard rejects parser-only and incomplete ABI instead of fallback', () => {
  assert.doesNotThrow(() => checkBinding(binding, 'real payload', '0.8.7'));
  assert.throws(() => checkBinding({ parse() {} }, 'parser', '0.8.7'), /CompiledStylesV1/);
  for (const name of ['metadata', 'prepare', 'dispose']) {
    class Broken extends CompiledStylesV1 {}
    Broken.prototype[name] = null;
    assert.throws(() => checkBinding({ ...binding, CompiledStylesV1: Broken }, 'broken', '0.8.7'), new RegExp(name));
  }
  class MissingRetention { metadata() {} prepare() {} dispose() {} }
  assert.throws(() => checkBinding({ ...binding, CompiledStylesV1: MissingRetention }, 'broken', '0.8.7'), /retainedBytes/);
  class GetterRetention extends MissingRetention { get retainedBytes() { throw Error('must not read instance getter'); } }
  assert.doesNotThrow(() => checkBinding({ ...binding, CompiledStylesV1: GetterRetention }, 'getter', '0.8.7'));
});

test('ABI and compiled binary version must match the source package', () => {
  assert.doesNotThrow(() => checkBinding({ ...binding, cssAbiVersion: 1 }, 'numeric ABI', '0.8.7'));
  for (const cssAbiVersion of [undefined, 2, () => 2]) {
    assert.throws(() => checkBinding({ ...binding, cssAbiVersion }, 'wrong ABI', '0.8.7'), /CSS ABI/);
  }
  for (const cssBackendVersion of [undefined, () => '0.8.6']) {
    assert.throws(() => checkBinding({ ...binding, cssBackendVersion }, 'stale binary', '0.8.7'), /binary version/);
  }
});

function fixture(fn) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'turbo-css-packaging-'));
  try {
    fs.writeFileSync(path.join(root, 'package.json'), JSON.stringify({ version: '0.8.7' }));
    for (const directory of ['pkg', 'pkg-web', 'pkg-css', 'pkg-css-web']) {
      fs.mkdirSync(path.join(root, directory));
      fs.writeFileSync(path.join(root, directory, 'package.json'), JSON.stringify({ version: '0.8.7' }));
    }
    fn(root);
  } finally { fs.rmSync(root, { recursive: true, force: true }); }
}

test('native-only smoke never loads or inspects WASM', () => fixture(root => {
  const calls = [];
  const original = Object.getOwnPropertyDescriptor(globalThis, 'WebAssembly');
  Object.defineProperty(globalThis, 'WebAssembly', { configurable: true, get() { throw Error('WASM must not be required'); } });
  try {
    checkHost(root, true, mac, file => { calls.push(file); return binding; });
  } finally { Object.defineProperty(globalThis, 'WebAssembly', original); }
  assert.deepEqual(calls, [path.join(root, 'native-css/turbo-dom-css.darwin-arm64.node')]);
}));

test('full smoke requires real native and Node WASM ABI with coherent versions', () => fixture(root => {
  const calls = [];
  checkHost(root, false, mac, file => { calls.push(file); return binding; });
  assert.equal(calls.length, 2);
  assert.equal(calls[1], path.join(root, 'pkg-css/turbo_dom_css.js'));
  fs.writeFileSync(path.join(root, 'pkg-css-web/package.json'), JSON.stringify({ version: '0.8.6' }));
  assert.throws(() => checkHost(root, false, mac, () => binding), /version does not match/);
}));

test('initialization and nested-module errors retain their identity', () => fixture(root => {
  const original = new TypeError('internal CSS dependency failed');
  assert.throws(() => checkHost(root, true, mac, () => { throw original; }), error => error === original);
}));
