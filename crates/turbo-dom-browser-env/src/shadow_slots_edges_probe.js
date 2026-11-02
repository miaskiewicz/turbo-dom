(async () => {
  // Slot assignment edges: manual roots before any assign(), assign() argument conversion,
  // reassignment between slots of one root, assignedSlot through closed roots, flattening.
  const out = {};
  const tick = () => new Promise(resolve => setTimeout(resolve, 0));
  const ids = nodes => nodes.map(node => node.id || node.nodeName);
  const slotOf = node => { const slot = node.assignedSlot; return slot === null ? null : slot.id; };
  const error = run => { try { run(); return 'ok'; } catch (e) { return [e.name, e instanceof TypeError]; } };
  const host = document.body.appendChild(document.createElement('div'));
  host.innerHTML = '<b id=b1></b><i id=i1></i>text';
  const root = host.attachShadow({ mode: 'open', slotAssignment: 'manual' });
  const s1 = root.appendChild(document.createElement('slot'));
  s1.id = 's1';
  const s2 = root.appendChild(document.createElement('slot'));
  s2.id = 's2';
  const changes = [];
  root.addEventListener('slotchange', event => changes.push(event.target.id));
  out.unassigned = [s1.assignedNodes().length, s1.assignedElements().length, s1.assignedNodes({ flatten: true }).length, root.slotAssignment];
  out.assignErrors = {
    number: error(() => s1.assign(1)),
    plain: error(() => s1.assign({})),
    nullish: error(() => s1.assign(null)),
    document: error(() => s1.assign(document)),
    comment: error(() => s1.assign(document.createComment('c'))),
    second: error(() => s1.assign(host.firstChild, 'x')),
  };
  out.afterErrors = ids(s1.assignedNodes());
  const [b1, i1, text] = host.childNodes;
  s1.assign(b1, i1, b1, text);
  await tick();
  out.assigned = [ids(s1.assignedNodes()), ids(s1.assignedElements()), slotOf(b1), slotOf(text)];
  // Moving a node to the second slot drops it from the first one's list.
  s2.assign(i1);
  await tick();
  out.moved = [ids(s1.assignedNodes()), ids(s2.assignedNodes()), slotOf(i1)];
  // A node removed from the host stays recorded but is not assigned; re-adding restores it.
  b1.remove();
  await tick();
  out.removed = [ids(s1.assignedNodes()), slotOf(b1)];
  host.prepend(b1);
  await tick();
  out.readded = [ids(s1.assignedNodes()), slotOf(b1)];
  // A node of another host is recorded but never assigned.
  const other = document.body.appendChild(document.createElement('p'));
  const stray = other.appendChild(document.createElement('em'));
  s1.assign(stray, text);
  await tick();
  out.stray = [ids(s1.assignedNodes()), slotOf(stray), slotOf(b1)];
  // Moving the stray into the host makes it assigned.
  host.append(stray);
  await tick();
  out.strayMoved = [ids(s1.assignedNodes()), slotOf(stray)];
  // A slot removed from the root stops assigning; the nodes become unassigned.
  s1.remove();
  await tick();
  out.slotRemoved = [ids(s1.assignedNodes()), slotOf(text), slotOf(stray)];
  root.append(s1);
  await tick();
  out.slotReadded = [ids(s1.assignedNodes()), slotOf(text)];
  out.changes = changes.slice();
  // Closed roots hide assignedSlot; named slots use the slot attribute.
  const closedHost = document.body.appendChild(document.createElement('div'));
  closedHost.innerHTML = '<span id=c1 slot=x></span><span id=c2></span>';
  const closed = closedHost.attachShadow({ mode: 'closed' });
  closed.innerHTML = '<slot id=cx name=x></slot><slot id=cd></slot>';
  out.closed = [slotOf(closedHost.firstChild), closed.getElementById('cx').assignedNodes().map(n => n.id), closed.getElementById('cd').assignedNodes().map(n => n.id), closed.slotAssignment];
  // Flattening through a nested slot and fallback content.
  const outer = document.body.appendChild(document.createElement('div'));
  outer.innerHTML = '<u id=u1></u>';
  const outerRoot = outer.attachShadow({ mode: 'open' });
  outerRoot.innerHTML = '<div id=inner><slot id=pass></slot><slot id=empty name=none><q id=fallback></q>tx</slot></div>';
  const inner = outerRoot.getElementById('inner');
  const innerRoot = inner.attachShadow({ mode: 'open' });
  innerRoot.innerHTML = '<slot id=deep></slot>';
  const deep = innerRoot.getElementById('deep');
  out.flatten = [ids(deep.assignedNodes()), ids(deep.assignedNodes({ flatten: true })), ids(deep.assignedElements({ flatten: true })), ids(outerRoot.getElementById('empty').assignedNodes({ flatten: true }))];
  out.detachedSlot = [document.createElement('slot').assignedNodes({ flatten: true }).length, error(() => document.createElement('slot').assignedNodes(1))];
  out.mode = [error(() => Object.getOwnPropertyDescriptor(ShadowRoot.prototype, 'slotAssignment').get.call(host))];
  return out;
})()
