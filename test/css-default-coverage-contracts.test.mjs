import assert from "node:assert/strict";
import test from "node:test";
import { createCssArtifactResolver } from "../src/runtime/css-artifact.mjs";
import {
	CssDomSnapshotLimitError,
	createDefaultCssCapture,
} from "../src/runtime/css-default-capture.mjs";
import {
	createEnvironment,
	setParser,
	setParserMode,
} from "./opt-in-helpers/runtime.mjs";
import {
	compileCssFactsProgram,
	matchesCssFacts,
} from "../src/runtime/selectors.mjs";

const backend = createCssArtifactResolver().resolve({ mode: "auto" });
const policy = JSON.parse(backend.cssRuntimePolicyV1());

test("native capture preserves doctype identifiers and bounds owned attributes after mutation", () => {
	const { document } = createEnvironment(
		'<!DOCTYPE html PUBLIC "fixture-public" "fixture-system"><main id="target">text</main>',
	);
	const capture = createDefaultCssCapture();
	const initial = capture.capture(document, policy.snapshot);
	const doctype = initial.records.find(({ kind }) => kind === "doctype");
	assert.equal(doctype.element, document.doctype);
	assert.deepEqual(
		[doctype.element.name, doctype.element.publicId, doctype.element.systemId],
		["html", "fixture-public", "fixture-system"],
	);

	const target = document.getElementById("target");
	target.setAttribute("title", "déjà vu");
	const after = capture.capture(document, policy.snapshot);
	const maxAttributes = after.used.maxAttributes;
	assert.equal(maxAttributes, initial.used.maxAttributes + 1);
	assert.deepEqual(
		capture
			.capture(document, { ...policy.snapshot, maxAttributes })
			.records.find(({ element }) => element === target)
			.attrs.map(({ name, value }) => [name, value]),
		[
			["id", "target"],
			["title", "déjà vu"],
		],
	);
	assert.throws(
		() =>
			capture.capture(document, {
				...policy.snapshot,
				maxAttributes: maxAttributes - 1,
			}),
		(error) =>
			error instanceof CssDomSnapshotLimitError &&
			error.limit === "maxAttributes",
	);
});

test("capture handles focused closed shadow content before detach and after reinsert", () => {
	const { document } = createEnvironment("<main>outside</main>");
	const host = document.createElement("section");
	const root = host.attachShadow({ mode: "closed" });
	const input = document.createElement("input");
	root.append(input);
	document.body.append(host);
	input.focus();

	const capture = createDefaultCssCapture();
	const focus = compileCssFactsProgram(":focus");
	const focused = (snapshot, target) =>
		matchesCssFacts(
			snapshot.nodeId(target),
			focus,
			{},
			capture.selectorFacts(snapshot, policy.selectorFacts),
		);
	const connected = capture.capture(document, policy.snapshot);
	assert.equal(focused(connected, input), true);
	assert.equal(focused(connected, host), true);

	host.remove();
	const detached = capture.capture(document, policy.snapshot);
	assert.equal(detached.records.some(({ element }) => element === input), false);
	assert.equal(focused(detached, document.querySelector("main")), false);

	document.body.append(host);
	input.focus();
	const reinserted = capture.capture(document, policy.snapshot);
	assert.equal(reinserted.nodeId(input), connected.nodeId(input));
	assert.equal(focused(reinserted, host), true);
});

test("public computed used sizes track both viewport axes and refuse an unavailable height", () => {
	const { document, window } = createEnvironment(`
		<style>
			#target { display:block; width:12.5vw; height:25vh; color:blue }
			@media (min-height:600px) { #target { color:red } }
		</style><p id="target">text</p>
	`);
	const target = document.getElementById("target");
	const values = () => {
		const style = window.getComputedStyle(target);
		return [style.width, style.height, style.color];
	};
	const originalVersion = document.__version;
	assert.deepEqual(values(), ["128px", "192px", "rgb(255, 0, 0)"]);
	window.innerHeight = 400;
	assert.deepEqual(values(), ["128px", "100px", "rgb(0, 0, 255)"]);
	window.innerWidth = 800;
	assert.deepEqual(values(), ["100px", "100px", "rgb(0, 0, 255)"]);
	window.innerHeight = undefined;
	assert.throws(values, /CSS window innerHeight unavailable/);
	assert.equal(document.__version, originalVersion);

	window.innerHeight = 400;
	target.remove();
	const detached = window.getComputedStyle(target);
	assert.equal(detached.length, 0);
	assert.equal(detached.width, "");
	document.body.append(target);
	assert.deepEqual(values(), ["100px", "100px", "rgb(0, 0, 255)"]);
});

