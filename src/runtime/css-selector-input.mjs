// Assemble an already validated native CSS capture. The readers perform fresh
// host validation and the input stays private to its capture owner.
const InputMap = Map;
const sameValue = Object.is;
const nodeTypes = { __proto__: null, element: 1, text: 3, comment: 8, document: 9, doctype: 10, shadow: 11 };

export function updateCssSelectorState(state, previous, name, value) {
  if (state === previous) {
    if (sameValue(state[name], value)) return state;
    state = { __proto__: null, ...state };
  }
  state[name] = value;
  return state;
}

function shadowHost(record, snapshot) {
  return snapshot.roots.find(root => root.id === record.id).host;
}

function selectorRow(record, snapshot, readers) {
  const nodeType = nodeTypes[record.kind];
  if (nodeType === undefined) readers.unsupported('unknown native selector node kind');
  const row = {
    id: record.id, nodeType, parentId: record.parent,
    ownerDocumentId: snapshot.documentId, childIds: [],
  };
  if (record.kind === 'element') return {
    ...row,
    localName: record.tag,
    namespaceURI: record.namespace,
    attributes: record.attrs.map(({ name, value, prefix }) => ({ name, value, prefix })),
    assignedSlotId: record.assignedSlot,
    state: readers.state(record.element),
  };
  if (record.kind === 'text' || record.kind === 'comment') return { ...row, text: readers.text(record.element) };
  if (record.kind === 'shadow') return { ...row, hostId: shadowHost(record, snapshot) };
  return row;
}

function linkSelectorRows(rows, byId, readers) {
  for (const row of rows) {
    if (row.parentId === null) continue;
    const parent = byId.get(row.parentId);
    if (!parent) readers.unsupported('foreign native selector parent');
    parent.childIds.push(row.id);
  }
}

function selectorInput(snapshot, limits, readers) {
  const byId = new InputMap(), rows = [], documentId = snapshot.documentId;
  for (const record of snapshot.records) {
    const row = selectorRow(record, snapshot, readers);
    rows.push(row);
    byId.set(row.id, row);
  }
  linkSelectorRows(rows, byId, readers);
  const root = snapshot.records.find(record => record.kind === 'element' && record.parent === documentId);
  return {
    documentId,
    documentElementId: root?.id ?? null,
    nodes: rows,
    focusTarget: readers.focus(snapshot, limits),
  };
}

function sameSelectorAttributes(attributes, previous) {
  if (attributes.length !== previous.length) return false;
  for (let index = 0; index < attributes.length; index++) {
    const { name, value, prefix } = attributes[index], old = previous[index];
    if (name !== old.name || value !== old.value || prefix !== old.prefix) return false;
  }
  return true;
}

function sameSelectorPayload(record, previous, snapshot, readers) {
  if (record.kind === 'element') return record.tag === previous.localName &&
    record.namespace === previous.namespaceURI && record.assignedSlot === previous.assignedSlotId &&
    sameSelectorAttributes(record.attrs, previous.attributes);
  if (record.kind === 'text' || record.kind === 'comment') return readers.text(record.element) === previous.text;
  if (record.kind === 'shadow') return shadowHost(record, snapshot) === previous.hostId;
  return true;
}

function sameSelectorStructure(snapshot, previous, readers) {
  const records = snapshot.records, rows = previous.nodes;
  if (snapshot.documentId !== previous.documentId || records.length !== rows.length) return false;
  // Record order plus each id and parent fixes ordered child lists and the
  // first document element. Both captures already validated their edges.
  for (let index = 0; index < records.length; index++) {
    const record = records[index], row = rows[index];
    if (record.id !== row.id || record.parent !== row.parentId ||
        nodeTypes[record.kind] !== row.nodeType ||
        !sameSelectorPayload(record, row, snapshot, readers)) return false;
  }
  return true;
}

export function refreshCssSelectorInput(snapshot, limits, previous, readers) {
  if (!previous || !sameSelectorStructure(snapshot, previous, readers)) return selectorInput(snapshot, limits, readers);
  let nodes = previous.nodes;
  for (let index = 0; index < snapshot.records.length; index++) {
    const record = snapshot.records[index];
    if (record.kind !== 'element') continue;
    const row = nodes[index], state = readers.state(record.element, row.state);
    if (state === row.state) continue;
    if (nodes === previous.nodes) nodes = nodes.slice();
    nodes[index] = { ...row, state };
  }
  const focusTarget = readers.focus(snapshot, limits);
  if (nodes === previous.nodes && focusTarget === previous.focusTarget) return previous;
  return { ...previous, nodes, focusTarget };
}
