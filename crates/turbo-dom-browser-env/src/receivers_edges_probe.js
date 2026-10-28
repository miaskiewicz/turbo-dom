(async () => {
  // Brand checks of ShadowRoot / Document / slot / text members implemented natively: each
  // accessor and method called on primitives, plain objects, the wrong node kind and the
  // right one. Records the exception name (or the result type).
  const host = document.body.appendChild(document.createElement('div'));
  const root = host.attachShadow({ mode: 'open', clonable: true, serializable: true });
  root.innerHTML = '<slot></slot><p>x</p>';
  const receivers = { number: 1, string: 's', nullish: null, plain: {}, element: host, text: document.createTextNode('t'), document, root, fragment: document.createDocumentFragment(), slot: root.firstChild, nodeList: document.childNodes, implementation: document.implementation };
  const describe = value => value === null ? 'null' : value === undefined ? 'undefined' : typeof value === 'object' ? (value.nodeName || 'object') : typeof value;
  const call = run => { try { return describe(run()); } catch (e) { return e.name; } };
  const out = {};
  const members = [
    [ShadowRoot.prototype, ['clonable', 'serializable', 'delegatesFocus', 'activeElement', 'pointerLockElement', 'fullscreenElement', 'slotAssignment', 'mode', 'host', 'innerHTML'], ['getHTML', 'setHTMLUnsafe', 'setHTML']],
    [Document.prototype, ['activeElement', 'pointerLockElement', 'fullscreenElement', 'doctype', 'documentElement'], ['importNode', 'adoptNode', 'createProcessingInstruction', 'moveBefore', 'normalize']],
    [HTMLSlotElement.prototype, ['name'], ['assignedNodes', 'assignedElements', 'assign']],
    [Text.prototype, ['assignedSlot', 'wholeText'], ['splitText']],
    [Element.prototype, ['assignedSlot', 'innerHTML', 'outerHTML'], ['getHTML', 'setHTMLUnsafe', 'setHTML', 'moveBefore']],
    [CharacterData.prototype, [], ['replaceData', 'appendData']],
    [DOMImplementation.prototype, [], ['createHTMLDocument', 'createDocument', 'createDocumentType', 'hasFeature']],
    [Document.prototype, ['contentType', 'URL', 'characterSet', 'baseURI', 'implementation'], []],
  ];
  for (const [prototype, getters, methods] of members) {
    const name = prototype.constructor.name;
    for (const getter of getters) {
      const descriptor = Object.getOwnPropertyDescriptor(prototype, getter);
      if (!descriptor || !descriptor.get) { out[name + '.' + getter] = 'missing'; continue; }
      const row = {};
      for (const [kind, receiver] of Object.entries(receivers)) row[kind] = call(() => descriptor.get.call(receiver));
      out[name + '.' + getter] = row;
    }
    for (const method of methods) {
      const fn = prototype[method];
      if (typeof fn !== 'function') { out[name + '.' + method + '()'] = 'missing'; continue; }
      const row = {};
      for (const [kind, receiver] of Object.entries(receivers)) row[kind] = call(() => fn.call(receiver, document.createElement('i'), null));
      out[name + '.' + method + '()'] = row;
    }
  }
  return out;
})()