test("one document retains native computed semantics when switching CSS transport twice", () => {
	try {
		setParserMode("native");
		const { document, window } = createEnvironment(`
			<style>
				main { --reference: 30px; color: green }
				p { width: calc(var(--reference) + 10px); color: inherit }
			</style><main><p>text</p></main>
		`);
		const target = document.querySelector("p");
		const style = () => {
			const computed = window.getComputedStyle(target);
			return [computed.width, computed.color];
		};
		assert.deepEqual(style(), ["40px", "rgb(0, 128, 0)"]);
		setParserMode("wasm");
		assert.deepEqual(style(), ["40px", "rgb(0, 128, 0)"]);
		target.parentElement.style.setProperty("--reference", "50px");
		assert.deepEqual(style(), ["60px", "rgb(0, 128, 0)"]);
		setParserMode("native");
		assert.deepEqual(style(), ["60px", "rgb(0, 128, 0)"]);
	} finally {
		setParserMode(null);
	}
});

test("public styles refuse incomplete CSS resource policy and recover on the shipped backend", () => {
	const { document, window } = createEnvironment(
		"<style>p { color:green }</style><p>text</p>",
	);
	const target = document.querySelector("p");
	const color = () => window.getComputedStyle(target).color;
	assert.equal(color(), "rgb(0, 128, 0)");

	const incompleteAdapter = { ...policy.adapter };
	delete incompleteAdapter.contextualMatching;
	const cases = [
		[undefined, /cssRuntimePolicyV1/],
		[() => JSON.stringify({ ...policy, abiVersion: 2 }), /policy ABI/],
		[() => JSON.stringify({ ...policy, adapter: null }), /policy.adapter.compile/],
		[
			() => JSON.stringify({ ...policy, adapter: incompleteAdapter }),
			/policy.adapter.contextualMatching/,
		],
	];
	try {
		for (const [cssRuntimePolicyV1, reason] of cases) {
			setParser({ ...backend, cssBackendVersion: "0.8.7", cssRuntimePolicyV1 });
			assert.throws(color, reason);
		}
	} finally {
		setParser(null);
	}
	assert.equal(color(), "rgb(0, 128, 0)");
});

test("computed styles use the target owner viewport and require a bound viewport for used sizes", () => {
	const first = createEnvironment(
		'<style>p { width:25vw }</style><p>first</p>',
	);
	const second = createEnvironment(
		'<style>p { width:25vw }</style><p>second</p>',
	);
	first.window.innerWidth = 400;
	second.window.innerWidth = 800;
	assert.equal(first.window.getComputedStyle(first.document.querySelector("p")).width, "100px");
	assert.equal(first.window.getComputedStyle(second.document.querySelector("p")).width, "200px");

	const inert = first.document.implementation.createHTMLDocument("Inert");
	const target = inert.createElement("p");
	inert.body.append(target);
	assert.throws(
		() => first.window.getComputedStyle(target),
		/CSS native window context unavailable/,
	);
});

test("capture preserves null-namespace names and refuses non-document roots or foreign snapshots", () => {
	const { document, window } = createEnvironment("<main>text</main>");
	const root = document.querySelector("main");
	const unqualified = document.createElementNS(null, "xml-name");
	root.append(unqualified);

	const capture = createDefaultCssCapture();
	const snapshot = capture.capture(document, policy.snapshot);
	const row = snapshot.records.find(({ element }) => element === unqualified);
	assert.equal(row.tag, "xml-name");
	assert.equal(row.namespace, null);
	assert.equal(row.namespaceUnits, null);
	assert.throws(() => capture.capture(null, policy.snapshot), /foreign document/);
	assert.throws(() => capture.capture(root, policy.snapshot), /foreign document/);

	const other = createDefaultCssCapture();
	assert.throws(
		() => other.sources(snapshot, policy.adapter.compile),
		/unknown native source snapshot/,
	);
	assert.throws(
		() => other.selectorFacts(snapshot, policy.selectorFacts),
		/unknown native selector snapshot/,
	);
	assert.throws(() => window.getComputedStyle({}), /foreign element/);
	assert.equal(capture.capture(document, policy.snapshot).nodeId(unqualified), row.id);
});

test('native CSS host platform data is validated without invoking accessors',()=>{
 const {document,window}=createEnvironment('<style>p{color:blue}</style><p>platform</p>');
 const target=document.querySelector('p'),color=()=>window.getComputedStyle(target).color;
 assert.equal(color(),'rgb(0, 0, 255)');
 const field=Object.getOwnPropertyDescriptor(process,'platform');
 let calls=0;
 try{
  for(const value of [undefined,7]){
   Object.defineProperty(process,'platform',{configurable:true,value});
   assert.throws(color,/host platform/);
  }
  delete process.platform;
  assert.throws(color,/host platform/);
  Object.defineProperty(process,'platform',{configurable:true,get(){calls++;return 'darwin';}});
  assert.throws(color,/host platform/);
  assert.equal(calls,0);
  Object.defineProperty(process,'platform',{configurable:true,value:'linux'});
  assert.equal(color(),'rgb(0, 0, 255)');
 }finally{Object.defineProperty(process,'platform',field);}
 assert.equal(color(),'rgb(0, 0, 255)');
});
