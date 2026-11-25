import test from 'node:test';
import assert from 'node:assert/strict';
import { createComputedStyleProxy } from '../src/runtime/css-computed-proxy.mjs';
import { createSharedComputedStyle } from '../src/runtime/css-gcs-factory.mjs';
const units = (text) => Array.from(text, (letter) => letter.charCodeAt(0));
const row = (usedProperties) => ({
  id: 7,
  values: { color: units('red'), width: units('auto'), height: units('auto') },
  rendererValues: { width: units('auto') },
  custom: {},
  usedProperties,
});
test('only canonical Rust-used keys invoke lazy presentation, once on success', () => {
  const input = row(['width', 'height']),
    original = JSON.stringify(input),
    fault = { cause: 'height unknown' },
    calls = [];
  let ready = false;
  const proxy = createComputedStyleProxy(input, 1, {
    propertyContext: { tag: 'owned' },
    resolveProperty(name, base, context) {
      calls.push(name);
      assert.equal(base, 'auto');
      assert.equal(context.tag, 'owned');
      if (name === 'height' && !ready) throw fault;
      return name === 'width' ? '42px' : '20px';
    },
  });
  assert.equal(proxy.color, 'red');
  assert.equal(proxy.length, 3);
  assert.equal(proxy.item(0), 'color');
  assert.deepEqual([...proxy], ['color', 'width', 'height']);
  assert.equal(proxy.getPropertyPriority('width'), '');
  assert.deepEqual(calls, []);
  assert.equal(proxy.width, '42px');
  assert.equal(proxy.getPropertyValue('WIDTH'), '42px');
  assert.deepEqual(calls, ['width']);
  assert.throws(
    () => proxy.height,
    (error) => error === fault,
  );
  assert.throws(
    () => proxy.height,
    (error) => error === fault,
  );
  ready = true;
  assert.equal(proxy.height, '20px');
  assert.equal(proxy.height, '20px');
  assert.deepEqual(calls, ['width', 'height', 'height', 'height']);
  assert.equal(JSON.stringify(input), original);
});
test('metadata is mandatory only for supplied callback and invalid success stays retryable', () => {
  assert.equal(createComputedStyleProxy(row(undefined), 1).width, 'auto');
  for (const metadata of [undefined, null, {}, ['missing'], [3], ['']])
    assert.throws(() => createComputedStyleProxy(row(metadata), 1, { resolveProperty: () => '' }), TypeError);
  assert.throws(() => createComputedStyleProxy(row([]), 1, { resolveProperty: null }), TypeError);
  let calls = 0;
  const empty = createComputedStyleProxy(row([]), 1, {
    resolveProperty() {
      calls++;
      throw Error('not admitted');
    },
  });
  assert.equal(empty.width, 'auto');
  assert.equal(calls, 0);
  let valid = false;
  const retry = createComputedStyleProxy(row(['width']), 1, {
    resolveProperty() {
      calls++;
      return valid ? '1px' : 4;
    },
  });
  assert.throws(() => retry.width, /return a string/);
  valid = true;
  assert.equal(retry.width, '1px');
  assert.equal(retry.width, '1px');
  assert.equal(calls, 2);
});
function fixture(metadata = ['width', 'height'], overrides = {}) {
  const document = {},
    element = {},
    context = { document, version: 1, renderToken: {}, environmentKey: {}, resultKey: 'owned' },
    providerContext = { scene: { rows: [{ id: 7, width: 42, height: 20 }] } };
  const sourceRow = row(metadata),
    batch = { targets: [sourceRow] },
    calls = { resolve: 0, disposed: 0 },
    backend = {
      CompiledStylesV1: class {
        metadata() {
          return '{}';
        }
        prepare() {
          return {
            metadata: () => '{}',
            resolveBatch() {
              calls.resolve++;
              return JSON.stringify(batch);
            },
            dispose() {
              calls.disposed++;
            },
          };
        }
        dispose() {
          calls.disposed++;
        }
      },
    };
  const options = {
    selectBackend: () => backend,
    readContext: () => context,
    captureSources: () => ({ status: 'captured' }),
    compileRequest: () => ({}),
    prepareRequest: () => ({}),
    resolveRequest: () => ({ request: {}, resultId: 7, providerContext }),
    finalizeResult: (value) => value,
    ...overrides,
  };
  return {
    factory: createSharedComputedStyle(options),
    options,
    context,
    providerContext,
    sourceRow,
    batch,
    calls,
    document,
    element,
  };
}
test('factory retained proxies own frozen renderer snapshots after dispose and reset', () => {
  const contexts = [],
    f = fixture(['width', 'height'], {
      resolveProperty(name, base, context) {
        contexts.push(context);
        assert.equal(base, 'auto');
        assert.equal(Object.isFrozen(context), true);
        assert.equal(Object.isFrozen(context.row.values.width), true);
        assert.equal(Object.isFrozen(context.batch.targets[0].rendererValues.width), true);
        assert.equal(Object.isFrozen(context.providerContext.scene.rows), true);
        assert.equal(context.context.document, f.document);
        assert.equal('prepared' in context, false);
        assert.equal('backend' in context, false);
        return name === 'width' ? '42px' : '20px';
      },
    });
  const proxy = f.factory.get(f.element);
  assert.equal(proxy.color, 'red');
  assert.equal(contexts.length, 0);
  f.factory.reset(f.document);
  f.factory.dispose();
  f.providerContext.scene.rows[0].width = 999;
  assert.equal(proxy.width, '42px');
  assert.equal(proxy.width, '42px');
  assert.equal(proxy.height, '20px');
  assert.equal(contexts.length, 2);
  assert.equal(contexts[0].providerContext.scene.rows[0].width, 42);
  assert.deepEqual(f.sourceRow.values.width, units('auto'));
  assert.deepEqual(contexts[0].batch.targets[0].rendererValues.width, units('auto'));
  assert.equal(f.calls.resolve, 1);
  assert.equal(f.calls.disposed, 2);
});
test('empty metadata never traverses provider context or batch and missing metadata fails clearly', () => {
  let reads = 0;
  const f = fixture([], {
    resolveProperty() {
      throw Error('not admitted');
    },
    resolveRequest() {
      const provider = {};
      Object.defineProperty(provider, 'scene', {
        enumerable: true,
        get() {
          reads++;
          throw Error('not traversed');
        },
      });
      return { request: {}, resultId: 7, providerContext: provider };
    },
  });
  assert.equal(f.factory.get(f.element).color, 'red');
  assert.equal(reads, 0);
  f.factory.dispose();
  assert.throws(
    () =>
      fixture([], {
        resolveProperty: () => '',
        finalizeResult: (value) => ({ ...value, usedProperties: undefined }),
      }).factory.get({}),
    /usedProperties/,
  );
  assert.throws(() => fixture([], { resolveProperty: null }), /resolveProperty/);
});
test('per-factory weak memo reuses owned batch and provider snapshots between used views', () => {
  const observed = [],
    source = row(['width']),
    batch = { targets: [source] },
    providerContext = { scene: { rows: [{ id: 7 }] } },
    f = fixture(['width'], {
      decodeResponse: (value) => (typeof value === 'string' ? JSON.parse(value) : value),
      resolveProperty(name, base, context) {
        observed.push(context);
        return '1px';
      },
    });
  // A direct backend intentionally returns the SAME immutable source objects.
  const backend = {
    CompiledStylesV1: class {
      metadata() {
        return {};
      }
      prepare() {
        return { metadata: () => ({}), resolveBatch: () => batch, dispose() {} };
      }
      dispose() {}
    },
  };
  const factory = createSharedComputedStyle({
    ...f.options,
    selectBackend: () => backend,
    resolveRequest: () => ({ request: {}, resultId: 7, providerContext }),
    decodeResponse: (value) => value,
    encodeRequest: (value) => value,
  });
  factory.get({}).width;
  factory.get({}).width;
  assert.equal(observed[0].batch, observed[1].batch);
  assert.equal(observed[0].providerContext, observed[1].providerContext);
  factory.dispose();
  f.factory.dispose();
});
test('new lazy context rejects retained native handles and preserves original getter errors', () => {
  class NativeOwner {}
  const f = fixture(['width'], {
    resolveProperty: () => '',
    resolveRequest: () => ({ request: {}, resultId: 7, providerContext: new NativeOwner() }),
  });
  assert.throws(() => f.factory.get(f.element), /native or opaque/);
  f.factory.dispose();
  const fault = {},
    bad = {};
  Object.defineProperty(bad, 'scene', {
    enumerable: true,
    get() {
      throw fault;
    },
  });
  const g = fixture(['width'], {
    resolveProperty: () => '',
    resolveRequest: () => ({ request: {}, resultId: 7, providerContext: bad }),
  });
  assert.throws(
    () => g.factory.get(g.element),
    (error) => error === fault,
  );
  assert.throws(
    () => g.factory.get(g.element),
    (error) => error === fault,
  );
  g.factory.dispose();
});

