import { SharedMap, sharedMapKeys, sharedAppend } from "./css-shared-codec.mjs";
import { sharedApply } from "./css-shared-codec.mjs";
import { sharedText } from "./css-shared-codec.mjs";
import { cssFocusTarget } from "./css-invalidation.mjs";
import { compileCssMatchFacts } from "./css-match-facts.mjs";

export function compileSharedPrograms(metadata) {
	const flat = [],
		selectorIds = new SharedMap(),
		contextual = new SharedMap();
	function item(value) {
		const id = flat.length;
		sharedApply("arrayPush", flat, {
			id,
			units: value.units,
			seed: value.seed
				? { kind: value.seed.kind, value: sharedText(value.seed.valueUnits) }
				: null,
			pseudo: value.pseudo,
			implicitScopeAnchor: value.implicitScopeAnchor,
			allowScopeReference: true,
		});
		return { id, ...flat[id] };
	}
	collectSelectors(metadata.selectors, item, selectorIds, contextual);
	const groups = collectGroups(metadata.groups, item);
	// Boundary programs are compiled through their own metadata exactly once.
	const ordinary = sharedApply("arrayFilter", flat, (program) =>
		sharedApply("mapHas", selectorIds, program.id),
	);
	return {
		collector: compileCssMatchFacts({
			selectors: ordinary,
			scopeGroups: groups,
		}),
		selectorIds,
		groups,
		contextual,
	};
}
export function collectSharedAnswers(program, snapshot, document, limits) {
	const elements = snapshot.matchNodes,
		ids = sharedApply("arrayMap", elements, (row) => row.id),
		roots = sharedAppend([null], ids);
	const selectorQueries = sharedApply(
		"arrayMap",
		sharedMapKeys(program.selectorIds),
		(selectorId) => ({
			selectorId,
			scopeRootIds:
				sharedApply("mapGet", program.contextual, selectorId) === "anchor"
					? ids
					: sharedApply("mapHas", program.contextual, selectorId)
						? roots
						: [null],
		}),
	);
	const scopeQueries = [];
	for (let index = 0; index < program.groups.length; index++)
		for (let edge = 0; edge < 2; edge++) {
			const group = program.groups[index],
				boundary = edge === 0 ? "start" : "end";
			if (group[boundary] !== undefined)
				sharedApply("arrayPush", scopeQueries, {
					groupId: group.id,
					boundary,
					enclosingRootIds: roots,
					candidateIds: ids,
				});
		}
	const facts = program.collector.collect(
		elements,
		{
			ownerDocument: document,
			sheetRootIds: [],
			targetIds: ids,
			selectorQueries,
			scopeQueries,
			liveState: { focusTarget: cssFocusTarget },
		},
		limits({ snapshot, selectorQueries, scopeQueries }),
	);
	return {
		domains: sharedApply("arrayMap", facts.domains, (row) => ({
			...row,
			nodeIds: sharedAppend(row.nodeIds, [snapshot.documentId]),
		})),
		selectorAnswers: sharedApply("arrayMap", facts.selectorAnswers, (row) => ({
			...sharedApply("mapGet", program.selectorIds, row.selectorId),
			rootId: row.scopeRootId,
			domainId: row.domainId,
			matches: row.matches,
		})),
		scopeAnswers: sharedApply("arrayMap", facts.scopeAnswers, (row) => ({
			groupId: row.groupId,
			boundary: row.boundary,
			rootId: row.enclosingRootId ?? snapshot.documentId,
			domainId: row.domainId,
			matches: row.matches,
		})),
	};
}

function collectSelectors(selectors, item, selectorIds, contextual) {
	for (let index = 0; index < selectors.length; index++) {
		const selector = selectors[index];
		if (selector.items === null) continue;
		for (let child = 0; child < selector.items.length; child++) {
			const compiled = selector.items[child];
			const program = item(compiled);
			sharedApply("mapSet", selectorIds, program.id, {
				selectorId: selector.id,
				itemId: compiled.index,
			});
			if (compiled.implicitScopeAnchor || compiled.referencesScope)
				sharedApply(
					"mapSet",
					contextual,
					program.id,
					compiled.implicitScopeAnchor ? "anchor" : "scope",
				);
		}
	}
}
function collectGroups(source, item) {
	const groups = [];
	for (let index = 0; index < source.length; index++) {
		const group = source[index];
		if (skipGroup(group)) continue;
		const row = { id: group.id };
		if (group.scope.start !== null)
			row.start =
				group.scope.startItems === null
					? null
					: sharedApply("arrayMap", group.scope.startItems, item);
		if (group.scope.end !== null)
			row.end =
				group.scope.endItems === null
					? null
					: sharedApply("arrayMap", group.scope.endItems, item);
		sharedApply("arrayPush", groups, row);
	}
	return groups;
}

function skipGroup(group) {
	return !group.scope || group.invalidScope;
}
