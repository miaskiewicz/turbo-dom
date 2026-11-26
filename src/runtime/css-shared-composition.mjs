// Mechanical composition of an already-admitted snapshot. This confers no DOM,
// Proxy, ownership or world authority; the trusted producer owns those proofs.
import {
	SharedMap,
	sharedApply,
	sharedDescriptor,
} from "./css-shared-codec.mjs";
const safe = Number.isSafeInteger;
const HTML = "http://www.w3.org/1999/xhtml";
export class SharedCompositionUnavailable extends Error {
	constructor(reason) {
		super(`CSS composed Scene unavailable: ${reason}`);
		this.name = "SharedCompositionUnavailable";
	}
}
function refuse(reason) {
	throw new SharedCompositionUnavailable(reason);
}
function get(map, key) {
	return sharedApply("mapGet", map, key);
}
function has(map, key) {
	return sharedApply("mapHas", map, key);
}
function set(map, key, value) {
	sharedApply("mapSet", map, key, value);
}
function push(array, value) {
	sharedApply("arrayPush", array, value);
}
function data(object, key, optional = false) {
	const descriptor = sharedDescriptor(object, key);
	if (!descriptor) {
		if (optional) return null;
		refuse(`missing ${key}`);
	}
	if (!("value" in descriptor)) refuse(`accessor ${key}`);
	return descriptor.value;
}
function id(value) {
	if (!safe(value) || value < 0 || value > 0xffffffff)
		refuse("invalid source ID");
	return value;
}
function charge(state, name, count) {
	if (!safe(count) || count < 0 || count > state[name]) refuse(`${name} limit`);
	state[name] -= count;
}
function policy(limits) {
	const state = {},
		names = ["maxRoots", "maxNodes", "maxDepth", "maxEdges", "maxPathUnits"];
	for (let index = 0; index < names.length; index++) {
		const name = names[index],
			value = data(limits, name);
		if (!safe(value) || value < 0) refuse(`invalid ${name}`);
		state[name] = value;
	}
	return state;
}

function item(array, index) {
	return data(array, index);
}
function rows(array) {
	const length = data(array, "length");
	if (!safe(length) || length < 0) refuse("invalid list");
	return length;
}
function append(map, key, value) {
	let array = get(map, key);
	if (!array) {
		array = [];
		set(map, key, array);
	}
	push(array, value);
}
function loadRoots(snapshot, state) {
	const source = data(snapshot, "roots"),
		map = new SharedMap(),
		list = [];
	charge(state, "maxRoots", rows(source));
	for (let index = 0; index < source.length; index++) {
		const row = item(source, index),
			identity = id(data(row, "id"));
		if (has(map, identity)) refuse("duplicate root");
		const root = {
			id: identity,
			host: data(row, "host"),
			parentScope: data(row, "parentScope"),
		};
		if (root.host !== null) id(root.host);
		if (root.parentScope !== null) {
			id(root.parentScope);
			if (!has(map, root.parentScope))
				refuse("cyclic or foreign root ancestry");
		}
		set(map, identity, root);
		push(list, root);
	}
	return { map, list };
}
function physicalParent(node, nodes, roots, children, budget) {
	if (node.parent === null) {
		if (!has(roots.map, node.id) || node.scope !== node.id)
			refuse("foreign physical root");
		return;
	}
	id(node.parent);
	const parent = get(nodes, node.parent);
	if (!parent || parent.scope !== node.scope)
		refuse("nonpreceding or foreign physical parent");
	if (
		parent.kind !== "element" &&
		parent.kind !== "document" &&
		parent.kind !== "shadow"
	)
		refuse("invalid physical parent kind");
	charge(budget, "maxEdges", 1);
	append(children, node.parent, node.id);
}
function loadNodes(snapshot, roots, state) {
	const source = data(snapshot, "records"),
		nodes = new SharedMap(),
		children = new SharedMap(),
		list = [];
	charge(state, "maxNodes", rows(source));
	for (let index = 0; index < source.length; index++) {
		const row = item(source, index),
			node = {
				id: id(data(row, "id")),
				kind: data(row, "kind"),
				parent: data(row, "parent"),
				scope: id(data(row, "treeScope")),
				shadow: data(row, "shadowRootId", true),
				assigned: data(row, "assignedSlot", true),
				index,
			};
		if (has(nodes, node.id) || !has(roots.map, node.scope))
			refuse("duplicate or foreign node");
		physicalParent(node, nodes, roots, children, state);
		set(nodes, node.id, node);
		push(list, node);
	}
	return { source, nodes, children, list };
}
function slot(node, state) {
	if (node.kind !== "element") return false;
	const row = item(state.source, node.index);
	return (
		data(row, "tag") === "slot" &&
		data(row, "namespace") === HTML &&
		get(state.roots.map, node.scope).host !== null
	);
}
function rootRecord(root, state) {
	const node = get(state.nodes, root.id);
	if (!node || node.parent !== null || node.scope !== root.id)
		refuse("missing root record");
	return node;
}
function shadowOwner(root, node, state) {
	const host = get(state.nodes, root.host);
	if (
		!host ||
		host.kind !== "element" ||
		host.shadow !== root.id ||
		host.scope !== root.parentScope ||
		node.kind !== "shadow"
	)
		refuse("foreign shadow host");
}
function rootOwnership(state) {
	let document = null;
	for (let index = 0; index < state.roots.list.length; index++) {
		const root = state.roots.list[index],
			node = rootRecord(root, state);
		if (root.host !== null) {
			shadowOwner(root, node, state);
			continue;
		}
		if (
			document !== null ||
			root.parentScope !== null ||
			node.kind !== "document"
		)
			refuse("foreign document root");
		document = root.id;
	}
	if (document === null) refuse("missing document root");
	return document;
}

