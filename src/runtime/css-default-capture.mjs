import { sameOwnedSelectorInput } from './css-native-input-equality.mjs';
import { captureNativeCssSources } from './css-default-sources.mjs';
import { captureCssFactsBudget, reuseCssFactsBudget, createCssFactsNavigatorFromBudget } from './css-facts-navigator.mjs';
import { customElementState, isNativeCssCustomPrototype } from './custom-elements.mjs';
import { matchesSelector } from './selectors.mjs';
import { readNativeCssBuffer } from './buffer.mjs';
import { Node, isNativeCssNode, isNativeCssArray, isNativeCssAttribute } from './dom.mjs';
// Bounded transient DOM facts. Only stable weak IDs survive capture; no CSS or
// layout policy is evaluated here and native models receive primitive DTOs only.
import {
  Document,
  Element,
  Text,
  Comment,
  DocumentType,
  DocumentFragment,
  ShadowRoot,
  SVGElement,
  MathMLElement,
} from './dom.mjs';
import { Buffer } from './buffer.mjs';
import { cssFocusTarget, cssStateSnapshot } from './css-invalidation.mjs';

const NativeArray=Array, NativeObject=Object, NativeString=String, NativeMap=Map, NativeSet=Set, NativeWeakMap=WeakMap, NativeWeakSet=WeakSet;
const descriptor = NativeObject.getOwnPropertyDescriptor;
const nativeCreate = NativeObject.create;
const ownNames = NativeObject.getOwnPropertyNames,
  objectEntries = NativeObject.entries,
  objectFromEntries = NativeObject.fromEntries;
const isArray = NativeArray.isArray,
  isInteger = Number.isInteger,
  safeInteger = Number.isSafeInteger;
const freeze = NativeObject.freeze,
  hasOwn = NativeObject.hasOwn;
const prototype = NativeObject.getPrototypeOf;
const define = Reflect.defineProperty;
const views = ArrayBuffer.isView;
const apply = Reflect.apply;
const weakGet = NativeWeakMap.prototype.get, weakSet = NativeWeakMap.prototype.set,
  weakHas = NativeWeakMap.prototype.has, weakDelete = NativeWeakMap.prototype.delete;
function weakRead(map,key){return apply(weakGet,map,[key]);}
function weakWrite(map,key,value){apply(weakSet,map,[key,value]);}
const arrayMethods = objectFromEntries(
  ['push', 'find', 'filter', 'map', 'some', 'slice', 'includes'].map((name) => [name, NativeArray.prototype[name]]),
);
const arrayIterator = NativeArray.prototype[Symbol.iterator];
const stringMethods = objectFromEntries(
  ['charCodeAt', 'startsWith', 'split', 'slice', 'toLowerCase', 'toUpperCase', 'indexOf', 'includes'].map((name) => [name, NativeString.prototype[name]]),
);
const nodeAt = Document.prototype.__nodeAt;
const nativeChildren = Node.prototype.__children;
const templateDocument = Document.prototype.__templateDocument;
const bufferMethods = objectFromEntries(
  ownNames(Buffer.prototype)
    .filter((name) => name !== 'constructor' && name !== 'soa')
    .map((name) => [name, descriptor(Buffer.prototype, name).value]),
);
const constructors = [Element, Text, Comment, DocumentType, DocumentFragment, SVGElement, MathMLElement, Document];
const originalPrototypes = new NativeWeakSet();
const prototypeHas = NativeWeakSet.prototype.has, prototypeAdd = NativeWeakSet.prototype.add;
for (const Constructor of [...constructors, Node, ShadowRoot, Buffer]) {
  for (let current=Constructor.prototype;current;current=prototype(current)) apply(prototypeAdd,originalPrototypes,[current]);
}
function knownPrototype(value) { if (!apply(prototypeHas,originalPrototypes,[value]) && !isNativeCssCustomPrototype(value)) unsupported('unknown native prototype'); }
const builtinGuards = [];
for (const [owner,names] of [[NativeMap.prototype,['get','set','has','delete']],[NativeWeakMap.prototype,['get','set','has','delete']],[NativeSet.prototype,['add','has']],[Array,['from']],[RegExp.prototype,['test','exec']]]) {
  for (const name of names) builtinGuards.push({owner,name,original:descriptor(owner,name)});
}
const arrayConstructor = descriptor(NativeArray.prototype,'constructor');
const arraySpecies = descriptor(Array,Symbol.species);
function sameDescriptor(left,right) {
  if(left===undefined)return false;
  const data=hasOwn(left,'value');
  if(data!==hasOwn(right,'value'))return false;
  return data?left.value===right.value:left.get===right.get&&left.set===right.set;
}
function originalValue(object,name,value) { const field=descriptor(object,name); return field!==undefined&&hasOwn(field,'value')&&field.value===value; }

