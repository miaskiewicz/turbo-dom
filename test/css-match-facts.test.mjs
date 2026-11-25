import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';
import { compileCssMatchFacts, CssMatchFactsLimitError, CssMatchFactsCapabilityError } from '../src/runtime/css-match-facts.mjs';

const item = (id, units, seed = null, extra = {}) => ({ id, units, seed, allowScopeReference: true, ...extra });
const metadata = (selectors, scopeGroups = []) => ({ selectors, scopeGroups });
function scene(document, extraRoots = []) {
  const elements = [];
  function visit(element) { elements.push(element); for (const child of element.children) visit(child); }
  const roots = [document.documentElement, ...extraRoots];
  for (const root of roots) visit(root);
  const ids = new Map(elements.map((element, id) => [element, id]));
  return { document, ids, nodes: elements.map((element, id) => ({ id, parent: ids.get(element.parentNode) ?? null, element })), roots: roots.map((element) => ids.get(element)) };
}
function request(view, targets, selectorQueries = [], scopeQueries = []) {
  return { ownerDocument: view.document, sheetRootIds: view.roots, targetIds: targets.map((element) => view.ids.get(element)), selectorQueries, scopeQueries };
}
const query = (selectorId, scopeRootIds = [null]) => ({ selectorId, scopeRootIds });
const limits = (view, maxAnswers = 1000, maxAncestorDepth = 100) => ({ maxNodes: view.nodes.length, maxAnswers, maxAncestorDepth });
function positives(result, selectorId, scopeRootId = null) { return result.selectorAnswers.find((row) => row.selectorId === selectorId && row.scopeRootId === scopeRootId).matches; }
const resource = (limit) => (error) => error instanceof CssMatchFactsLimitError && error.limit === limit;

test('seed buckets retain duplicate IDs and full live matching remains authoritative', () => {
  const { document } = createEnvironment('<button id=same class="target one"></button><button id=same class=target disabled></button>');
  const buttons = Array.from(document.querySelectorAll('button'));
  const view = scene(document);
  const collector = compileCssMatchFacts(metadata([
    item(0, '#same:not([disabled])', { kind: 'id', value: 'same' }),
    item(1, '.target', { kind: 'class', value: 'target' }),
    item(2, 'BUTTON', { kind: 'type', value: 'BUTTON' }),
    item(3, ':is(.one,.missing)'),
  ]));
  const input = request(view, buttons, [query(0), query(1), query(2), query(3)]);
  let facts = collector.collect(view.nodes, input, limits(view));
  const ids = buttons.map((element) => view.ids.get(element));
  assert.deepEqual(positives(facts, 0), [ids[0]]);
  assert.deepEqual(positives(facts, 1), ids);
  assert.deepEqual(positives(facts, 2), ids);
  assert.deepEqual(positives(facts, 3), [ids[0]]);
  assert.equal(facts.domains.length, 1);
  assert.deepEqual(facts.domains[0].nodeIds, ids);
  buttons[0].setAttribute('disabled', ''); buttons[1].removeAttribute('disabled');
  facts = collector.collect(view.nodes, input, limits(view));
  assert.deepEqual(positives(facts, 0), [ids[1]]);
  input.targetIds[0] = 999;
  assert.deepEqual(facts.domains[0].nodeIds, ids); // Returned facts own their scalar domain snapshot.
});

test('scoped selectors and start/end predicates emit every requested root context without choosing one', () => {
  const { document } = createEnvironment('<main class=outside id=outer><section class=root id=root><div class=limit id=limit><button class=target id=target>T</button></div></section></main>');
  const view = scene(document);
  const outer = document.getElementById('outer'), root = document.getElementById('root'), target = document.getElementById('target'), end = document.getElementById('limit');
  const rootIds = [view.ids.get(outer), view.ids.get(root)];
  const candidates = [outer, root, end, target].map((element) => view.ids.get(element));
  const collector = compileCssMatchFacts(metadata([
    item(0, '.target', { kind: 'class', value: 'target' }, { implicitScopeAnchor: true }),
    item(1, '.outside :scope .target'),
    item(2, ':scope'),
  ], [{ id: 7, start: [item(10, ':scope > .root'), item(11, '.never')], end: [item(12, ':scope > .limit')] }]));
  const input = request(view, [outer, root, target], [query(0, rootIds), query(1, rootIds), query(2, rootIds)], [
    { groupId: 7, boundary: 'start', enclosingRootIds: [rootIds[0], null], candidateIds: candidates },
    { groupId: 7, boundary: 'end', enclosingRootIds: rootIds, candidateIds: candidates },
  ]);
  const facts = collector.collect(view.nodes, input, limits(view));
  assert.equal(facts.selectorAnswers.length, 6);
  assert.deepEqual(positives(facts, 0, rootIds[0]), [view.ids.get(target)]);
  assert.deepEqual(positives(facts, 0, rootIds[1]), [view.ids.get(target)]);
  assert.deepEqual(positives(facts, 1, rootIds[0]), []);
  assert.deepEqual(positives(facts, 1, rootIds[1]), [view.ids.get(target)]);
  assert.deepEqual(positives(facts, 2, rootIds[0]), [view.ids.get(outer)]);
  assert.deepEqual(positives(facts, 2, rootIds[1]), [view.ids.get(root)]);
  assert.deepEqual(facts.scopeAnswers.map((row) => row.matches), [[view.ids.get(root)], [], [], [view.ids.get(end)]]);
  assert.equal(facts.domains.length, 2);
  assert.equal(facts.scopeAnswers[0].domainId, facts.scopeAnswers[3].domainId);
});

