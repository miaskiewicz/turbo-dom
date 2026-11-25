// Mechanical source/use capture. CSS grammar and applicability belong to Rust.
import { Node, Element, Text } from './dom.mjs';
import { cssFocusTarget } from './css-invalidation.mjs';
import { cssSheetSnapshot } from './cssom.mjs';

const children = Node.prototype.__children;
const attribute = Element.prototype.getAttribute;
const namespace = Object.getOwnPropertyDescriptor(Element.prototype, 'namespaceURI').get;
const descriptor = Object.getOwnPropertyDescriptor;
const identity = (value) => value;
const UNKNOWN = Symbol('unknown CSS source provenance');

function own(node, key) {
  const value = descriptor(node, key);
  return value === undefined ? undefined : 'value' in value ? value.value : UNKNOWN;
}
function rawMedia(value) {
  if (typeof value === 'string') return value;
  if (!Array.isArray(value)) return undefined;
  const parts = [];
  for (let index = 0; index < value.length; index++) {
    if (typeof value[index] !== 'string') return undefined;
    parts.push(value[index]);
  }
  return parts;
}
function physicalText(owner) {
  let value = '';
  for (const child of children.call(owner)) {
    // CharacterData's own storage avoids STYLE's legacy textContent CSSOM suffix.
    if (child instanceof Text) {
      const data = own(child, '_data');
      if (typeof data !== 'string') return undefined;
      value += data;
    }
  }
  return value;
}
function source(state, value) {
  const id = state.sources.length;
  state.sources.push({ id, text: value });
  return id;
}
function captureRules(state, record, rules) {
  if (!Array.isArray(rules)) {
    state.unsupported = true;
    return;
  }
  for (let index = 0; index < rules.length; index++) {
    const rule = rules[index];
    if (!rule) {
      state.unsupported = true;
      continue;
    }
    const cssText = rule.cssText;
    if (typeof cssText !== 'string') {
      state.unsupported = true;
      continue;
    }
    record.ids.push(source(state, cssText));
  }
}
function sheetSources(state, sheet) {
  if (!sheet || (typeof sheet !== 'object' && typeof sheet !== 'function')) {
    state.unsupported = true;
    return { ids: [], media: undefined, disabled: false, owner: null };
  }
  if (state.sheets.has(sheet)) return state.sheets.get(sheet);
  const snapshot = cssSheetSnapshot(sheet);
  const media = rawMedia(sheet.media);
  if (media === undefined) state.unsupported = true;
  const record = { ids: [], media, disabled: sheet.disabled, owner: sheet.ownerNode };
  state.stores.push({ sheet, snapshot, mediaFingerprint: media });
  captureRules(state, record, sheet.cssRules);
  state.sheets.set(sheet, record);
  return record;
}
function addUse(state, ids, root, owner, facts, adopted) {
  for (const sourceId of ids)
    state.uses.push({
      sourceId,
      scopeId: state.scopeId(root),
      ownerId: owner ? state.nodeId(owner) : null,
      implicitScopeRootId: owner ? state.nodeId(own(owner, 'parentNode')) : null,
      ...facts,
      adopted,
    });
}
function ownerStyle(state, owner, root) {
  const sheet = own(owner, '__sheet');
  if (sheet === UNKNOWN) {
    state.unsupported = true;
    return;
  }
  const facts = {
    ownerLocalName: own(owner, 'localName'),
    ownerNamespace: namespace.call(owner),
    ownerType: attribute.call(owner, 'type'),
    ownerMedia: attribute.call(owner, 'media'),
    sheetMedia: [],
    disabled: false,
  };
  const physical = physicalText(owner);
  if (physical === undefined) state.unsupported = true;
  if (physical) addUse(state, [source(state, physical)], root, owner, facts, false);
  if (!sheet) return;
  const record = sheetSources(state, sheet);
  if (record.owner !== owner) state.cacheable = false;
  const sheetFacts = { ...facts, sheetMedia: record.media, disabled: record.disabled };
  // Owner conditions also govern its physical child text.
  if (physical) Object.assign(state.uses[state.uses.length - 1], sheetFacts);
  addUse(state, record.ids, root, owner, sheetFacts, false);
}
function adoptedSources(state, root) {
  const list = own(root, 'adoptedStyleSheets');
  if (list === undefined) return;
  if (!Array.isArray(list)) {
    state.unsupported = true;
    return;
  }
  // Public externally mutable arrays need an exact per-capture fingerprint.
  state.cacheable = false;
  for (const sheet of list) {
    const record = sheetSources(state, sheet);
    addUse(
      state,
      record.ids,
      root,
      null,
      {
        ownerLocalName: null,
        ownerNamespace: null,
        ownerType: null,
        ownerMedia: null,
        sheetMedia: record.media,
        disabled: record.disabled,
      },
      true,
    );
  }
}
function walk(state, node, root) {
  if (state.nodes.has(node)) {
    state.unsupported = true;
    return;
  }
  state.nodes.add(node);
  if (node instanceof Element) {
    if (own(node, 'localName') === 'style') ownerStyle(state, node, root);
    const shadow = own(node, '__shadow');
    if (shadow === UNKNOWN) state.unsupported = true;
    else if (shadow) visitRoot(state, shadow);
  }
  for (const child of children.call(node)) walk(state, child, root);
}
function visitRoot(state, root) {
  if (state.roots.has(root)) {
    state.unsupported = true;
    return;
  }
  state.roots.add(root);
  walk(state, root, root);
  adoptedSources(state, root);
}
export function captureCssSources(document, { nodeId = identity, scopeId = identity } = {}) {
  if (cssFocusTarget(document) === undefined)
    return { status: 'unsupported', cacheable: false, reason: 'document provenance' };
  const state = {
    sources: [],
    uses: [],
    stores: [],
    sheets: new Map(),
    roots: new Set(),
    nodes: new Set(),
    cacheable: false,
    unsupported: false,
    nodeId,
    scopeId,
  };
  visitRoot(state, document);
  return {
    status: state.unsupported ? 'unsupported' : 'captured',
    cacheable: state.cacheable && !state.unsupported,
    sources: state.sources,
    uses: state.uses,
    provenance: { documentCertified: false, requiresSourceScan: true, stores: state.stores },
    fingerprint: { sources: state.sources.map((item) => item.text), uses: state.uses.map((item) => ({ ...item })) },
  };
}
