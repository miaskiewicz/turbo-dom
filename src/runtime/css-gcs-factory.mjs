// Model ownership and exact mechanical cache keys only. The caller supplies all
// CSS contexts, matching facts, requests and limits; this module evaluates none.
import { captureCssSources } from './css-sources.mjs';
import { createComputedStyleProxy } from './css-computed-proxy.mjs';

const encode = JSON.stringify;
const decode = JSON.parse;
const freezePresentation = Object.freeze;
const presentationEntries = Object.entries;
const presentationPrototype = Object.getPrototypeOf;
const presentationDefine = Object.defineProperty;
const presentationArray = Array.isArray;
const PresentationWeakMap = WeakMap;
const presentationGet = WeakMap.prototype.get,
  presentationSet = WeakMap.prototype.set,
  presentationDelete = WeakMap.prototype.delete;
const presentationOwn = Object.hasOwn;
const presentationApply = Reflect.apply;
const plainPresentationPrototype = Object.prototype;
function dataProperty(target, key, value) {
  presentationDefine(target, key, { __proto__: null, value, writable: true, enumerable: true, configurable: true });
}
function presentationContainer(value) {
  const array = presentationArray(value),
    proto = presentationPrototype(value);
  if (!array && proto !== plainPresentationPrototype && proto !== null)
    throw new TypeError('turbo-dom: lazy property context cannot retain native or opaque handles');
  return array ? [] : Object.create(null);
}
function cloneData(value, seen, created) {
  if (typeof value === 'function') throw new TypeError('turbo-dom: lazy property context must be stateless data');
  if (value === null || typeof value !== 'object') return value;
  const prior = presentationApply(presentationGet, seen, [value]);
  if (prior) return prior;
  const owned = presentationContainer(value);
  presentationApply(presentationSet, seen, [value, owned]);
  dataProperty(created, created.length, value);
  for (const [key, entry] of presentationEntries(value)) dataProperty(owned, key, cloneData(entry, seen, created));
  return freezePresentation(owned);
}
function immutableData(value, seen) {
  const created = [];
  try {
    return cloneData(value, seen, created);
  } catch (error) {
    for (let index = 0; index < created.length; index++) presentationApply(presentationDelete, seen, [created[index]]);
    throw error;
  }
}
function immutableReadContext(context) {
  const copy = Object.create(null);
  for (const [key, value] of presentationEntries(context)) dataProperty(copy, key, value);
  return freezePresentation(copy);
}
function propertyContext(resolved, request, context, snapshots) {
  const row = immutableData(resolved.row, snapshots),
    batch = immutableData(resolved.batch, snapshots);
  const providerContext = immutableData(request.providerContext, snapshots);
  return freezePresentation({ row, batch, providerContext, context: immutableReadContext(context) });
}
function presentationOptions(resolveProperty, row, resolved, request, context, snapshots) {
  if (resolveProperty === undefined) return undefined;
  return {
    resolveProperty,
    propertyContext: row.usedProperties.length ? propertyContext(resolved, request, context, snapshots) : undefined,
  };
}
function usedMetadata(row, snapshots) {
  const metadata = row.usedProperties;
  if (!presentationArray(metadata)) throw new TypeError('turbo-dom: missing or invalid Rust usedProperties metadata');
  for (let index = 0; index < metadata.length; index++) {
    const name = metadata[index];
    if (typeof name !== 'string' || name.length === 0)
      throw new TypeError('turbo-dom: invalid Rust usedProperties metadata');
    if (!presentationOwn(row.values ?? {}, name) && !presentationOwn(row.custom ?? {}, name))
      throw new TypeError('turbo-dom: invalid Rust usedProperties metadata');
  }
  return immutableData(metadata, snapshots);
}