test('ordinary pseudo and focus programs use caller-supplied canonical facts and propagate original errors', () => {
  const { document } = createEnvironment('<button class=target>T</button>');
  const target = document.querySelector('button'), view = scene(document);
  const collector = compileCssMatchFacts(metadata([
    item(0, '.target::before', { kind: 'class', value: 'target' }, { pseudo: 'before' }),
    item(1, '.target:focus', { kind: 'class', value: 'target' }),
  ]));
  const input = request(view, [target], [query(0), query(1)]);
  input.liveState = { focusTarget: (doc) => doc.__active };
  assert.deepEqual(positives(collector.collect(view.nodes, input, limits(view)), 1), []);
  target.focus();
  const facts = collector.collect(view.nodes, input, limits(view));
  assert.deepEqual(positives(facts, 0), [view.ids.get(target)]);
  assert.deepEqual(positives(facts, 1), [view.ids.get(target)]);
  const error = new Error('original focus provider error');
  input.liveState.focusTarget = () => { throw error; };
  assert.throws(() => collector.collect(view.nodes, input, limits(view)), (actual) => actual === error);
  input.liveState.focusTarget = () => undefined;
  assert.throws(() => collector.collect(view.nodes, input, limits(view)), /focus state is unavailable/);
});

test('compiled CSS programs retain source snapshots while scene facts remain transient', () => {
  const { document } = createEnvironment('<div class=target></div>');
  const target = document.querySelector('div'), view = scene(document);
  const source = [46,116,97,114,103,101,116], proof = { kind: 'class', value: 'target' };
  let reads = 0;
  const record = { id: 0, get units() { reads++; return source; }, seed: proof };
  const collector = compileCssMatchFacts(metadata([record]));
  const compiledReads = reads;
  source[1] = 111; proof.value = 'changed';
  const input = request(view, [target], [query(0)]);
  assert.deepEqual(positives(collector.collect(view.nodes, input, limits(view)), 0), [view.ids.get(target)]);
  target.className = 'changed';
  assert.deepEqual(positives(collector.collect(view.nodes, input, limits(view)), 0), []);
  assert.equal(reads, compiledReads);
});

test('detached forests and explicit boundaries do not fabricate root relationships', () => {
  const { document } = createEnvironment('<div class=root><button class=target></button></div>');
  const detached = document.createElement('section'); detached.className = 'root';
  detached.innerHTML = '<button class=target></button>';
  const view = scene(document, [detached]);
  const roots = Array.from(document.querySelectorAll('.root')).concat(detached);
  const targets = [document.querySelector('button'), detached.querySelector('button')];
  const collector = compileCssMatchFacts(metadata([item(0, '& .target', { kind: 'class', value: 'target' })]));
  const facts = collector.collect(view.nodes, request(view, targets, [query(0, roots.map((element) => view.ids.get(element)))]), limits(view));
  assert.deepEqual(facts.selectorAnswers.map((row) => row.matches), targets.map((element) => [view.ids.get(element)]));
  const cut = [{ id: 0, parent: null, element: targets[0] }];
  const input = { ownerDocument: document, sheetRootIds: [], targetIds: [0], selectorQueries: [], scopeQueries: [] };
  assert.throws(() => collector.collect(cut, input, { maxNodes: 1, maxAnswers: 0, maxAncestorDepth: 0 }), /ancestor boundary/);
  input.sheetRootIds = [0];
  assert.deepEqual(collector.collect(cut, input, { maxNodes: 1, maxAnswers: 0, maxAncestorDepth: 0 }), { domains: [], selectorAnswers: [], scopeAnswers: [] });
});

