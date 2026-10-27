(async () => {
  // Close watchers and light dismiss under trusted input: Esc close requests (dialogs,
  // popovers, closedby, cancel prevention gated on history-action activation, grouping of
  // watchers created without activation) and pointer light dismiss (auto popovers, nested
  // popovers, closedby=any dialogs, the modal ::backdrop). Trusted input goes through the
  // collector's `__cdpInput` binding: {type:"down"|"up", x, y, target, backdrop} presses or
  // releases the mouse at x,y (offline drivers use `target`, the element the point hits,
  // and `backdrop`, whether the hit lands on a modal dialog's ::backdrop); {type:"key"} is a key.
  const waiters = {}; let next = 0;
  globalThis.__cdpInputDone = id => { const resolve = waiters[id]; delete waiters[id]; resolve(); };
  const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
  // Input starts after the tasks already queued (toggle events) have run.
  const input = async (...steps) => { await wait(30); await new Promise(resolve => { const id = ++next; waiters[id] = resolve; __cdpInput(JSON.stringify({ id, steps })); }); };
  const esc = { type: 'key', key: 'Escape', code: 'Escape', keyCode: 27 };
  const click = (x, y, target, backdrop = false) => [{ type: 'down', x, y, target, backdrop }, { type: 'up', x, y, target, backdrop }];
  const tag = n => n == null ? null : n === document ? 'document' : n === window ? 'window' : n.id || n.nodeName;
  let log = [];
  const record = event => log.push([event.type, tag(event.target), event.cancelable, event.isTrusted].concat(event.newState ? [event.newState] : []));
  for (const type of ['cancel', 'close', 'beforetoggle', 'toggle', 'pointerdown', 'pointerup', 'click']) document.addEventListener(type, record, true);
  const style = document.head.appendChild(document.createElement('style'));
  style.textContent = 'body{margin:0} .box{position:fixed;margin:0;padding:0;border:0;width:100px;height:100px;inset:auto;overflow:hidden} dialog::backdrop{background:none}';
  const place = (element, x, y) => { element.className = 'box'; element.style.cssText = `position:fixed;margin:0;padding:0;border:0;width:100px;height:100px;inset:auto;overflow:hidden;left:${x}px;top:${y}px`; return element; };
  const fixture = html => {
    document.body.innerHTML = html;
    for (const element of document.body.querySelectorAll('[data-x]')) place(element, element.dataset.x, element.dataset.y);
    log = [];
  };
  const state = (...ids) => ids.map(id => { const element = document.getElementById(id); return element.localName === 'dialog' ? element.open : element.matches(':popover-open'); });
  const out = {};
  const scenario = async (name, html, run) => {
    fixture(html);
    const states = await run();
    await wait(30);
    out[name] = { states, log };
  };
  const OUT = 'data-x=0 data-y=0';

  // A modal dialog opened without activation: Esc fires an uncancelable cancel, then close.
  await scenario('modalEsc', `<div id=out ${OUT}></div><dialog id=d data-x=300 data-y=300>d</dialog>`, async () => {
    document.getElementById('d').showModal();
    const before = state('d');
    await input(esc);
    return [before, state('d'), document.getElementById('d').returnValue];
  });

  // After a click (history-action activation) the first cancel is cancelable; a prevented
  // cancel keeps the dialog open and consumes the activation, so the next Esc closes it.
  await scenario('modalPreventCancel', `<div id=out ${OUT}></div><button id=b data-x=120 data-y=0>b</button><dialog id=d data-x=300 data-y=300>d</dialog>`, async () => {
    const dialog = document.getElementById('d');
    dialog.addEventListener('cancel', event => event.preventDefault());
    await input(...click(150, 30, 'b'));
    dialog.showModal();
    await input(esc);
    const first = state('d');
    await input(esc);
    return [first, state('d')];
  });

  // closedby=none: Esc never closes; closedby=any non-modal: a click outside closes it,
  // a click inside does not; closedby=closerequest: clicks never close, Esc does.
  await scenario('closedbyNone', `<div id=out ${OUT}></div><dialog id=d closedby=none data-x=300 data-y=300>d</dialog>`, async () => {
    document.getElementById('d').showModal();
    await input(esc);
    return [state('d'), document.getElementById('d').closedBy];
  });
  await scenario('closedbyAny', `<div id=out ${OUT}></div><dialog id=d closedby=any data-x=300 data-y=300>i</dialog>`, async () => {
    document.getElementById('d').show();
    await input(...click(310, 310, 'd'));
    const inside = state('d');
    await input(...click(10, 10, 'out'));
    return [inside, state('d')];
  });
  await scenario('closedbyCloserequest', `<div id=out ${OUT}></div><dialog id=d closedby=closerequest data-x=300 data-y=300>d</dialog>`, async () => {
    document.getElementById('d').show();
    await input(...click(10, 10, 'out'));
    const clicked = state('d');
    await input(esc);
    return [clicked, state('d')];
  });
  // A non-modal dialog without closedby computes closedby none: Esc ignores it.
  await scenario('nonModalEsc', `<div id=out ${OUT}></div><dialog id=d data-x=300 data-y=300>d</dialog>`, async () => {
    document.getElementById('d').show();
    await input(esc);
    return [state('d')];
  });

  // A modal dialog's ::backdrop: closedby=any closes on a backdrop click, the default
  // (closerequest for modal dialogs) does not.
  await scenario('backdropAny', `<div id=out ${OUT}></div><dialog id=d closedby=any data-x=300 data-y=300>d</dialog>`, async () => {
    document.getElementById('d').showModal();
    await input(...click(10, 10, 'out', true));
    return [state('d')];
  });
  await scenario('backdropDefault', `<div id=out ${OUT}></div><dialog id=d data-x=300 data-y=300>d</dialog>`, async () => {
    document.getElementById('d').showModal();
    await input(...click(10, 10, 'out', true));
    return [state('d')];
  });
  // Pressing inside the dialog and releasing on the backdrop is not a light dismiss.
  await scenario('backdropDrag', `<div id=out ${OUT}></div><dialog id=d closedby=any data-x=300 data-y=300>i</dialog>`, async () => {
    document.getElementById('d').showModal();
    await input({ type: 'down', x: 310, y: 310, target: 'd', backdrop: false }, { type: 'up', x: 10, y: 10, target: 'out', backdrop: true });
    return [state('d')];
  });

  // Two modal dialogs opened without activation share one close watcher group: one Esc
  // closes both. Opened with activation in between, each Esc closes one.
  await scenario('groupedEsc', `<div id=out ${OUT}></div><dialog id=d1 data-x=300 data-y=300>1</dialog><dialog id=d2 data-x=420 data-y=300>2</dialog>`, async () => {
    document.getElementById('d1').showModal();
    document.getElementById('d2').showModal();
    await input(esc);
    return [state('d1', 'd2')];
  });
  // The same two dialogs, then a click (on the top dialog's ::backdrop) grants activation:
  // the group they share may now cancel.
  await scenario('groupedThenActivation', `<div id=out ${OUT}></div><dialog id=d1 data-x=300 data-y=300>1</dialog><dialog id=d2 data-x=420 data-y=300>2</dialog>`, async () => {
    document.getElementById('d1').showModal();
    document.getElementById('d2').showModal();
    await input(...click(10, 10, 'out', true));
    await input(esc);
    return [state('d1', 'd2')];
  });
  await scenario('separateEsc', `<div id=out ${OUT}></div><button id=b data-x=120 data-y=0>b</button><dialog id=d1 data-x=300 data-y=300>1</dialog><dialog id=d2 data-x=420 data-y=300>2</dialog>`, async () => {
    document.getElementById('d1').showModal();
    await input(...click(310, 310, 'd1'));
    document.getElementById('d2').showModal();
    await input(esc);
    const first = state('d1', 'd2');
    await input(esc);
    return [first, state('d1', 'd2')];
  });

  // Auto popovers: Esc hides the topmost; light dismiss by a click outside hides them all;
  // a click inside a nested popover keeps it and its ancestor open; manual popovers stay.
  await scenario('popoverEsc', `<div id=out ${OUT}></div><div id=p popover data-x=300 data-y=300>p</div>`, async () => {
    document.getElementById('p').showPopover();
    await input(esc);
    return [state('p')];
  });
  await scenario('popoverNested', `<div id=out ${OUT}></div><div id=p1 popover data-x=300 data-y=300>1<div id=p2 popover data-x=420 data-y=300>2</div></div><div id=m popover=manual data-x=540 data-y=300>m</div>`, async () => {
    document.getElementById('m').showPopover();
    document.getElementById('p1').showPopover();
    document.getElementById('p2').showPopover();
    const opened = state('p1', 'p2', 'm');
    await input(...click(430, 310, 'p2'));
    const inside = state('p1', 'p2', 'm');
    await input(...click(310, 310, 'p1'));
    const parent = state('p1', 'p2', 'm');
    await input(...click(10, 10, 'out'));
    return [opened, inside, parent, state('p1', 'p2', 'm')];
  });
  await scenario('popoverInvoker', `<button id=inv popovertarget=p data-x=120 data-y=0>i</button><div id=out ${OUT}></div><div id=p popover data-x=300 data-y=300>p</div>`, async () => {
    document.getElementById('p').showPopover();
    await input(...click(150, 30, 'inv'));
    return [state('p')];
  });
  // A popover opened from inside a dialog: Esc closes the popover first, then the dialog.
  await scenario('dialogPopoverEsc', `<div id=out ${OUT}></div><dialog id=d data-x=300 data-y=300>d<div id=p popover data-x=420 data-y=300>p</div></dialog>`, async () => {
    document.getElementById('d').showModal();
    document.getElementById('p').showPopover();
    await input(esc);
    const first = state('d', 'p');
    await input(esc);
    return [first, state('d', 'p')];
  });
  // A dialog removed from the document leaves the open dialogs list: Esc does nothing.
  await scenario('removedDialog', `<div id=out ${OUT}></div><div id=wrap><dialog id=d closedby=any data-x=300 data-y=300>d</dialog></div>`, async () => {
    const dialog = document.getElementById('d');
    dialog.show();
    document.getElementById('wrap').remove();
    await input(esc);
    await input(...click(10, 10, 'out'));
    return [dialog.open, dialog.isConnected];
  });
  // A popover inside a shadow tree: a click inside it (retargeted to the host) keeps it open.
  await scenario('shadowPopover', `<div id=out ${OUT}></div><div id=host></div>`, async () => {
    const root = document.getElementById('host').attachShadow({ mode: 'open' });
    root.innerHTML = '<div id=sp popover>s</div>';
    const popover = place(root.getElementById('sp'), 300, 300);
    popover.showPopover();
    await input(...click(310, 310, 'host>sp'));
    const inside = popover.matches(':popover-open');
    await input(...click(10, 10, 'out'));
    return [inside, popover.matches(':popover-open')];
  });
  // An invoker inside an open popover makes its target that popover's nested popover.
  await scenario('invokerInPopover', `<div id=out ${OUT}></div><div id=p1 popover data-x=300 data-y=300><button id=inv popovertarget=p2>i</button></div><div id=p2 popover data-x=420 data-y=300>2</div>`, async () => {
    place(document.getElementById('inv'), 300, 300);
    document.getElementById('p1').showPopover();
    await input(...click(310, 310, 'inv'));
    const opened = state('p1', 'p2');
    await input(...click(430, 310, 'p2'));
    const inside = state('p1', 'p2');
    await input(...click(10, 10, 'out'));
    return [opened, inside, state('p1', 'p2')];
  });
  // A dialog inside a shadow tree with closedby=any: clicks inside keep it open.
  await scenario('shadowDialog', `<div id=out ${OUT}></div><div id=host></div>`, async () => {
    const root = document.getElementById('host').attachShadow({ mode: 'open' });
    root.innerHTML = '<dialog id=sd closedby=any>d</dialog>';
    const dialog = place(root.getElementById('sd'), 300, 300);
    dialog.show();
    await input(...click(310, 310, 'host>sd'));
    const inside = dialog.open;
    await input(...click(10, 10, 'out'));
    return [inside, dialog.open];
  });
  // Removing an open modal dialog's ancestor removes it from the open dialogs list.
  await scenario('removedAncestor', `<div id=out ${OUT}></div><section id=outer><div id=mid><dialog id=d closedby=any data-x=300 data-y=300>d</dialog></div></section>`, async () => {
    const dialog = document.getElementById('d');
    dialog.showModal();
    document.getElementById('mid').remove();
    await input(esc);
    return [dialog.open, document.querySelector(':modal')];
  });
  style.remove();
  document.body.innerHTML = '';
  for (const type of ['cancel', 'close', 'beforetoggle', 'toggle', 'pointerdown', 'pointerup', 'click']) document.removeEventListener(type, record, true);
  return out;
})()
