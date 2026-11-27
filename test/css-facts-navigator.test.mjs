import test from "node:test";
import assert from "node:assert/strict";
import { createEnvironment } from "../src/runtime/index.mjs";
import {
	compileCssSelectorProgram,
	matchesCssFacts,
	matchesSelector,
} from "../src/runtime/selectors.mjs";
import {
	createCssFactsNavigator,
	CssFactsUnavailable,
} from "../src/runtime/css-facts-navigator.mjs";
import { captureCssPrimitivePublicFacts } from "./helpers/css-primitive-public-facts.mjs";
const limits = {
	maxNodes: 128,
	maxEdges: 1024,
	maxAttributeUnits: 16384,
	maxTextUnits: 16384,
};
const program = (source) =>
	compileCssSelectorProgram(source, {
		shadow: true,
		allowScopeReference: true,
	});
function fixture() {
	const { document } = createEnvironment(
		'<main id="root"><input id="check" type="checkbox"><b id="target" class="on">text</b><b class="last"></b></main>',
	);
	const facts = captureCssPrimitivePublicFacts(document, { focusTarget: null });
	const id = (name) =>
		facts.nodes.find((row) =>
			row.attributes?.some((attr) => attr.name === "id" && attr.value === name),
		).id;
	return { document, facts, id, nav: createCssFactsNavigator(facts, limits) };
}
test("same evaluator matches numeric native primitive IDs on released DOM", () => {
	const f = fixture(),
		target = f.id("target");
	for (const selector of [
		"#target",
		".on",
		"main > b:nth-child(2)",
		"b:has(+ .last)",
		":lang(en)",
		":root",
	]) {
		const element = f.document.querySelector("#target");
		assert.equal(
			matchesCssFacts(target, program(selector), {}, f.nav),
			matchesSelector(element, selector),
			selector,
		);
	}
	assert.equal(
		matchesCssFacts(f.facts.documentElementId, program(":root"), {}, f.nav),
		true,
	);
	assert.equal(f.facts.documentId, 0);
});
test("dirty primitive IDL state stays separate from attribute fallback", () => {
	const f = fixture(),
		input = f.document.querySelector("#check");
	input.checked = true;
	const next = captureCssPrimitivePublicFacts(f.document, {
		focusTarget: null,
	});
	const nav = createCssFactsNavigator(next, limits);
	assert.equal(
		matchesCssFacts(f.id("check"), program(":checked"), {}, nav),
		true,
	);
	assert.equal(
		matchesCssFacts(f.id("check"), program(":checked"), {}, f.nav),
		false,
	);
	delete next.nodes.find((row) => row.id === f.id("check")).state.checked;
	const missing = createCssFactsNavigator(next, limits);
	assert.throws(
		() => matchesCssFacts(f.id("check"), program(":checked"), {}, missing),
		CssFactsUnavailable,
	);
	assert.equal(
		matchesCssFacts(f.id("target"), program(".on"), {}, missing),
		true,
	);
});
test("owned facts cannot call original DTO getters after admission; missing capabilities are demand-driven", () => {
	const f = fixture(),
		target = f.id("target");
	for (const row of f.facts.nodes)
		Object.defineProperty(row, "parentId", {
			get() {
				throw Error("input read after admission");
			},
		});
	assert.equal(matchesCssFacts(target, program(".on"), {}, f.nav), true);
	assert.throws(
		() => matchesCssFacts(target, program(":hover"), {}, f.nav),
		CssFactsUnavailable,
	);
	assert.equal(
		matchesSelector(f.document.querySelector("#target"), ".on"),
		true,
	);
	assert.equal(matchesCssFacts(target, program(".on"), {}, f.nav), true);
});
test("zero element identity is not confused with null", () => {
	const source = {
		documentId: 7,
		documentElementId: 0,
		focusTarget: 0,
		nodes: [
			{ id: 7, nodeType: 9, parentId: null, ownerDocumentId: 7, childIds: [0] },
			{
				id: 0,
				nodeType: 1,
				parentId: 7,
				ownerDocumentId: 7,
				childIds: [],
				localName: "svg",
				namespaceURI: "http://www.w3.org/2000/svg",
				attributes: [],
				assignedSlotId: null,
				state: { customElementState: "uncustomized" },
			},
		],
	};
	const nav = createCssFactsNavigator(source, limits);
	assert.equal(matchesCssFacts(0, program(":root"), {}, nav), true);
	assert.equal(
		matchesCssFacts(0, program(":scope"), { scopeRoot: 0 }, nav),
		true,
	);
	assert.equal(matchesCssFacts(0, program(":focus"), {}, nav), true);
	assert.equal(matchesCssFacts(null, program("*"), {}, nav), false);
});
test("custom element states reuse canonical enum interpretation and unknown refuses on access", () => {
	const f = fixture(),
		target = f.id("target");
	for (const [state, expected] of [
		["uncustomized", true],
		["custom", true],
		["undefined", false],
		["failed", false],
	]) {
		const source = captureCssPrimitivePublicFacts(f.document, {
			focusTarget: null,
		});
		source.nodes.find((row) => row.id === target).state.customElementState =
			state;
		const nav = createCssFactsNavigator(source, limits);
		assert.equal(
			matchesCssFacts(target, program(":defined"), {}, nav),
			expected,
			state,
		);
	}
	const source = captureCssPrimitivePublicFacts(f.document, {
		focusTarget: null,
	});
	source.nodes.find((row) => row.id === target).state.customElementState =
		"future";
	const nav = createCssFactsNavigator(source, limits);
	assert.throws(
		() => matchesCssFacts(target, program(":defined"), {}, nav),
		CssFactsUnavailable,
	);
	assert.equal(matchesCssFacts(target, program(".on"), {}, nav), true);
});
test("owned closed shadow facts preserve featureless host, physical ancestry and real slot identity", () => {
	const { document } = createEnvironment(
		'<main><div id="host" class="on"><b id="light" class="on"></b></div></main>',
	);
	const host = document.querySelector("#host"),
		root = host.attachShadow({ mode: "closed" });
	root.innerHTML =
		'<slot></slot><section><b id="shadow" class="on"></b></section>';
	const facts = captureCssPrimitivePublicFacts(document, {
		closedRoots: [root],
		focusTarget: null,
	});
	const id = (name) =>
		facts.nodes.find((row) =>
			row.attributes?.some((attr) => attr.name === "id" && attr.value === name),
		).id;
	const rootId = facts.nodes.find((row) => row.nodeType === 11).id,
		slotId = facts.nodes.find((row) => row.localName === "slot").id;
	const nav = createCssFactsNavigator(facts, limits),
		context = { shadowRoot: rootId };
	for (const [selector, target, expected] of [
		[":host", id("host"), true],
		[":host(.on)", id("host"), true],
		["#host", id("host"), false],
		[":host > section > b", id("shadow"), true],
		["main b", id("shadow"), false],
		[":is(:host)", id("host"), true],
		[":not(.missing)", id("host"), false],
	])
		assert.equal(
			matchesCssFacts(target, program(selector), context, nav),
			expected,
			selector,
		);
	assert.equal(
		matchesCssFacts(
			id("light"),
			program("slot::slotted(.on)"),
			{ ...context, assignedSlot: slotId, assignedSlotRoot: rootId },
			nav,
		),
		true,
	);
	assert.equal(
		matchesCssFacts(
			id("light"),
			program("slot::slotted(.on)"),
			{ ...context, assignedSlot: slotId, assignedSlotRoot: facts.documentId },
			nav,
		),
		false,
	);
	assert.equal(
		matchesCssFacts(
			id("host"),
			program(":host"),
			{ ...context, scopeRoot: id("host") },
			nav,
		),
		false,
	);
	assert.equal(
		matchesCssFacts(
			id("host"),
			program(":scope"),
			{ ...context, scopeRoot: id("host") },
			nav,
		),
		true,
	);
	assert.equal(host.matches(":host"), false);
});
test('known undefined non-form IDL falls back exactly like DOM; missing fact refuses', () => {
  const f = fixture(), target = f.id('target');
  for (const selector of [':required', ':optional']) assert.equal(matchesCssFacts(target, program(selector), {}, f.nav), matchesSelector(f.document.querySelector('#target'), selector));
  Object.defineProperty(f.document.querySelector('#target'), 'required', {value:undefined});
  const source = captureCssPrimitivePublicFacts(f.document, {focusTarget:null});
  const row = source.nodes.find(row => row.id === target);
  assert.ok(Object.hasOwn(row.state, 'required'));
  assert.equal(row.state.required, undefined);
  const known = createCssFactsNavigator(source, limits);
  assert.equal(matchesCssFacts(target, program(':required'), {}, known), false);
  delete row.state.required;
  const missing = createCssFactsNavigator(source, limits);
  assert.throws(() => matchesCssFacts(target, program(':required'), {}, missing), CssFactsUnavailable);
  assert.equal(matchesCssFacts(target, program(':optional'), {}, missing), false);
});
test('canonical generated slotted pseudo projection keeps exact chain and actual slot', () => {
 const{document}=createEnvironment('<div id="host"><b class="on"></b></div>');const host=document.querySelector('#host'),root=host.attachShadow({mode:'open'});root.innerHTML='<section><slot></slot></section>';
 const facts=captureCssPrimitivePublicFacts(document,{focusTarget:null}),nav=createCssFactsNavigator(facts,limits);const target=facts.nodes.find(row=>row.localName==='b').id,shadow=facts.nodes.find(row=>row.nodeType===11).id,slot=facts.nodes.find(row=>row.localName==='slot').id;
 const context={shadowRoot:shadow,assignedSlot:slot,assignedSlotRoot:shadow};
 for(const source of ['::slotted(.on)::before','section > slot::slotted(.on)::before']){
  const compiled=compileCssSelectorProgram(source,{shadow:true,pseudo:'before'});assert.equal(matchesCssFacts(target,compiled,context,nav),true,source);
  assert.equal(compileCssSelectorProgram(source,{shadow:true,pseudo:'after'}).selectorCount,0);
 }
 assert.equal(compileCssSelectorProgram('::slotted(.on)',{shadow:true,pseudo:'before'}).selectorCount,0);
});