test('seed proofs avoid the full scene cross-product under the same explicit answer budget', () => {
  const { document } = createEnvironment('<main>' + Array.from({ length: 80 }, (_, index) => `<i class=c${index}></i>`).join('') + '</main>');
  const view = scene(document), targets = Array.from(document.querySelectorAll('i'));
  const selectors = Array.from({ length: 100 }, (_, index) => item(index, `.c${index}`, { kind: 'class', value: `c${index}` }));
  const input = request(view, targets, selectors.map((record) => query(record.id)));
  const quota = limits(view, 180);
  const facts = compileCssMatchFacts(metadata(selectors)).collect(view.nodes, input, quota);
  assert.equal(facts.selectorAnswers.length, 100);
  assert.equal(facts.domains.length, 1);
  for (let index = 0; index < 100; index++) assert.deepEqual(positives(facts, index), index < targets.length ? [view.ids.get(targets[index])] : []);
  const unseeded = selectors.map((record) => ({ ...record, seed: null }));
  assert.throws(() => compileCssMatchFacts(metadata(unseeded)).collect(view.nodes, input, quota), resource('answers'));
});

test('unknown and foreign owners are errors without invoking descriptor accessors', () => {
  const { document } = createEnvironment('<button></button>'), foreign = createEnvironment('<button></button>').document;
  const target = document.querySelector('button'), view = scene(document);
  const collector = compileCssMatchFacts(metadata([item(0, 'button')]));
  const input = request(view, [target], [query(0)]);
  assert.throws(() => collector.collect(view.nodes, { ...input, ownerDocument: undefined }, limits(view)), CssMatchFactsCapabilityError);
  assert.throws(() => collector.collect(view.nodes, { ...input, ownerDocument: foreign }, limits(view)), /foreign CSS element owner/);
  const descriptor = Object.getOwnPropertyDescriptor(target, 'ownerDocument');
  let reads = 0;
  Object.defineProperty(target, 'ownerDocument', { configurable: true, get() { reads++; throw new Error('must not read owner getter'); } });
  assert.throws(() => collector.collect(view.nodes, input, limits(view)), CssMatchFactsCapabilityError);
  assert.equal(reads, 0);
  Object.defineProperty(target, 'ownerDocument', descriptor);
  const error = new Error('original reflection error');
  const proxy = new Proxy(target, { getOwnPropertyDescriptor() { throw error; } });
  const nodes = view.nodes.map((node) => node.element === target ? { ...node, element: proxy } : node);
  assert.throws(() => collector.collect(nodes, input, limits(view)), (actual) => actual === error);
  target.getAttribute = () => { throw error; };
  assert.throws(() => collector.collect(view.nodes, input, limits(view)), (actual) => actual === error);
});

test('malformed metadata and requests never turn unknown capability into empty matches', () => {
  assert.throws(() => compileCssMatchFacts(null), CssMatchFactsCapabilityError);
  assert.throws(() => compileCssMatchFacts(metadata([null])), CssMatchFactsCapabilityError);
  assert.throws(() => compileCssMatchFacts(metadata([item(0, null)])), CssMatchFactsCapabilityError);
  assert.throws(() => compileCssMatchFacts(metadata([item(0, 'div'), item(0, 'span')])), /duplicate.*program/);
  assert.throws(() => compileCssMatchFacts(metadata([], [{ id: 0, start: null }])), CssMatchFactsCapabilityError);
  assert.throws(() => compileCssMatchFacts(metadata([], [{ id: 0 }, { id: 0 }])), /duplicate.*group/);
  for (const proof of [{ kind: 'wrong', value: 'x' }, { kind: 'id', value: '' }, { kind: 'type', value: '*' }, { kind: 'type', value: 'élement' }]) assert.throws(() => compileCssMatchFacts(metadata([item(0, 'div', proof)])), TypeError);
  const { document } = createEnvironment('<div></div>'), view = scene(document), target = document.querySelector('div');
  const collector = compileCssMatchFacts(metadata([item(0, 'div')], [{ id: 1 }]));
  const input = request(view, [target], [query(0)]);
  const run = (request) => collector.collect(view.nodes, request, limits(view));
  assert.throws(() => run({ ...input, targetIds: [999] }), /unknown.*node/);
  assert.throws(() => run({ ...input, selectorQueries: [query(999)] }), /unknown.*program/);
  assert.throws(() => run({ ...input, selectorQueries: [query(0, [999])] }), /unknown.*node/);
  assert.throws(() => run({ ...input, selectorQueries: [query(0, [null, null])] }), /duplicate.*root/);
  assert.throws(() => run({ ...input, selectorQueries: [query(0), query(0)] }), /duplicate.*answer/);
  for (const [groupId, boundary, pattern] of [[999, 'start', /unknown.*group/], [1, 'wrong', /invalid.*boundary/], [1, 'start', /boundary is absent/]]) {
    assert.throws(() => run({ ...input, selectorQueries: [], scopeQueries: [{ groupId, boundary, enclosingRootIds: [null], candidateIds: input.targetIds }] }), pattern);
  }
});

