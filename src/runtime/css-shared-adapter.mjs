import { readCssFactsNavigator } from './css-facts-navigator.mjs';
import { createSharedRouting } from './css-shared-routing.mjs';
import { compileSharedShadowFacts } from './css-shared-shadow-facts.mjs';
import {
	SharedMap,
	sharedInvoke,
	sharedDefine,
	sharedAppend,
} from "./css-shared-codec.mjs";
import {
	sharedApply,
	sharedIsArray,
	sharedKeys,
	sharedDescriptor,
	sharedEntries,
	sharedFromEntries,
} from "./css-shared-codec.mjs";
// Private ABI orchestration. CSS parsing, cascade and property policy stay in Rust.
// captureSnapshot must be the trusted native snapshot producer. Reflection below
// copies already-owned data; it neither brands arbitrary objects nor proves a
// Proxy trap-free. Original hook/Proxy errors propagate unchanged.
import { createSharedComputedStyle } from "./css-gcs-factory.mjs";
import { captureCssSources } from "./css-sources.mjs";
import {
	compileSharedPrograms,
	collectSharedAnswers,
} from "./css-shared-programs.mjs";
import { resolveSharedContainers } from "./css-shared-container.mjs";
import {
	sharedUnits as units,
	sharedText,
	sharedDecode as decode,
	sharedEncode,
} from "./css-shared-codec.mjs";
function required(value, name) {
	if (typeof value !== "function")
		throw new TypeError(`turbo-dom: shared adapter requires ${name}`);
	return value;
}
function call(backend, name, request) {
	return decode(
		sharedInvoke(required(backend[name], name), backend, [
			sharedEncode(request),
		]),
	);
}
function isScalar(value) {
	return (
		value === null ||
		sharedApply("arrayIncludes", ["string", "number", "boolean"], typeof value)
	);
}
const plainPrototype = Object.prototype;
const prototypeOf = Object.getPrototypeOf;
function primitive(value, budget, depth) {
	if (depth > budget.maxDepth)
		throw new RangeError("CSS provider snapshot depth");
	if (isScalar(value)) return value;
	if (typeof value !== "object")
		throw new TypeError("CSS provider snapshot is not primitive data");
	const output = primitiveContainer(value);
	const keys = sharedKeys(value);
	for (let index = 0; index < keys.length; index++) {
		const key = keys[index];
		if (--budget.remaining < 0)
			throw new RangeError("CSS provider snapshot entries");
		const descriptor = sharedDescriptor(value, key);
		if (!descriptor || !("value" in descriptor))
			throw new TypeError("CSS provider snapshot accessor");
		sharedDefine(output, key, {
			value: primitive(descriptor.value, budget, depth + 1),
			enumerable: true,
			writable: true,
			configurable: true,
		});
	}
	return output;
}
function primitiveContainer(value) {
	const proto = prototypeOf(value);
	if (sharedIsArray(value)) return [];
	if (proto !== plainPrototype && proto !== null)
		throw new TypeError("CSS provider snapshot opaque object");
	return {};
}
function cleanRecord(record) {
	const output = {};
	const keys = sharedKeys(record);
	for (let index = 0; index < keys.length; index++) {
		const key = keys[index];
		if (key === "element") continue;
		const descriptor = sharedDescriptor(record, key);
		if (!descriptor || !("value" in descriptor))
			throw new TypeError("CSS provider snapshot accessor");
		sharedDefine(output, key, { value: descriptor.value, enumerable: true });
	}
	return output;
}
export function sharedProviderSnapshot(snapshot, policy) {
	if (
		!Number.isSafeInteger(policy.maxDepth) ||
		policy.maxDepth < 0 ||
		!Number.isSafeInteger(policy.maxEntries) ||
		policy.maxEntries < 0
	)
		throw new TypeError("CSS provider snapshot policy");
	if (snapshot.records.length + snapshot.roots.length > policy.maxEntries)
		throw new RangeError("CSS provider snapshot entries");
	const clean = cleanRecord;
	return primitive(
		{
			roots: sharedApply("arrayMap", snapshot.roots, clean),
			used: snapshot.used,
			records: sharedApply("arrayMap", snapshot.records, clean),
		},
		{ maxDepth: policy.maxDepth, remaining: policy.maxEntries },
		0,
	);
}
function identities(response, id, property) {
	if (
		response.abiVersion !== 1 ||
		response.id !== id ||
		response.property !== property
	)
		throw new Error("CSS used projection identity");
	return response.result;
}
function absentMetric(result) {
	if (result?.status !== "unavailable") return null;
	if (result.reason === "excluded") return { status: "noBox" };
	if (result.reason === "inline_box") return { status: "nonAtomicInline" };
	return null;
}
function metric(result, dimension) {
	const absent = absentMetric(result);
	if (absent) return absent;
	if (result?.status !== "used_box")
		throw new Error(
			"CanonicalUsedMeasurementUnavailable:" + sharedEncode(result),
		);
	return {
		status: "pixels",
		content: result.geometry.contentBox[dimension],
		border: result.geometry.borderBox[dimension],
	};
}
function axisDimension(axis) {
	if (axis === "horizontal") return "width";
	if (axis === "vertical") return "height";
	throw new Error("CSS used projection axis");
}
function publicRow(row, backend, profile, limits) {
	const target = {
		id: row.id,
		values: sharedEntries(row.rendererValues),
		publicKeys: row.publicKeys,
		fontSizePx: row.fontSizePx,
		declaredTextDecoration: row.declaredTextDecoration
			? {
					value: row.declaredTextDecoration.value,
					origin: row.declaredTextDecoration.origin,
				}
			: null,
	};
	const request = {
		abiVersion: 1,
		targets: [target],
		textDecoration: profile.textDecoration,
		includeUsedProperties: true,
		limits: limits.serialize(target),
	};
	const result = call(backend, "serializeCssomV1", request);
	if (
		result.abiVersion !== 1 ||
		result.targets.length !== 1 ||
		result.targets[0].id !== row.id
	)
		throw new Error("CSS serialization identity");
	return {
		...row,
		values: sharedFromEntries(result.targets[0].values),
		usedProperties: result.targets[0].usedProperties,
	};
}
function uses(sources, snapshot, ua, profile, routing) {
	const root = snapshot.documentId;
	const documentRoot = sharedApply(
		"arrayFind",
		snapshot.records,
		(row) => row.kind === "element" && row.parent === root,
	)?.id;
	if (documentRoot === undefined)
		throw new Error("CSS original document element missing");
	const uaUse = (id) => ({
		sourceId: id,
		origin: "userAgent",
		treeScope: root,
		scopeDepth: 0,
		disabled: false,
		media: null,
		implicitScopeRoot: documentRoot,
	});
	return sharedAppend(
		sharedAppend(
            ua.documentText ? [uaUse(0), uaUse(1)] : [uaUse(0)],
            routing ? shadowUserAgentUses(snapshot, routing) : [],
        ),
		sharedApply("arrayMap", sources.uses, (use) => ({
			sourceId: use.sourceId + 2,
			origin: "author",
			treeScope: use.scopeId,
			scopeDepth: routing ? routing.scopeDepth(use.scopeId) : profile.scopeDepth(use.scopeId, snapshot),
			disabled: use.disabled,
			media: use.sheetMedia.length
				? units(sharedApply("arrayJoin", use.sheetMedia, ", "))
				: use.ownerMedia
					? units(use.ownerMedia)
					: null,
			implicitScopeRoot: use.implicitScopeRootId,
		})),
	);
}
function targets(snapshot, pseudo, element, profile, routing) {
	const byId = new SharedMap(
		sharedApply("arrayMap", snapshot.records, (row) => [row.id, row]),
	);
	let targets = sharedApply(
		"arrayMap",
		sharedApply(
			"arrayFilter",
			snapshot.records,
			(row) => row.kind === "element",
		),
		(row) => {
			const parent = sharedApply("mapGet", byId, row.parent);
			return {
				id: row.id,
				subjectId: row.id,
				parentId: parent?.kind === "element" ? parent.id : null,
				path: row.physicalPath,
				treeScope: row.treeScope,
				scopeDepth: routing ? routing.scopeDepth(row.treeScope) : profile.scopeDepth(row.treeScope, snapshot),
				implicitScopeRoot: snapshot.documentId,
				pseudo: null,
				inline: row.inlineUnits,
				hostOf: null,
				assignedSlot: null,
                ...(routing ? routing.targetFacts(row.id) : {}),
				context: profile.targetContext(row, {
					root: row.parent === snapshot.documentId,
					pseudo: null,
					snapshot,
				}),
			};
		},
	);
	if (routing) targets = parentFirstTargets(targets, routing);
	let resultId = snapshot.nodeId(element);
	if (pseudo) {
		const subject = sharedApply(
			"arrayFind",
			targets,
			(row) => row.id === resultId,
		);
		if (!subject) throw new Error("CSS original pseudo subject missing");
		const id = nextPseudoId(snapshot.records);
		const name = sharedApply("stringStartsWith", pseudo, "::")
			? sharedApply("stringSlice", pseudo, 2)
			: pseudo;
		sharedApply("arrayPush", targets, {
			...subject,
			id,
			parentId: resultId,
			pseudo: name,
			inline: null,
			context: profile.targetContext(sharedApply("mapGet", byId, resultId), {
				root: false,
				pseudo: name,
				snapshot,
			}),
		});
		resultId = id;
	}
	return { targets, resultId };
}
/** Explicit caller environment, limits and trusted snapshot; no runtime defaults. */
export function createSharedCssAdapter({
	selectBackend,
	readContext,
	captureSnapshot,
	captureSelectorFacts,
	profile,
	limits,
	measureScene,
}) {
	if (captureSelectorFacts !== undefined) required(captureSelectorFacts, 'captureSelectorFacts');
	const callbacks = {
		selectBackend,
		readContext,
		captureSnapshot,
		measureScene,
	};
	const names = sharedKeys(callbacks);
	for (let index = 0; index < names.length; index++)
		required(callbacks[names[index]], names[index]);
	requireMethods(
		limits,
		[
			"compile",
			"prepare",
			"matching",
			"resolve",
			"serialize",
			"property",
			"providerSnapshot",
		],
		"limits",
	);
	requireMethods(
		profile,
		["environment", "targetContext", "scopeDepth", "world", "backendIdentity"],
		"profile",
	);
	let active = null;
	let selectedBackend = null;
	const factory = createSharedComputedStyle({
		encodeRequest: sharedEncode,
		encodeResultKey: sharedEncode,
		decodeResponse: decode,
		selectBackend() {
			selectedBackend = selectBackend();
			return selectedBackend;
		},
		readContext,
		captureSources(document, context) {
			const snapshot = captureSnapshot(document, context);
            const navigator = captureSelectorFacts === undefined ? undefined : sharedInvoke(captureSelectorFacts, undefined, [snapshot, context]);
            if (captureSelectorFacts !== undefined) readCssFactsNavigator(navigator);
            if (snapshot.roots.length > 1 && navigator === undefined) throw new TypeError('turbo-dom: shared adapter shadow matching context unsupported');
            const contextual = navigator !== undefined
                ? sharedInvoke(required(limits.contextualMatching, 'limits.contextualMatching'), limits, [{snapshot}])
                : null;
            const routing = contextual ? createSharedRouting(snapshot, contextual.routing) : null;
			const sources = captureCssSources(document, {
				nodeId: snapshot.nodeId,
				scopeId: snapshot.nodeId,
			});
			const backend = selectedBackend;
			const userAgentSource = profile.userAgentSource;
			const ua =
				userAgentSource === undefined
					? decode(
							sharedInvoke(
								required(backend.cssUserAgentSourceV1, "cssUserAgentSourceV1"),
								backend,
								[],
							),
						)
					: sharedInvoke(
							required(userAgentSource, "profile.userAgentSource"),
							profile,
							[backend],
						);
			if (ua.abiVersion !== 1) throw new Error("CSS user agent ABI");
			active = { snapshot, ua, backend, context, contextual, routing, navigator };
			return sources;
		},
		compileRequest(sources) {
			const ua = active.ua;
			const uaSources = ua.documentText
				? [
						{ id: 0, text: ua.text },
						{ id: 1, text: ua.documentText },
					]
				: [{ id: 0, text: ua.text }];
			const all = sharedAppend(
				uaSources,
				sharedApply("arrayMap", sources.sources, (source) => ({
					id: source.id + 2,
					text: units(source.text),
				})),
			);
			return { abiVersion: 1, sources: all, limits: limits.compile(all) };
		},
		prepareRequest(sources, context) {
			const rows = uses(sources, active.snapshot, active.ua, profile, active.routing);
			return {
				abiVersion: 1,
				uses: rows,
				environment: profile.environment(context),
				limits: limits.prepare({ sources, uses: rows, ua: active.ua }),
			};
		},
		createFacts(metadata) { return { metadata, ordinary: captureSelectorFacts === undefined ? compileSharedPrograms(metadata) : null, contextual: null }; },
		resolveRequest({ element, pseudo, context, metadata, prepared, facts }) {
			const snapshot = active.snapshot;
            let mechanical;
            if (active.contextual) {
                facts.contextual ??= compileSharedShadowFacts(facts.metadata, active.navigator !== undefined);
                mechanical = facts.contextual.collect(snapshot, context.document, active.contextual, active.routing, active.navigator);
            } else mechanical = collectSharedAnswers(facts.ordinary, snapshot, context.document, limits.matching);
			const selected = targets(snapshot, pseudo, element, profile, active.routing);
			const request = {
				abiVersion: 1,
				targets: selected.targets,
				contextRoots: active.routing?.contextRoots ?? sharedApply("arrayMap", snapshot.roots, (root) => ({
					treeScope: root.id,
					root: root.id,
					host: root.host,
					parentTreeScope: root.parentScope,
				})),
				...mechanical,
				limits: limits.resolve({
					snapshot,
					targets: selected.targets,
					mechanical,
				}),
			};
			const world = profile.world(context, snapshot);
			const provider = sharedProviderSnapshot(
				snapshot,
				limits.providerSnapshot(snapshot),
			);
			const resolved = resolveSharedContainers({
				prepared,
				request,
				metadata,
				snapshot: provider,
				measureScene,
				world,
				environment: profile.environment(context),
				limits,
			});
			const subject = sharedApply(
				"arrayFind",
				snapshot.records,
				(row) => row.id === snapshot.nodeId(element),
			);
			if (!subject) throw new Error("CSS original subject missing");
			return {
				resultId: selected.resultId,
				request: resolved.request,
				providerContext: {
					snapshot: provider,
					namespace: subject.namespaceUnits,
					pseudo: pseudo || null,
					world,
					backendIdentity: profile.backendIdentity(active.backend),
				},
			};
		},
		finalizeResult(row, { backend }) {
			return publicRow(row, backend, profile, limits);
		},
		resolveProperty(name, base, { row, batch, providerContext, context }) {
			const backend = selectBackend();
			if (profile.backendIdentity(backend) !== providerContext.backendIdentity)
				throw new Error("CSS property backend changed");
			const target = {
				id: row.id,
				world: units(providerContext.world),
				namespace: providerContext.namespace,
				pseudo: providerContext.pseudo,
				ordinary: sharedEntries(row.rendererValues),
				publicKeys: row.publicKeys,
			};
			const probe = (measurement) => {
				const request = {
					abiVersion: 1,
					target,
					property: name,
					measurement,
					limits: limits.property({
						target,
						property: name,
						measurement,
						context,
					}),
				};
				return identities(
					call(backend, "projectCssomPropertyV1", request),
					row.id,
					name,
				);
			};
			let result = probe(null);
			if (result.status === "needsMeasurement") {
				const dimension = axisDimension(result.axis);
				const rows = measureScene({
					snapshot: providerContext.snapshot,
					rows: batch.targets,
					requested: [row.id],
					world: providerContext.world,
				});
				const measured = sharedApply(
					"arrayFind",
					rows,
					(value) => value.id === row.id,
				)?.result;
				const measurement = {
					target: row.id,
					world: target.world,
					horizontal: { status: "unknown" },
					vertical: { status: "unknown" },
				};
				measurement[result.axis] = metric(measured, dimension);
				result = probe(measurement);
			}
			if (result.status === "unchanged") return base;
			if (result.status !== "value")
				throw new Error(
					"MissingCanonicalUsedMeasurement:" + sharedEncode(result),
				);
			return sharedText(result.value);
		},
	});
	return {
		get(element, pseudo) {
			try {
				return factory.get(element, pseudo ?? null);
			} finally {
				active = null;
				selectedBackend = null;
			}
		},
		reset(document) {
			active = null;
			selectedBackend = null;
			factory.reset(document);
		},
		dispose() {
			active = null;
			selectedBackend = null;
			factory.dispose();
		},
	};
}

function requireMethods(owner, names, label) {
	for (let index = 0; index < names.length; index++)
		required(owner[names[index]], `${label}.${names[index]}`);
}
function nextPseudoId(records) {
	let maximum = 0;
	for (let index = 0; index < records.length; index++)
		if (records[index].id > maximum) maximum = records[index].id;
	return maximum + 1;
}

function shadowUserAgentUses(snapshot, routing) {
    const rows = [];
    for (let index=0; index<snapshot.roots.length; index++) {
        const root=snapshot.roots[index];
        if(root.host===null)continue;
        sharedApply('arrayPush', rows, {sourceId:0,origin:'userAgent',treeScope:root.id,scopeDepth:routing.scopeDepth(root.id),disabled:false,media:null,implicitScopeRoot:root.id});
    }
    return rows;
}

function parentFirstTargets(targets, routing) {
    const rows = new SharedMap(sharedApply('arrayMap', targets, (row)=>[row.id,row]));
    return sharedApply('arrayMap', routing.targetOrder, (id)=>sharedApply('mapGet',rows,id));
}
