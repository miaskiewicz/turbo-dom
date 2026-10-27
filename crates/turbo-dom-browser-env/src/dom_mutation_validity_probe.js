(async () => {
  // Pre-insertion / replace validity of the variadic ChildNode / ParentNode mutation methods
  // (append, prepend, before, after, replaceWith, replaceChildren, moveBefore) and of
  // insertBefore / appendChild / replaceChild: each node kind into each parent kind, cycles,
  // documents with an element / doctype already present, and the resulting child lists.
  const out = {};
  const attempt = run => { try { const value = run(); return value === undefined ? 'ok' : typeof value === 'object' && value ? (value.nodeName || 'object') : String(value); } catch (e) { return e.name; } };
  const names = parent => Array.from(parent.childNodes, node => node.nodeName);
  const kinds = doc => ({
    element: doc.createElement('b'),
    text: doc.createTextNode('t'),
    pi: doc.createProcessingInstruction('p', 'd'),
    comment: doc.createComment('c'),
    doctype: document.implementation.createDocumentType('html', '', ''),
    fragment: (() => { const f = doc.createDocumentFragment(); f.append(doc.createElement('i'), 'x'); return f; })(),
    emptyFragment: doc.createDocumentFragment(),
    twoElementFragment: (() => { const f = doc.createDocumentFragment(); f.append(doc.createElement('i'), doc.createElement('u')); return f; })(),
    textFragment: (() => { const f = doc.createDocumentFragment(); f.append('only text'); return f; })(),
    attr: doc.createAttribute('a'),
    document: document.implementation.createHTMLDocument(''),
  });
  const parents = () => {
    const empty = document.implementation.createDocument(null, null, null);
    const withElement = document.implementation.createDocument(null, 'root', null);
    const withDoctype = document.implementation.createDocument(null, null, document.implementation.createDocumentType('d', '', ''));
    const withBoth = document.implementation.createHTMLDocument('');
    return { element: document.createElement('div'), fragment: document.createDocumentFragment(), empty, withElement, withDoctype, withBoth };
  };
  const methods = {
    append: (parent, node) => parent.append(node),
    prepend: (parent, node) => parent.prepend(node),
    replaceChildren: (parent, node) => parent.replaceChildren(node),
    appendChild: (parent, node) => parent.appendChild(node),
    insertBeforeFirst: (parent, node) => parent.insertBefore(node, parent.firstChild),
  };
  out.matrix = {};
  for (const [method, run] of Object.entries(methods)) {
    const row = {};
    for (const parentName of Object.keys(parents())) {
      for (const nodeName of Object.keys(kinds(document))) {
        const parent = parents()[parentName];
        const node = kinds(parent.ownerDocument || parent)[nodeName];
        if (!node) continue;
        const result = attempt(() => run(parent, node));
        row[parentName + '<' + nodeName] = [result, names(parent).join(',')];
      }
    }
    out.matrix[method] = row;
  }
  // Doctype placement relative to the document element.
  const placement = {};
  for (const [name, run] of Object.entries({
    doctypeAfterElement: doc => doc.append(document.implementation.createDocumentType('x', '', '')),
    doctypeBeforeElement: doc => doc.prepend(document.implementation.createDocumentType('x', '', '')),
    elementBeforeDoctype: doc => { doc.documentElement.remove(); doc.prepend(doc.createElement('z')); },
    elementAfterDoctype: doc => { doc.documentElement.remove(); doc.append(doc.createElement('z')); },
    secondDoctype: doc => { doc.documentElement.remove(); doc.append(document.implementation.createDocumentType('y', '', '')); },
    commentBeforeDoctype: doc => doc.prepend(doc.createComment('c')),
    replaceDoctypeWithElement: doc => { doc.documentElement.remove(); doc.doctype.replaceWith(doc.createElement('z')); },
    replaceElementWithDoctype: doc => { const d = doc.doctype; d.remove(); doc.documentElement.replaceWith(document.implementation.createDocumentType('y', '', '')); },
    replaceChildrenTwoElements: doc => doc.replaceChildren(doc.createElement('a'), doc.createElement('b')),
    replaceChildrenDoctypeAfterElement: doc => doc.replaceChildren(doc.createElement('a'), document.implementation.createDocumentType('y', '', '')),
    replaceChildrenDoctypeElement: doc => doc.replaceChildren(document.implementation.createDocumentType('y', '', ''), doc.createElement('a')),
    replaceChildrenText: doc => doc.replaceChildren('text'),
    beforeElementText: doc => doc.documentElement.before('text'),
    afterDoctypeElement: doc => doc.doctype.after(doc.createElement('z')),
    beforeDoctypeElement: doc => { doc.documentElement.remove(); doc.doctype.before(doc.createElement('z')); },
    afterElementDoctype: doc => { doc.doctype.remove(); doc.documentElement.after(document.implementation.createDocumentType('y', '', '')); },
    moveBeforeText: doc => doc.moveBefore(doc.createTextNode('t'), null),
    moveBeforeElementBeforeDoctype: doc => { const e = doc.documentElement; e.remove(); doc.body; doc.moveBefore(doc.createElement('z'), doc.doctype); },
    replaceChildDoctypeWithElement: doc => { const html = doc.documentElement; html.remove(); doc.replaceChild(doc.createElement('z'), doc.doctype); },
    insertBeforeElementBeforeDoctype: doc => { doc.documentElement.remove(); doc.insertBefore(doc.createElement('z'), doc.doctype); },
  })) {
    const doc = document.implementation.createHTMLDocument('');
    placement[name] = [attempt(() => run(doc)), names(doc).join(',')];
  }
  out.placement = placement;
  // Cycles: inserting an ancestor (or a shadow host) into its descendant.
  const a = document.createElement('a'), b = a.appendChild(document.createElement('b')), c = b.appendChild(document.createElement('c'));
  const host = document.createElement('div');
  const root = host.attachShadow({ mode: 'open' });
  const inside = root.appendChild(document.createElement('span'));
  out.cycles = {
    self: attempt(() => a.append(a)),
    ancestor: attempt(() => c.append(a)),
    ancestorPrepend: attempt(() => c.prepend(document.createTextNode('x'), a)),
    ancestorBefore: attempt(() => c.firstChild ? 0 : c.appendChild(document.createElement('d')).before(a)),
    ancestorAfter: attempt(() => c.firstChild.after(a)),
    ancestorReplaceWith: attempt(() => c.firstChild.replaceWith(a)),
    ancestorReplaceChildren: attempt(() => c.replaceChildren(a)),
    hostIntoShadow: attempt(() => inside.append(host)),
    hostIntoShadowBefore: attempt(() => inside.appendChild(document.createElement('e')).before(host)),
    shadowRootAppend: attempt(() => document.createElement('p').append(root)),
    sameFragmentTwice: (() => { const f = document.createDocumentFragment(); f.append('x', document.createElement('y')); const t = document.createElement('t'); return [attempt(() => t.append(f, f)), names(t).join(','), f.childNodes.length]; })(),
    tree: [names(a).join(','), names(b).join(','), names(c).join(',')],
  };
  // Viable siblings when the reference is itself among the inserted nodes.
  const p = document.createElement('p');
  const [x, y, z] = ['x', 'y', 'z'].map(n => p.appendChild(document.createElement(n)));
  y.before(x, y);
  out.viablePrevious = names(p).join(',');
  y.after(z, y);
  out.viableNext = names(p).join(',');
  x.after(y, x, z);
  out.viableNextChain = names(p).join(',');
  z.before(z, x, y);
  out.viablePreviousChain = names(p).join(',');
  // Moving nodes from another document adopts them.
  const other = document.implementation.createHTMLDocument('');
  const foreign = other.body.appendChild(other.createElement('em'));
  const target = document.body.appendChild(document.createElement('div'));
  target.append(foreign, other.createTextNode('ft'));
  out.adopted = [foreign.ownerDocument === document, names(target).join(','), names(other.body).join(',')];
  const foreignFragment = other.createDocumentFragment();
  foreignFragment.append(other.createElement('ff'));
  target.replaceChildren(foreignFragment);
  out.adoptedFragment = [names(target).join(','), target.firstChild.ownerDocument === document];
  out.moveBefore = {
    foreign: attempt(() => target.moveBefore(other.createElement('mb'), null)),
    foreignChild: attempt(() => target.moveBefore(target.firstChild, other.body)),
    notChild: attempt(() => target.moveBefore(target.firstChild, document.createElement('nc'))),
    disconnected: attempt(() => target.moveBefore(document.createElement('dc'), null)),
    text: attempt(() => { const t = document.body.appendChild(document.createTextNode('mt')); target.moveBefore(t, null); return names(target).join(','); }),
  };
  return out;
})()