test('lazy callbacks cannot redirect private presentation stores through changed collection primordials', () => {
  const proxy = createComputedStyleProxy(row(['width']), 1, { resolveProperty: () => '42px' }),
    changes = [];
  let calls = 0,
    result;
  function replace(object, key, value) {
    changes.push([object, key, Object.getOwnPropertyDescriptor(object, key)]);
    Object.defineProperty(object, key, { configurable: true, writable: true, value });
  }
  try {
    for (const key of ['get', 'set', 'has'])
      replace(Map.prototype, key, function () {
        calls++;
        throw Error('private map exposed');
      });
    for (const key of ['has', 'delete'])
      replace(Set.prototype, key, function () {
        calls++;
        throw Error('private set exposed');
      });
    replace(Array.prototype, Symbol.iterator, function () {
      calls++;
      throw Error('private array exposed');
    });
    result = {
      width: proxy.width,
      again: proxy.width,
      color: proxy.color,
      item: proxy.item(0),
      iter: proxy[Symbol.iterator]().next().value,
    };
  } finally {
    for (let index = changes.length - 1; index >= 0; index--) {
      const item = changes[index];
      Object.defineProperty(item[0], item[1], item[2]);
    }
  }
  assert.equal(calls, 0);
  assert.deepEqual(result, { width: '42px', again: '42px', color: 'red', item: 'color', iter: 'color' });
});

