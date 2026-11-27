'use strict';
// A full runtime gate must exercise real payloads, never optional-capability fallback.
const fs = require('node:fs');
const path = require('node:path');

function linuxPlatform(runtime) {
  const glibc = runtime.report?.getReport().header.glibcVersionRuntime;
  if (runtime.arch === 'arm64' && !glibc) throw new Error('CSS package has no linux-arm64-musl payload');
  return `linux-${runtime.arch}-${glibc ? 'gnu' : 'musl'}`;
}

function hostPlatform(runtime) {
  const { platform, arch } = runtime;
  const known = { 'darwin-arm64': 'darwin-arm64', 'win32-x64': 'win32-x64-msvc' };
  const match = known[`${platform}-${arch}`];
  if (match) return match;
  if (platform === 'linux' && ['x64', 'arm64'].includes(arch)) return linuxPlatform(runtime);
  throw new Error(`CSS package has no payload for ${platform}-${arch}`);
}

function checkVersions(binding, label, version) {
  const abi = typeof binding.cssAbiVersion === 'function' ? binding.cssAbiVersion() : binding.cssAbiVersion;
  if (abi !== 1) throw new TypeError(`${label}: CSS ABI ${abi} is unsupported`);
  if (typeof binding.cssBackendVersion !== 'function' || binding.cssBackendVersion() !== version) {
    throw new TypeError(`${label}: CSS binary version does not match ${version}`);
  }
}

function checkBinding(binding, label, version) {
  const Class = binding.CompiledStylesV1;
  if (typeof Class !== 'function') throw new TypeError(`${label}: CompiledStylesV1 ABI is missing`);
  for (const method of ['metadata', 'prepare', 'dispose']) {
    if (typeof Class.prototype[method] !== 'function') throw new TypeError(`${label}: ${method} ABI is missing`);
  }
  const retained = Object.getOwnPropertyDescriptor(Class.prototype, 'retainedBytes');
  if (!retained || (typeof retained.get !== 'function' && typeof retained.value !== 'function')) {
    throw new TypeError(`${label}: retainedBytes ABI is missing`);
  }
  checkVersions(binding, label, version);
}

function checkHost(root, nativeOnly = false, runtime = process, load = require) {
  const platform = hostPlatform(runtime);
  const native = path.join(root, 'native-css', `turbo-dom-css.${platform}.node`);
  const version = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).version;
  checkBinding(load(native), native, version);
  if (!nativeOnly) {
    for (const directory of ['pkg', 'pkg-web', 'pkg-css', 'pkg-css-web']) {
      const manifest = JSON.parse(fs.readFileSync(path.join(root, directory, 'package.json'), 'utf8'));
      if (manifest.version !== version) throw new Error(`${directory}: CSS payload version does not match ${version}`);
    }
    checkBinding(load(path.join(root, 'pkg-css', 'turbo_dom_css.js')), 'CSS nodejs WASM', version);
  }
}

module.exports = { hostPlatform, checkBinding, checkHost };
if (require.main === module) {
  checkHost(path.resolve(__dirname, '..'), process.argv.includes('--native-only'));
  console.log('CSS payload ABI/version check passed');
}