test('node, answer and ancestor caps remain caller-supplied and exact boundaries work', () => {
  const { document } = createEnvironment('<div class=target></div>'), view = scene(document), target = document.querySelector('div');
  const collector = compileCssMatchFacts(metadata([item(0, '.target', { kind: 'class', value: 'target' })]));
  const input = request(view, [target], [query(0)]);
  assert.throws(() => collector.collect(view.nodes, input, undefined), TypeError);
  assert.throws(() => collector.collect(view.nodes, input, limits(view, -1)), TypeError);
  assert.throws(() => collector.collect(view.nodes, input, { ...limits(view), maxNodes: view.nodes.length - 1 }), resource('nodes'));
  assert.throws(() => collector.collect(view.nodes, input, limits(view, 1)), resource('answers'));
  assert.deepEqual(positives(collector.collect(view.nodes, input, limits(view, 2, 2)), 0), [view.ids.get(target)]);
  assert.throws(() => collector.collect(view.nodes, input, limits(view, 2, 1)), resource('ancestorDepth'));
  const blank = compileCssMatchFacts(metadata([]));
  assert.deepEqual(blank.collect([], { ownerDocument: document, sheetRootIds: [], targetIds: [], selectorQueries: [], scopeQueries: [] }, { maxNodes: 0, maxAnswers: 0, maxAncestorDepth: 0 }), { domains: [], selectorAnswers: [], scopeAnswers: [] });
});

test('scene protocol errors and unknown data never become false matching answers', () => {
  const { document } = createEnvironment('<div></div><div></div>'), view = scene(document);
  const targets = Array.from(document.querySelectorAll('div'));
  const collector = compileCssMatchFacts(metadata([item(0, 'div')]));
  const input = request(view, targets, [query(0)]);
  const run = (nodes = view.nodes, requestValue = input) => collector.collect(nodes, requestValue, limits(view));
  assert.throws(() => run([{ id: 0, parent: null, element: null }]), CssMatchFactsCapabilityError);
  assert.throws(() => run([{ id: 0, parent: null, element: {} }]), CssMatchFactsCapabilityError);
  const text = document.createTextNode('text');
  assert.throws(() => run([{ id: 0, parent: null, element: text }]), /not an element/);
  assert.throws(() => run([view.nodes[0], { ...view.nodes[1], id: view.nodes[0].id }]), /duplicate.*node id/);
  assert.throws(() => run([view.nodes[0], { ...view.nodes[0], id: 99 }]), /duplicate.*element/);
  assert.throws(() => run(view.nodes, { ...input, targetIds: [input.targetIds[0], input.targetIds[0]] }), /duplicate/);
  assert.throws(() => run(view.nodes, { ...input, targetIds: Array(view.nodes.length + 1).fill(input.targetIds[0]) }), resource('nodes'));
  const changed = view.nodes.map((row) => row.element === targets[0] ? { ...row, parent: 0 } : row);
  assert.throws(() => run(changed), /parent mismatch/);
  const parentDescriptor = Object.getOwnPropertyDescriptor(targets[0], 'parentNode');
  let reads = 0;
  Object.defineProperty(targets[0], 'parentNode', { configurable: true, get() { reads++; throw new Error('unknown parent getter'); } });
  assert.throws(() => run(), CssMatchFactsCapabilityError);
  assert.equal(reads, 0);
  Object.defineProperty(targets[0], 'parentNode', parentDescriptor);
  const original = targets[0].getAttribute;
  targets[0].getAttribute = () => undefined;
  assert.throws(() => run(), /unknown attribute/);
  targets[0].getAttribute = original;
  targets[0].localName = undefined;
  assert.throws(() => run(), /unknown local name/);
});

