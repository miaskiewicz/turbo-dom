import assert from 'node:assert/strict';
import test from 'node:test';
import { createSharedComputedStyle, SharedCssCapabilityError } from '../src/runtime/css-gcs-factory.mjs';
const units = (value) => Array.from(value, (char) => char.charCodeAt(0));
function fixture(overrides = {}) {
  const calls = { compile: 0, prepare: 0, resolve: 0, finalize: 0, capture: 0, facts: 0, dispose: [] };
  const document = {},
    element = {};
  const context = { document, version: 1, renderToken: {}, environmentKey: {}, resultKey: 'target' };
  let source = 'red';
  const backend = {
    CompiledStylesV1: class {
      constructor(request) {
        calls.compile++;
        this.source = JSON.parse(request).source;
      }
      metadata() {
        return JSON.stringify({ source: this.source });
      }
      prepare(request) {
        calls.prepare++;
        const color = this.source;
        return {
          metadata: () => JSON.stringify({ request }),
          resolveBatch() {
            calls.resolve++;
            return JSON.stringify({ targets: [{ id: 7, values: { color: units(color) }, custom: {} }] });
          },
          dispose() {
            calls.dispose.push('prepared');
          },
        };
      }
      dispose() {
        calls.dispose.push('compiled');
      }
    },
  };
  const options = {
    selectBackend: () => backend,
    readContext: () => context,
    captureSources: () => {
      calls.capture++;
      return { status: 'captured', source, provenance: { documentCertified: false, requiresSourceScan: true } };
    },
    compileRequest: (source) => ({ source: source.source }),
    prepareRequest: () => ({ environment: 'screen' }),
    resolveRequest: () => ({ request: { target: 7 }, resultId: 7 }),
    createFacts: (metadata) => {
      calls.facts++;
      return metadata;
    },
    finalizeResult: (row) => {
      calls.finalize++;
      return row;
    },
    ...overrides,
  };
  const factory = createSharedComputedStyle(options);
  return {
    factory,
    context,
    element,
    document,
    backend,
    calls,
    options,
    setSource(value) {
      source = value;
    },
  };
}
test('exact requests and certified target state reuse owned finalized style with zero warm bridge calls', () => {
  const f = fixture();
  const first = f.factory.get(f.element);
  assert.equal(first.color, 'red');
  assert.equal(f.factory.get(f.element), first);
  assert.deepEqual(
    [f.calls.compile, f.calls.prepare, f.calls.resolve, f.calls.finalize, f.calls.capture, f.calls.facts],
    [1, 1, 1, 1, 2, 1],
  );
  f.context.version++;
  const detached = f.factory.get(f.element);
  assert.notEqual(detached, first);
  assert.equal(detached.__v, 2);
  assert.equal(detached.color, 'red');
  assert.equal(first.__v, 1);
  assert.equal(f.calls.resolve, 1);
  f.factory.dispose();
  assert.equal(first.color, 'red');
  assert.equal(detached.color, 'red');
  assert.deepEqual(f.calls.dispose, ['prepared', 'compiled']);
  assert.throws(() => f.factory.get(f.element), SharedCssCapabilityError);
});
test('uncertified source reads detect direct CSSOM changes even while caller certificates stay unchanged', () => {
  const f = fixture();
  f.factory.get(f.element);
  f.setSource('blue');
  assert.equal(f.factory.get(f.element).color, 'blue');
  assert.equal(f.factory.get(f.element).color, 'blue');
  assert.deepEqual([f.calls.compile, f.calls.prepare, f.calls.resolve, f.calls.capture], [2, 2, 2, 3]);
  assert.deepEqual(f.calls.dispose, ['prepared', 'compiled']);
  f.factory.dispose();
});
test('connected/state/environment/target-key changes invalidate only required stages', () => {
  const f = fixture();
  f.factory.get(f.element);
  f.context.renderToken = {};
  f.factory.get(f.element);
  assert.equal(f.calls.resolve, 2);
  f.context.resultKey = 'focus';
  f.factory.get(f.element);
  assert.equal(f.calls.resolve, 3);
  f.context.environmentKey = {};
  f.factory.get(f.element);
  assert.equal(f.calls.resolve, 4);
  f.factory.get(f.element, '::before');
  assert.equal(f.calls.resolve, 5);
  f.context.renderToken = null;
  f.factory.get(f.element);
  f.factory.get(f.element);
  assert.equal(f.calls.resolve, 7);
  f.context.renderToken = {};
  f.context.environmentKey = null;
  f.factory.get(f.element);
  f.factory.get(f.element);
  assert.equal(f.calls.resolve, 9);
  assert.equal(f.calls.compile, 1);
  f.factory.dispose();
});
test('prepare environment and explicit limit changes have exact keys and preserve compiled owners', () => {
  let environment = 'screen';
  const f = fixture({ prepareRequest: () => ({ environment }) });
  f.factory.get(f.element);
  environment = 'print';
  f.factory.get(f.element);
  assert.deepEqual([f.calls.compile, f.calls.prepare, f.calls.resolve], [1, 2, 2]);
  assert.deepEqual(f.calls.dispose, ['prepared']);
  f.factory.reset(f.document);
  assert.deepEqual(f.calls.dispose, ['prepared', 'prepared', 'compiled']);
  f.factory.get(f.element);
  assert.equal(f.calls.compile, 2);
  f.factory.reset({});
  f.factory.reset();
  f.factory.reset();
});
test('capability/lifecycle absence is explicit before capture or compilation', () => {
  let reads = 0;
  const absent = fixture({
    selectBackend: () => null,
    readContext() {
      reads++;
    },
  });
  assert.throws(() => absent.factory.get(absent.element), SharedCssCapabilityError);
  assert.equal(reads, 0);
  for (const intrinsics of [
    { WeakRef: undefined, FinalizationRegistry },
    { WeakRef, FinalizationRegistry: undefined },
  ]) {
    const f = fixture({
      intrinsics,
      readContext() {
        reads++;
      },
    });
    assert.throws(() => f.factory.get(f.element), SharedCssCapabilityError);
  }
  assert.equal(reads, 0);
  assert.throws(() => createSharedComputedStyle({}), /finalizeResult/);
});
test('owner/source/request/finalization errors propagate without stale fallback', () => {
  const error = {};
  for (const key of [
    'selectBackend',
    'readContext',
    'captureSources',
    'compileRequest',
    'prepareRequest',
    'resolveRequest',
    'finalizeResult',
    'createFacts',
  ]) {
    const f = fixture({
      [key]() {
        throw error;
      },
    });
    assert.throws(
      () => f.factory.get(f.element),
      (value) => value === error,
    );
    f.factory.reset();
  }
  const invalid = fixture({ readContext: () => ({ document: null }) });
  assert.throws(() => invalid.factory.get(invalid.element), TypeError);
  const unsupported = fixture({ captureSources: () => ({ status: 'unsupported' }) });
  assert.throws(() => unsupported.factory.get(unsupported.element), SharedCssCapabilityError);
});
test('global reset retires every live document, snapshots survive, deterministic unregister occurs', () => {
  const registered = new Map(),
    deleted = [];
  class Finalizer {
    constructor(callback) {
      this.callback = callback;
    }
    register(target, holding, token) {
      registered.set(token, { target, holding });
    }
    unregister(token) {
      deleted.push(token);
      registered.delete(token);
    }
  }
  const f = fixture({ intrinsics: { WeakRef, FinalizationRegistry: Finalizer } });
  const first = f.factory.get(f.element);
  f.context.document = {};
  const second = f.factory.get({});
  assert.equal(registered.size, 2);
  f.factory.reset();
  assert.equal(registered.size, 0);
  assert.equal(deleted.length, 2);
  assert.equal(first.color, 'red');
  assert.equal(second.color, 'red');
  assert.deepEqual(f.calls.dispose, ['prepared', 'compiled', 'prepared', 'compiled']);
});
test('original initialization error survives cleanup failures and missing target is explicit', () => {
  const original = {},
    cleanup = {};
  const backend = {
    CompiledStylesV1: class {
      metadata() {
        throw original;
      }
      dispose() {
        throw cleanup;
      }
    },
  };
  const f = fixture({ selectBackend: () => backend });
  assert.throws(
    () => f.factory.get(f.element),
    (error) => error === original,
  );
  f.factory.dispose();
  const missing = fixture({ resolveRequest: () => ({ request: {}, resultId: 99 }) });
  assert.throws(() => missing.factory.get(missing.element), /result target missing/);
  missing.factory.dispose();
});
test('retirement failures dispose new owners, clear invalid state and retain the first error', () => {
  const first = {},
    second = {},
    calls = [];
  let bad = false;
  const backend = {
    CompiledStylesV1: class {
      metadata() {
        return '{}';
      }
      prepare() {
        return {
          metadata: () => '{}',
          resolveBatch: () => JSON.stringify({ targets: [{ id: 7, values: {}, custom: {} }] }),
          dispose() {
            calls.push('prepared');
            if (bad) throw first;
          },
        };
      }
      dispose() {
        calls.push('compiled');
        if (bad) throw second;
      }
    },
  };
  const f = fixture({ selectBackend: () => backend });
  f.factory.get(f.element);
  bad = true;
  f.setSource('changed');
  assert.throws(
    () => f.factory.get(f.element),
    (error) => error === first,
  );
  assert.deepEqual(calls, ['prepared', 'compiled', 'prepared', 'compiled']);
  bad = false;
  f.factory.get(f.element);
  f.context.document = {};
  f.factory.get({});
  bad = true;
  assert.throws(
    () => f.factory.reset(),
    (error) => error === first,
  );
  assert.deepEqual(calls.slice(-4), ['prepared', 'compiled', 'prepared', 'compiled']);
});
test('prepare-only retirement failure still retires the shared compiled owner exactly once', () => {
  let environment = 'a',
    bad = false;
  const first = {},
    calls = [];
  const backend = {
    CompiledStylesV1: class {
      metadata() {
        return '{}';
      }
      prepare() {
        return {
          metadata: () => '{}',
          resolveBatch: () => JSON.stringify({ targets: [{ id: 7, values: {}, custom: {} }] }),
          dispose() {
            calls.push('prepared');
            if (bad) throw first;
          },
        };
      }
      dispose() {
        calls.push('compiled');
      }
    },
  };
  const f = fixture({ selectBackend: () => backend, prepareRequest: () => ({ environment }) });
  f.factory.get(f.element);
  environment = 'b';
  bad = true;
  assert.throws(
    () => f.factory.get(f.element),
    (error) => error === first,
  );
  assert.deepEqual(calls, ['prepared', 'prepared', 'compiled']);
  bad = false;
  f.factory.dispose();
});
test('optional matching compiler and default mechanical capture work without hidden request defaults', async () => {
  const { createEnvironment } = await import('../src/runtime/index.mjs');
  const { document } = createEnvironment('<style>.a{}</style>');
  const f = fixture({
    createFacts: undefined,
    captureSources: undefined,
    readContext: () => ({ document, version: 1, renderToken: {}, environmentKey: {} }),
    compileRequest: (sources) => ({ source: sources.sources[0].text }),
  });
  assert.equal(f.factory.get(f.element).color, '.a{}');
  f.factory.dispose();
});
test('finalizer removes weak registry entries and dead weak references are not retained', () => {
  let callback, holding;
  const deleted = [];
  class Weak {
    constructor(target) {
      this.target = target;
    }
    deref() {
      return this.target;
    }
  }
  class Finalizer {
    constructor(fn) {
      callback = fn;
    }
    register(_target, value) {
      holding = value;
    }
    unregister(token) {
      deleted.push(token);
    }
  }
  const f = fixture({ intrinsics: { WeakRef: Weak, FinalizationRegistry: Finalizer } });
  f.factory.get(f.element);
  holding.target = undefined;
  callback(holding);
  f.factory.reset();
  assert.equal(deleted.length, 0);
});
test('only owned final strings survive: version-only reuse never rereads provider rows or retains extra handles', () => {
  let reads = 0;
  const row = {
    get values() {
      reads++;
      return { color: [114, 101, 100] };
    },
    custom: {},
    scene: { hostHandle: true },
  };
  const f = fixture({ finalizeResult: () => row });
  const original = f.factory.get(f.element);
  assert.equal(reads, 1);
  Object.defineProperty(row, 'values', {
    get() {
      throw Error('retired provider');
    },
  });
  f.context.version = 2;
  const updated = f.factory.get(f.element);
  assert.equal(updated.__v, 2);
  assert.equal(original.__v, 1);
  assert.equal(updated.color, 'red');
  assert.equal(reads, 1);
  f.factory.dispose();
  assert.equal(updated.color, 'red');
});
test('old public proxy descriptor edits cannot poison a new version of the owned result', () => {
  const f = fixture();
  const first = f.factory.get(f.element);
  Object.defineProperty(first, '__v', { value: 1, writable: false, configurable: false });
  f.context.version = 2;
  const second = f.factory.get(f.element);
  assert.equal(second.__v, 2);
  assert.equal(second.color, 'red');
  f.factory.dispose();
  const bad = fixture({ finalizeResult: () => ({ values: { color: 'not units' } }) });
  assert.throws(() => bad.factory.get(bad.element), TypeError);
  bad.factory.dispose();
});
test('factory weak ownership never roots document or element through native owner registry', async () => {
  const { spawnSync } = await import('node:child_process');
  const module = new URL('../src/runtime/css-gcs-factory.mjs', import.meta.url).href;
  const program = `
    const {createSharedComputedStyle}=await import(${JSON.stringify(module)});
    const backend={CompiledStylesV1:class{metadata(){return '{}';}prepare(){return {metadata:()=> '{}',resolveBatch:()=>JSON.stringify({targets:[{id:1,values:{color:[114,101,100]},custom:{}}]}),dispose(){}};}dispose(){}}};
    const factory=createSharedComputedStyle({selectBackend:()=>backend,readContext:el=>el.context,captureSources:()=>({status:'captured'}),compileRequest:()=>({}),prepareRequest:()=>({}),resolveRequest:()=>({request:{},resultId:1}),finalizeResult:row=>row});
    let held,style;
    function scene(){const document={},element={context:{document,version:1,renderToken:{},environmentKey:{},resultKey:'x'}};style=factory.get(element);return [new WeakRef(document),new WeakRef(element)];}
    const weak=scene();await new Promise(resolve=>setImmediate(resolve));for(let index=0;index<3;index++)gc();
    if(weak.some(value=>value.deref()!==undefined))throw Error('factory rooted DOM');
    if(style.color!=='red')throw Error('retained style lost value');factory.dispose();if(style.color!=='red')throw Error('dispose lost snapshot');
  `;
  const child = spawnSync(process.execPath, ['--expose-gc', '--input-type=module', '-e', program], {
    encoding: 'utf8',
  });
  assert.equal(child.status, 0, child.stderr);
});
