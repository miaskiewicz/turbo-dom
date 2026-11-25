// Optional CSS capability resolution. Parser selection supplies the mode and revision;
// this module neither changes that selection nor owns compiled/prepared models.

function capability(binding) {
	if (binding == null) return null;
	const { cssAbiVersion, CompiledStylesV1, measureSceneV1 } = binding;
	if (cssAbiVersion === undefined && CompiledStylesV1 === undefined && measureSceneV1 === undefined)
		return null;
	if (cssAbiVersion !== 1 || typeof CompiledStylesV1 !== "function") {
		throw new TypeError(
			"turbo-dom: incompatible CSS backend ABI (expected ABI 1 and CompiledStylesV1)",
		);
	}
	return cssCapabilityDescriptor(cssAbiVersion, CompiledStylesV1, measureSceneV1);
}

function load(loader) {
	// Loaders return null/undefined for known absence. Exceptions, including nested
	// MODULE_NOT_FOUND failures, belong to the original loader and must propagate.
	return loader ? capability(loader()) : null;
}

export function createCssBackendResolver({ loadNative, loadWasm } = {}) {
	let cached = null;
	const loaders = {
		native: () => load(loadNative),
		wasm: () => load(loadWasm),
		auto: () => load(loadNative) ?? load(loadWasm),
	};
	return {
		resolve({ mode, binding, revision }) {
			if (mode !== "native" && mode !== "wasm" && mode !== "auto") {
				throw new TypeError("turbo-dom: invalid CSS backend mode");
			}
			if (
				cached &&
				cached.mode === mode &&
				cached.binding === binding &&
				cached.revision === revision
			) {
				return cached.backend;
			}
			const backend = capability(binding) ?? loaders[mode]();
			cached = { mode, binding, revision, backend };
			return backend;
		},
		reset() {
			cached = null;
		},
	};
}

// Shared optional descriptor normalization for raw artifact and injected bindings.
export function cssCapabilityDescriptor(cssAbiVersion, CompiledStylesV1, measureSceneV1) {
  if (measureSceneV1 !== undefined && typeof measureSceneV1 !== 'function') {
    throw new TypeError('turbo-dom: incompatible CSS backend ABI (measureSceneV1 must be a function)');
  }
  return Object.freeze({ cssAbiVersion, CompiledStylesV1, ...(measureSceneV1 === undefined ? {} : { measureSceneV1 }) });
}
