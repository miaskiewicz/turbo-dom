// Mechanical ABI boundary negatives; cascade and geometry remain canonical providers.
import assert from "node:assert/strict";
import test from "node:test";
import { resolveSharedContainers } from "../src/runtime/css-shared-container.mjs";
function input() {
	return {
		prepared: {},
		request: { targets: [{ id: 2, path: [2, 1], pseudo: null }], limits: {} },
		metadata: { groups: [{ kind: "Container" }] },
		snapshot: {
			records: [
				{ id: 1, kind: "element" },
				{ id: 2, kind: "element" },
			],
		},
		world: "fixture",
		environment: {
			unitBases: [
				["vw", 10],
				["em", 16],
				["rem", 16],
			],
		},
		measureScene: () => [],
		limits: { resolve: (context) => ({ pathNodes: context.pathNodes }) },
	};
}
test("container capability/certificate and pseudo context failures remain explicit", () => {
	assert.throws(
		() => resolveSharedContainers(input()),
		/MissingCanonicalContainerGeometryProvider/,
	);
	const pseudo = input();
	pseudo.prepared.resolveContainerGeometry = () => {
		throw Error("must not call");
	};
	pseudo.request.targets[0].pseudo = "before";
	assert.throws(
		() => resolveSharedContainers(pseudo),
		/MissingCanonicalPseudoContainerContext/,
	);
	const uncertified = input();
	uncertified.prepared.resolveContainerGeometry = () =>
		JSON.stringify({ geometryOnly: false, targets: [] });
	assert.throws(
		() => resolveSharedContainers(uncertified),
		/ContainerGeometryCertificateMissing/,
	);
	const missing = input();
	missing.prepared.resolveContainerGeometry = () =>
		JSON.stringify({ geometryOnly: true, targets: [] });
	assert.throws(
		() => resolveSharedContainers(missing),
		/MissingOriginalAncestorStyle/,
	);
});
test("container unknown actual size stays null instead of authored or zero metrics", () => {
	const request = input();
	request.prepared.resolveContainerGeometry = () =>
		JSON.stringify({
			geometryOnly: true,
			targets: [
				{ id: 1, rendererValues: {}, fontSizePx: 12, rootFontSizePx: 16 },
			],
		});
	request.measureScene = () => [
		{ id: 1, result: { status: "unavailable", reason: "missing_box" } },
	];
	const result = resolveSharedContainers(request),
		ancestor = result.request.targets[0].containerAncestors[0];
	assert.equal(ancestor.widthPx, null);
	assert.equal(ancestor.heightPx, null);
	assert.equal(ancestor.containerName, null);
	assert.equal(ancestor.containerType, null);
	assert.equal(ancestor.writingMode, null);
	assert.deepEqual(ancestor.unitBases, [
		["vw", 10],
		["em", 12],
		["rem", 16],
	]);
	assert.equal(result.request.limits.pathNodes, 3);
});
