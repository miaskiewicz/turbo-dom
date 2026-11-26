// One synchronous, transient scene produces mechanical matching facts. Immutable
// program records retain CSS only; no node/owner/scene index survives collect().
import {
	compileCssSelectorProgram,
	compileCssFactsProgram,
  checkCssFactsDependencies,
	matchesCssProgram,
	matchesCssFacts,
} from "./selectors.mjs";
import { readCssFactsNavigator } from "./css-facts-navigator.mjs";

export class CssMatchFactsLimitError extends Error {
	constructor(limit) {
		super(`turbo-dom: CSS match facts ${limit} limit exceeded`);
		this.name = "CssMatchFactsLimitError";
		this.limit = limit;
	}
}
export class CssMatchFactsCapabilityError extends Error {
	constructor(message) {
		super(`turbo-dom: CSS match facts ${message}`);
		this.name = "CssMatchFactsCapabilityError";
	}
}
const ownDescriptor = Object.getOwnPropertyDescriptor;
const isArray = Array.isArray;
const isInteger = Number.isSafeInteger;

function array(value, label) {
	if (!isArray(value)) throw new TypeError(`invalid CSS match facts ${label}`);
	return value;
}
function id(value, label) {
	if (!isInteger(value) || value < 0)
		throw new TypeError(`invalid CSS match facts ${label}`);
	return value;
}
function ownData(object, name) {
	const descriptor = ownDescriptor(object, name);
	if (!descriptor || !("value" in descriptor))
		throw new CssMatchFactsCapabilityError(`unknown ${name}`);
	return descriptor.value;
}
function budget(limits) {
	for (const name of ["maxNodes", "maxAnswers", "maxAncestorDepth"])
		id(limits?.[name], name);
	return {
		limits,
		answers: 0,
		domainNodes: limits.maxNodes,
		domainAnswers: limits.maxAnswers,
	};
}
function chargeAnswer(state) {
	if (state.answers >= state.limits.maxAnswers)
		throw new CssMatchFactsLimitError("answers");
	state.answers++;
}
function chargeDomain(state, length) {
	// Domain reference storage is bounded independently of predicate work, using
	// the caller's node allowance plus answer allowance (no hidden/default quota).
	const fromNodes = Math.min(length, state.domainNodes);
	const remainder = length - fromNodes;
	if (remainder > state.domainAnswers)
		throw new CssMatchFactsLimitError("answers");
	state.domainNodes -= fromNodes;
	state.domainAnswers -= remainder;
}
function asciiType(value) {
	return value.replace(/[A-Z]/g, (letter) => letter.toLowerCase());
}
function addBucket(buckets, key, node) {
	const bucket = buckets.get(key);
	if (bucket) bucket.push(node);
	else buckets.set(key, [node]);
}
function seed(value) {
	if (value == null) return null;
	if (
		!["id", "class", "type"].includes(value.kind) ||
		typeof value.value !== "string" ||
		!value.value
	)
		throw new TypeError("invalid CSS subject seed");
	if (
		value.kind === "type" &&
		(value.value === "*" || /[^\x00-\x7f]/.test(value.value))
	)
		throw new TypeError("invalid CSS type seed");
	return {
		kind: value.kind,
		value: value.kind === "type" ? asciiType(value.value) : value.value,
	};
}
function compileItem(item, programs, compiler = compileCssSelectorProgram) {
	if (item == null || item.units == null)
		throw new CssMatchFactsCapabilityError("uncompiled selector");
	const key = id(item.id, "program id");
	if (programs.has(key))
		throw new TypeError("duplicate CSS selector program id");
	const compiled = {
		id: key,
		seed: seed(item.seed),
		program: compiler(item.units, {
			implicitScopeAnchor: item.implicitScopeAnchor,
			pseudo: item.pseudo,
			allowScopeReference: item.allowScopeReference,
			shadow: item.shadow,
		}),
	};
	programs.set(key, compiled);
	return compiled;
}
function compileBoundary(items, programs, compiler) {
	if (items === undefined) return undefined; // Omitted syntax is not an empty compiled list.
	if (items === null)
		throw new CssMatchFactsCapabilityError("uncompiled scope boundary");
	return array(items, "scope boundary").map((item) =>
		compileItem(item, programs, compiler),
	);
}
function compileGroups(metadata, programs, compiler) {
	const groups = new Map();
	for (const item of array(metadata.scopeGroups, "scope groups")) {
		const key = id(item.id, "group id");
		if (groups.has(key)) throw new TypeError("duplicate CSS scope group id");
		groups.set(key, {
			start: compileBoundary(item.start, programs, compiler),
			end: compileBoundary(item.end, programs, compiler),
		});
	}
	return groups;
}

