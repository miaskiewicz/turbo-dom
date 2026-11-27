import assert from "node:assert/strict";
import { test } from "node:test";
import {
	projectComposedScene,
	requireComposedTargets,
	SharedCompositionUnavailable,
} from "../src/runtime/css-shared-composition.mjs";
// Synthetic protocol contracts, not a certification of DOM capture or native layout.
const limits = {
	maxRoots: 20,
	maxNodes: 100,
	maxDepth: 20,
	maxEdges: 200,
	maxPathUnits: 1000,
};
const html = "http://www.w3.org/1999/xhtml";
function node(id, parent, treeScope, extra = {}) {
	return {
		id,
		parent,
		treeScope,
		kind: "element",
		tag: "div",
		namespace: html,
		...extra,
	};
}
function fixture() {
	return {
		roots: [
			{ id: 0, host: null, parentScope: null },
			{ id: 4, host: 1, parentScope: 0 },
		],
		records: [
			node(0, null, 0, { kind: "document" }),
			node(1, 0, 0, { shadowRootId: 4 }),
			node(2, 1, 0, { assignedSlot: 5 }),
			node(3, 1, 0),
			node(4, null, 4, { kind: "shadow" }),
			node(5, 4, 4, { tag: "slot" }),
			node(6, 5, 4),
		],
	};
}
function ids(result) {
	return result.records.map((row) => [row.sourceId, row.parentId]);
}
test("composes owned shadow and slot while preserving source IDs and slot node", () => {
	const result = projectComposedScene(fixture(), limits);
	assert.deepEqual(ids(result), [
		[1, null],
		[5, 1],
		[2, 5],
	]);
	assert.deepEqual(result.excluded, [
		{ sourceId: 3, reason: "unassigned-light" },
		{ sourceId: 6, reason: "inactive-fallback" },
	]);
	requireComposedTargets(result, [1, 2, 5]);
	assert.throws(
		() => requireComposedTargets(result, [3]),
		SharedCompositionUnavailable,
	);
});
test("complete trusted metadata carries ordered text and element assignment", () => {
	const input = fixture();
	input.records.splice(3, 0, node(7, 1, 0, { kind: "text" }));
	input.slotAssignments = [{ slot: 5, assigned: [7, 2], complete: true }];
	assert.deepEqual(ids(projectComposedScene(input, limits)), [
		[1, null],
		[5, 1],
		[7, 5],
		[2, 5],
	]);
	delete input.slotAssignments;
	assert.throws(() => projectComposedScene(input, limits), /uncertified text/);
});
test("empty certified list retains fallback and excludes unassigned light", () => {
	const input = fixture();
	delete input.records[2].assignedSlot;
	input.slotAssignments = [{ slot: 5, assigned: [], complete: true }];
	assert.deepEqual(ids(projectComposedScene(input, limits)), [
		[1, null],
		[5, 1],
		[6, 5],
	]);
});
test("refuses foreign, duplicate, contradictory and uncertified assignment facts", () => {
	for (const change of [
		(x) => {
			x.records[2].assignedSlot = 99;
		},
		(x) => {
			x.slotAssignments = [{ slot: 5, assigned: [2, 2], complete: true }];
		},
		(x) => {
			x.slotAssignments = [{ slot: 5, assigned: [], complete: true }];
		},
		(x) => {
			x.slotAssignments = [{ slot: 5, assigned: [2], complete: false }];
		},
		(x) => {
			x.roots[1].parentScope = 4;
		},
		(x) => {
			x.records[2].parent = 99;
		},
	]) {
		const input = fixture();
		change(input);
		assert.throws(
			() => projectComposedScene(input, limits),
			SharedCompositionUnavailable,
		);
	}
});
test("explicit quotas and own-data boundary preserve original errors", () => {
	for (const name of Object.keys(limits))
		assert.throws(
			() => projectComposedScene(fixture(), { ...limits, [name]: 0 }),
			SharedCompositionUnavailable,
		);
	const input = fixture();
	let reads = 0;
	Object.defineProperty(input, "roots", {
		get() {
			reads++;
			throw Error("getter");
		},
	});
	assert.throws(() => projectComposedScene(input, limits), /accessor roots/);
	assert.equal(reads, 0);
	const error = new Error("original trap");
	const proxy = new Proxy(
		{},
		{
			getOwnPropertyDescriptor() {
				throw error;
			},
		},
	);
	assert.throws(
		() => projectComposedScene(proxy, limits),
		(value) => value === error,
	);
});

test("nested shadow substitution preserves physical sources and fallback descendants", () => {
	const input = fixture();
	input.roots.push({ id: 8, host: 2, parentScope: 0 });
	input.records[2].shadowRootId = 8;
	input.records.push(node(8, null, 8, { kind: "shadow" }), node(9, 8, 8));
	assert.deepEqual(ids(projectComposedScene(input, limits)), [
		[1, null],
		[5, 1],
		[2, 5],
		[9, 2],
	]);
});
test("uncertified multiple assignments refuse rather than guessing native order", () => {
	const input = fixture();
	input.records.splice(3, 0, node(7, 1, 0, { assignedSlot: 5 }));
	assert.throws(
		() => projectComposedScene(input, limits),
		/uncertified assigned-node order/,
	);
	input.slotAssignments = [{ slot: 5, assigned: [7, 2], complete: true }];
	assert.deepEqual(ids(projectComposedScene(input, limits)), [
		[1, null],
		[5, 1],
		[7, 5],
		[2, 5],
	]);
});
test("plain document projection preserves parent order and exact path quota", () => {
	const input = {
		roots: [{ id: 0, host: null, parentScope: null }],
		records: [
			node(0, null, 0, { kind: "document" }),
			node(10, 0, 0),
			node(20, 10, 0, { kind: "text" }),
		],
	};
	assert.deepEqual(
		ids(projectComposedScene(input, { ...limits, maxPathUnits: 3 })),
		[
			[10, null],
			[20, 10],
		],
	);
	assert.throws(
		() => projectComposedScene(input, { ...limits, maxPathUnits: 2 }),
		/maxPathUnits limit/,
	);
});

test("inactive fallback excludes its nested slot assignment instead of leaking light nodes", () => {
	const input = fixture();
	input.records.splice(3, 0, node(7, 1, 0, { assignedSlot: 8 }));
	input.records.push(node(8, 6, 4, { tag: "slot" }));
	const result = projectComposedScene(input, limits);
	assert.ok(result.excluded.some((row) => row.sourceId === 7));
	assert.deepEqual(ids(result), [
		[1, null],
		[5, 1],
		[2, 5],
	]);
});
test("nonrendering records are skipped and malformed required facts refuse", () => {
	const input = fixture();
	input.records.push(
		node(9, 4, 4, { kind: "comment" }),
		node(10, 4, 4, { kind: "doctype" }),
	);
	assert.deepEqual(ids(projectComposedScene(input, limits)), [
		[1, null],
		[5, 1],
		[2, 5],
	]);
	input.records.push(node(11, 4, 4, { kind: "unknown" }));
	assert.throws(
		() => projectComposedScene(input, limits),
		/unsupported composed node kind/,
	);
	const missing = fixture();
	delete missing.records[1].id;
	assert.throws(() => projectComposedScene(missing, limits), /missing id/);
});