function assignmentHost(node, target, state) {
	const root = get(state.roots.map, target.scope);
	if (!slot(target, state) || node.parent !== root.host)
		refuse("foreign assigned slot");
}
function metadataSlot(row, bySlot, state) {
	const identity = id(data(row, "slot")),
		target = get(state.nodes, identity);
	if (!target || !slot(target, state) || has(bySlot, identity))
		refuse("foreign or duplicate slot metadata");
	if (data(row, "complete") !== true) refuse("incomplete slot assignment");
	return target;
}
function metadataNode(identity, target, assigned, state) {
	const node = get(state.nodes, identity);
	if (
		!node ||
		has(assigned, identity) ||
		(node.kind !== "element" && node.kind !== "text")
	)
		refuse("foreign or duplicate assigned node");
	assignmentHost(node, target, state);
	if (node.assigned !== null && node.assigned !== target.id)
		refuse("contradictory assigned slot");
	if (node.kind === "element" && node.assigned !== target.id)
		refuse("contradictory element assignment");
	return node;
}
function metadataEntries(row, target, assigned, state) {
	const entries = data(row, "assigned"),
		list = [];
	charge(state.budget, "maxEdges", rows(entries));
	for (let child = 0; child < entries.length; child++) {
		const identity = id(item(entries, child));
		metadataNode(identity, target, assigned, state);
		set(assigned, identity, target.id);
		push(list, identity);
	}
	return list;
}
function loadAssignments(snapshot, state) {
	const source = data(snapshot, "slotAssignments", true),
		bySlot = new SharedMap(),
		assigned = new SharedMap();
	if (source === null) return { bySlot, assigned };
	charge(state.budget, "maxEdges", rows(source));
	for (let index = 0; index < source.length; index++) {
		const row = item(source, index),
			target = metadataSlot(row, bySlot, state);
		set(bySlot, target.id, metadataEntries(row, target, assigned, state));
	}
	return { bySlot, assigned };
}

function inferredAssignments(target, state) {
	const root = get(state.roots.map, target.scope),
		light = get(state.children, root.host) || [],
		assigned = [];
	for (let index = 0; index < light.length; index++) {
		const node = get(state.nodes, light[index]);
		if (node.kind === "text" && node.assigned === null)
			refuse("uncertified text slot assignment");
		if (node.assigned === target.id) push(assigned, node.id);
	}
	if (assigned.length > 1) refuse("uncertified assigned-node order");
	return assigned;
}
function slotChildren(node, state) {
	if (has(state.assignments.bySlot, node.id))
		return get(state.assignments.bySlot, node.id);
	return inferredAssignments(node, state);
}
function physicalChildren(node, state) {
	return get(state.children, node.id) || [];
}
function assignedTo(node, state) {
	return get(state.assignments.assigned, node.id) ?? node.assigned;
}
function directShadow(node, state) {
	if (node.shadow === null) return null;
	const root = get(state.roots.map, node.shadow);
	if (!root || root.host !== node.id) refuse("foreign owned shadow root");
	return root.id;
}
function excludeChildren(node, stack, state) {
	const children = physicalChildren(node, state);
	for (let index = 0; index < children.length; index++)
		push(stack, children[index]);
	const shadow = directShadow(node, state);
	if (shadow !== null) push(stack, shadow);
	if (slot(node, state)) {
		const assigned = slotChildren(node, state);
		for (let index = 0; index < assigned.length; index++)
			push(stack, assigned[index]);
	}
}
function exclusion(start, reason, state) {
	const stack = [start];
	while (stack.length) {
		const identity = stack[stack.length - 1];
		stack.length--;
		const node = get(state.nodes, identity);
		if (has(state.excluded, identity)) continue;
		if (has(state.seen, identity)) refuse("overlapping excluded composition");
		set(state.excluded, identity, reason);
		if (node.kind === "element" || node.kind === "text")
			push(state.output.excluded, { sourceId: identity, reason });
		excludeChildren(node, stack, state);
	}
}

