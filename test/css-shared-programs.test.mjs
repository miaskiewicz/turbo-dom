// Unsupported compiler metadata is a capability error, never an empty match set.
import assert from "node:assert/strict";
import test from "node:test";
import { compileSharedPrograms } from "../src/runtime/css-shared-programs.mjs";
test("invalid selector items are retained as invalid, without inventing a matcher", () => {
	const compiled = compileSharedPrograms({
		selectors: [{ id: 1, items: null }],
		groups: [],
	});
	assert.equal(compiled.selectorIds.size, 0);
	assert.equal(compiled.groups.length, 0);
});
test("unavailable scope-boundary compilation refuses both start and limit capabilities", () => {
	for (const boundary of ["start", "end"]) {
		const scope = { start: null, end: null, startItems: null, endItems: null };
		scope[boundary] = [46, 120];
		assert.throws(
			() =>
				compileSharedPrograms({ selectors: [], groups: [{ id: 1, scope }] }),
			/scope boundary/,
		);
	}
});