test('type uncertainty and ASCII class tokenization retain conservative completeness', () => {
  const { document } = createEnvironment('<div class="target target"></div><div class="a&#160;b"></div>');
  const uncertain = document.createElementNS('http://www.w3.org/2000/svg', 'K'); document.body.appendChild(uncertain);
  const view = scene(document), targets = Array.from(document.querySelectorAll('div'));
  const collector = compileCssMatchFacts(metadata([
    item(0, '.target', { kind: 'class', value: 'target' }),
    item(1, '.a\u00a0b', { kind: 'class', value: 'a\u00a0b' }),
    item(2, 'x', { kind: 'type', value: 'x' }),
  ]));
  const input = request(view, [targets[0]], [query(0)]);
  assert.deepEqual(positives(collector.collect(view.nodes, input, limits(view)), 0), [view.ids.get(targets[0])]);
  const named = request(view, [targets[1]], [query(1)]);
  assert.deepEqual(positives(collector.collect(view.nodes, named, limits(view)), 1), [view.ids.get(targets[1])]);
  targets[0].className = ' \t\r\n ';
  assert.deepEqual(positives(collector.collect(view.nodes, input, limits(view)), 0), []);
  const unproven = request(view, [uncertain], [query(2)]);
  assert.deepEqual(positives(collector.collect(view.nodes, unproven, limits(view, 2)), 2), []);
  // The uncertain type gets a real predicate check, not a guessed missing bucket.
  assert.throws(() => collector.collect(view.nodes, unproven, limits(view, 1)), resource('answers'));
});

test('ancestor walks work independently of record order and reject cyclic facts', () => {
  const { document } = createEnvironment('<main><section><button></button></section></main>'), view = scene(document);
  const target = document.querySelector('button');
  const collector = compileCssMatchFacts(metadata([item(0, 'button')]));
  const input = request(view, [target], [query(0)]);
  const reversed = view.nodes.slice().reverse();
  assert.deepEqual(positives(collector.collect(reversed, input, limits(view)), 0), [view.ids.get(target)]);
  assert.throws(() => collector.collect(reversed, input, limits(view, 100, 0)), resource('ancestorDepth'));
  const left = document.createElement('i'), right = document.createElement('i');
  left.parentNode = right; right.parentNode = left;
  const cycle = [{ id: 0, parent: 1, element: left }, { id: 1, parent: 0, element: right }];
  assert.throws(() => collector.collect(cycle, { ownerDocument: document, sheetRootIds: [], targetIds: [0], selectorQueries: [query(0)], scopeQueries: [] }, { maxNodes: 2, maxAnswers: 100, maxAncestorDepth: 10 }), /cyclic/);
});

test('scope domains and request cardinalities are bounded before sparse result allocation', () => {
  const { document } = createEnvironment('<main><button></button></main>'), view = scene(document);
  const candidateIds = view.nodes.slice(0, 4).map((node) => node.id);
  const collector = compileCssMatchFacts(metadata([item(0, 'button')], [{ id: 1, start: [] }, { id: 2, start: [] }, { id: 3, start: [] }]));
  const input = request(view, [], [], [1, 2, 3].map((groupId) => ({ groupId, boundary: 'start', enclosingRootIds: [null], candidateIds: candidateIds.slice() })));
  assert.throws(() => collector.collect(view.nodes, input, limits(view, 3)), resource('answers'));
  const malformed = { ...input, scopeQueries: [{ groupId: 1, boundary: 'start', enclosingRootIds: [null], candidateIds: undefined }] };
  assert.throws(() => collector.collect(view.nodes, malformed, limits(view)), TypeError);
  const tooMany = { ...input, scopeQueries: [{ groupId: 1, boundary: 'start', enclosingRootIds: [null], candidateIds: Array(view.nodes.length + 1).fill(0) }] };
  assert.throws(() => collector.collect(view.nodes, tooMany, limits(view)), resource('nodes'));
  const roots = request(view, [], [query(0, Array(view.nodes.length + 2).fill(null))]);
  assert.throws(() => collector.collect(view.nodes, roots, limits(view)), resource('nodes'));
  const empty = request(view, [], [query(0)]);
  assert.deepEqual(positives(collector.collect(view.nodes, empty, limits(view)), 0), []);
  assert.throws(() => compileCssMatchFacts({ selectors: null, scopeGroups: [] }), TypeError);
  assert.throws(() => compileCssMatchFacts(metadata([item(-1, 'div')])), TypeError);
  assert.throws(() => compileCssMatchFacts(metadata([item(0, 'div', { kind: 'id', value: 1 })])), TypeError);
  assert.throws(() => compileCssMatchFacts(metadata([], [{ id: 1, start: {} }])), TypeError);
  assert.throws(() => collector.collect(view.nodes, { ...empty, selectorQueries: null }, limits(view)), TypeError);
  assert.deepEqual(compileCssMatchFacts(metadata([{ id: 0, units: 'button' }])).collect(view.nodes, request(view, [], [query(0)]), limits(view)).selectorAnswers[0].matches, []);
});