/// Compile each flat PROGRAM id once. The caller maps program ids to canonical
/// Core list/item ids and supplies scope-start/end arrays; this module ranks none.
export function compileCssMatchFacts(metadata) {
	return compileMetadata(metadata, compileCssSelectorProgram);
}
export function compileCssPrimitiveMatchFacts(metadata) {
  checkCssFactsDependencies();
	return compileMetadata(metadata, compileCssFactsProgram);
}
function compileMetadata(metadata, compiler) {
	if (metadata == null)
		throw new CssMatchFactsCapabilityError("uncompiled metadata");
	const programs = new Map();
	for (const item of array(metadata.selectors, "selectors"))
		compileItem(item, programs, compiler);
	const groups = compileGroups(metadata, programs, compiler);
	return {
		collect(nodes, request, limits) {
			if (compiler === compileCssFactsProgram) checkCssFactsDependencies();
      return collectFacts(programs, groups, nodes, request, limits);
		},
	};
}

function sceneOwner(element, navigator) { return navigator ? navigator.property(element, 'ownerDocument') : ownData(element, 'ownerDocument'); }
function sceneType(element, navigator) { return navigator ? navigator.property(element, 'nodeType') : element.nodeType; }
function requireElement(element, navigator) {
  if (!navigator && (!element || typeof element !== 'object')) throw new CssMatchFactsCapabilityError('unknown element');
}
function requireOwnerDocument(owner, navigator) {
  if (navigator ? owner !== navigator.documentId : (!owner || typeof owner !== 'object')) throw new CssMatchFactsCapabilityError('owner document is required');
}
function sceneNode(record, ownerDocument, byElement, navigator) {
	const key = id(record.id, "node id");
	const parent = record.parent === null ? null : id(record.parent, "parent id");
	const element = navigator ? key : record.element;
  requireElement(element, navigator);
  if (sceneOwner(element, navigator) !== ownerDocument) throw new TypeError('foreign CSS element owner');
  if (sceneType(element, navigator) !== 1) throw new TypeError('CSS scene node is not an element');
	if (byElement.has(element))
		throw new TypeError("duplicate CSS scene element");
	byElement.set(element, key);
	return { id: key, parent, element };
}
function attribute(element, name, navigator) {
	const value = navigator
		? navigator.attribute(element, name)
		: element.getAttribute(name);
	if (value !== null && typeof value !== "string")
		throw new CssMatchFactsCapabilityError("unknown attribute value");
	return value;
}
function indexNode(scene, node) {
	const identifier = attribute(node.element, "id", scene.navigator);
	if (identifier) addBucket(scene.ids, identifier, node);
	const classes = attribute(node.element, "class", scene.navigator);
	if (classes)
		for (const name of new Set(classes.split(/[\t\n\f\r ]+/)))
			if (name) addBucket(scene.classes, name, node);
	const name = scene.navigator
		? scene.navigator.property(node.element, "localName")
		: node.element.localName;
	if (typeof name !== "string")
		throw new CssMatchFactsCapabilityError("unknown local name");
	if (/[^\x00-\x7f]/.test(name)) scene.uncertainTypes.push(node);
	else addBucket(scene.types, asciiType(name), node);
}
function knownNode(scene, key) {
	const node = scene.nodes.get(id(key, "node id"));
	if (!node) throw new TypeError("unknown CSS scene node id");
	return node;
}
function nodeIds(scene, values, label) {
	const input = array(values, label);
	if (input.length > scene.nodes.size)
		throw new CssMatchFactsLimitError("nodes");
	const seen = new Set();
	for (const key of input) {
		knownNode(scene, key);
		if (seen.has(key))
			throw new TypeError(`duplicate CSS match facts ${label}`);
		seen.add(key);
	}
	return seen;
}
function validateEdges(scene, roots) {
	for (const node of scene.nodes.values()) {
		const parent = scene.navigator
			? scene.navigator.property(node.element, "parentNode")
			: ownData(node.element, "parentNode");
		if (node.parent !== null) {
			if (parent !== knownNode(scene, node.parent).element)
				throw new CssMatchFactsCapabilityError("scene parent mismatch");
		} else if (
			parent !== null &&
			parent !== scene.ownerDocument &&
			!roots.has(node.id)
		) {
			throw new CssMatchFactsCapabilityError("incomplete ancestor boundary");
		}
	}
}
function recordDepth(scene, node, depths, maximum) {
	const path = [],
		seen = new Set();
	let current = node;
	while (current && !depths.has(current.id)) {
		if (seen.has(current.id)) throw new TypeError("cyclic CSS scene parents");
		if (path.length > maximum)
			throw new CssMatchFactsLimitError("ancestorDepth");
		seen.add(current.id);
		path.push(current.id);
		current = current.parent === null ? null : knownNode(scene, current.parent);
	}
	let depth = current ? depths.get(current.id) : -1;
	while (path.length) {
		depth++;
		if (depth > maximum) throw new CssMatchFactsLimitError("ancestorDepth");
		depths.set(path.pop(), depth);
	}
}
function contextNodes(request, navigator) {
	const result = new Map();
	for (const row of request.scopeNodes ?? []) {
		const key = id(row.id, "context node id");
		if (result.has(key)) throw new TypeError("duplicate CSS context node");
		const element = navigator ? key : row.element;
    if (element !== request.ownerDocument && sceneOwner(element, navigator) !== request.ownerDocument) throw new TypeError('foreign CSS context owner');
    const type = sceneType(element, navigator);
		if (type !== 9 && type !== 11)
			throw new TypeError("invalid CSS context node");
		result.set(key, { id: key, element });
	}
	return result;
}
function scopeNode(scene, key) {
	return (
		scene.contextNodes?.get(id(key, "context node id")) ?? knownNode(scene, key)
	);
}
function matchingContext(scene, request, root) {
	const context = {
		scopeRoot: root === null ? null : scopeNode(scene, root).element,
		liveState: request.liveState,
	};
	if (request.cssContext) context.shadowRoot = request.cssContext.shadowRoot;
	return context;
}
function slotContext(scene, request, node, context) {
	const row = request.cssContext?.slots?.get(node.id);
	if (!row) return context;
	return {
		...context,
		assignedSlot: knownNode(scene, row.slot).element,
		assignedSlotRoot: scopeNode(scene, row.treeScope).element,
	};
}
function makeScene(nodes, request, state) {
	const input = array(nodes, "nodes");
	if (input.length > state.limits.maxNodes)
		throw new CssMatchFactsLimitError("nodes");
	const navigator =
		request.navigator === undefined
			? null
			: readCssFactsNavigator(request.navigator);
	const ownerDocument = request.ownerDocument;
  requireOwnerDocument(ownerDocument, navigator);
	const scene = {
		ownerDocument,
		navigator,
		contextNodes: request.scopeNodes ? contextNodes(request, navigator) : null,
		nodes: new Map(),
		ids: new Map(),
		classes: new Map(),
		types: new Map(),
		uncertainTypes: [],
	};
	const byElement = navigator ? new Map() : new WeakMap();
	for (const record of input) {
		const node = sceneNode(record, ownerDocument, byElement, navigator);
		if (scene.nodes.has(node.id))
			throw new TypeError("duplicate CSS scene node id");
		scene.nodes.set(node.id, node);
	}
	validateContextIds(scene, state.limits.maxNodes - input.length);
	const roots = nodeIds(scene, request.sheetRootIds, "sheet roots");
	validateEdges(scene, roots);
	const depths = new Map();
	for (const node of scene.nodes.values())
		recordDepth(scene, node, depths, state.limits.maxAncestorDepth);
	for (const node of scene.nodes.values()) indexNode(scene, node);
	return scene;
}

