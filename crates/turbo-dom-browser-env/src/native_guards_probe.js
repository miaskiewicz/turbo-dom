(() => {
  // Native DOM members: receiver brand checks, argument conversion, and the edge results of
  // attribute / character-data / tree operations. Errors are recorded as [constructor, name].
  const run = f => { try { const v = f(); return v === undefined ? 'undefined' : v; } catch (e) { return [e && e.constructor && e.constructor.name, e && e.name]; } };
  const tag = n => n == null ? null : n.nodeType === 1 ? n.localName + (n.id ? '#' + n.id : '') : n.nodeType === 2 ? '@' + n.name + '=' + n.value : n.nodeName + (n.nodeValue == null ? '' : ':' + n.nodeValue);
  const show = v => Array.isArray(v) ? v : v && typeof v === 'object' ? (v.length !== undefined && typeof v !== 'function' ? Array.from(v, tag) : tag(v)) : v;
  const out = {};
  const host = document.body.appendChild(document.createElement('div'));
  host.id = 'host';
  host.innerHTML = '<p id=a title=t data-one-two=1 data-x-Y=2>text<b id=b>bold</b></p><!--c--><span id=s></span>';
  const pristine = host.querySelector('#a').cloneNode(true);
  const p = host.querySelector('#a'), text = p.firstChild, comment = host.childNodes[1], span = host.querySelector('#s');
  const doctype = document.implementation.createDocumentType('html', '-//W3C//DTD XHTML 1.0//EN', 'http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd');
  const plain = {}, sym = Symbol('s');
  const P = (proto, name) => proto[name];
  const N = Node.prototype, E = Element.prototype, H = HTMLElement.prototype;

  // Methods called on a non-node receiver and on a node of the wrong kind.
  const receivers = {
    'Node.appendChild': [N, 'appendChild', [span]],
    'Node.removeChild': [N, 'removeChild', [span]],
    'Node.insertBefore': [N, 'insertBefore', [span, null]],
    'Node.replaceChild': [N, 'replaceChild', [span, span]],
    'Node.contains': [N, 'contains', [span]],
    'Node.hasChildNodes': [N, 'hasChildNodes', []],
    'Node.cloneNode': [N, 'cloneNode', []],
    'Node.getRootNode': [N, 'getRootNode', []],
    'Element.setAttribute': [E, 'setAttribute', ['x', 'y']],
    'Element.getAttribute': [E, 'getAttribute', ['x']],
    'Element.getAttributeNode': [E, 'getAttributeNode', ['x']],
    'Element.getAttributeNodeNS': [E, 'getAttributeNodeNS', [null, 'x']],
    'Element.setAttributeNode': [E, 'setAttributeNode', [document.createAttribute('q')]],
    'Element.setAttributeNodeNS': [E, 'setAttributeNodeNS', [document.createAttribute('q')]],
    'Element.removeAttributeNode': [E, 'removeAttributeNode', [document.createAttribute('q')]],
    'Element.hasAttribute': [E, 'hasAttribute', ['x']],
    'Element.removeAttribute': [E, 'removeAttribute', ['x']],
    'Element.querySelector': [E, 'querySelector', ['b']],
    'Element.querySelectorAll': [E, 'querySelectorAll', ['b']],
    'Element.matches': [E, 'matches', ['p']],
    'Element.closest': [E, 'closest', ['p']],
    'Element.append': [E, 'append', ['x']],
    'Element.prepend': [E, 'prepend', ['x']],
    'Element.remove': [E, 'remove', []],
    'Element.before': [E, 'before', ['x']],
    'Element.after': [E, 'after', ['x']],
    'Element.replaceWith': [E, 'replaceWith', ['x']],
    'Element.replaceChildren': [E, 'replaceChildren', ['x']],
    'Element.insertAdjacentElement': [E, 'insertAdjacentElement', ['beforeend', document.createElement('i')]],
    'Element.insertAdjacentHTML': [E, 'insertAdjacentHTML', ['beforeend', '<i></i>']],
    'Element.toggleAttribute': [E, 'toggleAttribute', ['x']],
    'Element.getAttributeNS': [E, 'getAttributeNS', [null, 'x']],
    'Element.hasAttributeNS': [E, 'hasAttributeNS', [null, 'x']],
    'Element.setAttributeNS': [E, 'setAttributeNS', [null, 'x', 'y']],
    'Element.removeAttributeNS': [E, 'removeAttributeNS', [null, 'x']],
    'Element.scrollIntoView': [E, 'scrollIntoView', []],
    'Element.scrollTo': [E, 'scrollTo', []],
    'Element.scrollBy': [E, 'scrollBy', []],
    'Element.getBoundingClientRect': [E, 'getBoundingClientRect', []],
    'Element.attachShadow': [E, 'attachShadow', [{ mode: 'open' }]],
    'HTMLElement.focus': [H, 'focus', []],
    'HTMLElement.blur': [H, 'blur', []],
    'HTMLElement.click': [H, 'click', []],
    'Text.splitText': [Text.prototype, 'splitText', [1]],
    'HTMLInputElement.checkValidity': [HTMLInputElement.prototype, 'checkValidity', []],
    'HTMLInputElement.reportValidity': [HTMLInputElement.prototype, 'reportValidity', []],
    'HTMLInputElement.setCustomValidity': [HTMLInputElement.prototype, 'setCustomValidity', ['x']],
    'HTMLInputElement.select': [HTMLInputElement.prototype, 'select', []],
    'HTMLInputElement.setSelectionRange': [HTMLInputElement.prototype, 'setSelectionRange', [0, 0]],
    'HTMLInputElement.setRangeText': [HTMLInputElement.prototype, 'setRangeText', ['x']],
    'EventTarget.addEventListener': [EventTarget.prototype, 'addEventListener', ['x', () => {}]],
    'EventTarget.removeEventListener': [EventTarget.prototype, 'removeEventListener', ['x', () => {}]],
    'EventTarget.dispatchEvent': [EventTarget.prototype, 'dispatchEvent', [new Event('x')]],
  };
  out.receivers = {};
  for (const [label, [proto, name, args]] of Object.entries(receivers)) {
    const method = P(proto, name);
    out.receivers[label] = [run(() => method.apply(plain, args)), run(() => show(method.apply(comment.cloneNode(), args)))];
  }

  // Accessors read from a plain object and from a node of the wrong kind.
  const getters = {
    Node: ['nodeType', 'nodeName', 'isConnected', 'ownerDocument', 'parentNode', 'parentElement', 'childNodes', 'firstChild', 'nextSibling', 'nodeValue', 'textContent'],
    Element: ['namespaceURI', 'prefix', 'localName', 'tagName', 'id', 'className', 'attributes', 'innerHTML', 'outerHTML', 'children', 'firstElementChild', 'shadowRoot'],
    HTMLElement: ['style', 'dataset', 'contentEditable', 'isContentEditable'],
    CharacterData: ['data'],
    DocumentType: ['publicId', 'systemId', 'name'],
    Attr: ['name', 'value', 'ownerElement', 'specified', 'namespaceURI', 'prefix', 'localName'],
  };
  out.getters = {};
  for (const [owner, names] of Object.entries(getters)) {
    for (const name of names) {
      const get = Object.getOwnPropertyDescriptor(globalThis[owner].prototype, name).get;
      out.getters[owner + '.' + name] = [run(() => get.call(plain)), run(() => show(get.call(owner === 'Node' ? comment : comment.cloneNode())))];
    }
  }

  // Argument conversion: too few arguments, and Symbols where strings are expected.
  out.arguments = {
    setAttribute1: run(() => p.setAttribute('x')),
    setAttributeSym: run(() => p.setAttribute(sym, 'v')),
    setAttributeValueSym: run(() => p.setAttribute('x', sym)),
    getAttributeNone: run(() => p.getAttribute()),
    getAttributeNS1: run(() => p.getAttributeNS(null)),
    getAttributeNSSym: run(() => p.getAttributeNS(sym, 'x')),
    hasAttributeNS1: run(() => p.hasAttributeNS(null)),
    hasAttributeNSSym: run(() => p.hasAttributeNS(null, sym)),
    setAttributeNS2: run(() => p.setAttributeNS(null, 'x')),
    setAttributeNSSym: run(() => p.setAttributeNS(null, sym, 'v')),
    removeAttributeNS1: run(() => p.removeAttributeNS(null)),
    removeAttributeNSSym: run(() => p.removeAttributeNS(sym, 'x')),
    removeAttributeNone: run(() => p.removeAttribute()),
    removeAttributeSym: run(() => p.removeAttribute(sym)),
    hasAttributeNone: run(() => p.hasAttribute()),
    toggleAttributeNone: run(() => p.toggleAttribute()),
    toggleAttributeBad: run(() => p.toggleAttribute('a b')),
    getAttributeNodeNone: run(() => p.getAttributeNode()),
    getAttributeNodeNS1: run(() => p.getAttributeNodeNS(null)),
    setAttributeNodeNone: run(() => p.setAttributeNode()),
    setAttributeNodePlain: run(() => p.setAttributeNode(plain)),
    setAttributeNodeElement: run(() => p.setAttributeNode(span)),
    setAttributeNodeNSPlain: run(() => p.setAttributeNodeNS(plain)),
    removeAttributeNodeNone: run(() => p.removeAttributeNode()),
    removeAttributeNodePlain: run(() => p.removeAttributeNode(plain)),
    removeAttributeNodeElement: run(() => p.removeAttributeNode(span)),
    removeAttributeNodeForeign: run(() => p.removeAttributeNode(document.createAttribute('title'))),
    querySelectorNone: run(() => show(p.querySelector())),
    querySelectorSym: run(() => p.querySelector(sym)),
    querySelectorBad: run(() => p.querySelector('[')),
    querySelectorAllNone: run(() => show(p.querySelectorAll())),
    querySelectorAllBad: run(() => p.querySelectorAll('[')),
    matchesNone: run(() => p.matches()),
    matchesBad: run(() => p.matches('[')),
    closestNone: run(() => show(p.closest())),
    closestBad: run(() => p.closest('[')),
    closestSym: run(() => p.closest(sym)),
    containsNone: run(() => p.contains()),
    containsPlain: run(() => p.contains(plain)),
    containsNull: run(() => p.contains(null)),
    containsSelf: run(() => p.contains(p)),
    containsText: run(() => p.contains(text)),
    containsParent: run(() => p.contains(host)),
    appendChildNone: run(() => p.appendChild()),
    appendChildPlain: run(() => p.appendChild(plain)),
    removeChildNone: run(() => p.removeChild()),
    removeChildPlain: run(() => p.removeChild(plain)),
    removeChildStranger: run(() => p.removeChild(span)),
    insertBefore1: run(() => p.insertBefore(span)),
    insertBeforePlain: run(() => p.insertBefore(plain, null)),
    insertBeforeBadRef: run(() => p.insertBefore(document.createElement('i'), plain)),
    replaceChild1: run(() => p.replaceChild(span)),
    replaceChildPlain: run(() => p.replaceChild(plain, text)),
    replaceChildBadOld: run(() => p.replaceChild(document.createElement('i'), plain)),
    insertAdjacentElement1: run(() => p.insertAdjacentElement('beforeend')),
    insertAdjacentElementPlain: run(() => p.insertAdjacentElement('beforeend', plain)),
    insertAdjacentElementBadWhere: run(() => p.insertAdjacentElement('nowhere', document.createElement('i'))),
    insertAdjacentElementSym: run(() => p.insertAdjacentElement(sym, document.createElement('i'))),
    insertAdjacentHTML1: run(() => p.insertAdjacentHTML('beforeend')),
    insertAdjacentHTMLBadWhere: run(() => p.insertAdjacentHTML('nowhere', '<i></i>')),
    insertAdjacentHTMLSym: run(() => p.insertAdjacentHTML('beforeend', sym)),
    insertAdjacentHTMLWhereSym: run(() => p.insertAdjacentHTML(sym, '')),
    replaceChildrenPlain: run(() => document.createElement('i').replaceChildren(plain, 'x').length),
    appendSym: run(() => document.createElement('i').append(sym)),
    prependSym: run(() => document.createElement('i').prepend(sym)),
    beforeSym: run(() => p.before(sym)),
    afterSym: run(() => p.after(sym)),
    replaceWithSym: run(() => document.createElement('i').replaceWith(sym)),
    replaceChildrenSym: run(() => document.createElement('i').replaceChildren(sym)),
    attachShadowNone: run(() => document.createElement('div').attachShadow()),
    attachShadowNull: run(() => document.createElement('div').attachShadow(null)),
    attachShadowNumber: run(() => document.createElement('div').attachShadow(5)),
    attachShadowBadMode: run(() => document.createElement('div').attachShadow({ mode: 'x' })),
    attachShadowNoMode: run(() => document.createElement('div').attachShadow({})),
    attachShadowSlotNamed: run(() => document.createElement('div').attachShadow({ mode: 'open', slotAssignment: 'named' }).slotAssignment),
    attachShadowSlotBad: run(() => document.createElement('div').attachShadow({ mode: 'open', slotAssignment: 'x' })),
    attachShadowSpan: run(() => document.createElement('img').attachShadow({ mode: 'open' })),
    attachShadowTwice: run(() => { const d = document.createElement('div'); d.attachShadow({ mode: 'open' }); return d.attachShadow({ mode: 'open' }); }),
    splitTextNone: run(() => document.createTextNode('abc').splitText()),
    splitTextFar: run(() => document.createTextNode('abc').splitText(9)),
    splitTextNeg: run(() => document.createTextNode('abc').splitText(-1)),
    splitTextDetached: run(() => { const t = document.createTextNode('abcd'); const r = t.splitText(1); return [t.data, r.data, r.parentNode]; }),
    cloneNodeComment: run(() => show(comment.cloneNode(true))),
    cloneNodeDeep: run(() => pristine.cloneNode(true).outerHTML),
    cloneNodeShallow: run(() => pristine.cloneNode(false).outerHTML),
    hasChildNodesText: run(() => text.hasChildNodes()),
    hasChildNodesP: run(() => p.hasChildNodes()),
    hasChildNodesEmpty: run(() => span.hasChildNodes()),
    getRootNodeDetached: run(() => show(document.createElement('i').getRootNode())),
    addEventListener1: run(() => p.addEventListener('x')),
    removeEventListener1: run(() => p.removeEventListener('x')),
    dispatchEventNone: run(() => p.dispatchEvent()),
    dispatchEventPlain: run(() => p.dispatchEvent(plain)),
    dispatchEventNull: run(() => p.dispatchEvent(null)),
    dispatchEventTwice: run(() => { const e = new Event('x'); let inner; p.addEventListener('x', () => { inner = run(() => p.dispatchEvent(e)); }, { once: true }); p.dispatchEvent(e); return inner; }),
  };

  // Attribute nodes: ownership, replacement and removal.
  out.attrs = {};
  {
    const el = document.createElement('div'), other = document.createElement('div');
    el.setAttribute('a', '1');
    const a = el.getAttributeNode('a');
    out.attrs.inUse = run(() => other.setAttributeNode(a));
    out.attrs.sameAgain = run(() => show(el.setAttributeNode(a)));
    const replacement = document.createAttribute('a'); replacement.value = '2';
    out.attrs.replaced = run(() => show(el.setAttributeNode(replacement)));
    out.attrs.oldOwner = [show(a.ownerElement), a.value];
    out.attrs.value = el.getAttribute('a');
    const ns = document.createAttributeNS('urn:x', 'p:n'); ns.value = 'v';
    out.attrs.nsSet = run(() => show(el.setAttributeNodeNS(ns)));
    out.attrs.nsGet = run(() => show(el.getAttributeNodeNS('urn:x', 'n')));
    out.attrs.nsGetMissing = run(() => el.getAttributeNodeNS('urn:y', 'n'));
    out.attrs.nsHas = [el.hasAttributeNS('urn:x', 'n'), el.hasAttributeNS(null, 'n'), el.hasAttributeNS('', 'a')];
    out.attrs.nsGetValue = [el.getAttributeNS('urn:x', 'n'), el.getAttributeNS(null, 'p:n'), el.getAttributeNS('', 'a')];
    out.attrs.removed = run(() => show(el.removeAttributeNode(replacement)));
    out.attrs.removedOwner = show(replacement.ownerElement);
    out.attrs.removedAgain = run(() => el.removeAttributeNode(replacement));
    out.attrs.removeNS = run(() => { el.removeAttributeNS('urn:x', 'n'); return el.attributes.length; });
    out.attrs.removeNSMissing = run(() => { el.removeAttributeNS('urn:x', 'n'); return el.attributes.length; });
    out.attrs.specified = [a.specified, ns.specified];
    out.attrs.valueSet = run(() => { replacement.value = 'z'; return [replacement.value, el.getAttribute('a')]; });
    out.attrs.ownedValueSet = run(() => { el.setAttributeNode(replacement); replacement.value = 'w'; return el.getAttribute('a'); });
    out.attrs.createAttributeBad = run(() => document.createAttribute('a b'));
    out.attrs.createAttributeNone = run(() => document.createAttribute());
    out.attrs.createAttributeUpper = run(() => document.createAttribute('ABC').name);
    out.attrs.createAttributeNSBad = run(() => document.createAttributeNS('urn:x', '1x'));
    out.attrs.createAttributeNSXmlns = run(() => document.createAttributeNS('urn:x', 'xmlns'));
    out.attrs.createAttributeNSNone = run(() => document.createAttributeNS('urn:x'));
    out.attrs.setAttributeNSPrefixNoNs = run(() => el.setAttributeNS(null, 'p:x', 'v'));
    out.attrs.setAttributeNSXml = run(() => el.setAttributeNS('urn:x', 'xml:x', 'v'));
    out.attrs.setAttributeNSBadName = run(() => el.setAttributeNS(null, '1', 'v'));
    out.attrs.setAttributeBadName = run(() => el.setAttribute('1 2', 'v'));
    out.attrs.setAttributeEmpty = run(() => el.setAttribute('', 'v'));
    out.attrs.upperNS = run(() => { el.setAttributeNS(null, 'MiXed', 'v'); return [el.getAttribute('mixed'), el.getAttribute('MiXed'), el.getAttributeNS(null, 'MiXed')]; });
    out.attrs.htmlLower = run(() => { el.setAttribute('UP', '1'); return [el.getAttribute('up'), el.attributes[el.attributes.length - 1].name]; });
  }

  // DocumentType and node names.
  out.doctype = [doctype.name, doctype.publicId, doctype.systemId, doctype.nodeName, doctype.nodeValue, doctype.textContent];
  out.docDoctype = document.doctype ? [document.doctype.name, document.doctype.publicId, document.doctype.systemId] : null;
  out.names = [
    document.createProcessingInstruction('pi', 'd').nodeName,
    document.createComment('c').nodeName,
    document.createDocumentFragment().nodeName,
    document.createAttribute('Ab').nodeName,
    document.createAttributeNS('urn:x', 'p:Ab').nodeName,
    document.createElementNS('urn:x', 'p:Ab').nodeName,
    document.createElementNS('http://www.w3.org/1999/xhtml', 'p:ab').nodeName,
    document.nodeName,
    document.createElement('div').attachShadow({ mode: 'open' }).nodeName,
  ];

  // dataset reads and writes.
  out.dataset = [Object.entries(p.dataset), run(() => { p.dataset.newKey = 'n'; return p.getAttribute('data-new-key'); }), run(() => { p.dataset['bad-Key'] = 1; }), run(() => delete p.dataset.oneTwo), p.hasAttribute('data-one-two')];

  // contentEditable.
  out.contentEditable = [span.contentEditable, run(() => { span.contentEditable = 'TRUE'; return [span.contentEditable, span.getAttribute('contenteditable'), span.isContentEditable]; }), run(() => { span.contentEditable = 'plaintext-only'; return span.contentEditable; }), run(() => { span.contentEditable = 'bogus'; }), span.getAttribute('contenteditable'), run(() => { span.contentEditable = 'inherit'; return [span.contentEditable, span.hasAttribute('contenteditable')]; })];

  // Collections: item() / namedItem().
  {
    const form = document.createElement('form');
    form.innerHTML = '<input name=n1 id=i1><input name=n2><img name=im>';
    const children = form.children, nodes = form.childNodes, all = form.querySelectorAll('input');
    out.collections = {
      childrenItem: [show(children.item(0)), children.item(-1), children.item(9), show(children.item('1')), children.item(NaN) && tag(children.item(NaN)), show(children.item(4294967296))],
      childrenNamed: [show(children.namedItem('n2')), show(children.namedItem('i1')), children.namedItem('none'), show(children.namedItem('im')), run(() => children.namedItem())],
      nodesItem: [show(nodes.item(0)), nodes.item(-1), nodes.item(7)],
      allItem: [show(all.item(1)), all.item(5), run(() => all.item())],
      docGetsByName: [document.getElementsByName('none').length],
    };
  }

  // getBoundingClientRect / scroll family results on detached elements.
  out.geometry = run(() => { const r = document.createElement('i').getBoundingClientRect(); return [r.x, r.y, r.width, r.height, r.top, r.left, r.right, r.bottom]; });
  out.noops = [run(() => span.scrollIntoView()), run(() => span.scrollIntoView(true)), run(() => span.scrollIntoView({ block: 'center' }))];
  out.validity = (() => { const i = document.createElement('input'); i.required = true; return [i.checkValidity(), run(() => i.setCustomValidity('x')), i.validationMessage, i.checkValidity()]; })();
  // Event listener registration and dispatch edges.
  {
    const outer = document.body.appendChild(document.createElement('div')), inner = outer.appendChild(document.createElement('span'));
    const fire = type => inner.dispatchEvent(new Event(type, { bubbles: true }));
    const log = [];
    const throwing = { get capture() { throw new RangeError('options'); } };
    out.events = {
      throwingOptions: run(() => inner.addEventListener('a', () => {}, throwing)),
      throwingRemove: run(() => inner.removeEventListener('a', () => {}, throwing)),
      nullCallback: run(() => inner.addEventListener('a', null)),
      nullRemove: run(() => inner.removeEventListener('a', null)),
      abortedSignal: (() => { const controller = new AbortController(); controller.abort(); let calls = 0; inner.addEventListener('b', () => calls++, { signal: controller.signal }); fire('b'); return calls; })(),
      abortDuring: (() => { const controller = new AbortController(); const calls = []; inner.addEventListener('c', () => { calls.push(1); controller.abort(); }); inner.addEventListener('c', () => calls.push(2), { signal: controller.signal }); fire('c'); fire('c'); return calls; })(),
      stopInCapture: (() => { log.length = 0; outer.addEventListener('d', event => { log.push('outer-capture'); event.stopPropagation(); }, true); inner.addEventListener('d', () => log.push('inner')); outer.addEventListener('d', () => log.push('outer-bubble')); fire('d'); return log.slice(); })(),
      stopImmediate: (() => { log.length = 0; inner.addEventListener('e', event => { log.push(1); event.stopImmediatePropagation(); }); inner.addEventListener('e', () => log.push(2)); outer.addEventListener('e', () => log.push(3)); fire('e'); return log.slice(); })(),
      once: (() => { let calls = 0; inner.addEventListener('f', () => calls++, { once: true }); fire('f'); fire('f'); return calls; })(),
      handleEvent: (() => { const listener = { calls: 0, handleEvent(event) { this.calls++; this.self = this === listener; } }; inner.addEventListener('g', listener); fire('g'); return [listener.calls, listener.self]; })(),
      duplicate: (() => { let calls = 0; const f = () => calls++; inner.addEventListener('h', f); inner.addEventListener('h', f); inner.addEventListener('h', f, true); fire('h'); return calls; })(),
      removeCaptureOnly: (() => { let calls = 0; const f = () => calls++; inner.addEventListener('i', f); inner.addEventListener('i', f, true); inner.removeEventListener('i', f, { capture: true }); fire('i'); return calls; })(),
    };
    outer.remove();
  }
  // Attribute nodes carrying style / nonce, foreign-document attributes, removeAttribute('nonce').
  {
    const el = document.body.appendChild(document.createElement('div'));
    const color = () => getComputedStyle(el).color;
    const other = document.implementation.createHTMLDocument('');
    const style = document.createAttribute('style'); style.value = 'color: rgb(1, 2, 3)';
    const nonce = document.createAttribute('nonce'); nonce.value = 'n1';
    const foreign = other.createAttribute('data-f'); foreign.value = 'f';
    out.attrNodes = {
      setNumber: run(() => el.setAttributeNode(5)),
      removeString: run(() => el.removeAttributeNode('x')),
      styleSet: [run(() => show(el.setAttributeNode(style))), color(), el.style.color],
      styleRemoved: [run(() => show(el.removeAttributeNode(style))), color(), el.style.color],
      nonceSet: [run(() => show(el.setAttributeNode(nonce))), el.nonce],
      nonceRemoved: [run(() => show(el.removeAttributeNode(nonce))), el.nonce, el.hasAttribute('nonce')],
      foreignSet: [run(() => show(el.setAttributeNode(foreign))), foreign.ownerDocument === document, el.getAttribute('data-f')],
      foreignRemove: run(() => el.removeAttributeNode(other.createAttribute('data-f'))),
      removeNonceAttribute: (() => { el.setAttribute('nonce', 'n2'); const before = el.nonce; el.removeAttribute('nonce'); return [before, el.nonce]; })(),
      setNonceNS: (() => { el.setAttributeNS(null, 'nonce', 'n3'); const before = el.nonce; el.removeAttributeNS(null, 'nonce'); return [before, el.nonce]; })(),
      styleNS: (() => { el.setAttributeNS(null, 'style', 'color: rgb(4, 5, 6)'); const before = color(); el.removeAttributeNS(null, 'style'); return [before, color()]; })(),
    };
    el.remove();
  }
  // Importing and adopting nodes of every kind from another document.
  {
    const other = document.implementation.createHTMLDocument('t');
    const kinds = {
      attr: () => { const a = other.createAttribute('Data-A'); a.value = 'v'; return a; },
      attrNS: () => { const a = other.createAttributeNS('urn:x', 'p:q'); a.value = 'w'; return a; },
      pi: () => other.createProcessingInstruction('target', 'data'),
      comment: () => other.createComment('note'),
      text: () => other.createTextNode('words'),
      doctype: () => other.implementation.createDocumentType('html', 'pub', 'sys'),
      fragment: () => { const f = other.createDocumentFragment(); f.append(other.createElement('i'), 'x'); return f; },
      element: () => { const e = other.createElement('b'); e.setAttribute('k', 'v'); e.textContent = 't'; return e; },
      shadow: () => other.createElement('div').attachShadow({ mode: 'open' }),
      document: () => other,
    };
    const describe = node => node == null ? null : [node.nodeType, node.nodeName, node.nodeValue, node.ownerDocument === document, node.namespaceURI === undefined ? 'undefined' : node.namespaceURI, node.prefix === undefined ? 'undefined' : node.prefix, node.localName === undefined ? 'undefined' : node.localName, node.nodeType === 10 ? [node.publicId, node.systemId] : node.nodeType === 11 ? node.childNodes.length : node.nodeType === 1 ? node.outerHTML : null];
    out.imports = {};
    for (const [name, make] of Object.entries(kinds)) {
      out.imports[name] = [run(() => describe(document.importNode(make(), true))), run(() => describe(document.importNode(make(), false))), run(() => { const node = make(); const adopted = document.adoptNode(node); return [describe(adopted), adopted === node]; })];
    }
  }
  // HTMLInputElement checked / defaultChecked brand checks.
  {
    const proto = HTMLInputElement.prototype, input = document.createElement('input');
    out.checkedBrands = {};
    for (const name of ['checked', 'defaultChecked']) {
      const { get, set } = Object.getOwnPropertyDescriptor(proto, name);
      out.checkedBrands[name] = [plain, 5, document.createElement('div'), input].map(receiver => [run(() => get.call(receiver)), run(() => set.call(receiver, true))]);
    }
  }
  // Argument conversions that throw: Symbols and throwing toString in every position.
  {
    const el = document.createElement('div'), bad = { toString() { throw new RangeError('toString'); } };
    el.setAttribute('a', '1');
    const attr = el.getAttributeNode('a');
    const cases = {
      getAttribute: v => el.getAttribute(v),
      hasAttribute: v => el.hasAttribute(v),
      getAttributeNode: v => el.getAttributeNode(v),
      toggleAttribute: v => el.toggleAttribute(v),
      getAttributeNSLocal: v => el.getAttributeNS(null, v),
      hasAttributeNSNamespace: v => el.hasAttributeNS(v, 'a'),
      hasAttributeNSLocal: v => el.hasAttributeNS(null, v),
      setAttributeNSNamespace: v => el.setAttributeNS(v, 'a', 'v'),
      setAttributeNSValue: v => el.setAttributeNS(null, 'a', v),
      removeAttributeNSLocal: v => el.removeAttributeNS(null, v),
      getAttributeNodeNSNamespace: v => el.getAttributeNodeNS(v, 'a'),
      getAttributeNodeNSLocal: v => el.getAttributeNodeNS(null, v),
      createAttribute: v => document.createAttribute(v),
      createAttributeNSNamespace: v => document.createAttributeNS(v, 'a'),
      createAttributeNSName: v => document.createAttributeNS('urn:x', v),
      attrValue: v => { attr.value = v; },
      splitText: v => document.createTextNode('abc').splitText(v),
      outerHTML: v => { document.createElement('p').appendChild(document.createElement('i')).outerHTML = v; },
    };
    out.conversions = {};
    for (const [name, call] of Object.entries(cases)) out.conversions[name] = [run(() => call(sym)), run(() => call(bad))];
    out.conversions.after = [el.getAttribute('a'), el.attributes.length];
    out.conversions.insertAdjacentString = run(() => el.insertAdjacentElement('beforeend', 'text'));
  }
  host.remove();
  return out;
})()
