import assert from 'node:assert/strict';
import test from 'node:test';
import { createCssFactsNavigator, readCssFactsNavigator } from '../src/runtime/css-facts-navigator.mjs';
import { refreshCssSelectorInput, updateCssSelectorState } from '../src/runtime/css-selector-input.mjs';

const bounds = { maxNodes: 128, maxEdges: 512, maxTextUnits: 65536, maxAttributeUnits: 65536 };
const html = 'http://www.w3.org/1999/xhtml';
function element(id, parent, tag, extra = {}) {
  return {
    id, parent, kind: 'element', tag, namespace: html, attrs: [], element: { checked: false },
    assignedSlot: null, ...extra,
  };
}
function fixture() {
  const snapshot = {
    documentId: 1,
    roots: [{ id: 1, host: null }, { id: 7, host: 3, parentScope: 1 }],
    records: [
      { id: 1, parent: null, kind: 'document' },
      { id: 2, parent: 1, kind: 'doctype' },
      element(3, 1, 'div'),
      element(4, 3, 'input', {
        assignedSlot: 8, attrs: [{ name: 'type', prefix: '', value: 'checkbox' }],
      }),
      element(5, 3, 'p'),
      { id: 6, parent: 5, kind: 'text', element: 'text' },
      { id: 7, parent: null, kind: 'shadow' },
      element(8, 7, 'slot'),
      { id: 9, parent: 8, kind: 'comment', element: 'comment' },
    ],
  };
  let focus = null, stateReads = 0;
  const read = previous => refreshCssSelectorInput(snapshot, bounds, previous, {
    state(element, prior = null) {
      stateReads++;
      return updateCssSelectorState(prior ?? Object.create(null), prior, 'checked', element.checked);
    },
    text: text => text,
    focus(_snapshot, limits) {
      assert.equal(limits, bounds);
      return focus;
    },
    unsupported(reason) { throw new TypeError(reason); },
  });
  return { snapshot, read, focus(id) { focus = id; }, stateReads: () => stateReads };
}

test('identical selector input avoids DTO construction but rereads state; state and focus copy only changed data', () => {
  const { snapshot, read, focus, stateReads } = fixture();
  const initial = read();
  assert.equal(initial.documentElementId, 3);
  assert.deepEqual(initial.nodes[2].childIds, [4, 5]);
  assert.equal(read(initial), initial);
  assert.equal(stateReads(), 8);
  snapshot.records[3].element.checked = true;
  const changed = read(initial);
  assert.notEqual(changed, initial);
  assert.notEqual(changed.nodes[3], initial.nodes[3]);
  assert.equal(changed.nodes[4], initial.nodes[4]);
  assert.equal(initial.nodes[3].state.checked, false);
  assert.equal(readCssFactsNavigator(createCssFactsNavigator(changed, bounds)).state(4, 'checked'), true);

  focus(4);
  const focused = read(changed);
  assert.equal(focused.nodes, changed.nodes);
  assert.equal(focused.focusTarget, 4);
  assert.equal(read(focused), focused);

  for (const id of [3, 5]) snapshot.records.find(row => row.id === id).element.checked = true;
  const multiple = read(focused);
  assert.equal(multiple.nodes[2].state.checked, true);
  assert.equal(multiple.nodes[4].state.checked, true);
  assert.equal(focused.nodes[2].state.checked, false);
});

test('selector proof rebuilds on every structural or payload difference', () => {
  const changes = [
    snapshot => { snapshot.documentId = 100; },
    snapshot => { snapshot.records.pop(); },
    snapshot => { [snapshot.records[3], snapshot.records[4]] = [snapshot.records[4], snapshot.records[3]]; },
    snapshot => { snapshot.records[4].parent = 1; },
    snapshot => { snapshot.records[5].kind = 'comment'; },
    snapshot => { snapshot.records[3].tag = 'button'; },
    snapshot => { snapshot.records[3].namespace = 'urn:example'; },
    snapshot => { snapshot.records[3].assignedSlot = null; },
    snapshot => { snapshot.records[3].attrs = []; },
    snapshot => { snapshot.records[3].attrs[0].name = 'data-type'; },
    snapshot => { snapshot.records[3].attrs[0].value = 'radio'; },
    snapshot => { snapshot.records[3].attrs[0].prefix = 'x'; },
    snapshot => { snapshot.records[5].element = 'changed'; },
    snapshot => { snapshot.roots[1].host = 5; },
  ];
  for (const change of changes) {
    const { snapshot, read } = fixture();
    const before = read();
    change(snapshot);
    const current = read(before);
    assert.notEqual(current.nodes, before.nodes);
    assert.deepEqual(current, read());
    assert.equal(read(current), current);
  }
});

test('empty scene and invalid rebuilding retain the capture reader contract', () => {
  const { snapshot, read } = fixture();
  snapshot.records.splice(1);
  assert.equal(read().documentElementId, null);
  snapshot.records.push({ id: 2, parent: 1, kind: 'unknown' });
  assert.throws(() => read(), /unknown native selector node kind/);
  snapshot.records[1] = element(2, 100, 'div');
  assert.throws(() => read(), /foreign native selector parent/);
});