function domainStore(scene, state) {
	const cache = new WeakMap(),
		domains = [];
	return {
		domains,
		get(input) {
			const cached = input && cache.get(input);
			if (cached) return cached;
			const values = array(input, "domain ids");
			if (values.length > scene.nodes.size)
				throw new CssMatchFactsLimitError("nodes");
			chargeDomain(state, values.length);
			const members = nodeIds(scene, values, "domain ids");
			const ids = Array.from(members);
			const domain = {
				id: domains.length,
				nodeIds: ids,
				members,
				records: ids.map((key) => scene.nodes.get(key)),
			};
			domains.push({ id: domain.id, nodeIds: ids });
			cache.set(input, domain);
			return domain;
		},
	};
}
function candidates(scene, selector, domain) {
	if (!selector.seed) return domain.records;
	const buckets = { id: scene.ids, class: scene.classes, type: scene.types };
	const nodes = buckets[selector.seed.kind].get(selector.seed.value) || [];
	// Non-ASCII local-name matching is unproven across adapter/native case modes.
	return selector.seed.kind === "type"
		? nodes.concat(scene.uncertainTypes)
		: nodes;
}
function answers(programs, scene, domain, root, request, state) {
	const matched = new Set();
	const context = matchingContext(scene, request, root);
	for (const program of programs) {
		for (const node of candidates(scene, program, domain)) {
			if (!domain.members.has(node.id) || matched.has(node.id)) continue;
			chargeAnswer(state);
			const contextual = slotContext(scene, request, node, context);
			const matches = scene.navigator
				? matchesCssFacts(
						node.id,
						program.program,
						contextual,
						request.navigator,
					)
				: matchesCssProgram(node.element, program.program, contextual);
			if (matches) matched.add(node.id);
		}
	}
	return Array.from(matched);
}
function rootIds(scene, values) {
	const input = array(values, "scope root ids");
	if (input.length > scene.nodes.size + (scene.contextNodes?.size ?? 0) + 1)
		throw new CssMatchFactsLimitError("nodes");
	const roots = new Set();
	for (const key of input) {
		if (key !== null) scopeNode(scene, key);
		if (roots.has(key)) throw new TypeError("duplicate CSS scope root id");
		roots.add(key);
	}
	return roots;
}
function uniqueRow(seen, key) {
	if (seen.has(key))
		throw new TypeError("duplicate CSS matching answer domain");
	seen.add(key);
}
function ordinaryRows(programs, scene, request, domains, state, output, seen) {
	for (const query of request.selectorQueries) {
		const selectorId = id(query.selectorId, "program id");
		const program = programs.get(selectorId);
		if (!program) throw new TypeError("unknown CSS selector program id");
		const domain = domains.get(request.targetIds);
		for (const scopeRootId of rootIds(scene, query.scopeRootIds)) {
			chargeAnswer(state);
			uniqueRow(seen, `s:${selectorId}:${scopeRootId}:${domain.id}`);
			output.push({
				selectorId,
				scopeRootId,
				domainId: domain.id,
				matches: answers([program], scene, domain, scopeRootId, request, state),
			});
		}
	}
}
function groupPrograms(groups, query) {
	const group = groups.get(id(query.groupId, "group id"));
	if (!group) throw new TypeError("unknown CSS scope group id");
	if (query.boundary !== "start" && query.boundary !== "end")
		throw new TypeError("invalid CSS scope boundary");
	const programs = group[query.boundary];
	if (programs === undefined)
		throw new CssMatchFactsCapabilityError("scope boundary is absent");
	return programs;
}
function scopeRows(groups, scene, request, domains, state, output, seen) {
	for (const query of request.scopeQueries) {
		const programs = groupPrograms(groups, query);
		const domain = domains.get(query.candidateIds);
		for (const enclosingRootId of rootIds(scene, query.enclosingRootIds)) {
			chargeAnswer(state);
			uniqueRow(
				seen,
				`g:${query.groupId}:${query.boundary}:${enclosingRootId}:${domain.id}`,
			);
			output.push({
				groupId: query.groupId,
				boundary: query.boundary,
				enclosingRootId,
				domainId: domain.id,
				matches: answers(
					programs,
					scene,
					domain,
					enclosingRootId,
					request,
					state,
				),
			});
		}
	}
}
function collectFacts(programs, groups, nodes, request, limits) {
	const state = budget(limits);
	const selectors = array(request.selectorQueries, "selector queries");
	const scopes = array(request.scopeQueries, "scope queries");
	if (selectors.length > limits.maxAnswers - scopes.length)
		throw new CssMatchFactsLimitError("answers");
	const scene = makeScene(nodes, request, state);
	nodeIds(scene, request.targetIds, "target ids"); // Even unused requested ids must be known.
	const domains = domainStore(scene, state);
	const selectorAnswers = [],
		scopeAnswers = [],
		seen = new Set();
	ordinaryRows(programs, scene, request, domains, state, selectorAnswers, seen);
	scopeRows(groups, scene, request, domains, state, scopeAnswers, seen);
	const result = { domains: domains.domains, selectorAnswers, scopeAnswers };
	if (request.collectUsage) result.used = { answers: state.answers };
	return result;
}

function validateContextIds(scene, remaining) {
	if (scene.contextNodes === null) return;
	if (scene.contextNodes.size > remaining)
		throw new CssMatchFactsLimitError("nodes");
	for (const key of scene.contextNodes.keys())
		if (scene.nodes.has(key))
			throw new TypeError("duplicate CSS context identity");
}