const object = (value) => value !== null && (typeof value === 'object' || typeof value === 'function');
export class SharedCssCapabilityError extends Error {
  constructor(message) {
    super(`turbo-dom: shared CSS ${message}`);
    this.name = 'SharedCssCapabilityError';
  }
}
function release(compiled, prepared) {
  let failed = false,
    failure;
  for (const owner of [prepared, compiled]) {
    try {
      owner?.dispose();
    } catch (error) {
      if (!failed) {
        failed = true;
        failure = error;
      }
    }
  }
  if (failed) throw failure;
}
function registry(intrinsics) {
  const Weak = intrinsics.WeakRef,
    Finalizer = intrinsics.FinalizationRegistry;
  if (typeof Weak !== 'function' || typeof Finalizer !== 'function')
    throw new SharedCssCapabilityError('requires WeakRef and FinalizationRegistry');
  const entries = new Set();
  const finalizer = new Finalizer((reference) => entries.delete(reference));
  return {
    add(resources) {
      const reference = new Weak(resources);
      entries.add(reference);
      finalizer.register(resources, reference, reference);
      return reference;
    },
    remove(reference) {
      entries.delete(reference);
      finalizer.unregister(reference);
    },
    all() {
      return Array.from(entries, (reference) => reference.deref()).filter(Boolean);
    },
    clear() {
      for (const reference of entries) finalizer.unregister(reference);
      entries.clear();
    },
  };
}
function retire(resources, owners) {
  const compiled = resources.compiled,
    prepared = resources.prepared;
  resources.compiled = resources.prepared = null;
  owners.remove(resources.reference);
  release(compiled, prepared);
}
function newState(owners) {
  const resources = { compiled: null, prepared: null };
  resources.reference = owners.add(resources);
  return { resources, compileKey: null, prepareKey: null, backend: null, targets: new WeakMap(), generation: {} };
}
function replacement(state, backend, compileKey, prepareKey, createFacts, decode) {
  const fresh = state.backend !== backend || state.compileKey !== compileKey;
  let compiled = state.resources.compiled,
    prepared;
  try {
    if (fresh) compiled = new backend.CompiledStylesV1(compileKey);
    const metadata = fresh ? decode(compiled.metadata()) : state.metadata;
    const facts = fresh ? createFacts?.(metadata) : state.facts;
    prepared = compiled.prepare(prepareKey);
    const preparedMetadata = decode(prepared.metadata());
    return { compiled, prepared, metadata, facts, preparedMetadata, fresh };
  } catch (error) {
    // Cleanup cannot replace the original provider/transport failure.
    try {
      release(fresh ? compiled : null, prepared);
    } catch {
      /* preserve original */
    }
    throw error;
  }
}
function synchronize(state, backend, compileKey, prepareKey, createFacts, decode) {
  if (state.backend === backend && state.compileKey === compileKey && state.prepareKey === prepareKey) return;
  const next = replacement(state, backend, compileKey, prepareKey, createFacts, decode);
  try {
    release(next.fresh ? state.resources.compiled : null, state.resources.prepared);
  } catch (error) {
    try {
      release(next.compiled, next.prepared);
    } catch {
      /* preserve original */
    }
    state.resources.compiled = state.resources.prepared = null;
    state.compileKey = state.prepareKey = null;
    throw error;
  }
  Object.assign(state.resources, { compiled: next.compiled, prepared: next.prepared });
  Object.assign(state, {
    compileKey,
    prepareKey,
    backend,
    metadata: next.metadata,
    facts: next.facts,
    preparedMetadata: next.preparedMetadata,
    generation: {},
  });
}
function cachedResult(entry, context, generation, key) {
  return (
    entry &&
    context.renderToken != null &&
    context.environmentKey != null &&
    entry.renderToken === context.renderToken &&
    entry.environmentKey === context.environmentKey &&
    entry.generation === generation &&
    entry.key === key
  );
}
function copyResultValues(values) {
  const copy = Object.create(null);
  for (const [name, units] of Object.entries(values ?? {})) {
    if (!Array.isArray(units) && !(units instanceof Uint16Array))
      throw new TypeError('turbo-dom: invalid UTF16 computed style result');
    const owned = [];
    for (let index = 0; index < units.length; index++) owned.push(units[index]);
    copy[name] = owned;
  }
  return copy;
}
function ownResult(row, resolveProperty, snapshots) {
  const owned = { values: copyResultValues(row.values), custom: copyResultValues(row.custom) };
  if (resolveProperty !== undefined) owned.usedProperties = usedMetadata(row, snapshots);
  return owned;
}
function resultProxy(entry, version) {
  if (entry.version !== version) {
    entry.proxy = createComputedStyleProxy(entry.row, version, entry.presentation);
    entry.version = version;
  }
  return entry.proxy;
}
function resolvedRow(prepared, request, encode, decode) {
  const output = decode(prepared.resolveBatch(encode(request.request)));
  if (output.geometryOnly === true)
    throw new SharedCssCapabilityError('geometry-only rows cannot become computed style');
  const row = output.targets.find((row) => row.id === request.resultId);
  if (!row) throw new TypeError('turbo-dom: shared CSS result target missing');
  return { row, batch: output };
}

