(async () => {
  // Document-level edges: :active-view-transition(-type) selector admission (no transition is
  // active), template contents (cloning, adoption, innerHTML of nested templates, documents
  // created by DOMImplementation), importNode / adoptNode errors, and moveBefore errors.
  const out = {};
  const attempt = run => { try { const value = run(); return value === undefined ? 'ok' : value; } catch (e) { return e.name; } };
  const vt = [
    ':active-view-transition', ':ACTIVE-VIEW-TRANSITION', ':active-view-transition()', ':active-view-transition-type(a)', ':active-view-transition-type(a, b)',
    ':active-view-transition-type()', ':active-view-transition-type(none)', ':active-view-transition-type(inherit)', ':active-view-transition-type(1)', ':active-view-transition-type(a b)',
    'html:active-view-transition', ':root:active-view-transition-type(x)', ':is(:active-view-transition, p)', ':where(:active-view-transition-type(none), p)', ':is(:active-view-transition-type(), :active-view-transition)',
    ':not(:active-view-transition)', '::active-view-transition', 'p /* c */ :active-view-transition', ':has(:active-view-transition)', ':is(p, , q):active-view-transition',
    'p:active-view-transition-type(a) > q', ':active-view-transition, [', ':active-view-transition-type(a,)', ':nth-child(1 of :active-view-transition)',
    ':is(:active-view-transition(), p)', ':where(p, :active-view-transition-type(), q)', ':is(:active-view-transition-type(a b), q)', ':is(p :active-view-transition)', 'p::before:active-view-transition',
    ':active-view-transition:hover', ':active-view-transition-type(a):not(p)', '[a=":active-view-transition"]', ':active-view-transition /*x*/ p', ':is(:active-view-transition) :is(, p)',
  ];
  document.body.innerHTML = '<p id=p1></p><q id=q1></q>';
  out.viewTransition = {};
  for (const selector of vt) {
    out.viewTransition[selector] = [
      attempt(() => document.querySelectorAll(selector).length),
      attempt(() => document.documentElement.matches(selector)),
      attempt(() => document.body.closest(selector) === null),
    ];
  }
  // Template contents.
  const holder = document.body.appendChild(document.createElement('div'));
  holder.innerHTML = '<template id=t1><b>x</b><template id=t2><i>y</i>&amp;&nbsp;"</template><!--c--></template>';
  const t1 = holder.firstChild;
  out.template = {
    inner: t1.innerHTML,
    outer: holder.innerHTML,
    nested: t1.content.querySelector('template').content.firstChild.nodeName,
    cloneDeep: (() => { const c = t1.cloneNode(true); return [c.content.childNodes.length, c.content.querySelector('template').content.childNodes.length, c.innerHTML === t1.innerHTML]; })(),
    cloneShallow: t1.cloneNode(false).content.childNodes.length,
    contentOwner: t1.content.ownerDocument === document,
    contentOwnerShared: t1.content.ownerDocument === t1.content.querySelector('template').content.ownerDocument,
  };
  const foreign = document.implementation.createHTMLDocument('');
  const ft = foreign.body.appendChild(foreign.createElement('template'));
  ft.innerHTML = '<u>z</u>';
  const adopted = document.adoptNode(ft);
  out.template.adopted = [adopted.content.firstChild.nodeName, adopted.content.ownerDocument === foreign, adopted.content.ownerDocument === document, adopted.innerHTML];
  const imported = document.importNode(ft, true);
  out.template.imported = [imported.innerHTML, imported.content.ownerDocument === t1.content.ownerDocument];
  const xml = document.implementation.createDocument('http://www.w3.org/1999/xhtml', 'html', null);
  const xt = xml.documentElement.appendChild(xml.createElementNS('http://www.w3.org/1999/xhtml', 'template'));
  out.template.xml = [xt.content ? xt.content.nodeName : null, xt.content && xt.content.ownerDocument !== xml];
  t1.content.textContent = '';
  out.template.cleared = [t1.innerHTML, t1.content.childNodes.length];
  t1.innerHTML = '<template><template><s></s></template></template>';
  out.template.reparsed = [t1.innerHTML, t1.content.firstChild.content.firstChild.content.firstChild.nodeName];
  // importNode / adoptNode errors.
  const host = document.createElement('div');
  const shadow = host.attachShadow({ mode: 'open' });
  out.nodes = {
    adoptShadow: attempt(() => document.adoptNode(shadow)),
    adoptDocument: attempt(() => document.adoptNode(foreign)),
    importDocument: attempt(() => document.importNode(foreign)),
    importShadow: attempt(() => document.importNode(shadow)),
    importAttr: attempt(() => { const a = foreign.createAttribute('x'); a.value = 'v'; const c = document.importNode(a); return [c.value, c.ownerDocument === document]; }),
    adoptAttr: attempt(() => { const e = foreign.createElement('e'); e.setAttribute('k', 'v'); const a = e.getAttributeNode('k'); document.adoptNode(a); return [e.hasAttribute('k'), a.ownerElement, a.ownerDocument === document]; }),
    importDoctype: attempt(() => document.importNode(foreign.doctype).nodeName),
    adoptDoctype: attempt(() => { const d = document.adoptNode(foreign.doctype); return [d.ownerDocument === document, foreign.doctype]; }),
    importPi: attempt(() => document.importNode(foreign.createProcessingInstruction('a', 'b')).data),
    importFragment: attempt(() => { const f = foreign.createDocumentFragment(); f.append('a', foreign.createElement('b')); return document.importNode(f, true).childNodes.length; }),
  };
  // moveBefore errors and successes.
  const parent = document.body.appendChild(document.createElement('div'));
  const kid = parent.appendChild(document.createElement('span'));
  const other = document.implementation.createHTMLDocument('');
  out.move = {
    foreignNode: attempt(() => parent.moveBefore(other.createElement('x'), null)),
    foreignChild: attempt(() => parent.moveBefore(kid, other.body)),
    notAChild: attempt(() => parent.moveBefore(kid, document.createElement('x'))),
    notANode: attempt(() => parent.moveBefore({}, null)),
    noChild: attempt(() => parent.moveBefore(kid)),
    disconnectedParent: attempt(() => document.createElement('d').moveBefore(kid, null)),
    textIntoDocument: attempt(() => document.moveBefore(document.body.appendChild(document.createTextNode('t')), null)),
    elementIntoDocument: attempt(() => document.moveBefore(kid, null)),
    ancestor: attempt(() => kid.moveBefore(parent, null)),
    commentIntoDocument: attempt(() => { const c = document.body.appendChild(document.createComment('c')); document.moveBefore(c, document.documentElement); return document.firstChild.nextSibling === c || document.childNodes[1] === c; }),
    self: attempt(() => { parent.moveBefore(kid, kid); return parent.firstChild === kid; }),
  };
  // Character data replacement through the native operation.
  const text = document.createTextNode('hello');
  out.characterData = [
    attempt(() => { text.replaceData(1, 2, 'XY'); return text.data; }),
    attempt(() => { text.replaceData(10, 1, 'Z'); return text.data; }),
    attempt(() => { text.replaceData(-1, 1, 'Z'); return text.data; }),
    attempt(() => { text.replaceData(2, 4294967295, ''); return text.data; }),
    attempt(() => { const pi = document.createProcessingInstruction('a', 'bcd'); pi.replaceData(0, 1, 'Q'); return pi.data; }),
    attempt(() => document.createProcessingInstruction('a', '?>')),
    attempt(() => document.createProcessingInstruction('1a', 'x')),
    attempt(() => document.createProcessingInstruction('a', 'x').target),
  ];
  const fragment = document.createDocumentFragment();
  fragment.append(Object.assign(document.createElement('b'), { id: '5' }), Object.assign(document.createElement('i'), { id: '[object Object]' }));
  out.fragmentIds = [
    attempt(() => fragment.getElementById(5).nodeName),
    attempt(() => fragment.getElementById({}).nodeName),
    attempt(() => fragment.getElementById(Symbol('s'))),
    attempt(() => fragment.getElementById({ toString() { throw new RangeError('id'); } })),
    attempt(() => fragment.getElementById('missing')),
    attempt(() => fragment.getElementById()),
  ];
  return out;
})()
