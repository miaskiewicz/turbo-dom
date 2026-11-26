// DOM inflation assigns these fields as own properties. Check for collisions
// on the current prototypes every time, even when the hierarchy is unchanged.
import { Element, SVGElement, MathMLElement, Node, CharacterData, Text, Comment,
  DocumentType, DocumentFragment, Document } from './dom.mjs';
import { EventTarget } from './events.mjs';

const descriptor = Object.getOwnPropertyDescriptor;
const parentOf = Object.getPrototypeOf;
const nodeFields = ['__listeners', 'ownerDocument', 'parentNode', '__idx', '__kids'];
const elementFields = [...nodeFields, 'localName', '__ns', '__attrs', '__attrIdx', '__content',
  'shadowRoot', '__childNodesList', '__childrenList', '__qaCache', '__qsCache', '__computedStyle', '__shadow', '__sheet'];
const characterFields = [...nodeFields, '_data'];

// Use class identities from the defining modules. A prototype inserted before
// the first CSS read must also take the live hierarchy walk.
const nodeChain = [Node.prototype, EventTarget.prototype, Object.prototype];
const elementChain = [Element.prototype, ...nodeChain];
const characterChain = [CharacterData.prototype, ...nodeChain];
const families = [
  [elementChain, elementFields],
  [[Text.prototype, ...characterChain], characterFields],
  [[Comment.prototype, ...characterChain], characterFields],
  [[DocumentType.prototype, ...nodeChain], [...nodeFields, 'name', 'publicId', 'systemId']],
  [[DocumentFragment.prototype, ...nodeChain], nodeFields],
  [[SVGElement.prototype, ...elementChain], elementFields],
  [[MathMLElement.prototype, ...elementChain], elementFields],
  [[Document.prototype, ...nodeChain], [...nodeFields, '__buf', '__version', '__cache', '__active',
    'defaultView', '__mo', '__moPending', '__ceRegistry', '__inert']],
];

function buildPlan() {
  const seen = new Map(), edges = [], checks = [];
  for (const [chain, fields] of families) {
    for (let i = 0; i < chain.length; i++) {
      const owner = chain[i];
      if (!seen.has(owner)) {
        seen.set(owner, new Set());
        edges.push({owner, parent: chain[i + 1] ?? null});
      }
      const names = seen.get(owner);
      for (const field of fields) {
        if (!names.has(field)) {
          names.add(field);
          checks.push({owner, field});
        }
      }
    }
  }
  return {edges, checks};
}
const {edges, checks} = buildPlan();

function liveCollision() {
  for (let i = 0; i < families.length; i++) {
    const chain = families[i][0], fields = families[i][1];
    for (let owner = chain[0]; owner; owner = parentOf(owner)) {
      for (let j = 0; j < fields.length; j++) {
        if (descriptor(owner, fields[j])) return fields[j];
      }
    }
  }
  return undefined;
}

export function cssConstructionFieldCollision() {
  for (let i = 0; i < edges.length; i++) {
    const {owner, parent} = edges[i];
    if (parentOf(owner) !== parent) return liveCollision();
  }
  for (let i = 0; i < checks.length; i++) {
    const {owner, field} = checks[i];
    if (descriptor(owner, field)) return field;
  }
  return undefined;
}