function retireAll(resources, owners) {
  let failed = false,
    failure;
  for (const resource of resources) {
    try {
      retire(resource, owners);
    } catch (error) {
      if (!failed) {
        failed = true;
        failure = error;
      }
    }
  }
  owners?.clear();
  if (failed) throw failure;
}

export function createSharedComputedStyle({
  selectBackend,
  readContext,
  captureSources = captureCssSources,
  compileRequest,
  prepareRequest,
  resolveRequest,
  finalizeResult,
  resolveProperty,
  createFacts,
  encodeRequest = encode,
  decodeResponse = decode,
  intrinsics = { WeakRef: globalThis.WeakRef, FinalizationRegistry: globalThis.FinalizationRegistry },
}) {
  if (typeof finalizeResult !== 'function') throw new TypeError('turbo-dom: shared CSS requires finalizeResult');
  if (resolveProperty !== undefined && typeof resolveProperty !== 'function')
    throw new TypeError('turbo-dom: resolveProperty must be a function');
  const presentationSnapshots = resolveProperty === undefined ? null : new PresentationWeakMap();
  let documents = new WeakMap(),
    owners,
    closed = false;
  function get(element, pseudo) {
    if (closed) throw new SharedCssCapabilityError('factory disposed');
    const backend = selectBackend();
    if (!backend) throw new SharedCssCapabilityError('backend unavailable');
    owners ??= registry(intrinsics);
    const context = readContext(element, pseudo);
    if (!object(context.document) || !object(element)) throw new TypeError('turbo-dom: shared CSS invalid owner');
    let state = documents.get(context.document);
    if (!state) {
      state = newState(owners);
      documents.set(context.document, state);
    }
    // Every read captures exact sources. Current mechanical snapshots explicitly
    // require scanning; an opaque document stamp never skips unknown CSS stores.
    const sources = captureSources(context.document, context);
    if (sources.status !== 'captured') throw new SharedCssCapabilityError('source capture unsupported');
    const compileKey = encodeRequest(compileRequest(sources, context));
    const prepareKey = encodeRequest(prepareRequest(sources, context));
    synchronize(state, backend, compileKey, prepareKey, createFacts, decodeResponse);
    const key = encode({ pseudo, value: context.resultKey });
    let entry = state.targets.get(element);
    if (!cachedResult(entry, context, state.generation, key)) {
      const request = resolveRequest({
        element,
        pseudo,
        context,
        sources,
        metadata: state.metadata,
        preparedMetadata: state.preparedMetadata,
        prepared: state.resources.prepared,
        facts: state.facts,
      });
      const resolved = resolvedRow(state.resources.prepared, request, encodeRequest, decodeResponse);
      const row = ownResult(
        finalizeResult(resolved.row, {
          element,
          pseudo,
          context,
          sources,
          backend,
          batch: resolved.batch,
          providerContext: request.providerContext,
        }),
        resolveProperty,
        presentationSnapshots,
      );
      const presentation = presentationOptions(resolveProperty, row, resolved, request, context, presentationSnapshots);
      const proxy = createComputedStyleProxy(row, context.version, presentation);
      entry = {
        row,
        version: context.version,
        proxy,
        presentation,
        key,
        generation: state.generation,
        renderToken: context.renderToken,
        environmentKey: context.environmentKey,
      };
      state.targets.set(element, entry);
    }
    return resultProxy(entry, context.version);
  }
  function reset(document) {
    if (document !== undefined) {
      const state = documents.get(document);
      if (state) {
        documents.delete(document);
        retire(state.resources, owners);
      }
      return;
    }
    const resources = owners?.all() ?? [];
    documents = new WeakMap();
    retireAll(resources, owners);
  }
  return {
    get,
    reset,
    dispose() {
      closed = true;
      reset();
    },
  };
}
