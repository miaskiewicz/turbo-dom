import { nativeCssSuffix, loadCssModule, detectNativeMusl } from './css-native-loader.mjs';
export { nativeCssSuffix, loadCssModule, detectNativeMusl } from './css-native-loader.mjs';
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
  const cssComputedEnvironmentV1 = binding.cssComputedEnvironmentV1;
  const cssRuntimePolicyV1 = binding.cssRuntimePolicyV1;
  if (
    abi === undefined &&
    CompiledStylesV1 === undefined &&
    cssOptionalAbsent([
      measureSceneV1,
      cssUserAgentSourceV1,
      serializeCssomV1,
      projectCssomPropertyV1,
      cssComputedPreferencesV1,
      cssComputedEnvironmentV1,
      cssRuntimePolicyV1,
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
    cssComputedEnvironmentV1,
    cssRuntimePolicyV1,
  );
}



// Resolve only the exact requested artifact before initialization: nested missing
// dependencies and every initialization exception remain the original exception.




function nodeRequire() {
  return createParserRequire();
}

export function createCssArtifactResolver({
  require = nodeRequire(),
  platform = globalThis.process?.platform,
  arch = globalThis.process?.arch,
  isMusl = () => detectNativeMusl(require),
  preinitializedWasm,
  beforeWasmLoad,
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
      if (!require) return null;
      if(beforeWasmLoad!==undefined)beforeWasmLoad();
      if (!hasWebAssembly()) return null;
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
