import test from "node:test";
import assert from "node:assert/strict";
import { createEnvironment } from "../src/runtime/index.mjs";
import { unscopedCssFactRoots, compileCssPrimitiveMatchFacts } from "../src/runtime/css-match-facts.mjs";
import {
	createCssFactsNavigator,
	CssFactsUnavailable,
} from "../src/runtime/css-facts-navigator.mjs";
import { captureCssPrimitivePublicFacts } from "./helpers/css-primitive-public-facts.mjs";
const units = {
	maxNodes: 128,
	maxEdges: 1024,
	maxAttributeUnits: 16384,
	maxTextUnits: 16384,
};
function fixture() {
	const { document } = createEnvironment(
		'<div id="host" class="on"><b id="light" class="on"></b></div>',
	);
	const host = document.querySelector("#host"),
		root = host.attachShadow({ mode: "closed" });
	root.innerHTML =
		'<slot></slot><section><b id="shadow" class="on"></b></section>';
	const facts = captureCssPrimitivePublicFacts(document, {
			closedRoots: [root],
			focusTarget: null,
		}),
		navigator = createCssFactsNavigator(facts, units);
	const elements = facts.nodes.filter((row) => row.nodeType === 1),
		ids = new Set(elements.map((row) => row.id));
	const nodes = elements.map((row) => ({
		id: row.id,
		parent: ids.has(row.parentId) ? row.parentId : null,
	}));
	const id = (name) =>
		elements.find((row) =>
			row.attributes.some((attr) => attr.name === "id" && attr.value === name),
		).id;
	const shadow = facts.nodes.find((row) => row.nodeType === 11).id,
		slot = elements.find((row) => row.localName === "slot").id;
	return { facts, navigator, nodes, id, shadow, slot };
}
const item = (id, source) => ({
	id,
	units: source,
	shadow: true,
	allowScopeReference: true,
});
test("primitive collector shares canonical domains and shadow-context matching without DOM wrappers", () => {
	const f = fixture(),
		collector = compileCssPrimitiveMatchFacts({
			selectors: [
				item(0, ":host(.on)"),
				item(1, ":host > section > b"),
				item(2, "slot::slotted(.on)"),
			],
			scopeGroups: [{ id: 9, start: [item(3, ":host")] }],
		});
	const request = {
		navigator: f.navigator,
		ownerDocument: f.facts.documentId,
		sheetRootIds: f.nodes
			.filter((row) => row.parent === null)
			.map((row) => row.id),
		scopeNodes: [{ id: f.facts.documentId }, { id: f.shadow }],
		targetIds: [f.id("host"), f.id("shadow"), f.id("light")],
		selectorQueries: [0, 1, 2].map((selectorId) => ({
			selectorId,
			scopeRootIds: [null],
		})),
		scopeQueries: [
			{
				groupId: 9,
				boundary: "start",
				enclosingRootIds: [f.shadow],
				candidateIds: [f.id("host")],
			},
		],
		cssContext: {
			shadowRoot: f.shadow,
			slots: new Map([[f.id("light"), { slot: f.slot, treeScope: f.shadow }]]),
		},
	};
	const result = collector.collect(f.nodes, request, {
		maxNodes: 128,
		maxAnswers: 1000,
		maxAncestorDepth: 100,
	});
	assert.deepEqual(
		result.selectorAnswers.map((row) => row.matches),
		[[f.id("host")], [f.id("shadow")], [f.id("light")]],
	);
	assert.deepEqual(result.scopeAnswers[0].matches, [f.id("host")]);
	assert.ok(f.nodes.every((row) => !Object.hasOwn(row, "element")));
	const original = new Error("proxy must not be reflected");
	const proxy = new Proxy(
		{},
		{
			get() {
				throw original;
			},
		},
	);
	assert.throws(
		() =>
			collector.collect(
				f.nodes,
				{ ...request, navigator: proxy },
				{ maxNodes: 128, maxAnswers: 1000, maxAncestorDepth: 100 },
			),
		CssFactsUnavailable,
	);
});

test('private unscoped rows match the general collector at exact and exhausted answer quotas', () => {
  const f = fixture();
  const metadata = {selectors: [item(0, 'b'), item(1, '.absent'), item(2, '#light'),
    item(3, ':scope > b')], scopeGroups: []};
  const collector = compileCssPrimitiveMatchFacts(metadata);
  const request = {
    navigator: f.navigator, ownerDocument: f.facts.documentId,
    sheetRootIds: f.nodes.filter(row => row.parent === null).map(row => row.id),
    scopeNodes: [{id: f.facts.documentId}, {id: f.shadow}],
    targetIds: f.nodes.map(row => row.id), scopeQueries: [], collectUsage: true,
  };
  function run(roots, maxAnswers, duplicate = false) {
    const selectorQueries = [0, 1, 2].map(selectorId => ({selectorId, scopeRootIds: roots}));
    selectorQueries.push({selectorId: 3, scopeRootIds: [f.id('host')]});
    if (duplicate) selectorQueries.push(selectorQueries[0]);
    return collector.collect(f.nodes, {...request, selectorQueries}, {maxNodes:128, maxAncestorDepth:100, maxAnswers});
  }
  const general = run([null], 1000), count = general.used.answers;
  const fast = run(unscopedCssFactRoots, count);
  assert.deepEqual(fast, general);
  assert.ok(fast.selectorAnswers[0].matches.length > 1);
  assert.equal(fast.selectorAnswers[1].matches.length, 0);
  for (const roots of [[null], unscopedCssFactRoots]) {
    assert.throws(() => run(roots, count - 1), e => e.limit === 'answers');
    assert.throws(() => run(roots, 1000, true), /duplicate CSS matching answer/);
  }
});
