import assert from 'node:assert/strict';
import test from 'node:test';
import { createDefaultSharedCssAdapter } from '../src/runtime/css-shared-adapter.mjs';
import { refreshCssSelectorInput, updateCssSelectorState } from '../src/runtime/css-selector-input.mjs';
import { customElementState } from '../src/runtime/custom-elements.mjs';
import { createCssFactsNavigator } from '../src/runtime/css-facts-navigator.mjs';
import { cssFocusTarget } from '../src/runtime/css-invalidation.mjs';
import { createRealCssFixture, loadRealCssBackend } from './helpers/css-shared-real.mjs';

const blue = 'rgb(0, 0, 255)';
const red = 'rgb(255, 0, 0)';
const green = 'rgb(0, 128, 0)';
const purple = 'rgb(128, 0, 128)';
const bounds = { maxNodes: 128, maxEdges: 512, maxTextUnits: 65536, maxAttributeUnits: 65536 };
const contextual = Object.freeze({
  maxNodes: 128, maxContexts: 4, maxAncestorDepth: 24, maxAnswers: 65536, maxDomainUnits: 65536,
  routing: Object.freeze({ maxRoots: 4, maxNodes: 128, maxPathUnits: 4096, maxScopeDepth: 4 }),
});
// Public DOM properties are intentional in this test-only reader.
const readers = {
  state(node, previous = null) {
    let result = previous ?? Object.create(null);
    for (const name of ['checked', 'selected', 'disabled', 'required', 'readOnly'])
      result = updateCssSelectorState(result, previous, name, node[name]);
    return updateCssSelectorState(result, previous, 'customElementState', customElementState(node));
  },
  text: node => node.data,
  focus(snapshot) {
    const node = cssFocusTarget(snapshot.roots[0].element);
    return node === null ? null : snapshot.nodeId(node);
  },
  unsupported(reason) { throw new TypeError(reason); },
};
async function fixture(useCapturedRoutes = false, overrides = {}) {
  const real = await loadRealCssBackend('native');
  let previous;
  return createRealCssFixture(real,
    '<style>.item{color:blue} .host:has(input:checked) .item{color:red}' +
    '@scope (.scope) { .item { color:green } }</style>' +
    '<main class="host"><input type="checkbox"><b class="item">x</b></main><section class="scope"></section>',
    {
      contextTransform: context => ({ ...context, renderToken: null }),
      captureSelectorFacts(snapshot) {
        const input = refreshCssSelectorInput(snapshot, bounds, previous?.input, readers);
        if (previous?.input === input) return previous.navigator;
        const navigator = createCssFactsNavigator(input, bounds);
        previous = { input, navigator };
        return navigator;
      },
      adapterFactory: options => createDefaultSharedCssAdapter(options, true, useCapturedRoutes),
      policy: { contextualMatching: () => contextual },
      ...overrides,
    });
}
test('opt-in native matching cache invalidates after state, ancestry and CSSOM changes', async () => {
  const { doc, env, dispose } = await fixture();
  try {
    const target = doc.querySelector('b'), control = doc.querySelector('input');
    const read = () => env.window.getComputedStyle(target).color;
    assert.equal(read(), blue);
    assert.equal(read(), blue);
    control.checked = true;
    assert.equal(read(), red);
    assert.equal(read(), red);
    doc.querySelector('section').appendChild(target);
    assert.equal(read(), green);
    assert.equal(read(), green);
    const sheet = doc.querySelector('style').sheet;
    sheet.insertRule('.item{color:purple!important}', sheet.cssRules.length);
    assert.equal(read(), purple);
    assert.equal(read(), purple);
    sheet.deleteRule(sheet.cssRules.length - 1);
    assert.equal(read(), green);
  } finally {
    dispose();
  }
});
test('warm native matching takes the fresh path when matcher dependencies change', async () => {
  const { doc, env, dispose } = await fixture();
  const target = doc.querySelector('b');
  const read = () => env.window.getComputedStyle(target).color;
  const original = Object.getOwnPropertyDescriptor(String.prototype, 'replace');
  const failure = new Error('fresh selector indexing reached');
  try {
    assert.equal(read(), blue);
    assert.equal(read(), blue);
    Object.defineProperty(String.prototype, 'replace', {
      ...original,
      value(pattern, replacement) {
        if (String(this) === 'b' && pattern.source === '[A-Z]') throw failure;
        return Reflect.apply(original.value, this, [pattern, replacement]);
      },
    });
    assert.throws(read, error => error === failure);
  } finally {
    Object.defineProperty(String.prototype, 'replace', original);
    dispose();
  }
});

test('ordinary captured targets follow reassigned slots and preserve pseudo reads', async () => {
  const {doc, env, dispose} = await fixture(true);
  try {
    const target = doc.querySelector('b');
    const read = () => env.window.getComputedStyle(target).color;
    assert.equal(read(), blue);
    const host = doc.querySelector('main'), shadow = host.attachShadow({mode:'open'});
    shadow.innerHTML = '<slot name="hot" style="color:red"></slot><slot name="cold" style="color:blue"></slot>';
    target.removeAttribute('class');
    target.setAttribute('slot','hot');
    assert.equal(read(), red);
    target.setAttribute('slot','cold');
    assert.equal(read(), blue);
    assert.equal(typeof env.window.getComputedStyle(target, '::before').color, 'string');
  } finally { dispose(); }
});

