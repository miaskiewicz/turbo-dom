// Test producer only: public DOM reads are intentional and may call user code.
import { customElementState } from '../../src/runtime/custom-elements.mjs';

export function captureCssPrimitivePublicFacts(document, {
  closedRoots = [], startId = 0, focusTarget, idOf,
} = {}) {
  if (!Number.isSafeInteger(startId) || startId < 0) throw new TypeError('invalid startId');
  if (idOf !== undefined && typeof idOf !== 'function') throw new TypeError('invalid idOf');
  const ids = new Map(), owners = new Map(), originals = [], rows = [], rowsById = new Map();
  const id = (node) => {
    if (node === null || node === undefined) return null;
    if (!ids.has(node)) {
      const value = idOf === undefined ? startId + originals.length : idOf(node);
      if (!Number.isInteger(value) || value < 0 || value > 0xffffffff) throw new TypeError('invalid node ID');
      if (owners.has(value)) throw new TypeError('duplicate node ID');
      owners.set(value, node); ids.set(node, value); originals.push(node);
    }
    return ids.get(node);
  };
  const documentId = id(document);
  const roots = new Map();
  for (const root of closedRoots) {
    if (root.ownerDocument !== document || !root.host) throw new TypeError('foreign owned shadow root');
    roots.set(root.host, root);
  }
  for (let index = 0; index < originals.length; index++) {
    const node = originals[index];
    const children = Array.from(node.childNodes);
    const row = {
      id: id(node), nodeType: node.nodeType, parentId: id(node.parentNode),
      ownerDocumentId: node === document ? documentId : id(node.ownerDocument),
      childIds: children.map(id),
    };
    if (node.nodeType === 1) {
      row.localName = node.localName;
      row.namespaceURI = node.namespaceURI;
      row.attributes = Array.from(node.attributes, (attribute) => ({
        name: attribute.localName ?? attribute.name, value: attribute.value,
        prefix: attribute.prefix ?? '', namespaceURI: attribute.namespaceURI,
      }));
      row.state = {
        checked: node.checked, selected: node.selected, disabled: node.disabled,
        required: node.required, readOnly: node.readOnly,
        customElementState: customElementState(node),
      };
      row.assignedSlotId = id(node.assignedSlot);
      const shadow = roots.get(node) ?? node.shadowRoot;
      if (shadow) {
        if (shadow.host !== node || shadow.ownerDocument !== document) throw new TypeError('foreign shadow owner');
        id(shadow);
      }
    }
    if (node.nodeType === 3 || node.nodeType === 8) row.text = node.data;
    if (node.nodeType === 11 && node.host) row.hostId = id(node.host);
    rows.push(row); rowsById.set(row.id, row);
  }
  // Owned closed-root assignment comes from the actual slot API, not slot-name guesses.
  for (const node of originals) {
    if (node.nodeType !== 1 || node.localName !== 'slot') continue;
    for (const assigned of node.assignedNodes()) {
      const assignedId = ids.get(assigned);
      if (assignedId === undefined) throw new TypeError('assigned node outside captured world');
      rowsById.get(assignedId).assignedSlotId = id(node);
    }
  }
  const result = { documentId, documentElementId: id(document.documentElement), nodes: rows };
  if (focusTarget !== undefined) {
    if (focusTarget !== null && !ids.has(focusTarget)) throw new TypeError('focus target outside captured world');
    result.focusTarget = focusTarget === null ? null : id(focusTarget);
  }
  return result;
}
