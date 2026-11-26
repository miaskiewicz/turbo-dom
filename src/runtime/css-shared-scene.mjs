import {
	projectComposedScene,
	requireComposedTargets,
} from "./css-shared-composition.mjs";
// Original primitive nodes + canonical Rust computed rows, never authored sizing.
import {
	SharedMap,
	sharedApply,
	sharedUnits,
	sharedEncode,
	sharedDecode,
	sharedInvoke,
	sharedKeys,
} from "./css-shared-codec.mjs";
function attribute(value) {
	if (value.prefix) throw new Error("MissingOriginalAttributeNamespace");
	return {
		nameUnits: value.nameUnits,
		namespaceUnits: null,
		valueUnits: value.valueUnits,
	};
}
function nodeRow(row, byId, computed, composedParent) {
	const parent = sharedApply("mapGet", byId, row.parent);
	const common = {
		id: row.id,
		parent:
			composedParent === undefined
				? parent?.kind === "element"
					? parent.id
					: null
				: composedParent,
	};
	if (row.kind === "text")
		return { ...common, node: { kind: "text", textUnits: row.textUnits } };
	const style = sharedApply("mapGet", computed, row.id);
	if (!style) throw new Error("MissingCanonicalComputedSceneStyle");
	const names = sharedKeys(style),
		properties = [];
	for (let index = 0; index < names.length; index++)
		sharedApply("arrayPush", properties, {
			nameUnits: sharedUnits(names[index]),
			valueUnits: style[names[index]],
		});
	return {
		...common,
		node: {
			kind: "element",
			tagUnits: row.tagUnits,
			namespaceUnits: row.namespaceUnits,
			attrs: sharedApply("arrayMap", row.attrs, attribute),
			computed: properties,
		},
	};
}
function counts(nodes) {
	let properties = 0,
		styleUnits = 0;
	for (let index = 0; index < nodes.length; index++) {
		const values = nodes[index].node.computed;
		if (!values) continue;
		properties += values.length;
		for (let field = 0; field < values.length; field++)
			styleUnits +=
				values[field].nameUnits.length + values[field].valueUnits.length;
	}
	return { properties, styleUnits };
}
/** Only a canonical trusted primitive snapshot is accepted by this private seam.
 * Policy owns viewport/font profile and every resource limit explicitly. */
function sceneNodes(snapshot, byId, computed, requested, compositionLimits) {
	if (snapshot.roots.length !== 1) {
		if (typeof compositionLimits !== "function")
			throw new Error("MissingCanonicalShadowScene");
		const projection = projectComposedScene(
			snapshot,
			compositionLimits(snapshot),
		);
		requireComposedTargets(projection, requested);
		return sharedApply("arrayMap", projection.records, (row) =>
			nodeRow(snapshot.records[row.recordIndex], byId, computed, row.parentId),
		);
	}
	const selected = sharedApply(
		"arrayFilter",
		snapshot.records,
		(row) => row.kind === "element" || row.kind === "text",
	);
	return sharedApply("arrayMap", selected, (row) =>
		nodeRow(row, byId, computed),
	);
}
export function createSharedSceneProvider({
	selectBackend,
	viewport,
	fonts,
	limits,
	compositionLimits,
}) {
	if (typeof selectBackend !== "function" || typeof limits !== "function")
		throw new TypeError("CSS Scene provider policy required");
	return function measureScene({ snapshot, rows, requested, world }) {
		if (snapshot.roots.length !== 1 && typeof compositionLimits !== "function")
			throw new Error("MissingCanonicalShadowScene");
		const backend = selectBackend();
		if (typeof backend.measureSceneV1 !== "function")
			throw new Error("MissingCanonicalSceneProvider");
		const byId = new SharedMap(
			sharedApply("arrayMap", snapshot.records, (row) => [row.id, row]),
		);
		const computed = new SharedMap(
			sharedApply("arrayMap", rows, (row) => [row.id, row.rendererValues]),
		);
		const nodes = sceneNodes(
			snapshot,
			byId,
			computed,
			requested,
			compositionLimits,
		);
		const request = {
			world: sharedUnits(world),
			viewport,
			fonts,
			nodes,
			requested,
			limits: limits({
				snapshot,
				nodes,
				requested,
				world,
				counts: counts(nodes),
			}),
		};
		const response = sharedDecode(
			sharedInvoke(backend.measureSceneV1, backend, [
				sharedEncode({ abiVersion: 1, scene: request }),
			]),
		);
		if (!sameWorld(response.world, request.world))
			throw new Error("CanonicalSceneWorldMismatch");
		return response.measurements;
	};
}
function sameWorld(actual, expected) {
	if (actual.length !== expected.length) return false;
	for (let index = 0; index < actual.length; index++)
		if (actual[index] !== expected[index]) return false;
	return true;
}
