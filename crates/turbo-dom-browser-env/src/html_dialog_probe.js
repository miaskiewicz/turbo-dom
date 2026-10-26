(async () => {
  // HTMLDialogElement control: descriptors, brands, reflection, exceptions, state transitions
  // with sync/microtask/task-phased event logs, focusing steps, modal inertness, popover
  // interplay, :modal/:open matching, form method=dialog and dialog invoker commands.
  // Chrome dispatches `close` as an animation-frame event and blurs a closed, focused dialog
  // during focus fixup; neither is ordered against tasks here: close events are compared as a
  // sorted settled set, and every focus case first focuses an element outside the dialog.
  const out = {}, body = document.body, HTML = 'http://www.w3.org/1999/xhtml';
  const P = HTMLDialogElement.prototype;
  const tick = () => new Promise(resolve => setTimeout(resolve, 0));
  const frame = () => new Promise(resolve => requestAnimationFrame(resolve));
  const settle = async () => { await tick(); await frame(); await frame(); await tick(); };
  const name = node => node === null ? null : node === undefined ? 'undefined' : node === document.body ? 'body' : node.id || node.localName;
  const run = f => { try { const value = f(); return value === undefined ? 'ok' : ['ok', value]; } catch (error) { return error.name + ': ' + error.message; } };
  const errorName = f => { try { f(); return 'ok'; } catch (error) { return error.name; } };
  const q = selector => body.querySelector(selector);
  const fn = value => typeof value === 'function' ? [value.name, value.length] : value;

  out.interface = {
    ownKeys: Reflect.ownKeys(P).map(String),
    chain: Object.getPrototypeOf(P) === HTMLElement.prototype,
    tag: Object.prototype.toString.call(document.createElement('dialog')),
    instance: document.createElement('dialog') instanceof HTMLDialogElement,
    parsed: (() => { body.innerHTML = '<dialog></dialog>'; return Object.getPrototypeOf(body.firstChild) === P; })(),
    svg: document.createElementNS('http://www.w3.org/2000/svg', 'dialog') instanceof HTMLDialogElement,
    foreign: document.createElementNS('urn:x', 'dialog') instanceof HTMLDialogElement,
    construct: errorName(() => new HTMLDialogElement()),
    length: HTMLDialogElement.length,
  };
  out.descriptors = {};
  for (const key of Reflect.ownKeys(P)) {
    if (key === Symbol.toStringTag) continue;
    const descriptor = Object.getOwnPropertyDescriptor(P, key), row = {};
    for (const field of Object.keys(descriptor)) row[field] = fn(descriptor[field]);
    out.descriptors[String(key)] = row;
  }

  out.brands = {};
  const receivers = { div: document.createElement('div'), svg: document.createElementNS('http://www.w3.org/2000/svg', 'dialog'), object: {}, proto: P, text: document.createTextNode('') };
  for (const [label, receiver] of Object.entries(receivers)) {
    const row = [];
    for (const key of ['open', 'returnValue', 'closedBy']) { const descriptor = Object.getOwnPropertyDescriptor(P, key); row.push(run(() => descriptor.get.call(receiver)), run(() => descriptor.set.call(receiver, 'x'))); }
    for (const key of ['close', 'requestClose', 'show', 'showModal']) row.push(run(() => P[key].call(receiver)));
    out.brands[label] = row;
  }
  // Brand checks precede argument conversion.
  out.brands.order = [run(() => P.close.call({}, Symbol())), run(() => Object.getOwnPropertyDescriptor(P, 'returnValue').set.call({}, Symbol()))];

  out.reflection = {};
  {
    const d = document.createElement('dialog'), rows = [];
    for (const value of [true, 0, 'yes', '', null, {}, 1n]) { d.open = value; rows.push([typeof value === 'bigint' ? 'bigint' : String(value), d.getAttribute('open'), d.open]); }
    out.reflection.open = rows;
    d.setAttribute('open', 'false'); out.reflection.openAttribute = d.open;
    d.removeAttribute('open'); d.setAttributeNS('urn:x', 'open', ''); out.reflection.foreignOpen = d.open; d.removeAttributeNS('urn:x', 'open');
    out.reflection.closedBySetter = [];
    for (const value of [null, undefined, 'ANY', 'closeRequest', 'bogus', 42]) { d.closedBy = value; out.reflection.closedBySetter.push([String(value), d.getAttribute('closedby'), d.closedBy]); }
    out.reflection.closedBySymbol = run(() => { d.closedBy = Symbol(); });
    body.append(d); const records = [], observer = new MutationObserver(list => { for (const record of list) records.push([record.attributeName, record.oldValue]); });
    observer.observe(d, { attributes: true, attributeOldValue: true });
    d.show(); d.close(); d.showModal(); d.requestClose(); d.open = true; d.open = false;
    records.push(...observer.takeRecords().map(record => [record.attributeName, record.oldValue])); observer.disconnect(); d.remove();
    out.reflection.mutations = records;
  }
  out.reflection.closedBy = {};
  for (const value of [null, '', 'any', 'ANY', ' any', 'closerequest', 'CloseRequest', 'none', 'NONE', 'auto', 'bogus']) {
    const d = document.createElement('dialog'); if (value !== null) d.setAttribute('closedby', value); body.append(d);
    const row = [d.closedBy]; d.show(); row.push(d.closedBy); d.close(); d.showModal(); row.push(d.closedBy);
    d.removeAttribute('open'); row.push(d.closedBy); d.remove(); row.push(d.closedBy);
    out.reflection.closedBy[String(value)] = row;
  }
  out.reflection.returnValue = (() => {
    const d = document.createElement('dialog'), rows = [d.returnValue];
    for (const value of [5, null, undefined, { toString() { return 'object'; } }, '']) { d.returnValue = value; rows.push(d.returnValue); }
    rows.push(run(() => { d.returnValue = Symbol(); }), d.returnValue);
    body.append(d); d.returnValue = 'start';
    d.show(); d.close(); rows.push(d.returnValue);
    d.show(); d.close(undefined); rows.push(d.returnValue);
    d.show(); d.close(null); rows.push(d.returnValue);
    d.show(); d.close(7); rows.push(d.returnValue);
    d.close('closed-noop'); rows.push(d.returnValue);
    d.show(); rows.push(run(() => d.close(Symbol())), d.open, d.returnValue);
    d.close({ toString() { return 'converted'; } }); rows.push(d.returnValue);
    d.show(); d.requestClose(); rows.push(d.open, d.returnValue);
    d.show(); d.requestClose(undefined); rows.push(d.returnValue);
    d.show(); d.requestClose(null); rows.push(d.returnValue);
    d.show(); rows.push(run(() => d.requestClose(Symbol())), d.open);
    d.requestClose(42); rows.push(d.open, d.returnValue);
    d.requestClose('closed-noop'); rows.push(d.returnValue);
    const clone = d.cloneNode(); rows.push(clone.returnValue);
    d.remove();
    return rows;
  })();

  // Exception table: [state, show(), showModal()] with full DOMException name and message.
  out.exceptions = {};
  {
    const states = {
      closedConnected: () => { const d = document.createElement('dialog'); body.append(d); return d; },
      closedDisconnected: () => document.createElement('dialog'),
      openNonModal: () => { const d = document.createElement('dialog'); body.append(d); d.show(); return d; },
      openAttribute: () => { const d = document.createElement('dialog'); d.setAttribute('open', ''); body.append(d); return d; },
      openModal: () => { const d = document.createElement('dialog'); body.append(d); d.showModal(); return d; },
      openDisconnected: () => { const d = document.createElement('dialog'); d.show(); return d; },
      modalAttributeRemoved: () => { const d = document.createElement('dialog'); body.append(d); d.showModal(); d.removeAttribute('open'); return d; },
      modalRemoved: () => { const d = document.createElement('dialog'); body.append(d); d.showModal(); d.remove(); body.append(d); return d; },
      popoverShowing: () => { const d = document.createElement('dialog'); d.popover = 'manual'; body.append(d); d.showPopover(); return d; },
      popoverHidden: () => { const d = document.createElement('dialog'); d.popover = 'manual'; body.append(d); return d; },
      inactiveDocument: () => { const other = document.implementation.createHTMLDocument(''); const d = other.createElement('dialog'); other.body.append(d); return d; },
    };
    for (const [label, make] of Object.entries(states)) {
      let d = make(); const show = run(() => d.show()), showOpen = d.open; d.remove();
      d = make(); const modal = run(() => d.showModal()), modalOpen = d.open, isModal = d.matches(':modal'); d.remove();
      out.exceptions[label] = [show, showOpen, modal, modalOpen, isModal];
    }
    body.replaceChildren(); await settle();
  }

  // Phased event logs. `sync` is everything dispatched during the operation, `microtask` up
  // to a microtask queued after it, `firstTask` the toggle events before a task queued after
  // it, `toggles` every settled toggle in order and `others` the remaining settled events sorted.
  const eventRow = (event, node) => [event.type, node.id || '', event.constructor.name, event.oldState === undefined ? null : event.oldState + '>' + event.newState, event.cancelable, event.bubbles, event.composed, event.isTrusted, event.source === undefined ? 'none' : name(event.source), node.open, node.matches(':modal'), node.matches(':open')];
  const phased = async (html, operation, setup) => {
    body.innerHTML = html;
    const nodes = [...body.querySelectorAll('dialog')], log = [];
    let phase = 'setup';
    const listen = node => { for (const type of ['beforetoggle', 'toggle', 'cancel', 'close']) node.addEventListener(type, event => log.push([phase, eventRow(event, node)])); };
    nodes.forEach(listen);
    const context = { nodes, d: nodes[0], log, listen, mark: label => log.push([phase, label]) };
    if (setup) await setup(context);
    await settle(); log.length = 0;
    phase = 'sync'; const result = run(() => operation(context)); phase = 'microtask';
    let microtaskDone = false, taskDone = false;
    queueMicrotask(() => { microtaskDone = true; phase = 'task'; });
    setTimeout(() => { taskDone = true; phase = 'settled'; }, 0);
    await settle();
    const of = name => log.filter(row => row[0] === name).map(row => row[1]);
    const late = log.filter(row => row[0] === 'task' || row[0] === 'settled');
    return {
      result, microtaskDone, taskDone,
      sync: of('sync'), microtask: of('microtask'),
      firstTask: of('task').filter(row => row[0] === 'toggle'),
      toggles: late.filter(row => row[1][0] === 'toggle').map(row => row[1]),
      others: late.filter(row => row[1][0] !== 'toggle').map(row => JSON.stringify(row[1])).sort(),
      state: nodes.map(node => [node.id, node.open, node.matches(':modal'), node.matches(':open'), node.returnValue, node.closedBy, node.isConnected]),
    };
  };
  const one = '<dialog id=d></dialog>', opened = '<dialog id=d open></dialog>';
  const before = (d, f, options) => d.addEventListener('beforetoggle', f, options);
  const cases = {
    show: [one, ({ d }) => d.show()],
    showModal: [one, ({ d }) => d.showModal()],
    close: [one, ({ d }) => d.close('x'), ({ d }) => d.show()],
    closeModal: [one, ({ d }) => d.close('x'), ({ d }) => d.showModal()],
    closeParsedOpen: [opened, ({ d }) => d.close('p')],
    closeClosed: [one, ({ d }) => d.close('x')],
    showOpen: [one, ({ d }) => d.show(), ({ d }) => d.show()],
    showModalModal: [one, ({ d }) => d.showModal(), ({ d }) => d.showModal()],
    showModalNonModal: [one, ({ d }) => d.showModal(), ({ d }) => d.show()],
    showModalModalRemoved: [one, ({ d }) => d.showModal(), ({ d }) => { d.showModal(); d.remove(); document.body.append(d); }],
    requestClose: [one, ({ d }) => d.requestClose('r'), ({ d }) => d.showModal()],
    requestCloseNonModal: [one, ({ d }) => d.requestClose('r'), ({ d }) => d.show()],
    requestCloseParsedOpen: [opened, ({ d }) => d.requestClose('r')],
    requestCloseAttributeOpen: [one, ({ d }) => d.requestClose('r'), ({ d }) => d.setAttribute('open', '')],
    requestCloseClosedByNone: ['<dialog id=d closedby=none></dialog>', ({ d }) => d.requestClose('r'), ({ d }) => d.showModal()],
    requestCloseClosedByAny: ['<dialog id=d closedby=any></dialog>', ({ d }) => d.requestClose('r'), ({ d }) => d.show()],
    requestCloseClosed: [one, ({ d }) => d.requestClose('r')],
    requestCloseDisconnected: ['', ({ d }) => d.requestClose('r'), context => { const d = document.createElement('dialog'); d.id = 'detached'; d.show(); context.d = d; context.listen(d); }],
    requestCloseReinserted: ['<div id=c><dialog id=d open></dialog></div>', ({ d }) => d.requestClose('r'), ({ d }) => { const c = d.parentNode; c.remove(); d.requestClose('lost'); document.body.append(c); }],
    requestClosePrevented: [one, ({ d }) => { d.requestClose('a'); d.requestClose('b'); d.requestClose('c'); }, ({ d }) => { d.showModal(); d.addEventListener('cancel', event => event.preventDefault()); }],
    requestClosePreventedThenClose: [one, ({ d }) => { d.requestClose('first'); d.close(); }, ({ d }) => { d.showModal(); d.addEventListener('cancel', event => event.preventDefault(), { once: true }); }],
    requestCloseNested: [one, ({ d }) => d.requestClose('outer'), ({ d, mark }) => { d.showModal(); d.addEventListener('cancel', () => { d.requestClose('nested'); mark('nested:' + d.open); }, { once: true }); }],
    requestCloseCloseInCancel: [one, ({ d }) => d.requestClose('outer'), ({ d }) => { d.showModal(); d.addEventListener('cancel', () => d.close('inner'), { once: true }); }],
    requestCloseRemoveInCancel: [one, ({ d }) => d.requestClose('outer'), ({ d }) => { d.showModal(); d.addEventListener('cancel', () => d.remove(), { once: true }); }],
    cancelShow: [one, ({ d }) => d.show(), ({ d }) => before(d, event => event.preventDefault())],
    cancelShowModal: [one, ({ d }) => d.showModal(), ({ d }) => before(d, event => event.preventDefault())],
    removeInBeforetoggleShow: [one, ({ d }) => d.show(), ({ d }) => before(d, () => d.remove(), { once: true })],
    removeInBeforetoggleShowModal: [one, ({ d }) => d.showModal(), ({ d }) => before(d, () => d.remove(), { once: true })],
    openInBeforetoggleShowModal: [one, ({ d }) => d.showModal(), ({ d }) => before(d, () => d.setAttribute('open', ''), { once: true })],
    nestedShowModal: [one, ({ d }) => d.showModal(), ({ d, mark }) => before(d, () => mark(run(() => d.showModal())), { once: true })],
    nestedShow: [one, ({ d }) => d.showModal(), ({ d, mark }) => before(d, () => mark(run(() => d.show())), { once: true })],
    popoverInBeforetoggle: ['<dialog id=d popover=manual></dialog>', ({ d }) => d.showModal(), ({ d }) => before(d, event => { if (event.newState === 'open' && !d.open) d.showPopover(); }, { once: true })],
    closeInBeforetoggle: [one, ({ d }) => d.close('outer'), ({ d, mark }) => { d.show(); before(d, () => mark(run(() => d.close('inner'))), { once: true }); }],
    removeInBeforetoggleClose: [one, ({ d }) => d.close('outer'), ({ d }) => { d.showModal(); before(d, () => d.remove(), { once: true }); }],
    showCloseShow: [one, ({ d }) => { d.show(); d.close('a'); d.show(); }],
    closeShowClose: [one, ({ d }) => { d.close('a'); d.show(); d.close('b'); }, ({ d }) => d.show()],
    requeueAcrossDialogs: ['<dialog id=a></dialog><dialog id=b></dialog>', ({ nodes: [a, b] }) => { a.show(); b.show(); a.close(); }],
    requeueReopen: ['<dialog id=a></dialog><dialog id=b></dialog>', ({ nodes: [a, b] }) => { a.show(); b.show(); a.close(); a.show(); }],
    openAttributeAdded: [one, ({ d }) => { d.open = true; }],
    openAttributeRemoved: [opened, ({ d }) => { d.open = false; }],
    modalOpenAttributeRemoved: [one, ({ d }) => d.removeAttribute('open'), ({ d }) => d.showModal()],
    modalOpenAttributeRestored: [one, ({ d }) => { d.removeAttribute('open'); d.setAttribute('open', ''); d.requestClose('again'); }, ({ d }) => d.showModal()],
    removeModal: [one, ({ d }) => d.remove(), ({ d }) => d.showModal()],
    removeModalParent: ['<div id=p><dialog id=d></dialog></div>', ({ d }) => { d.parentNode.innerHTML = ''; }, ({ d }) => d.showModal()],
    removeModalShadow: ['<div id=host></div>', ({ host, inner, mark }) => { host.remove(); mark('inner:' + inner.open + ':' + inner.matches(':modal') + ':' + inner.closedBy); }, context => { const host = document.getElementById('host'), d = host.attachShadow({ mode: 'open' }).appendChild(document.createElement('dialog')); d.id = 'shadow'; d.showModal(); context.host = host; context.inner = d; context.listen(d); }],
    moveModal: ['<div id=p></div><dialog id=d></dialog>', ({ d }) => document.getElementById('p').append(d), ({ d }) => d.showModal()],
    reinsertedModalClose: [one, ({ d }) => d.close('after'), ({ d }) => { d.showModal(); d.remove(); document.body.append(d); }],
    adoptModal: [one, ({ d }) => document.implementation.createHTMLDocument('').adoptNode(d), ({ d }) => d.showModal()],
    cloneModal: [one, ({ d, mark }) => { const clone = d.cloneNode(true); clone.id = 'clone'; document.body.append(clone); mark(['clone', clone.open, clone.matches(':modal'), clone.returnValue, errorName(() => clone.showModal())]); }, ({ d }) => { d.showModal(); d.returnValue = 'kept'; }],
    requestCloseDetachedThenShown: ['', ({ d, mark }) => { d.requestClose('r'); mark('open:' + d.open); document.body.append(d); d.requestClose('s'); mark('open:' + d.open + ':' + d.returnValue); }, context => { const d = document.createElement('dialog'); d.id = 'late'; d.show(); context.d = d; context.listen(d); }],
  };
  out.transitions = {};
  for (const [label, [html, operation, setup]] of Object.entries(cases)) out.transitions[label] = await phased(html, operation, setup);
  body.replaceChildren(); await settle();

  // Popover interplay: show() and showModal() hide unrelated auto/hint popovers.
  out.popovers = {};
  {
    body.innerHTML = '<div id=p popover=auto></div><div id=m popover=manual></div><div id=h popover=hint></div><dialog id=d><div id=inner popover=auto></div></dialog>';
    const log = [];
    for (const id of ['p', 'm', 'h', 'inner']) q('#' + id).addEventListener('beforetoggle', event => log.push(id + ':' + event.newState));
    const open = id => q('#' + id).matches(':popover-open');
    q('#p').showPopover(); q('#m').showPopover(); q('#d').show(); log.push(['show', open('p'), open('m')]);
    q('#d').close(); q('#p').showPopover(); q('#d').showModal(); log.push(['showModal', open('p'), open('m')]);
    q('#inner').showPopover(); log.push(['inner', open('inner')]); q('#d').close(); log.push(['afterClose', open('inner')]);
    q('#h').showPopover(); q('#d').show(); log.push(['hint', open('h')]); q('#d').close();
    out.popovers.hide = log; await settle();
  }
  {
    body.innerHTML = '<div id=p popover=auto><dialog id=d></dialog></div><div id=q popover=hint><dialog id=e></dialog></div>';
    q('#p').showPopover(); q('#d').showModal(); const row = [q('#p').matches(':popover-open'), q('#d').open]; q('#d').close();
    q('#q').showPopover(); q('#e').show(); row.push(q('#q').matches(':popover-open'), q('#p').matches(':popover-open')); q('#e').close();
    out.popovers.ancestors = row; body.replaceChildren(); await settle();
  }

  // Pseudo-classes.
  out.selectors = {};
  {
    body.innerHTML = '<dialog id=a><button id=ab>a</button></dialog><dialog id=b></dialog><dialog id=c open></dialog><details id=det open><summary>s</summary></details><details id=det2></details><select id=sel><option>1</option></select><div id=plain open></div>';
    const all = selector => [...document.querySelectorAll(selector)].map(name);
    const s = out.selectors;
    s.initial = [all(':modal'), all(':open'), all(':not(:open)').filter(id => /^(a|b|c|det|det2|sel|plain)$/.test(id))];
    q('#a').showModal(); q('#b').showModal();
    s.nested = [all(':modal'), all(':open'), all('dialog:modal:open'), all(':is(:modal)'), all(':where(:modal, #c)'), body.matches(':has(> :modal)'), name(q('#ab').closest(':modal')), all(':modal > button'), all('dialog:not(:modal)')];
    s.caseInsensitive = [q('#a').matches(':MODAL'), q('#a').matches(':Open'), q('#a').matches(':mo\\64 al')];
    q('#b').close(); s.afterTopClose = all(':modal');
    q('#a').removeAttribute('open'); s.attributeRemoved = [all(':modal'), all(':open')];
    q('#a').close(); q('#a').show(); s.afterReopen = [all(':modal'), all(':open')]; q('#a').close();
    q('#det').removeAttribute('open'); q('#det2').setAttribute('open', ''); s.details = all(':open');
    // Forgiving :is()/:where() lists drop an invalid item; blocks left open at EOF close.
    s.syntax = [':modal()', ':open()', ':modal(x)', ':not(:modal())', ':has(:open())', ':is(:modal(', ':is(:modal())', ':is(:modal(x), dialog)', ':where(div:open())', ':is(:not(:modal()))', ':is(:open', 'dialog:is(:open', 'dialog:where(:open'].map(selector => [selector, (() => { try { return q('#c').matches(selector); } catch (error) { return error.name; } })()]);
    const fragment = document.createDocumentFragment(), loose = document.createElement('dialog'); loose.setAttribute('open', ''); fragment.append(loose);
    s.fragment = [fragment.querySelectorAll(':open').length, loose.matches(':open'), loose.matches(':modal')];
    const detached = document.createElement('dialog'); detached.show(); s.detached = [detached.matches(':open'), detached.matches(':modal')];
    const host = document.createElement('div'); body.append(host); const shadow = host.attachShadow({ mode: 'open' }); shadow.innerHTML = '<dialog id=s></dialog>'; shadow.querySelector('#s').showModal();
    s.shadow = [shadow.querySelector('#s').matches(':modal'), document.querySelector(':modal'), name(shadow.querySelector(':modal')), host.matches(':modal')];
    shadow.querySelector('#s').close();
    q('#a').showModal(); s.markup = [q('#a').outerHTML, q('#a').matches('[modal]'), q('#a').matches('[open]')]; q('#a').close();
    body.replaceChildren(); await settle();
  }

  // Focusing steps: [activeElement after show(), after close()] then the same for showModal().
  out.focus = {};
  const focusCases = {
    empty: '', button: '<button id=x>x</button>', input: '<p>t</p><input id=x>', nested: '<div><div><textarea id=x></textarea></div></div>',
    autofocus: '<button id=x>x</button><input id=y autofocus>', autofocusNested: '<div><button id=x autofocus>x</button></div><input id=y autofocus>',
    dialogAutofocus: '@<button id=x>x</button>', autofocusDisabled: '<button id=x>x</button><button id=y autofocus disabled>y</button>',
    autofocusDiv: '<button id=x>x</button><div id=y autofocus>y</div>', autofocusTabindex: '<button id=x>x</button><div id=y autofocus tabindex=-1>y</div>',
    autofocusInert: '<div inert><button id=y autofocus>y</button></div><button id=x>x</button>',
    tabindex: '<span id=x tabindex=0>s</span>', tabindexNegative: '<span id=x tabindex=-1>s</span>', tabindexInvalid: '<span id=y tabindex=abc>s</span><button id=x>x</button>',
    disabled: '<button id=y disabled>y</button>', fieldsetDisabled: '<fieldset disabled><button id=y>y</button></fieldset><button id=x>x</button>',
    link: '<a id=y>a</a><a id=x href=#>b</a>', svgLink: '<svg><a id=x href=#><text>t</text></a></svg>', inputHidden: '<input id=y type=hidden><input id=x>',
    select: '<p>t</p><select id=x><option>1</option></select>', iframe: '<p>t</p><iframe id=x></iframe>', contenteditable: '<p>t</p><div id=x contenteditable>e</div>',
    audioControls: '<audio id=x controls></audio>', audioPlain: '<audio id=y></audio>', label: '<label id=y>l</label>',
    hiddenAttribute: '<button id=y hidden>y</button><button id=x>x</button>', hiddenUntilFound: '<div hidden=until-found><button id=y>y</button></div><button id=x>x</button>',
    inlineNone: '<button id=y style="display:none">y</button><button id=x>x</button>', ancestorNone: '<div style="display:none"><button id=y>y</button></div><button id=x>x</button>',
    sheetNone: '<style>.n{display:none}</style><button id=y class=n>y</button><button id=x>x</button>', visibilityHidden: '<button id=y style="visibility:hidden">y</button><button id=x>x</button>',
    inheritedVisibility: '<div style="visibility:hidden"><button id=y>y</button></div><button id=x>x</button>', inert: '<div inert><button id=y>y</button></div><button id=x>x</button>',
    innerClosedDialog: '<dialog><button id=y>y</button></dialog><button id=x>x</button>', innerOpenDialog: '<dialog id=x open><button id=y>y</button></dialog>',
    closedDetails: '<details><summary id=x>s</summary><button id=y>y</button></details>', openDetails: '<details open><summary id=x>s</summary><button id=y>y</button></details>',
    detailsSecondSummary: '<details><p>p</p><summary id=y>s</summary></details><button id=x>x</button>',
  };
  for (const [label, html] of Object.entries(focusCases)) {
    const row = [];
    for (const method of ['show', 'showModal']) {
      const attribute = html[0] === '@' ? ' autofocus' : '';
      body.innerHTML = '<button id=pre>p</button><dialog id=d' + attribute + '>' + html.replace(/^@/, '') + '</dialog>';
      q('#pre').focus(); q('#d')[method](); row.push(name(document.activeElement)); q('#d').close(); row.push(name(document.activeElement));
    }
    out.focus[label] = row;
  }
  {
    const f = out.focus, setup = html => { body.innerHTML = '<button id=pre>p</button><button id=other>o</button>' + html; q('#pre').focus(); };
    setup('<dialog id=d><button id=x>x</button></dialog>'); q('#d').show(); q('#other').focus(); q('#d').close(); f.nonModalMovedAway = name(document.activeElement);
    setup('<dialog id=d><button id=x>x</button></dialog>'); q('#d').show(); q('#d').focus(); q('#d').close(); f.nonModalOnDialog = name(document.activeElement);
    setup('<dialog id=d><button id=x>x</button></dialog>'); q('#d').showModal(); q('#other').focus(); const blocked = name(document.activeElement); document.body.focus(); f.modalInert = [blocked, name(document.activeElement)]; q('#d').close(); f.modalRestore = name(document.activeElement);
    setup('<dialog id=d><button id=x>x</button></dialog>'); q('#d').showModal(); q('#pre').remove(); q('#d').close(); f.restoreRemoved = name(document.activeElement);
    setup('<dialog id=a><button id=ab>a</button></dialog><dialog id=b><button id=bb>b</button></dialog>'); q('#a').showModal(); q('#b').showModal(); const top = name(document.activeElement); q('#ab').focus(); const lower = name(document.activeElement); q('#b').close(); const restored = name(document.activeElement); q('#ab').focus(); q('#a').close(); f.nestedModals = [top, lower, restored, name(document.activeElement)];
    setup('<dialog id=d><button id=x>x</button></dialog>'); q('#d').showModal(); q('#d').remove(); f.modalRemovedFocus = [name(document.activeElement), errorName(() => q('#other').focus()), name(document.activeElement)];
    setup('<div id=p popover=manual><button id=pb>p</button></div><dialog id=d><button id=x>x</button></dialog>'); q('#p').showPopover(); q('#d').showModal(); q('#pb').focus(); f.popoverBelowModal = name(document.activeElement); q('#d').close(); q('#p').hidePopover();
    body.replaceChildren(); await settle();
  }

  // Form submission with method=dialog: phased dialog events plus submit/formdata.
  out.form = {};
  const formCases = {
    button: ['<dialog id=d open><form method=dialog><button id=s value=bv>s</button></form></dialog>', 's'],
    buttonNoValue: ['<dialog id=d open><form method=dialog><button id=s>s</button></form></dialog>', 's'],
    buttonEmptyValue: ['<dialog id=d open><form method=dialog><button id=s value>s</button></form></dialog>', 's'],
    input: ['<dialog id=d open><form method=dialog><input id=s type=submit value=iv></form></dialog>', 's'],
    inputNoValue: ['<dialog id=d open><form method=dialog><input id=s type=submit></form></dialog>', 's'],
    image: ['<dialog id=d open><form method=dialog><input id=s type=image value=im name=n></form></dialog>', 's'],
    formmethod: ['<dialog id=d open><form method=post action=/x><button id=s formmethod=dialog value=fm>s</button></form></dialog>', 's'],
    formmethodUppercase: ['<dialog id=d open><form method=DIALOG><button id=s value=up>s</button></form></dialog>', 's'],
    formmethodOther: ['<dialog id=d open><form method=dialog><button id=s formmethod=dialog value=ok>s</button></form></dialog>', 's'],
    noAncestorDialog: ['<form id=f method=dialog><button id=s value=v>s</button></form><dialog id=d open></dialog>', 's'],
    formAttributeOutside: ['<form id=f method=dialog></form><dialog id=d open><button id=s form=f value=fa>s</button></dialog>', 's'],
    closedDialog: ['<dialog id=d><form method=dialog><button id=s value=v>s</button></form></dialog>', 's'],
    nestedDialogs: ['<dialog id=outer open><dialog id=d open><form method=dialog><button id=s value=v>s</button></form></dialog></dialog>', 's'],
    deepForm: ['<dialog id=d open><div><section><form method=dialog><button id=s value=deep>s</button></form></section></div></dialog>', 's'],
    canceledSubmit: ['<dialog id=d open><form method=dialog><button id=s value=v>s</button></form></dialog>', 's', form => form.addEventListener('submit', event => event.preventDefault())],
    invalid: ['<dialog id=d open><form method=dialog><input required><button id=s value=v>s</button></form></dialog>', 's'],
    novalidate: ['<dialog id=d open><form method=dialog novalidate><input required><button id=s value=v>s</button></form></dialog>', 's'],
    modal: ['<dialog id=d><form method=dialog><button id=s value=mv>s</button></form></dialog>', 's', (form, d) => d.showModal()],
    submitMethod: ['<dialog id=d open><form method=dialog><button id=s value=bv>s</button></form></dialog>', null, null, form => form.submit()],
    requestSubmit: ['<dialog id=d open><form method=dialog><button id=s value=bv>s</button></form></dialog>', null, null, form => form.requestSubmit()],
    requestSubmitButton: ['<dialog id=d open><form method=dialog><button id=s value=bv>s</button></form></dialog>', null, null, form => form.requestSubmit(form.querySelector('#s'))],
    requestSubmitInputNoValue: ['<dialog id=d open><form method=dialog><input id=s type=submit></form></dialog>', null, null, form => form.requestSubmit(form.querySelector('#s'))],
    submitDisconnectedForm: ['<dialog id=d open></dialog>', null, null, (form, d) => { const detached = document.createElement('form'); detached.method = 'dialog'; d.append(detached); detached.remove(); detached.submit(); }],
  };
  for (const [label, [html, submitter, setup, action]] of Object.entries(formCases)) {
    body.innerHTML = html;
    const d = q('#d'), form = body.querySelector('form') || document.createElement('form'), log = [];
    d.returnValue = 'init';
    if (setup) setup(form, d);
    for (const node of [d, q('#outer')].filter(Boolean)) for (const type of ['beforetoggle', 'toggle', 'close']) node.addEventListener(type, event => log.push(type + ':' + node.id + (event.oldState ? ':' + event.oldState + '>' + event.newState : '')));
    form.addEventListener('submit', event => log.push('submit:' + name(event.submitter)));
    form.addEventListener('formdata', () => log.push('formdata'));
    const result = run(() => action ? action(form, d) : q('#' + submitter).click());
    log.push('sync:' + d.open + ':' + d.returnValue);
    await settle();
    out.form[label] = [result, log.filter(row => !row.startsWith('close')), log.filter(row => row.startsWith('close')).length, d.open, d.returnValue, d.matches(':modal'), q('#outer') ? q('#outer').open : null];
  }
  body.replaceChildren(); await settle();

  // Invoker commands beyond the committed invoker control.
  out.invokers = {};
  const invoke = async (html, setup) => {
    body.innerHTML = html;
    let d = q('#d'); const log = [];
    const replaced = setup ? setup(d) : undefined; if (!d) d = replaced;
    for (const type of ['command', 'cancel', 'beforetoggle', 'toggle', 'close']) d.addEventListener(type, event => log.push([type, event.source === undefined ? 'none' : name(event.source), event.composed, event.oldState === undefined ? null : event.newState]));
    q('#b').click(); const sync = [d.open, d.matches(':modal'), d.returnValue];
    await settle();
    return [log.filter(row => row[0] !== 'close'), log.filter(row => row[0] === 'close').length, sync, d.open, d.returnValue];
  };
  const invokerCases = {
    showModal: ['<dialog id=d></dialog><button id=b commandfor=d command=show-modal></button>'],
    showModalUppercase: ['<dialog id=d></dialog><button id=b commandfor=d command=SHOW-MODAL></button>'],
    showModalPopoverOpen: ['<dialog id=d popover=manual></dialog><button id=b commandfor=d command=show-modal></button>', d => d.showPopover()],
    showModalNonModal: ['<dialog id=d open></dialog><button id=b commandfor=d command=show-modal></button>'],
    showModalRemovedInBeforetoggle: ['<dialog id=d></dialog><button id=b commandfor=d command=show-modal></button>', d => d.addEventListener('beforetoggle', () => d.remove(), { once: true })],
    showModalDisconnected: ['<button id=b command=show-modal></button>', () => { const d = document.createElement('dialog'); q('#b').commandForElement = d; return d; }],
    closeNoValue: ['<dialog id=d open></dialog><button id=b commandfor=d command=close></button>', d => { d.returnValue = 'init'; }],
    closeEmptyValue: ['<dialog id=d open></dialog><button id=b commandfor=d command=close value></button>', d => { d.returnValue = 'init'; }],
    closeModal: ['<dialog id=d></dialog><button id=b commandfor=d command=close value=m></button>', d => d.showModal()],
    closeCommandCanceled: ['<dialog id=d open></dialog><button id=b commandfor=d command=close value=c></button>', d => d.addEventListener('command', event => event.preventDefault())],
    requestCloseCanceled: ['<dialog id=d open></dialog><button id=b commandfor=d command=request-close value=v></button>', d => d.addEventListener('cancel', event => event.preventDefault())],
    requestCloseNoValue: ['<dialog id=d open></dialog><button id=b commandfor=d command=request-close></button>', d => { d.returnValue = 'init'; }],
    requestCloseClosedByNone: ['<dialog id=d open closedby=none></dialog><button id=b commandfor=d command=request-close value=n></button>'],
    requestCloseModal: ['<dialog id=d></dialog><button id=b commandfor=d command=request-close value=rm></button>', d => d.showModal()],
    showPopoverOnDialog: ['<dialog id=d popover=manual></dialog><button id=b commandfor=d command=show-popover></button>'],
    disabledInvoker: ['<dialog id=d></dialog><button id=b commandfor=d command=show-modal disabled></button>'],
    customCommand: ['<dialog id=d open></dialog><button id=b commandfor=d command=--close value=x></button>'],
    divTarget: ['<div id=d></div><button id=b commandfor=d command=show-modal></button>', d => { d.open = false; }],
  };
  for (const [label, [html, setup]] of Object.entries(invokerCases)) out.invokers[label] = await invoke(html, setup);
  body.replaceChildren(); await settle();
  return out;
})()
