(async () => {
  // Dialog and popover edge steps: the dialog focusing steps, close requests for a dialog whose open attribute was
  // removed or whose cancel listener closes it, light dismiss through an invoker inside a
  // popover, invokers whose target is detached or no popover, reentrant invoker activation
  // and show/hide calls from beforetoggle, the show steps' validity recheck after hiding
  // other popovers, and invalid / EOF-closed :modal, :open and :popover-open selectors.
  // Trusted input goes through the collector's `__cdpInput` binding: {type:"down"|"up", x, y,
  // target, backdrop} presses or releases the mouse at x,y (offline drivers use `target`, the
  // element the point hits, and `backdrop`, whether the hit lands on a modal dialog's
  // ::backdrop); {type:"key"} is a key.
  const waiters = {}; let next = 0;
  globalThis.__cdpInputDone = id => { const resolve = waiters[id]; delete waiters[id]; resolve(); };
  const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
  const input = async (...steps) => { await wait(30); await new Promise(resolve => { const id = ++next; waiters[id] = resolve; __cdpInput(JSON.stringify({ id, steps })); }); };
  const esc = { type: 'key', key: 'Escape', code: 'Escape', keyCode: 27 };
  const click = (x, y, target, backdrop = false) => [{ type: 'down', x, y, target, backdrop }, { type: 'up', x, y, target, backdrop }];
  const tag = n => n == null ? null : n === document ? 'document' : n === window ? 'window' : n.id || n.nodeName;
  const run = f => { try { const v = f(); return v === undefined ? 'undefined' : v; } catch (e) { return [e.name, e.message]; } };
  let log = [];
  // A dialog's close and toggle tasks run in either order in Chrome (separate task sources,
  // miaskiewicz/turbo-dom#116): dialog toggle events are not recorded.
  const record = event => { if (event.type === 'toggle' && event.target.localName === 'dialog') return; log.push([event.type, tag(event.target)].concat(event.newState ? [event.newState] : [])); };
  const types = ['cancel', 'close', 'beforetoggle', 'toggle'];
  for (const type of types) document.addEventListener(type, record, true);
  const style = document.head.appendChild(document.createElement('style'));
  style.textContent = 'body{margin:0} .box{position:fixed;margin:0;padding:0;border:0;width:100px;height:100px;inset:auto;overflow:hidden} dialog::backdrop{background:none}';
  const place = (element, x, y) => { element.className = 'box'; element.style.left = x + 'px'; element.style.top = y + 'px'; return element; };
  const $ = id => document.getElementById(id);
  const state = (...ids) => ids.map(id => { const element = $(id); return element.localName === 'dialog' ? element.open : element.matches(':popover-open'); });
  const out = {};
  const scenario = async (name, html, body) => {
    document.body.innerHTML = html;
    for (const element of document.body.querySelectorAll('[data-x]')) place(element, element.dataset.x, element.dataset.y);
    log = [];
    const result = await body();
    await wait(30);
    out[name] = { result, log: log.slice() };
  };
  const OUT = 'data-x=0 data-y=0';

  // A modal dialog keeps its close watcher after its open attribute is removed, but the
  // watcher is disabled: Esc neither cancels nor closes it.
  await scenario('openRemoved', `<div id=out ${OUT}></div><dialog id=d data-x=300 data-y=300>d</dialog>`, async () => {
    $('d').showModal();
    $('d').removeAttribute('open');
    await input(esc);
    return [$('d').open, $('d').matches(':modal')];
  });
  // A cancel listener that closes the dialog itself: the close request stops there.
  await scenario('cancelCloses', `<div id=out ${OUT}></div><button id=b data-x=120 data-y=0>b</button><dialog id=d data-x=300 data-y=300>d</dialog>`, async () => {
    $('d').addEventListener('cancel', () => $('d').close('fromCancel'));
    await input(...click(150, 30, 'b'));
    $('d').showModal();
    await input(esc);
    return [$('d').open, $('d').returnValue];
  });
  // A click on an invoker inside popover p1 that targets p2 (shown from that invoker): the
  // topmost clicked popover is p2, so nothing is light dismissed and the invoker hides p2.
  await scenario('invokerInPopover', `<div id=out ${OUT}></div><div id=p1 popover data-x=300 data-y=300><button id=inv popovertarget=p2 data-x=310 data-y=310>i</button></div><div id=p2 popover data-x=500 data-y=300>2</div>`, async () => {
    $('p1').showPopover();
    await input(...click(320, 320, 'inv'));
    const opened = state('p1', 'p2');
    await input(...click(320, 320, 'inv'));
    const toggled = state('p1', 'p2');
    $('p2').showPopover({ source: $('inv') });
    await input(...click(510, 310, 'p2'));
    const inside = state('p1', 'p2');
    await input(...click(10, 10, 'out'));
    return [opened, toggled, inside, state('p1', 'p2')];
  });
  // Invokers whose target is detached, or is no popover: activation does nothing.
  await scenario('invokerTargets', `<div id=out ${OUT}></div><button id=inv data-x=120 data-y=0>i</button><div id=plain>x</div>`, async () => {
    const detached = document.createElement('div');
    detached.id = 'detached';
    detached.popover = 'auto';
    $('inv').popoverTargetElement = detached;
    await input(...click(150, 30, 'inv'));
    const first = [detached.matches(':popover-open'), tag($('inv').popoverTargetElement)];
    $('inv').popoverTargetElement = $('plain');
    await input(...click(150, 30, 'inv'));
    return [first, tag($('inv').popoverTargetElement), $('plain').matches(':popover-open')];
  });
  // An invoker activated again from the popover's own beforetoggle (reentrant) is ignored.
  await scenario('reentrantInvoke', '<button id=inv popovertarget=p>i</button><div id=p popover>p</div>', async () => {
    let inner = null;
    $('p').addEventListener('beforetoggle', event => { if (inner === null) { inner = 'clicked'; $('inv').click(); inner = state('p')[0]; } });
    $('inv').click();
    return [inner, state('p')];
  });
  // showPopover() / hidePopover() / togglePopover() called from the popover's own beforetoggle.
  await scenario('nestedCalls', '<div id=p popover>p</div>', async () => {
    const calls = [];
    const listener = event => {
      if (calls.length >= 6) return;
      calls.push([event.newState, run(() => $('p').showPopover()), run(() => $('p').togglePopover()), run(() => $('p').hidePopover())]);
    };
    $('p').addEventListener('beforetoggle', listener);
    const shown = run(() => $('p').showPopover());
    const opened = state('p');
    const hidden = run(() => $('p').hidePopover());
    $('p').removeEventListener('beforetoggle', listener);
    return [shown, opened, hidden, state('p'), calls];
  });
  // Showing p2 hides p1, whose beforetoggle removes p2: p2's show stops after the recheck.
  await scenario('recheckRemoved', '<div id=p1 popover>1</div><div id=wrap><div id=p2 popover>2</div></div>', async () => {
    const p2 = $('p2');
    $('p1').showPopover();
    $('p1').addEventListener('beforetoggle', event => { if (event.newState === 'closed') p2.remove(); });
    return [run(() => p2.showPopover()), p2.matches(':popover-open'), state('p1'), p2.isConnected];
  });

  // The dialog focusing steps: an SVG link with only xlink:href is focusable, a control in a
  // closed details body is not (the summary is), and shadow trees precede light children.
  await scenario('focusDelegate', '<dialog id=d1><p>x</p><svg><a id=svga><text>t</text></a></svg></dialog><dialog id=d2><details><summary id=sum>s</summary><button id=hiddenb autofocus>b</button></details></dialog><dialog id=d3><div id=host><button id=light>l</button></div></dialog>', async () => {
    $('svga').setAttributeNS('http://www.w3.org/1999/xlink', 'xlink:href', '#x');
    const root = $('host').attachShadow({ mode: 'open' });
    root.innerHTML = '<button id=inner>i</button><slot></slot>';
    const focused = [];
    for (const id of ['d1', 'd2', 'd3']) {
      $(id).showModal();
      focused.push([tag(document.activeElement), tag(root.activeElement)]);
      $(id).close();
    }
    return focused;
  });

  // Selectors: invalid functional forms, and blocks / strings / comments left open at EOF.
  await scenario('selectors', '<dialog id=d>d</dialog><div id=p popover>p</div><details id=s open title="a(b"><summary>s</summary></details>', async () => {
    $('d').showModal();
    $('p').showPopover();
    const selectors = [':modal()', ':open()', ':popover-open()', ':is(:modal(), div)', ':modal/**/', ':open/* ( */', ':open, [title="a(b"]', ":open, [title='a(b']", ':open, [title=a\\(b]', ':open, [title', ':open, :is(details', ':popover-open, [title', ':modal!', ':open:nth-child(', ':open, [title="a(b', ':modal, :popover-open, :open'];
    const name = f => { try { return f(); } catch (e) { return e.name; } };
    return selectors.map(selector => [selector, name(() => [...document.querySelectorAll(selector)].map(tag)), name(() => $('s').matches(selector))]);
  });
  style.remove();
  document.body.innerHTML = '';
  for (const type of types) document.removeEventListener(type, record, true);
  return out;
})()
