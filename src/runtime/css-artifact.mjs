// Optional CSS artifacts are independent of the minimal HTML parser binding.
import { createCssBackendResolver, cssCapabilityDescriptor, cssOptionalAbsent } from './css-backend.mjs';

import { createParserRequire } from './parser.mjs';

const applyMetadata = Reflect.apply;
function metadata(binding, value) {
  return typeof value === 'function' ? applyMetadata(value, binding, []) : value;
}

export function normalizeCssArtifact(binding) {
  if (binding == null) return null;
  const abi = binding.cssAbiVersion;
  const CompiledStylesV1 = binding.CompiledStylesV1;
  const measureSceneV1 = binding.measureSceneV1;
  const cssUserAgentSourceV1 = binding.cssUserAgentSourceV1;
  const serializeCssomV1 = binding.serializeCssomV1;
  const projectCssomPropertyV1 = binding.projectCssomPropertyV1;
  const cssComputedPreferencesV1 = binding.cssComputedPreferencesV1;
  if (
    abi === undefined &&
    CompiledStylesV1 === undefined &&
    cssOptionalAbsent([
      measureSceneV1,
      cssUserAgentSourceV1,
      serializeCssomV1,
      projectCssomPropertyV1,
      cssComputedPreferencesV1,
    ])
  )
    return null;
  const cssAbiVersion = metadata(binding, abi);
  const version = binding.cssBackendVersion;
  const backendVersion = metadata(binding, version);
  if (cssAbiVersion !== 1 || typeof CompiledStylesV1 !== 'function' || backendVersion !== '0.8.7') {
    throw new TypeError('turbo-dom: incompatible CSS artifact (expected ABI 1, version 0.8.7)');
  }
  return cssCapabilityDescriptor(
    cssAbiVersion,
    CompiledStylesV1,
    measureSceneV1,
    cssUserAgentSourceV1,
    serializeCssomV1,
    projectCssomPropertyV1,
    cssComputedPreferencesV1,
  );
}

export function nativeCssSuffix(platform, arch, musl = false) {
  if (platform === 'darwin') return `darwin-${arch}`;
  if (platform === 'win32') return `win32-${arch}-msvc`;
  if (platform === 'linux') return `linux-${arch}-${musl ? 'musl' : 'gnu'}`;
  if (platform === 'android') return `android-${arch === 'arm' ? 'arm-eabi' : arch}`;
  if (platform === 'freebsd') return `freebsd-${arch}`;
  return null;
}

// Resolve only the exact requested artifact before initialization: nested missing
// dependencies and every initialization exception remain the original exception.
export function loadCssModule(require, spec) {
  let path;
  try {
    path = require.resolve(spec);
  } catch (error) {
    if (error?.code === 'MODULE_NOT_FOUND' && error.message?.startsWith(`Cannot find module '${spec}'`)) return null;
    throw error;
  }
  return require(path);
}

export function detectNativeMusl(require, process = globalThis.process) {
  if (process?.report && typeof process.report.getReport === 'function') {
    return !process.report.getReport().header.glibcVersionRuntime;
  }
  try {
    const lddPath = require('child_process').execSync('which ldd').toString().trim();
    return require('fs').readFileSync(lddPath, 'utf8').includes('musl');
  } catch {
    return true;
  }
}

function nodeRequire() {
  return createParserRequire();
}

export function createCssArtifactResolver({
  require = nodeRequire(),
  platform = globalThis.process?.platform,
  arch = globalThis.process?.arch,
  isMusl = () => detectNativeMusl(require),
  preinitializedWasm,
  hasWebAssembly = () => typeof WebAssembly !== 'undefined',
} = {}) {
  const resolver = createCssBackendResolver({
    loadNative() {
      if (!require) return null;
      const suffix = nativeCssSuffix(platform, arch, platform === 'linux' && isMusl());
      return suffix
        ? normalizeCssArtifact(loadCssModule(require, `../../native-css/turbo-dom-css.${suffix}.node`))
        : null;
    },
    loadWasm() {
      if (preinitializedWasm) return normalizeCssArtifact(preinitializedWasm);
      if (!require || !hasWebAssembly()) return null;
      return normalizeCssArtifact(loadCssModule(require, '../../pkg-css/turbo_dom_css.js'));
    },
  });
  let cached;
  return {
    resolve(selection) {
      const { mode, binding, revision } = selection;
      if (cached && cached.mode === mode && cached.binding === binding && cached.revision === revision)
        return cached.backend;
      const backend = resolver.resolve({ mode, binding: normalizeCssArtifact(binding), revision });
      cached = { mode, binding, revision, backend };
      return backend;
    },
    reset() {
      cached = undefined;
      resolver.reset();
    },
  };
}
