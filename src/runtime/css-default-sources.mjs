// Mechanical source text/owner conditions from an admitted native snapshot.
// Rust owns all grammar, conditions, layers and cascade. No public DOM getter runs.
import * as cssom from "./cssom-state.mjs";
const descriptor = Object.getOwnPropertyDescriptor;
const own = Object.hasOwn;
const NativeMap = Map;
const apply = Reflect.apply;
const mapGet = Map.prototype.get;
const mapSet = Map.prototype.set;
export class CssNativeSourcesUnavailable extends Error {
	constructor(reason) {
		super(`turbo-dom: native CSS sources ${reason}`);
		this.name = "CssNativeSourcesUnavailable";
	}
}
function unavailable(reason) {
	throw new CssNativeSourcesUnavailable(reason);
}
function data(value, name) {
	const field = descriptor(value, name);
	if (!field) return undefined;
	if (!own(field, "value")) unavailable(`accessor ${name}`);
	return field.value;
}
function attr(record, name) {
	for (let i = 0; i < record.attrs.length; i++) {
		const row = record.attrs[i];
		if (row.name === name && row.prefix === "") return row.value;
	}
	return null;
}
function sheetData(state, sheet) {
	if (typeof cssom.readNativeCssSheetData !== "function")
		unavailable("CSSOM original reader unavailable");
	const result = cssom.readNativeCssSheetData(sheet, {
		maxEntries: state.maximumSources - state.sources.length,
		maxUnits: state.remaining,
	});
	const mediaUnits =
		typeof result.media === "string"
			? result.media.length
			: result.media.reduce((sum, text) => sum + text.length, 0);
	if (mediaUnits > state.remaining)
		throw new RangeError("CSS source metadata units exceed runtime policy");
	state.remaining -= mediaUnits;
	return result;
}
function addSource(state, text) {
	if (typeof text !== "string") unavailable("non-string rule");
	if (text.length > state.remaining)
		throw new RangeError("CSS source units exceed runtime policy");
	if (state.sources.length >= state.maximumSources)
		throw new RangeError("CSS source count exceeds runtime policy");
	state.remaining -= text.length;
	const id = state.sources.length;
	state.sources.push({ id, text });
	return id;
}
function use(state, sourceId, root, owner, facts, adopted) {
	state.uses.push({
		sourceId,
		scopeId: root,
		ownerId: owner?.id ?? null,
		implicitScopeRootId: owner?.parent ?? null,
		...facts,
		adopted,
	});
}
function ownerFacts(owner, sheetFacts) {
	return {
		ownerLocalName: owner.tag,
		ownerNamespace: owner.namespace,
		ownerType: attr(owner, "type"),
		ownerMedia: attr(owner, "media"),
		sheetMedia: sheetFacts?.media ?? [],
		disabled: sheetFacts?.disabled ?? false,
	};
}
function physicalStyleText(state, owner) {
	let text = "";
	const children = apply(mapGet, state.children, [owner.id]) ?? [];
	for (let i = 0; i < children.length; i++) {
		const child = children[i];
		if (child.kind !== "text") continue;
		const part = data(child.element, "_data");
		if (typeof part !== "string") unavailable("unknown STYLE text");
		if (part.length > state.remaining - text.length)
			throw new RangeError("CSS source units exceed runtime policy");
		text += part;
	}
	return text;
}
function ownerSheetRules(state, owner, facts, sheetFacts, text) {
	if (!sheetFacts) return;
	if (!text && sheetFacts.rules.length === 0)
		use(state, addSource(state, ""), owner.treeScope, owner, facts, false);
	for (let i = 0; i < sheetFacts.rules.length; i++)
		use(
			state,
			addSource(state, sheetFacts.rules[i]),
			owner.treeScope,
			owner,
			facts,
			false,
		);
}
function ownerSources(state, owner) {
	const sheet = data(owner.element, "__sheet");
	const sheetFacts = sheet === undefined ? null : sheetData(state, sheet);
	const facts = ownerFacts(owner, sheetFacts);
	const text = physicalStyleText(state, owner);
	if (text)
		use(state, addSource(state, text), owner.treeScope, owner, facts, false);
	ownerSheetRules(state, owner, facts, sheetFacts, text);
}
function adoptedSources(state, root) {
	const sequence = data(root.element, "adoptedStyleSheets");
	if (sequence === undefined) return;
	if (typeof cssom.readNativeCssSheetSequence !== "function")
		unavailable("adopted stylesheet sequence reader unavailable");
	const sheets = cssom.readNativeCssSheetSequence(sequence, {
		maxEntries: state.maximumSources - state.sources.length,
		maxUnits: state.remaining,
	});
	for (let i = 0; i < sheets.length; i++) {
		const sheet = sheetData(state, sheets[i]);
		const facts = {
			ownerLocalName: null,
			ownerNamespace: null,
			ownerType: null,
			ownerMedia: null,
			sheetMedia: sheet.media,
			disabled: sheet.disabled,
		};
		if (sheet.rules.length === 0)
			use(state, addSource(state, ""), root.id, null, facts, true);
		for (let r = 0; r < sheet.rules.length; r++)
			use(state, addSource(state, sheet.rules[r]), root.id, null, facts, true);
	}
}
function sourceState(limits) {
	if (
		!limits ||
		!Number.isSafeInteger(limits.sourceUnits) ||
		limits.sourceUnits < 0 ||
		!Number.isSafeInteger(limits.sources) ||
		limits.sources < 0
	)
		throw new TypeError("CSS source policy unavailable");
	return {
		sources: [],
		uses: [],
		children: new NativeMap(),
		remaining: limits.sourceUnits,
		maximumSources: limits.sources,
	};
}
function indexChildren(state, records) {
	for (let i = 0; i < records.length; i++) {
		const row = records[i];
		if (row.parent === null) continue;
		let list = apply(mapGet, state.children, [row.parent]);
		if (!list) {
			list = [];
			apply(mapSet, state.children, [row.parent, list]);
		}
		list.push(row);
	}
}
export function captureNativeCssSources(snapshot, limits) {
	const state = sourceState(limits);
	indexChildren(state, snapshot.records);
	for (let i = 0; i < snapshot.records.length; i++) {
		const row = snapshot.records[i];
		if (row.kind === "element" && row.tag === "style") ownerSources(state, row);
	}
	for (let i = 0; i < snapshot.roots.length; i++)
		adoptedSources(state, snapshot.roots[i]);
	return {
		status: "captured",
		cacheable: false,
		sources: state.sources,
		uses: state.uses,
		provenance: {
			documentCertified: true,
			requiresSourceScan: true,
			stores: [],
		},
		fingerprint: {
			sources: state.sources.map((row) => row.text),
			uses: state.uses.map((row) => ({ ...row })),
		},
	};
}