test('metadata-only admission preserves custom case and accepts null-prototype pure context records', () => {
  const provider = Object.create(null);
  provider.token = 'owned';
  let seen;
  const f = fixture(['--Case'], {
    finalizeResult: () => ({
      custom: { '--Case': units('base'), '--case': units('other') },
      usedProperties: ['--Case'],
    }),
    resolveRequest: () => ({ request: {}, resultId: 7, providerContext: provider }),
    resolveProperty(name, base, context) {
      seen = context;
      assert.equal(name, '--Case');
      assert.equal(base, 'base');
      return 'presented';
    },
  });
  const proxy = f.factory.get(f.element);
  assert.equal(proxy.getPropertyValue('--case'), 'other');
  assert.equal(seen, undefined);
  assert.equal(proxy.getPropertyValue('--Case'), 'presented');
  assert.equal(seen.providerContext.token, 'owned');
  assert.equal(Object.getPrototypeOf(seen.providerContext), null);
  f.factory.dispose();
});

test('factory rejects malformed admitted names before copying provider context', () => {
  for (const metadata of [[3], [''], ['missing']]) {
    let reads = 0;
    const provider = {};
    Object.defineProperty(provider, 'scene', {
      enumerable: true,
      get() {
        reads++;
        throw Error('invalid metadata must stop before context copy');
      },
    });
    const f = fixture(metadata, {
      resolveProperty: () => 'unused',
      resolveRequest: () => ({ request: {}, resultId: 7, providerContext: provider }),
    });
    assert.throws(() => f.factory.get(f.element), /usedProperties/);
    assert.equal(reads, 0);
    f.factory.dispose();
  }
});

test('factory rejects function-bearing provider data and does not memoize failed projections', () => {
  let reads = 0;
  const provider = {};
  Object.defineProperty(provider, 'scene', {
    enumerable: true,
    get() {
      reads++;
      return { hiddenOwner() {} };
    },
  });
  const f = fixture(['width'], {
    resolveProperty: () => 'unused',
    resolveRequest: () => ({ request: {}, resultId: 7, providerContext: provider }),
  });
  assert.throws(() => f.factory.get(f.element), /stateless data/);
  assert.throws(() => f.factory.get(f.element), /stateless data/);
  assert.equal(reads, 2);
  f.factory.dispose();
});
