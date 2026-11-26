import { sharedDecode, sharedEncode } from "./css-shared-codec.mjs";
import { SharedMap, sharedUnique, sharedAppend } from "./css-shared-codec.mjs";
import { sharedApply } from "./css-shared-codec.mjs";
// Dependency resolution only: the native guarded projection preserves original
// rules/layers/scopes and certifies color-only conditional geometry invariance.
// No intermediate row is exposed through public GCS.
export function resolveSharedContainers({
	prepared,
	request,
	metadata,
	snapshot,
	measureScene,
	world,
	environment,
	limits,
}) {
	if (
		!sharedApply(
			"arraySome",
			metadata.groups,
			(group) => group.kind === "Container",
		)
	)
		return { request, receipt: null };
	if (typeof prepared.resolveContainerGeometry !== "function")
		throw new Error("MissingCanonicalContainerGeometryProvider");
	if (
		sharedApply(
			"arraySome",
			request.targets,
			(target) => target.pseudo !== null,
		)
	)
		throw new Error("MissingCanonicalPseudoContainerContext");
	const geometry = sharedDecode(
		prepared.resolveContainerGeometry(sharedEncode(request)),
	);
	if (geometry.geometryOnly !== true)
		throw new Error("ContainerGeometryCertificateMissing");
	const byId = new SharedMap(
		sharedApply("arrayMap", snapshot.records, (row) => [row.id, row]),
	);
	const rows = new SharedMap(
		sharedApply("arrayMap", geometry.targets, (row) => [row.id, row]),
	);
	const requested = sharedUnique(
		sharedApply("arrayFlatMap", request.targets, (target) =>
			sharedApply(
				"arrayFilter",
				sharedApply("arraySlice", target.path, 1),
				(id) => sharedApply("mapGet", byId, id)?.kind === "element",
			),
		),
	);
	const measured = measureScene({
		snapshot,
		rows: geometry.targets,
		requested,
		world,
	});
	const boxes = new SharedMap(
		sharedApply("arrayMap", measured, (row) => [row.id, row.result]),
	);
	const targets = sharedApply("arrayMap", request.targets, (target) => ({
		...target,
		containerAncestors: sharedApply(
			"arrayMap",
			sharedApply(
				"arrayFilter",
				sharedApply("arraySlice", target.path, 1),
				(id) => sharedApply("mapGet", byId, id)?.kind === "element",
			),
			(id) => {
				const style = sharedApply("mapGet", rows, id);
				if (!style) throw new Error("MissingOriginalAncestorStyle");
				const box = sharedApply("mapGet", boxes, id),
					content = box?.status === "used_box" ? box.geometry.contentBox : null;
				return ancestorFacts(id, style, content, environment);
			},
		),
	}));
	const pathNodes = sharedApply(
		"arrayReduce",
		targets,
		(sum, target) =>
			sum + target.path.length + target.containerAncestors.length,
		0,
	);
	return {
		request: {
			...request,
			targets,
			limits: limits.resolve({ snapshot, targets, request, pathNodes }),
		},
		receipt: {
			geometryOnly: true,
			originalNodeCount: snapshot.records.length,
			sceneElementTextNodes: sharedApply(
				"arrayFilter",
				snapshot.records,
				(row) => row.kind === "element" || row.kind === "text",
			).length,
			measured,
		},
	};
}

function ancestorFacts(id, style, content, environment) {
	return {
		id,
		containerName: style.rendererValues["container-name"] ?? null,
		containerType: style.rendererValues["container-type"] ?? null,
		writingMode: style.rendererValues["writing-mode"] ?? null,
		unitBases: sharedAppend(
			sharedApply(
				"arrayFilter",
				environment.unitBases,
				(pair) => pair[0] !== "em" && pair[0] !== "rem",
			),
			[
				["em", style.fontSizePx],
				["rem", style.rootFontSizePx],
			],
		),
		widthPx: content?.width ?? null,
		heightPx: content?.height ?? null,
	};
}
