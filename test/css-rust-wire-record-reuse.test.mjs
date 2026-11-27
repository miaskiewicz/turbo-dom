import assert from "node:assert/strict";
import test from "node:test";
import { encodeCssWire } from "../src/runtime/css-rust-wire.mjs";

test("wire record reuse preserves snapshot values and integer recursion order", () => {
	const visits = [];
	const source = Object.create(null);
	const child = (name) => new Proxy({ value: name }, {
		ownKeys(target) {
			visits.push(name);
			source.later = "mutated";
			return Reflect.ownKeys(target);
		},
	});
	source[2] = child("two");
	source[1] = child("one");
	source.later = "original";
	const proxy = new Proxy(source, {
		ownKeys: () => ["2", "1", "later"],
	});
	assert.equal(encodeCssWire(proxy), '{"1":{"value":"one"},"2":{"value":"two"},"later":"original"}');
	assert.deepEqual(visits, ["one", "two"]);
});

test("wire record reuse ignores hidden accessors and symbol serialization hooks", () => {
	let calls = 0;
	const source = Object.create(null);
	Object.defineProperty(source, "toJSON", { get() { calls++; throw Error("hidden"); } });
	Object.defineProperty(source, "hidden", { value: undefined });
	source[Symbol("ignored")] = () => { calls++; };
	source.kept = { value: -0 };
	assert.equal(encodeCssWire(source), '{"kept":{"value":0}}');
	assert.equal(calls, 0);
});

test("wire record reuse preserves nested reflection errors without invoking getters", () => {
	const failure = new Error("nested descriptor failure");
	const source = { child: new Proxy({}, {
		ownKeys() { throw failure; },
	}) };
	assert.throws(() => encodeCssWire(source), (error) => error === failure);
	let calls = 0;
	assert.throws(() => encodeCssWire({ get visible() { calls++; return 1; } }), TypeError);
	assert.equal(calls, 0);
});