test('compact captured reads retain sibling selector facts and restore the full used-width scene', async () => {
  const batches = [];
  const f = await fixture(true, {
    profile: {compactCascade: true},
    adapterFactory(options) {
      const resolve = options.limits.resolve;
      options.limits.resolve = args => {
        batches.push(args.targets.length);
        return resolve(args);
      };
      return createDefaultSharedCssAdapter(options, true, true);
    },
  });
  try {
    const {doc, env} = f, item = doc.querySelector('b');
    const style = () => env.window.getComputedStyle(item);
    assert.equal(style().color, blue);
    assert.deepEqual(batches, [4]);
    doc.querySelector('input').checked = true;
    assert.equal(style().color, red);
    const baseline = batches.length;
    doc.querySelector('main').setAttribute('style','width:200px');
    item.setAttribute('style','display:block;width:50%');
    assert.equal(style().width, '100px');
    assert.equal(batches.length, baseline + 2);
    assert.ok(batches.at(-1) > batches.at(-2));
  } finally {
    f.dispose();
  }
});

function observedMatchingModels(options, expose, events, refusing) {
  const real = options.selectBackend();
  const backend = { ...real, CompiledStylesV1: class {
    constructor(wire) { this.inner = new real.CompiledStylesV1(wire); }
    metadata() { return this.inner.metadata(); }
    dispose() { this.inner.dispose(); }
    prepare(wire) {
      const inner = this.inner.prepare(wire);
      const model = { entries: new Map() };
      events.push({ kind: 'prepare', model });
      const methods = {
        metadata: () => inner.metadata(),
        dispose() { events.push({ kind: 'dispose', model }); inner.dispose(); },
        resolveBatch(full) { events.push({ kind: 'full', model }); return inner.resolveBatch(full); },
      };
      if (expose.includes('store')) methods.storeMatchingV1 = (matching, limits) => {
        const token = String(model.entries.size + 1);
        assert.ok(JSON.parse(limits).targets > 0);
        events.push({ kind: 'store', model, token });
        if (refusing.value) throw refusing.value;
        model.entries.set(token, JSON.parse(matching));
        return token;
      };
      if (expose.includes('resolve')) methods.resolveMatchingV1 = (rest, token) => {
        assert.ok(model.entries.has(token));
        const partial = JSON.parse(rest);
        assert.ok(!Object.hasOwn(partial, 'contextRoots'));
        events.push({ kind: 'resolve', model, token });
        return inner.resolveBatch(JSON.stringify({ ...partial, ...model.entries.get(token) }));
      };
      return methods;
    }
  } };
  return createDefaultSharedCssAdapter({ ...options, selectBackend: () => backend }, true, true);
}

test('partial retained-matching capabilities keep using the full resolution wire', async () => {
  for (const expose of [[], ['store'], ['resolve']]) {
    const events = [];
    const f = await fixture(true, {
      adapterFactory: options => observedMatchingModels(options, expose, events, {}),
    });
    try {
      const read = () => f.env.window.getComputedStyle(f.doc.querySelector('b')).color;
      assert.equal(read(), blue);
      f.doc.querySelector('input').checked = true;
      assert.equal(read(), red);
      assert.deepEqual(events.map(e => e.kind), ['prepare', 'full', 'full']);
    } finally { f.dispose(); }
  }
});

test('retained matching is installed per exact input and prepared model, with refusal and disposal', async () => {
  const events = [], refusing = { value: null };
  const f = await fixture(true, {
    adapterFactory: options => observedMatchingModels(options, ['store', 'resolve'], events, refusing),
  });
  const target = f.doc.querySelector('b'), read = () => f.env.window.getComputedStyle(target).color;
  try {
    assert.equal(read(), blue);
    assert.equal(read(), blue);
    assert.deepEqual(events.map(e => e.kind), ['prepare','store','resolve','resolve']);
    const first = events[0].model;
    f.doc.querySelector('input').checked = true;
    refusing.value = new Error('matching storage refused');
    assert.throws(read, error => error === refusing.value);
    refusing.value = null;
    assert.equal(read(), red);
    assert.deepEqual(events.slice(4).map(e => e.kind), ['store','store','resolve']);
    assert.equal(events.at(-1).model, first);
    assert.equal(events.at(-1).token, '2');
    const sheet = f.doc.querySelector('style').sheet;
    sheet.insertRule('.item{color:purple!important}', sheet.cssRules.length);
    assert.equal(read(), purple);
    const last = events.at(-1);
    assert.notEqual(last.model, first);
    assert.equal(last.token, '1');
    assert.ok(events.some(e => e.kind === 'dispose' && e.model === first));
  } finally { f.dispose(); }
  assert.equal(events.filter(e => e.kind === 'dispose').length, 2);
});
