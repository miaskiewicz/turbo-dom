import assert from 'node:assert/strict';
import test from 'node:test';
import { createEnvironment } from './opt-in-helpers/runtime.mjs';
import { createDefaultCssCapture } from '../src/runtime/css-default-capture.mjs';
import { readCssFactsNavigator } from '../src/runtime/css-facts-navigator.mjs';

const snapshotLimits = { maxNodes: 100, maxDepth: 32, maxPathUnits: 1000, maxTextUnits: 1000, maxAttributes: 100, maxAttributeUnits: 1000 };
const factsLimits = { maxNodes: 100, maxEdges: 1000, maxAttributeUnits: 1000, maxTextUnits: 1000 };

test('default capture reads native checked state behind a React-style value tracker', () => {
  const { document } = createEnvironment('<input type="checkbox">');
  const input = document.querySelector('input');
  const descriptor = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(input), 'checked');
  let trackerReads = 0;
  // React DOM wraps the instance IDL descriptor for its value tracker. CSS must
  // read the underlying native state, not invoke or recognize that wrapper.
  Object.defineProperty(input, 'checked', {
    configurable: true,
    enumerable: descriptor.enumerable,
    get() { trackerReads++; return descriptor.get.call(this); },
    set(value) { descriptor.set.call(this, value); },
  });
  input.checked = true;
  const capture = createDefaultCssCapture();
  const snapshot = capture.capture(document, snapshotLimits);
  const navigator = capture.selectorFacts(snapshot, factsLimits);
  assert.equal(readCssFactsNavigator(navigator).state(snapshot.nodeId(input), 'checked'), true);
  assert.equal(trackerReads, 0);
  input.checked = false;
  const changed = capture.capture(document, snapshotLimits);
  assert.equal(readCssFactsNavigator(capture.selectorFacts(changed, factsLimits)).state(changed.nodeId(input), 'checked'), false);
  assert.equal(trackerReads, 0);
});