const nodeFields = '__listeners ownerDocument parentNode __idx __kids';
function constructionFields(Constructor) {
  if ([Element, SVGElement, MathMLElement].includes(Constructor))
    return `${nodeFields} localName __ns __attrs __attrIdx __content shadowRoot __childNodesList __childrenList __qaCache __qsCache __computedStyle __shadow __sheet`.split(
      ' ',
    );
  if ([Text, Comment].includes(Constructor)) return `${nodeFields} _data`.split(' ');
  if (Constructor === DocumentType) return `${nodeFields} name publicId systemId`.split(' ');
  if (Constructor === Document)
    return `${nodeFields} __buf __version __cache __active defaultView __mo __moPending __ceRegistry __inert`.split(' ');
  return nodeFields.split(' ');
}
function readCssBufferInput(document) {
  if (!isNativeCssNode(document)) unsupported('foreign document');
  const buffer = data(document, '__buf');
  if (buffer === null) return {buffer:null,input:null};
  return {buffer,input:readNativeCssBuffer(buffer)};
}
function beginCssBufferCapture(document) {
  const {buffer,input} = readCssBufferInput(document);
  return {
    buffer,
    cell(name,index) {
      if (!input || !hasOwn(input.fields,name) || !safeInteger(index) || index<0) unsupported('unknown input cell');
      const field=descriptor(input.fields[name],`${index}`);
      if (!field || !hasOwn(field,'value')) unsupported('column hole or accessor');
      return field.value;
    },
    finish() { if (readCssBufferInput(document).buffer !== buffer) unsupported('changed input owner'); }
  };
}
const limitNames = ['maxNodes', 'maxDepth', 'maxPathUnits', 'maxTextUnits', 'maxAttributes', 'maxAttributeUnits'];
export class CssDomSnapshotCapabilityError extends Error {
  constructor(reason) {
    super(`turbo-dom: CSS DOM snapshot ${reason}`);
    this.name = 'CssDomSnapshotCapabilityError';
  }
}
export class CssDomSnapshotLimitError extends Error {
  constructor(limit) {
    super(`turbo-dom: CSS DOM snapshot ${limit} limit exceeded`);
    this.name = 'CssDomSnapshotLimitError';
    this.limit = limit;
  }
}
function unsupported(reason) {
  throw new CssDomSnapshotCapabilityError(reason);
}
function data(value, key, optional = false) {
  const property = descriptor(value, key);
  if (!property && optional) return undefined;
  if (!property || !hasOwn(property,'value')) unsupported(`unknown ${key}`);
  return property.value;
}
function ancestor(value, target) {
  for (let current = prototype(value); current; current = prototype(current)) { knownPrototype(current); if (current === target) return true; }
  return false;
}
function indexed(name) {
  const index = +name;
  return isInteger(index) && index >= 0 && `${index}` === name;
}
function plainArray(value) {
  if (!isNativeCssArray(value)) unsupported('foreign original array');
  if (!isArray(value) || prototype(value) !== NativeArray.prototype) unsupported('unknown array storage');
  return value;
}
function originalMethod(value, name, expected) {
  let current = value;
  for (;;) {
    const property = descriptor(current, name);
    if (property) {
      if (!hasOwn(property,'value') || property.value !== expected) unsupported(`overridden ${name}`);
      return;
    }
    current = prototype(current);
    if (current) knownPrototype(current);
    if (!current) unsupported(`missing ${name}`);
  }
}
function guardMethodTable(base,methods,reason){
 for(const [name,method] of objectEntries(methods))if(!originalValue(base,name,method))unsupported(reason);
}
function guardHelpers() {
  if (!originalValue(NativeArray.prototype, Symbol.iterator, arrayIterator)) unsupported('overridden array iterator');
  for (const entry of builtinGuards) if (!sameDescriptor(descriptor(entry.owner,entry.name),entry.original)) unsupported('overridden collection helper');
  if (!sameDescriptor(descriptor(NativeArray.prototype,'constructor'),arrayConstructor) || !sameDescriptor(descriptor(Array,Symbol.species),arraySpecies)) unsupported('overridden array constructor/species');
  guardMethodTable(NativeArray.prototype,arrayMethods,'overridden array helper');
  guardMethodTable(NativeString.prototype,stringMethods,'overridden string helper');
}
function guardInflation(document) {
  guardHelpers();
  originalMethod(document, '__nodeAt', nodeAt);
  originalMethod(document, '__templateDocument', templateDocument);
  plainArray(data(document, '__cache'));
  for (const base of [NativeArray.prototype, NativeObject.prototype]) {
    if (ownNames(base).some(indexed)) unsupported('indexed prototype storage');
  }
  for (const Constructor of constructors) {
    for (let current = Constructor.prototype; current; current = prototype(current)) {
      for (const field of constructionFields(Constructor))
        if (descriptor(current, field)) unsupported(`constructor field ${field}`);
    }
  }
}
function charge(state, name, amount) {
  if (!safeInteger(amount) || amount < 0 || amount > state.limits[name] - state.used[name])
    throw new CssDomSnapshotLimitError(name);
  state.used[name] += amount;
}
function units(value) {
  if (typeof value !== 'string') unsupported('non-string text');
  const result = [];
  for (let index = 0; index < value.length; index++) result.push(value.charCodeAt(index));
  return result;
}
function number(state, name, index) {
  const value = state.bufferInput.cell(name, index);
  if (!isInteger(value)) unsupported(`invalid ${name} index`);
  return value;
}
function table(state, name, index) {
  const value = state.bufferInput.cell(name, index);
  if (typeof value !== 'string') unsupported(`invalid ${name} text`);
  return value;
}
function guardBuffer(state) {
  if (!state.buffer) return;
  if (!ancestor(state.buffer, Buffer.prototype)) unsupported('foreign buffer');
  for (const [name, method] of objectEntries(bufferMethods)) originalMethod(state.buffer, name, method);
  // Original column provenance is checked through the private borrowed input.
}
function validateBufferNode(state, index) {
  const type = number(state, 'nodeType', index);
  if (![1, 3, 8, 10, 11].includes(type)) unsupported('unknown buffered node type');
  if (type === 1 || type === 11) table(state, 'tagNames', number(state, 'tagId', index));
  if (type === 1) number(state, 'ns', index);
  if ([3, 8, 10].includes(type)) {
    for (const key of type === 10 ? ['textId', 'pubId', 'sysId'] : ['textId']) {
      const textIndex = number(state, key, index);
      if (textIndex >= 0) table(state, 'strings', textIndex);
    }
  }
  return type;
}
function childIndices(state, parentIndex, includeContent = false) {
  const result = [],
    seen = new NativeSet();
  for (let index = number(state, 'firstChild', parentIndex); index !== -1; index = number(state, 'nextSib', index)) {
    if (seen.has(index)) unsupported('cyclic buffer children');
    seen.add(index);
    if (result.length >= state.limits.maxNodes - state.records.length) throw new CssDomSnapshotLimitError('maxNodes');
    const type = validateBufferNode(state, index);
    if (includeContent || type !== 11 || table(state, 'tagNames', number(state, 'tagId', index)) !== 'content')
      result.push({ index, type });
  }
  return result;
}
function validateTemplate(state, index) {
  if (table(state, 'tagNames', number(state, 'tagId', index)) !== 'template') return;
  const inert = data(state.document, '__inert');
  if (inert !== undefined && inert !== true) {
    if (cssFocusTarget(inert) === undefined || readCssBufferInput(inert).buffer !== state.buffer)
      unsupported('foreign inert owner');
    originalMethod(inert, '__nodeAt', nodeAt);
    plainArray(data(inert, '__cache'));
  }
  // Native template construction materializes only its content-fragment handle,
  // never the inert fragment's descendants.
  validateInertCache(state, index, inert);
}
function validateInertCache(state, index, inert) {
  const children = childIndices(state, index, true);
  const cache = inert && inert !== true ? data(inert, '__cache') : data(state.document, '__cache');
  for (const child of children) {
    const property = descriptor(cache, `${child.index}`);
    if (property && !hasOwn(property,'value')) unsupported('unknown inert cached node');
  }
}
function physicalChildren(state, parent) {
  const kids = data(parent, '__kids');
  if (kids !== null) {
    plainArray(kids);
    const length = data(kids, 'length');
    if (length > state.limits.maxNodes - state.records.length) throw new CssDomSnapshotLimitError('maxNodes');
    return NativeArray.from({ length }, (_, index) => data(kids, `${index}`));
  }
  const index = data(parent, '__idx');
  if (index < 0 || !state.buffer) return [];
  return inflateChildren(state, parent, index);
}
function inflateChildren(state, parent, index) {
  const indices = childIndices(state, index), cache = data(state.document, '__cache');
  for (const child of indices) {
    const cached = descriptor(cache, `${child.index}`);
    if (cached && !hasOwn(cached, 'value')) unsupported('unknown cached node');
    if (cached?.value !== undefined && !isNativeCssNode(cached.value)) unsupported('foreign cached wrapper');
    if (child.type === 1) validateTemplate(state, child.index);
  }
  originalMethod(parent, '__children', nativeChildren);
  const result = apply(nativeChildren, parent, []);
  plainArray(result);
  return NativeArray.from({length:data(result,'length')}, (_, at)=>data(result, `${at}`));
}
function ownedAttributes(state, stored) {
  plainArray(stored);
  const length = data(stored, 'length');
  if (length > state.limits.maxAttributes - state.used.maxAttributes)
    throw new CssDomSnapshotLimitError('maxAttributes');
  const result = [];
  for (let index = 0; index < length; index++) {
    const raw = data(stored, `${index}`);
    if (!isNativeCssAttribute(raw)) unsupported('foreign original attribute');
    result.push({name:data(raw,'name'),value:data(raw,'value'),prefix:data(raw,'prefix',true) ?? ''});
  }
  return result;
}
function rawAttributes(state, element) {
  const stored = data(element, '__attrs');
  if (stored !== undefined) return ownedAttributes(state, stored);
  const index = data(element, '__attrIdx');
  if (index < 0 || !state.buffer) return [];
  const start = number(state, 'attrStart', index),
    count = number(state, 'attrCount', index),
    result = [];
  if (count > state.limits.maxAttributes - state.used.maxAttributes)
    throw new CssDomSnapshotLimitError('maxAttributes');
  for (let offset = 0; offset < count && start >= 0; offset++)
    result.push({
      name: table(state, 'attrNames', number(state, 'attrNameId', start + offset)),
      value: table(state, 'attrValues', number(state, 'attrValueId', start + offset)),
      prefix: table(state, 'attrPrefixes', number(state, 'attrPrefixId', start + offset)),
    });
  return result;
}
function attributes(state, element) {
  const result = [];
  for (const raw of rawAttributes(state, element)) {
    charge(state, 'maxAttributes', 1);
    const name = data(raw, 'name'),
      value = data(raw, 'value'),
      prefix = data(raw, 'prefix', true) ?? '';
    if (typeof name !== 'string' || typeof value !== 'string' || typeof prefix !== 'string')
      unsupported('unknown attribute');
    charge(state, 'maxAttributeUnits', name.length + value.length + prefix.length);
    result.push({ name, value, prefix, nameUnits: units(name), valueUnits: units(value), prefixUnits: units(prefix) });
  }
  return result;
}
function namespace(short) {
  if (typeof short !== 'string') unsupported('unknown namespace');
  const known = {
    '': 'http://www.w3.org/1999/xhtml',
    svg: 'http://www.w3.org/2000/svg',
    math: 'http://www.w3.org/1998/Math/MathML',
  };
  if (hasOwn(known, short)) return known[short];
  if (short.startsWith('ns:')) return short.slice(3) || null;
  unsupported('unknown namespace');
}
function attribute(record, name) {
  return record.attrs.find((item) => item.name === name)?.value ?? null;
}
function nodeKind(node) {
  const classes = [
    [Element, 'element'],
    [Text, 'text'],
    [Comment, 'comment'],
    [DocumentType, 'doctype'],
    [ShadowRoot, 'shadow'],
    [DocumentFragment, 'fragment'],
  ];
  for (const [Constructor, kind] of classes) if (ancestor(node, Constructor.prototype)) return kind;
  unsupported('foreign node');
}
function stableId(allocator, node) {
  let id = weakRead(allocator.ids,node);
  if (id === undefined) {
    if (allocator.next > 4294967295) throw new CssDomSnapshotLimitError('node IDs');
    id = allocator.next++;
    weakWrite(allocator.ids,node,id);
  }
  return id;
}
function elementPayload(state, node, record) {
  record.tag = data(node, 'localName');
  record.namespace = namespace(data(node, '__ns'));
  if (typeof record.tag !== 'string') unsupported('unknown tag');
  charge(state, 'maxAttributeUnits', record.tag.length + (record.namespace?.length ?? 0));
  record.tagUnits = units(record.tag);
  record.namespaceUnits = record.namespace === null ? null : units(record.namespace);
  record.attrs = attributes(state, node);
  record.inline = attribute(record, 'style');
  record.inlineUnits = record.attrs.find((item) => item.name === 'style')?.valueUnits ?? null;
}
function recordPayload(state, node, record) {
  if (record.kind === 'element') {
    elementPayload(state, node, record);
  } else if (record.kind === 'text' || record.kind === 'comment') {
    const text = data(node, '_data');
    if (typeof text !== 'string') unsupported('unknown text');
    charge(state, 'maxTextUnits', text.length);
    record.textUnits = units(text);
  }
}
function validateNode(state, node, parent) {
  if (!state.nativeNodeBrand(node)) unsupported('foreign node wrapper');
  knownPrototype(prototype(node));
  if (state.byNode.has(node)) unsupported('duplicate/cyclic node');
  if (data(node, 'ownerDocument') !== state.document) unsupported('foreign node owner');
  if (data(node, 'parentNode') !== (parent?.element ?? null)) unsupported('unknown physical parent');
}
function recordNode(state, node, parent, root, path, contexts) {
  validateNode(state, node, parent);
  charge(state, 'maxNodes', 1);
  if (path.length > state.limits.maxDepth) throw new CssDomSnapshotLimitError('maxDepth');
  const id = stableId(state.allocator, node),
    kind = nodeKind(node),
    ownPath = [id, ...path];
  charge(state, 'maxPathUnits', ownPath.length + contexts.length);
  const record = {
    id,
    kind,
    parent: parent?.id ?? null,
    physicalPath: ownPath,
    treeScope: root.id,
    contextRoots: contexts.slice(),
    hostOf: root.host,
    assignedSlot: null,
    assignedSlotPath: [],
    element: node,
  };
  recordPayload(state, node, record);
  state.records.push(record);
  state.byNode.set(node, record);
  state.byId.set(id, record);
  return record;
}
function visit(state, node, parent, root, path, contexts) {
  const record = recordNode(state, node, parent, root, path, contexts);
  for (const child of physicalChildren(state, node)) visit(state, child, record, root, record.physicalPath, contexts);
  if (record.kind === 'element') {
    const shadow = data(node, '__shadow');
    if (shadow !== undefined) visitRoot(state, shadow, record, contexts);
  }
}
function validateRoot(state, node, host) {
  if (!state.nativeNodeBrand(node)) unsupported('foreign root wrapper');
  knownPrototype(prototype(node));
  if (host && (nodeKind(node) !== 'shadow' || data(node, 'host') !== host.element)) unsupported('foreign shadow host');
}
function visitRoot(state, node, host, outerContexts) {
  validateRoot(state, node, host);
  const id = stableId(state.allocator, node),
    contexts = [id, ...outerContexts];
  const root = { id, host: host?.id ?? null, parentScope: host?.treeScope ?? null, element: node };
  state.roots.push(root);
  if (host) {
    state.shadowByHost.set(host.id, root);
    host.shadowRootId = id;
  }
  if (host) visit(state, node, null, root, [], contexts);
  else {
    charge(state, 'maxNodes', 1);
    charge(state, 'maxPathUnits', contexts.length + 1);
    const documentRecord = {
      id,
      kind: 'document',
      parent: null,
      element: node,
      physicalPath: [id],
      contextRoots: contexts,
      treeScope: id,
      hostOf: null,
    };
    state.byNode.set(node, documentRecord);
    state.byId.set(id, documentRecord);
    state.records.push(documentRecord);
    for (const child of physicalChildren(state, node)) visit(state, child, documentRecord, root, [id], contexts);
  }
}
function composedPath(state, record) {
  const path = [],
    seen = new NativeSet();
  for (let current = record; current; ) {
    if (seen.has(current.id)) unsupported('cyclic assigned slot path');
    if (path.length > state.limits.maxDepth) throw new CssDomSnapshotLimitError('maxDepth');
    seen.add(current.id);
    path.push(current.id);
    const next = current.assignedSlot ?? current.parent ?? (current.kind === 'shadow' ? current.hostOf : null);
    current = next === null ? null : state.byId.get(next);
  }
  return path;
}
function slotBuckets(state) {
  const buckets = new NativeMap();
  for (const record of state.records) {
    if (record.tag !== 'slot') continue;
    let bucket = buckets.get(record.treeScope);
    if (!bucket) {
      bucket = new NativeMap();
      buckets.set(record.treeScope, bucket);
    }
    const name = attribute(record, 'name') || '';
    if (!bucket.has(name)) bucket.set(name, record);
  }
  return buckets;
}
function assignSlots(state, buckets) {
  for (const record of state.records) {
    if (record.kind !== 'element') continue;
    const root = state.shadowByHost.get(record.parent);
    const slot = root && buckets.get(root.id)?.get(attribute(record, 'slot') || '');
    if (slot) record.assignedSlot = slot.id;
  }
}
function slots(state) {
  assignSlots(state, slotBuckets(state));
  for (const record of state.records) {
    if (record.assignedSlot === null || record.assignedSlot === undefined) continue;
    record.assignedSlotPath = composedPath(state, record);
    charge(state, 'maxPathUnits', record.assignedSlotPath.length);
  }
}
function unchanged(document, buffer, version) {
  if (data(document, '__version') !== version || readCssBufferInput(document).buffer !== buffer)
    unsupported('document changed during capture');
}
const nativeStateGetters = nativeCreate(null);
for (const name of ['checked','selected','disabled','required','readOnly']) nativeStateGetters[name]=descriptor(Element.prototype,name).get;
const nativeHasAttribute = Element.prototype.hasAttribute;
const nativeNodeTypeGetter=descriptor(Element.prototype,'nodeType').get;
function optionalScalar(node,name) {
  const field=descriptor(node,name);
  if(field){
    if(!hasOwn(field,'value'))unsupported(`unknown native state ${name}`);
    const value=field.value;
    if(value!==null && !['undefined','string','number','boolean'].includes(typeof value))unsupported(`nonprimitive native state ${name}`);
    return value;
  }
  for(let current=prototype(node);current;current=prototype(current)){
    knownPrototype(current);
    if(descriptor(current,name))unsupported(`inherited native state ${name}`);
  }
  return undefined;
}
function guardNativeElementType(node){
  const ownType=descriptor(node,'nodeType');
  if(ownType){if(!hasOwn(ownType,'value')||ownType.value!==1)unsupported('changed native nodeType');}
  else {const type=descriptor(Element.prototype,'nodeType');if(!type||!hasOwn(type,'get')||type.get!==nativeNodeTypeGetter)unsupported('changed native nodeType getter');}
}
function selectorState(node) {
  originalMethod(node,'hasAttribute',nativeHasAttribute);
  guardNativeElementType(node);
  for(const name of ['__checked','__selected','__ce','__ceIs'])optionalScalar(node,name);
  const result=nativeCreate(null);
  for(const name of ownNames(nativeStateGetters)){
    // CSS uses the native control state, not an author/React IDL accessor.
    const field=descriptor(Element.prototype,name);
    if(!field||!hasOwn(field,'get')||field.get!==nativeStateGetters[name])unsupported(`changed native ${name} getter`);
    result[name]=apply(nativeStateGetters[name],node,[]);
  }
  result.customElementState=customElementState(node);
  for(const name of ['hover','active','focus-visible'])result[name]=matchesSelector(node,`:${name}`);
  return result;
}
function detachedFocus(focused,document,maximumDepth){
  // Native focus can outlive a removed component. Prove disconnection through
  // bounded original parent/host edges, never catch an arbitrary lookup error.
  const seen=new NativeSet();let node=focused;
  for(let depth=0;depth<=maximumDepth;depth++){
    if(!isNativeCssNode(node))unsupported('foreign focus ancestor');
    if(node===document)unsupported('connected focus missing from snapshot');
    if(seen.has(node))unsupported('cyclic focused ancestry');seen.add(node);
    const parent=data(node,'parentNode');
    if(parent!==null){node=parent;continue;}
    if(ancestor(node,ShadowRoot.prototype)){node=data(node,'host');continue;}
    return null;
  }
  throw new CssDomSnapshotLimitError('maxDepth');
}
function focusedId(snapshot,focused,maximumDepth) {
  if(focused===null)return null;
  if(!isNativeCssNode(focused)||!ancestor(focused,Element.prototype))unsupported('foreign focused node');
  const document=snapshot.roots.find(root=>root.id===snapshot.documentId).element;
  if(data(focused,'ownerDocument')!==document)unsupported('foreign focus owner');
  const existing=snapshot.records.find(record=>record.element===focused);
  if(existing)return existing.id;
  return detachedFocus(focused,document,maximumDepth);
}
function selectorRow(record,documentId,snapshot){
    const row={id:record.id,nodeType:{element:1,text:3,comment:8,document:9,doctype:10,shadow:11}[record.kind],parentId:record.parent,ownerDocumentId:documentId,childIds:[]};
    if(row.nodeType===undefined)unsupported('unknown native selector node kind');
    if(record.kind==='element'){
      row.localName=record.tag;row.namespaceURI=record.namespace;
      row.attributes=record.attrs.map(attr=>({name:attr.name,value:attr.value,prefix:attr.prefix}));
      row.assignedSlotId=record.assignedSlot;row.state=selectorState(record.element);
    } else if(record.kind==='text'||record.kind==='comment')row.text=data(record.element,'_data');
    else if(record.kind==='shadow')row.hostId=snapshot.roots.find(root=>root.id===record.id).host;
  return row;
}
function linkSelectorRows(rows,byId){
  for(const row of rows)if(row.parentId!==null){const parent=byId.get(row.parentId);if(!parent)unsupported('foreign native selector parent');parent.childIds.push(row.id);}
}
function selectorInput(snapshot,limits) {
  const byId=new NativeMap(),rows=[],documentId=snapshot.documentId;
  for(const record of snapshot.records){
    const row=selectorRow(record,documentId,snapshot);
    rows.push(row);byId.set(row.id,row);
  }
  linkSelectorRows(rows,byId);
  const rootRecord=snapshot.records.find(record=>record.kind==='element'&&record.parent===documentId);
  const focused=cssFocusTarget(snapshot.roots.find(root=>root.id===documentId).element);
  return{documentId,documentElementId:rootRecord?.id??null,nodes:rows,focusTarget:focusedId(snapshot,focused,limits.maxDepth)};
}
export function nativeDocument(element) {
  if (!isNativeCssNode(element)) unsupported('foreign element');
  if (!ancestor(element,Element.prototype)) throw new TypeError('getComputedStyle requires an Element');
  const document = data(element,'ownerDocument');
  if (!isNativeCssNode(document) || !ancestor(document,Document.prototype)) unsupported('foreign owner document');
  return document;
}
// A disconnected genuine element has an empty computed view. Follow only
// certified native edges; this does not turn capture failures into empty CSS.
export function nativeCssConnected(element, document, maximumDepth) {
  const seen=new NativeSet();let current=element;
  for(let depth=0;depth<=maximumDepth;depth++){
    if(current===document)return true;
    if(!isNativeCssNode(current))unsupported('foreign element ancestor');
    if(seen.has(current))unsupported('cyclic element ancestry');seen.add(current);
    const parent=data(current,'parentNode');
    if(parent!==null){current=parent;continue;}
    if(ancestor(current,ShadowRoot.prototype)){current=data(current,'host');continue;}
    return false;
  }
  throw new CssDomSnapshotLimitError('maxDepth');
}
function sameSelectorOwner(left,right){return !!left && left.identity===right.identity && left.incarnation===right.incarnation;}
export function createDefaultCssCapture() {
  const nativeNodeBrand = isNativeCssNode;
  const documents = new NativeWeakMap(), selectorInputs = new NativeWeakMap(), selectorReuse = new NativeWeakMap();
  function capture(document, limits) {
    if (!nativeNodeBrand(document)) unsupported('foreign document');
    knownPrototype(prototype(document));
    guardHelpers();
    if (cssFocusTarget(document) === undefined) unsupported('foreign document');
    for (const name of limitNames)
      if (!safeInteger(limits?.[name]) || limits[name] < 0) throw new TypeError(`invalid CSS DOM snapshot ${name}`);
    const startVersion = data(document, '__version');
    guardInflation(document);
    const bufferInput = beginCssBufferCapture(document);
    const buffer = bufferInput.buffer;
    let allocator = weakRead(documents,document);
    if (!allocator) {
      allocator = { ids: new NativeWeakMap(), next: 1, identity: freeze({}), storage: buffer, incarnation: freeze({}) };
      weakWrite(documents,document,allocator);
    }
    if (allocator.storage !== buffer) {
      allocator.storage = buffer;
      allocator.incarnation = freeze({});
    }
    const state = {
      document,
      nativeNodeBrand,
      limits,
      buffer,
      bufferInput,
      allocator,
      records: [],
      roots: [],
      byNode: new NativeMap(),
      byId: new NativeMap(),
      shadowByHost: new NativeMap(),
      used: objectFromEntries(limitNames.map((name) => [name, 0])),
    };
    guardBuffer(state);
    visitRoot(state, document, null, []);
    slots(state);
    unchanged(document, buffer, startVersion);
    const snapshot = {
      records: state.records,
      roots: state.roots,
      matchNodes: state.records
        .filter((record) => record.kind === 'element')
        .map((record) => ({
          id: record.id,
          parent: state.byId.get(record.parent)?.kind === 'element' ? record.parent : null,
          element: record.element,
        })),
      documentId: stableId(allocator, document),
      world: { identity: allocator.identity, incarnation: allocator.incarnation, render: cssStateSnapshot(document) },
      broadVersion: data(document, '__version'),
      used: { ...state.used },
      nodeId(node) {
        const record = state.byNode.get(node);
        if (!record) unsupported('node outside snapshot');
        return record.id;
      },
    };
    weakWrite(selectorInputs,snapshot,{input:selectorInput(snapshot,limits),document,identity:allocator.identity,incarnation:allocator.incarnation});
    guardHelpers();
    unchanged(document,buffer,startVersion);
    bufferInput.finish();
    return snapshot;
  }
  return {
    capture,
    sources(snapshot,limits) {
      if(!apply(weakHas,selectorInputs,[snapshot]))unsupported('unknown native source snapshot');
      guardHelpers();
      return captureNativeCssSources(snapshot,limits);
    },
    selectorFacts(snapshot,limits) {
      const owned=weakRead(selectorInputs,snapshot);
      if(!owned)unsupported('unknown native selector snapshot');
      // Fresh capture owns every input field; exact equality retains its prior
      // validation only when the freshly read quotas are also unchanged.
      const budget=captureCssFactsBudget(limits), allocator=weakRead(documents,owned.document);
      const current=sameSelectorOwner(allocator,owned);
      const previous=current && weakRead(selectorReuse,owned.document);
      if(sameSelectorOwner(previous,owned) &&
          sameOwnedSelectorInput(previous.input,owned.input) && reuseCssFactsBudget(budget,previous.navigator))return previous.navigator;
      const fresh=createCssFactsNavigatorFromBudget(owned.input,budget);
      if(current)weakWrite(selectorReuse,owned.document,{identity:owned.identity,incarnation:owned.incarnation,input:owned.input,navigator:fresh});
      return fresh;
    },
    reset(document) {
      apply(weakDelete,documents,[document]);
      apply(weakDelete,selectorReuse,[document]);
    },
  };
}
