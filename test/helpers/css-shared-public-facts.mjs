// TEST CONTRACT ONLY: genuine public DOM nodes and mechanical flat facts.
// This is not production capture, a native-brand/security proof, a second DOM,
// a CSS parser, or a claim about unobservable closed/composed/shadow semantics.
const units = (value) =>
	Array.from({ length: value.length }, (_, index) => value.charCodeAt(index));
const limitNames = [
	"maxNodes",
	"maxDepth",
	"maxPathUnits",
	"maxTextUnits",
	"maxAttributes",
	"maxAttributeUnits",
];
function charge(state, name, amount) {
	state.used[name] += amount;
	if (state.used[name] > state.limits[name])
		throw new RangeError("test public CSS facts " + name);
}
function id(state, node) {
	let value = state.allocator.ids.get(node);
	if (value === undefined) {
		value = state.allocator.next++;
		state.allocator.ids.set(node, value);
	}
	return value;
}
function attributes(state, node) {
	const output = [],
		list = node.attributes;
	for (let index = 0; index < list.length; index++) {
		const attr = list.item(index);
		charge(state, "maxAttributes", 1);
		charge(state, "maxAttributeUnits", attr.name.length + attr.value.length);
		output.push({
			name: attr.name,
			namespace: attr.namespaceURI,
			prefix: attr.prefix,
			value: attr.value,
			nameUnits: units(attr.name),
			valueUnits: units(attr.value),
		});
	}
	return output;
}
function payload(state, node, record) {
	if (record.kind === "element") {
		record.tag = node.localName;
		record.tagUnits = units(record.tag);
		record.namespace = node.namespaceURI;
		record.namespaceUnits =
			record.namespace === null ? null : units(record.namespace);
		charge(
			state,
			"maxAttributeUnits",
			record.tag.length + (record.namespace?.length || 0),
		);
		record.attrs = attributes(state, node);
		record.inline = node.getAttribute("style");
		record.inlineUnits = record.inline === null ? null : units(record.inline);
	} else if (record.kind === "text" || record.kind === "comment") {
		record.textUnits = units(node.data);
		charge(state, "maxTextUnits", record.textUnits.length);
	}
}
function kind(node) {
	return { 1: "element", 3: "text", 8: "comment", 9: "document", 11: "shadow" }[
		node.nodeType
	];
}
function record(state, node, parent, scope, path) {
	const nodeKind = kind(node);
	if (!nodeKind) return null;
	const ownId = id(state, node),
		physicalPath = [ownId, ...path];
	if (physicalPath.length > state.limits.maxDepth)
		throw new RangeError("test public CSS facts maxDepth");
	charge(state, "maxNodes", 1);
	charge(state, "maxPathUnits", physicalPath.length + 1);
	const result = {
		id: ownId,
		kind: nodeKind,
		element: node,
		parent: parent?.id ?? null,
		physicalPath,
		treeScope: scope,
		contextRoots: [scope],
		hostOf: null,
		assignedSlot: null,
		assignedSlotPath: [],
	};
	payload(state, node, result);
	state.records.push(result);
	state.byNode.set(node, result);
	return result;
}
function children(state, node, parent, scope, path) {
	const list = node.childNodes;
	for (let index = 0; index < list.length; index++)
		visit(state, list.item(index), parent, scope, path);
}
function visit(state, node, parent, scope, path) {
	const own = record(state, node, parent, scope, path);
	if (!own) return;
	children(state, node, own, scope, own.physicalPath);
	if (own.kind === "element") {
    const root = state.closedRoots.get(node) ?? node.shadowRoot;
    if (root) shadow(state, root, own, scope);
  }
}
function shadow(state, node, host, parentScope) {
	const scope = id(state, node),
		own = record(state, node, null, scope, []);
	own.hostOf = host.id;
	host.shadowRootId = scope;
	state.roots.push({ id: scope, host: host.id, parentScope, element: node });
	children(state, node, own, scope, own.physicalPath);
}
export function createTestPublicCssFacts({ closedRoots = [] } = {}) {
  const ownedRoots = new Map();
  for (const root of closedRoots) {
    if (!root.host) throw new TypeError("test owned root has no host");
    ownedRoots.set(root.host, root);
  }
	const documents = new WeakMap();
	return {
		capture(document, limits) {
			for (const name of limitNames)
				if (!Number.isSafeInteger(limits[name]) || limits[name] < 0)
					throw new TypeError("explicit test public CSS facts limit " + name);
			let allocator = documents.get(document);
			if (!allocator) {
				allocator = { ids: new WeakMap(), next: 1 };
				documents.set(document, allocator);
			}
			for (const root of ownedRoots.values()) if (root.ownerDocument !== document) throw new TypeError("foreign test owned root");
			const state = {
        closedRoots: ownedRoots,
				allocator,
				records: [],
				roots: [],
				byNode: new Map(),
				limits,
				used: Object.fromEntries(limitNames.map((name) => [name, 0])),
			};
			const documentId = id(state, document),
				root = record(state, document, null, documentId, []);
			state.roots.push({
				id: documentId,
				host: null,
				parentScope: null,
				element: document,
			});
			children(state, document, root, documentId, root.physicalPath);
      const slotAssignments = [];
      for (const row of state.records) {
        if (row.kind !== 'element' || row.tag !== 'slot') continue;
        const assigned = [];
        for (const node of row.element.assignedNodes()) {
          const entry = state.byNode.get(node);
          if (!entry) throw new TypeError('foreign test slot assignment');
          entry.assignedSlot = row.id;
          entry.assignedSlotPath = row.physicalPath.slice();
          assigned.push(entry.id);
        }
        slotAssignments.push({slot:row.id, complete:true, assigned});
      }
			const byId = new Map(state.records.map((row) => [row.id, row]));
			return {
				records: state.records,
				roots: state.roots,
				documentId,
        slotAssignments,
				used: state.used,
				matchNodes: state.records
					.filter((row) => row.kind === "element")
					.map((row) => ({
						id: row.id,
						parent:
							byId.get(row.parent)?.kind === "element" ? row.parent : null,
						element: row.element,
					})),
				nodeId(node) {
					const value = state.byNode.get(node);
					if (!value)
						throw new Error("test public CSS facts node outside snapshot");
					return value.id;
				},
			};
		},
	};
}
