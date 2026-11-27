import assert from "node:assert/strict";
import test from "node:test";
import { sharedProviderSnapshot } from "../src/runtime/css-shared-adapter.mjs";

const policy = { maxDepth: 8, maxEntries: 100 };
function copy(value) {
	return sharedProviderSnapshot(
		{ roots: [], records: [], used: { value } },
		policy,
	).used.value;
}

test("provider scalar classification preserves all primitive values without coercion", () => {
	for (const value of [null, "", "x", 0, -0, 1, NaN, Infinity, true, false])
		assert.ok(Object.is(copy(value), value));
	for (const value of [undefined, 1n, Symbol("value"), () => 1])
		assert.throws(() => copy(value), /not primitive data/);
});

test("provider scalar copy does not coerce objects or invoke payload accessors", () => {
	let calls = 0;
	const value = {
		get payload() {
			calls++;
			return 1;
		},
	};
	assert.throws(() => copy(value), /snapshot accessor/);
	assert.equal(calls, 0);
	const source = { nested: ["x", 2, null, false] };
	const result = copy(source);
	assert.deepEqual(result, source);
	assert.notEqual(result, source);
	assert.notEqual(result.nested, source.nested);
	source.nested[0] = "changed";
	assert.equal(result.nested[0], "x");
});
