// Request-local transport over an already trusted native DOM snapshot. This is
// not a snapshot brand/Proxy guard and never parses selectors or ranks rules.
const MapConstructor = Map;
const apply = Reflect.apply;
const get = Map.prototype.get;
const set = Map.prototype.set;
const has = Map.prototype.has;
const safe = Number.isSafeInteger;
function mapGet(map, key) { return apply(get, map, [key]); }
function mapSet(map, key, value) { apply(set, map, [key, value]); }
function mapHas(map, key) { return apply(has, map, [key]); }
export class SharedRoutingError extends Error {
  constructor(reason) { super(`turbo-dom: CSS routing ${reason}`); this.name = 'SharedRoutingError'; }
}
function refuse(reason) { throw new SharedRoutingError(reason); }
function charge(state, name, amount) {
  if (!safe(amount) || amount < 0 || amount > state[name]) refuse(`${name} limit`);
  state[name] -= amount;
}
function identifier(value) {
  if (!safe(value) || value < 0) refuse('invalid node identity');
  return value;
}
function copyPath(source, state) {
  charge(state, 'maxPathUnits', source.length);
  const path = [];
  for (let index = 0; index < source.length; index++) path[index] = identifier(source[index]);
  return path;
}
function rootsOf(snapshot, state) {
  charge(state, 'maxRoots', snapshot.roots.length);
  const roots = new MapConstructor(), contextRoots = [];
  for (let index = 0; index < snapshot.roots.length; index++) {
    const row = snapshot.roots[index], id = identifier(row.id);
    if (mapHas(roots, id)) refuse('duplicate context root');
    const context = { treeScope: id, root: id, host: row.host, parentTreeScope: row.parentScope };
    if (context.host !== null) identifier(context.host);
    if (context.parentTreeScope !== null) identifier(context.parentTreeScope);
    mapSet(roots, id, context); contextRoots[index] = context;
  }
  return { roots, contextRoots };
}
function depthsOf(contextRoots, roots, maximum) {
  const depths = new MapConstructor();
  for (let index = 0; index < contextRoots.length; index++) {
    const start = contextRoots[index];
    let current = start, depth = 0;
    while (current.parentTreeScope !== null) {
      depth++;
      if (depth > maximum || depth >= contextRoots.length) refuse('invalid context ancestry');
      if (current.host === null) refuse('missing shadow host');
      current = mapGet(roots, current.parentTreeScope);
      if (!current) refuse('foreign parent context');
    }
    if (current.host !== null) refuse('foreign document context');
    mapSet(depths, start.treeScope, depth);
  }
  return depths;
}
function nodesOf(snapshot, state) {
  charge(state, 'maxNodes', snapshot.records.length);
  const nodes = new MapConstructor(), elements = [];
  for (let index = 0; index < snapshot.records.length; index++) {
    const record = snapshot.records[index], id = identifier(record.id);
    if (mapHas(nodes, id)) refuse('duplicate node identity');
    // Deliberately omit element/native-wrapper/text/attributes and snapshot refs.
    const node = { id, index, kind: record.kind, parent: record.parent, treeScope: record.treeScope, path: copyPath(record.physicalPath, state), shadowRootId: record.shadowRootId ?? null, assignedSlot: record.assignedSlot ?? null };
    mapSet(nodes, id, node);
    if (node.kind === 'element') elements[elements.length] = node;
  }
  return { nodes, elements };
}
function physicalEntry(current, id, scope) {
  if (!current || current.id !== id || current.treeScope !== scope) refuse('invalid physical path');
}
function physical(node, nodes, roots) {
  const root = mapGet(roots, node.treeScope);
  if (!root) refuse('foreign tree scope');
  if (node.path[0] !== node.id || node.path[node.path.length - 1] !== root.root) refuse('foreign physical path');
  let current = node;
  for (let index = 0; index < node.path.length; index++) {
    physicalEntry(current, node.path[index], node.treeScope);
    current = current.parent === null ? null : mapGet(nodes, current.parent);
  }
  if (current !== null) refuse('incomplete physical path');
  return root;
}
function slotOf(node, nodes, roots, state) {
  if (node.assignedSlot === null) return null;
  const slot = mapGet(nodes, node.assignedSlot);
  if (!slot || slot.kind !== 'element') refuse('foreign assigned slot');
  const root = physical(slot, nodes, roots);
  if (root.host !== node.parent || root.parentTreeScope !== node.treeScope) refuse('foreign assigned context');
  return { slot: slot.id, treeScope: slot.treeScope, path: copyPath(slot.path, state) };
}
function parentOf(node, slot, nodes, root) {
  if (slot) return slot.slot;
  if (node.parent === root.root && root.host !== null) return root.host;
  const parent = mapGet(nodes, node.parent);
  return parent?.kind === 'element' ? parent.id : null;
}
function targetOf(node, nodes, roots, depths, state) {
  const root = physical(node, nodes, roots), slot = slotOf(node, nodes, roots, state);
  if (node.shadowRootId !== null) {
    const shadow = mapGet(roots, node.shadowRootId);
    if (!shadow || shadow.host !== node.id || shadow.parentTreeScope !== node.treeScope) refuse('foreign owned shadow');
  }
  return { parentId: parentOf(node, slot, nodes, root), path: node.path, treeScope: node.treeScope, scopeDepth: mapGet(depths, node.treeScope), hostOf: node.shadowRootId, assignedSlot: slot };
}
function validateParents(elements, routes) {
  for (let index = 0; index < elements.length; index++) {
    let current = mapGet(routes, elements[index].id), depth = 0;
    while (current.parentId !== null) {
      current = mapGet(routes, current.parentId);
      if (!current) refuse('foreign inheritance parent');
      if (++depth >= elements.length) refuse('cyclic inheritance parent');
    }
  }
}
export function createSharedRouting(snapshot, limits) {
  const state = {};
  for (const name of ['maxRoots', 'maxNodes', 'maxPathUnits', 'maxScopeDepth']) {
    if (!safe(limits?.[name]) || limits[name] < 0) refuse('invalid limits');
    state[name] = limits[name];
  }
  const { roots, contextRoots } = rootsOf(snapshot, state);
  const depths = depthsOf(contextRoots, roots, state.maxScopeDepth);
  const { nodes, elements } = nodesOf(snapshot, state), routes = new MapConstructor();
  for (let index = 0; index < elements.length; index++) {
    const node = elements[index];
    mapSet(routes, node.id, targetOf(node, nodes, roots, depths, state));
  }
  validateParents(elements, routes);
  return {
    contextRoots,
    targetOrder: parentFirst(elements, routes),
    scopeDepth(id) { if (!mapHas(depths, id)) refuse('unknown context'); return mapGet(depths, id); },
    recordIndex(id) { return mapGet(nodes, id).index; },
    targetFacts(id) { if (!mapHas(routes, id)) refuse('unknown target'); return mapGet(routes, id); },
  };
}

function parentFirst(elements, routes) {
  const ordered=[],seen=new MapConstructor();
  for(let index=0;index<elements.length;index++) {
    let id=elements[index].id;
    const path=[];
    while(id!==null&&!mapHas(seen,id)) {
      path[path.length]=id;
      id=mapGet(routes,id).parentId;
    }
    for(let at=path.length-1;at>=0;at--) {
      id=path[at];mapSet(seen,id,true);ordered[ordered.length]=id;
    }
  }
  return ordered;
}