function composedChildren(node, state) {
	const physical = physicalChildren(node, state),
		shadow = directShadow(node, state);
	if (shadow !== null) {
		for (let index = 0; index < physical.length; index++)
			if (assignedTo(get(state.nodes, physical[index]), state) === null)
				exclusion(physical[index], "unassigned-light", state);
		return [shadow];
	}
	if (!slot(node, state)) return physical;
	const assigned = slotChildren(node, state);
	if (assigned.length) {
		for (let index = 0; index < physical.length; index++)
			exclusion(physical[index], "inactive-fallback", state);
		return assigned;
	}
	return physical;
}
function enteredNode(frame, state) {
	const node = get(state.nodes, frame.id);
	if (!node || has(state.excluded, node.id) || has(state.seen, node.id))
		refuse("cyclic or repeated composed node");
	set(state.seen, node.id, true);
	return node;
}
function participation(node) {
	switch (node.kind) {
		case "element":
		case "text":
			return "emitted";
		case "document":
		case "shadow":
			return "transparent";
		case "comment":
		case "doctype":
			return "skip";
		default:
			refuse("unsupported composed node kind");
	}
}
function enter(frame, state) {
	const node = enteredNode(frame, state),
		kind = participation(node);
	if (kind === "skip") return;
	const emitted = kind === "emitted",
		depth = frame.depth + (emitted ? 1 : 0);
	if (depth > state.budget.maxDepth) refuse("maxDepth limit");
	if (emitted) {
		charge(state.budget, "maxPathUnits", depth);
		push(state.output.records, {
			sourceId: node.id,
			parentId: frame.parent,
			recordIndex: node.index,
		});
	}
	const children = composedChildren(node, state),
		parent = emitted ? node.id : frame.parent;
	for (let index = children.length - 1; index >= 0; index--)
		push(state.stack, { id: children[index], parent, depth });
}
function assignedOwnership(state) {
	for (let index = 0; index < state.list.length; index++) {
		const node = state.list[index];
		if (node.assigned === null) continue;
		const target = get(state.nodes, id(node.assigned));
		if (!target) refuse("foreign assigned slot");
		assignmentHost(node, target, state);
		if (
			has(state.assignments.bySlot, target.id) &&
			get(state.assignments.assigned, node.id) !== target.id
		)
			refuse("incomplete certified slot list");
	}
}
function completed(state) {
	for (let index = 0; index < state.list.length; index++) {
		const node = state.list[index];
		if (!has(state.seen, node.id) && !has(state.excluded, node.id))
			refuse("unreachable composition source");
	}
}

/** Trusted primitive input only; metadata completeness is a producer capability,
 * never inferred from this structural validation. Original input errors propagate. */
export function projectComposedScene(snapshot, limits) {
	const budget = policy(limits),
		roots = loadRoots(snapshot, budget),
		loaded = loadNodes(snapshot, roots, budget);
	const state = {
		...loaded,
		roots,
		budget,
		seen: new SharedMap(),
		excluded: new SharedMap(),
		output: { records: [], excluded: [] },
		stack: [],
	};
	const document = rootOwnership(state);
	state.assignments = loadAssignments(snapshot, state);
	assignedOwnership(state);
	push(state.stack, { id: document, parent: null, depth: 0 });
	while (state.stack.length) {
		const frame = state.stack[state.stack.length - 1];
		state.stack.length--;
		enter(frame, state);
	}
	completed(state);
	return state.output;
}
/** Until a separately owned no-box policy exists, do not forge a native result
 * for a structurally excluded requested source. */
export function requireComposedTargets(projection, requested) {
	const available = new SharedMap();
	for (let index = 0; index < projection.records.length; index++)
		set(available, projection.records[index].sourceId, true);
	for (let index = 0; index < requested.length; index++)
		if (!has(available, id(item(requested, index))))
			refuse("requested source is excluded or unknown");
}
