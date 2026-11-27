import test from "node:test";
import assert from "node:assert/strict";
import { createEnvironment } from "../src/runtime/index.mjs";
import {
	compileCssFactsProgram,
	matchesCssFacts,
} from "../src/runtime/selectors.mjs";
import {
	createCssFactsNavigator,
	CssFactsUnavailable,
} from "../src/runtime/css-facts-navigator.mjs";
import { captureCssPrimitivePublicFacts } from "./helpers/css-primitive-public-facts.mjs";

function setup() {
	const { document } = createEnvironment('<b class="on"></b>');
	const facts = captureCssPrimitivePublicFacts(document, { focusTarget: null });
	const target = facts.nodes.find((row) => row.localName === "b").id;
	const navigator = createCssFactsNavigator(facts, {
		maxNodes: 100,
		maxEdges: 1000,
		maxTextUnits: 10000,
		maxAttributeUnits: 10000,
	});
	return { target, navigator, program: compileCssFactsProgram(":is(.on)") };
}
function poison(object, key, descriptor, operation) {
	const before = Object.getOwnPropertyDescriptor(object, key);
	let error;
	try {
		Object.defineProperty(object, key, descriptor);
		operation();
	} catch (caught) {
		error = caught;
	} finally {
		if (before) Object.defineProperty(object, key, before);
		else delete object[key];
	}
	return error;
}
test("CSS facts decline replaced array callbacks before exposing AST", () => {
	const fixture = setup();
	let calls = 0;
	const error = poison(
		Array.prototype,
		"some",
		{
			configurable: true,
			value() {
				calls++;
				throw new Error("observed private AST");
			},
		},
		() =>
			matchesCssFacts(fixture.target, fixture.program, {}, fixture.navigator),
	);
	assert.ok(error instanceof CssFactsUnavailable);
	assert.equal(calls, 0);
	assert.equal(
		matchesCssFacts(fixture.target, fixture.program, {}, fixture.navigator),
		true,
	);
});
test("facts compile refuses species and indexed prototype setters before parser allocation", () => {
	let calls = 0;
	for (const [object, key] of [
		[Array, Symbol.species],
		[Array.prototype, "0"],
	]) {
		const error = poison(
			object,
			key,
			{
				configurable: true,
				get() {
					calls++;
					throw new Error("array getter");
				},
				set() {
					calls++;
					throw new Error("array setter");
				},
			},
			() => compileCssFactsProgram(":is(.on)"),
		);
		assert.ok(error instanceof CssFactsUnavailable);
	}
	assert.equal(calls, 0);
	assert.equal(compileCssFactsProgram(".on").selectorCount, 1);
});
test("facts matcher refuses changed AST program collection lookup", () => {
	const fixture = setup();
	let calls = 0;
	const error = poison(
		WeakMap.prototype,
		"get",
		{
			configurable: true,
			value() {
				calls++;
				throw new Error("observed private program");
			},
		},
		() =>
			matchesCssFacts(fixture.target, fixture.program, {}, fixture.navigator),
	);
	assert.ok(error instanceof CssFactsUnavailable);
	assert.equal(calls, 0);
	assert.equal(
		matchesCssFacts(fixture.target, fixture.program, {}, fixture.navigator),
		true,
	);
});

test("indexed prototype checks preserve the full canonical nonnegative number domain", () => {
	const fixture = setup();
	const matches = () =>
		matchesCssFacts(fixture.target, fixture.program, {}, fixture.navigator);
	let calls = 0;
	const descriptor = {
		configurable: true,
		get() { calls++; },
		set() { calls++; },
	};
	for (const name of ["17", "0.5", "1e+21", "Infinity"])
		assert.match(poison(Array.prototype, name, descriptor, matches)?.message, /array indexed prototype/);
	for (const name of ["", "00", "-0", "-1", "1e3", "NaN", "1name"])
		assert.equal(poison(Array.prototype, name, descriptor, matches), undefined);
	assert.equal(calls, 0);
});

test("indexed prototype validation does not invoke replaced numeric constructors", () => {
	const fixture = setup();
	const number = Number;
	let numericCalls = 0;
	const replacement = new Proxy(number, {
		apply(target, receiver, args) {
			numericCalls++;
			return Reflect.apply(target, receiver, args);
		},
	});
	let result;
	const error = poison(globalThis, "Number", { configurable: true, value: replacement }, () => {
		result = matchesCssFacts(fixture.target, fixture.program, {}, fixture.navigator);
	});
	assert.equal(error, undefined);
	assert.equal(result, true);
	assert.equal(numericCalls, 0);
});
