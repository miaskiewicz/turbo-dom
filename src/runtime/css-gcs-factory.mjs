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
const createRecord = Object.create,
  resultEntries = Object.entries,
  NativeWeakMap = WeakMap,
  NativeSet = Set,
  NativeU16 = Uint16Array;
const weakGet = WeakMap.prototype.get,
  weakSet = WeakMap.prototype.set,
  weakDelete = WeakMap.prototype.delete;
const ownerAdd = Set.prototype.add,
  ownerDelete = Set.prototype.delete,
  ownerForEach = Set.prototype.forEach,
  ownerClear = Set.prototype.clear;
const DefaultWeak = globalThis.WeakRef,
  DefaultFinalizer = globalThis.FinalizationRegistry;
const defaultDeref = DefaultWeak?.prototype.deref,
  defaultRegister = DefaultFinalizer?.prototype.register,
  defaultUnregister = DefaultFinalizer?.prototype.unregister;
function dataProperty(target, key, value) {
  presentationDefine(target, key, { __proto__: null, value, writable: true, enumerable: true, configurable: true });
}
function presentationContainer(value) {
  const array = presentationArray(value),
    proto = presentationPrototype(value);
  if (!array && proto !== plainPresentationPrototype && proto !== null)
    throw new TypeError('turbo-dom: lazy property context cannot retain native or opaque handles');
  return array ? [] : createRecord(null);
}
function cloneData(value, seen, created) {
  if (typeof value === 'function') throw new TypeError('turbo-dom: lazy property context must be stateless data');
  if (value === null || typeof value !== 'object') return value;
  const prior = presentationApply(presentationGet, seen, [value]);
  if (prior) return prior;
  const owned = presentationContainer(value);
  presentationApply(presentationSet, seen, [value, owned]);
  dataProperty(created, created.length, value);
  const entries = presentationEntries(value);
  for (let index = 0; index < entries.length; index++) {
    const pair = entries[index];
    dataProperty(owned, pair[0], cloneData(pair[1], seen, created));
  }
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
  const copy = createRecord(null),
    entries = presentationEntries(context);
  for (let index = 0; index < entries.length; index++) dataProperty(copy, entries[index][0], entries[index][1]);
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
  const owners = [prepared, compiled];
  for (let index = 0; index < owners.length; index++) {
    try {
      owners[index]?.dispose();
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
  const entries = new NativeSet();
  const finalizer = new Finalizer((reference) => presentationApply(ownerDelete, entries, [reference]));
  const unregister = (reference) =>
    Finalizer === DefaultFinalizer ? presentationApply(defaultUnregister, finalizer, [reference]) : finalizer.unregister(reference);
  return {
    add(resources) {
      const reference = new Weak(resources);
      presentationApply(ownerAdd, entries, [reference]);
      if (Finalizer === DefaultFinalizer) presentationApply(defaultRegister, finalizer, [resources, reference, reference]);
      else finalizer.register(resources, reference, reference);
      return reference;
    },
    remove(reference) {
      presentationApply(ownerDelete, entries, [reference]);
      unregister(reference);
    },
    all() {
      const resources = [];
      presentationApply(ownerForEach, entries, [
        (reference) => {
          const resource = Weak === DefaultWeak ? presentationApply(defaultDeref, reference, []) : reference.deref();
          if (resource) dataProperty(resources, resources.length, resource);
        },
      ]);
      return resources;
    },
    clear() {
      presentationApply(ownerForEach, entries, [unregister]);
      presentationApply(ownerClear, entries, []);
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
  dataProperty(resources, 'reference', owners.add(resources));
  return {
    resources,
    compileKey: null,
    prepareKey: null,
    backend: null,
    targets: new NativeWeakMap(),
    generation: {},
  };
}
function assignData(target, values) {
  const entries = resultEntries(values);
  for (let index = 0; index < entries.length; index++) dataProperty(target, entries[index][0], entries[index][1]);
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
  assignData(state.resources, {
    compiled: next.compiled,
    prepared: next.prepared,
  });
  assignData(state, {
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
function resultKey(pseudo, value, encoder = encode) {
  const data = createRecord(null);
  if (pseudo !== undefined) dataProperty(data, 'pseudo', pseudo);
  if (value !== undefined) dataProperty(data, 'value', value);
  return encoder(data);
}
function copyResultValues(values) {
  const copy = createRecord(null),
    entries = resultEntries(values ?? {});
  for (let pair = 0; pair < entries.length; pair++) {
    const name = entries[pair][0],
      units = entries[pair][1];
    if (!presentationArray(units) && !(units instanceof NativeU16)) throw new TypeError('turbo-dom: invalid UTF16 computed style result');
    const owned = [];
    for (let index = 0; index < units.length; index++) dataProperty(owned, index, units[index]);
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
function resolvedRow(prepared, request, resolve, decode) {
  const output = decode(resolve(prepared, request.request));
  if (output.geometryOnly === true) throw new SharedCssCapabilityError('geometry-only rows cannot become computed style');
  let row;
  for (let index = 0; index < output.targets.length; index++) {
    if (output.targets[index].id === request.resultId) {
      row = output.targets[index];
      break;
    }
  }
  if (!row) throw new TypeError('turbo-dom: shared CSS result target missing');
  return { row, batch: output };
}

function presentationBatch(prepared, resolved, row, request, resolve, decode) {
  if (!presentationOwn(request, 'fullRequest') || !row.usedProperties?.length) return resolved;
  // Read the full request once. A trusted capture can defer its other target
  // rows until Rust reports that geometry is needed for a used property.
  const fullRequest = request.fullRequest;
  const complete = resolvedRow(prepared, { resultId: request.resultId, request: fullRequest }, resolve, decode);
  request.providerContext = request.materializeProvider();
  return complete;
}

function retireAll(resources, owners) {
  let failed = false,
    failure;
  for (let index = 0; index < resources.length; index++) {
    try {
      retire(resources[index], owners);
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

function presentationCache(resolveProperty) {
  if (resolveProperty !== undefined && typeof resolveProperty !== 'function')
    throw new TypeError('turbo-dom: resolveProperty must be a function');
  return resolveProperty === undefined ? null : new PresentationWeakMap();
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
  encodeCompileKey = encodeRequest,
  encodePrepareKey = encodeRequest,
  resolveBatch = (prepared, request) => prepared.resolveBatch(encodeRequest(request)),
  encodeResultKey,
  decodeResponse = decode,
  intrinsics = { WeakRef: DefaultWeak, FinalizationRegistry: DefaultFinalizer },
}) {
  if (typeof finalizeResult !== 'function') throw new TypeError('turbo-dom: shared CSS requires finalizeResult');
  const presentationSnapshots = presentationCache(resolveProperty);
  let documents = new NativeWeakMap(),
    owners,
    closed = false;
  function get(element, pseudo) {
    if (closed) throw new SharedCssCapabilityError('factory disposed');
    const backend = selectBackend();
    if (!backend) throw new SharedCssCapabilityError('backend unavailable');
    owners ??= registry(intrinsics);
    const context = readContext(element, pseudo);
    if (!object(context.document) || !object(element)) throw new TypeError('turbo-dom: shared CSS invalid owner');
    let state = presentationApply(weakGet, documents, [context.document]);
    if (!state) {
      state = newState(owners);
      presentationApply(weakSet, documents, [context.document, state]);
    }
    // Every read captures exact sources. Current mechanical snapshots explicitly
    // require scanning; an opaque document stamp never skips unknown CSS stores.
    const sources = captureSources(context.document, context);
    if (sources.status !== 'captured') throw new SharedCssCapabilityError('source capture unsupported');
    const compileKey = encodeCompileKey(compileRequest(sources, context));
    const prepareKey = encodePrepareKey(prepareRequest(sources, context));
    synchronize(state, backend, compileKey, prepareKey, createFacts, decodeResponse);
    const key = resultKey(pseudo, context.resultKey, encodeResultKey);
    let entry = presentationApply(weakGet, state.targets, [element]);
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
      const resolved = resolvedRow(state.resources.prepared, request, resolveBatch, decodeResponse);
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
      const batch = presentationBatch(state.resources.prepared, resolved, row, request, resolveBatch, decodeResponse);
      const presentation = presentationOptions(resolveProperty, row, batch, request, context, presentationSnapshots);
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
      presentationApply(weakSet, state.targets, [element, entry]);
    }
    return resultProxy(entry, context.version);
  }
  function reset(document) {
    if (document !== undefined) {
      const state = presentationApply(weakGet, documents, [document]);
      if (state) {
        presentationApply(weakDelete, documents, [document]);
        retire(state.resources, owners);
      }
      return;
    }
    const resources = owners?.all() ?? [];
    documents = new NativeWeakMap();
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
