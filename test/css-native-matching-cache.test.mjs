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
async function fixture() {
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
      adapterFactory: options => createDefaultSharedCssAdapter(options, true),
      policy: { contextualMatching: () => contextual },
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
