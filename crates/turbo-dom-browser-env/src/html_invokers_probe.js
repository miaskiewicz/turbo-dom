(async () => {
  const out = {}, body = document.body, q = selector => body.querySelector(selector);
  const tick = () => new Promise(resolve => setTimeout(resolve, 0));
  const name = node => node === null ? null : node === undefined ? 'undefined' : node.id || node.localName || node.nodeName;
  const caught = fn => { try { fn(); return 'ok'; } catch (error) { return error.name; } };
  const describe = (proto, key) => {
    const d = Object.getOwnPropertyDescriptor(proto, key);
    if (!d) return null;
    const fn = f => f && [f.name, f.length, String(f), caught(() => new f())];
    return { enumerable: d.enumerable, configurable: d.configurable, get: fn(d.get), set: fn(d.set) };
  };

  // Enumerated, element and Auto-state reflection.
  {
    const b = document.createElement('button'), r = {};
    r.command = [];
    for (const v of [null, '', 'toggle-popover', 'TOGGLE-POPOVER', 'Show-Popover', 'hide-popover', 'show-modal', 'CLOSE', 'request-close', '--foo', '--Foo', '--', '-foo', 'custom', 'toggle', ' close', 'x']) {
      if (v === null) b.removeAttribute('command'); else b.setAttribute('command', v);
      r.command.push([v, b.command]);
    }
    b.command = '--Bar'; r.commandSet = [b.getAttribute('command'), b.command];
    b.command = null; r.commandNull = [b.getAttribute('command'), b.command];
    b.command = undefined; r.commandUndefined = [b.getAttribute('command'), b.command];
    r.commandSymbol = caught(() => { b.command = Symbol(); });
    r.action = [];
    for (const v of [null, '', 'show', 'HIDE', 'toggle', 'Toggle', 'x']) {
      if (v === null) b.removeAttribute('popovertargetaction'); else b.setAttribute('popovertargetaction', v);
      r.action.push([v, b.popoverTargetAction]);
    }
    b.popoverTargetAction = 'nonsense'; r.actionSet = [b.getAttribute('popovertargetaction'), b.popoverTargetAction];
    r.types = [];
    for (const attrs of ['', 'command=x', 'commandfor=x', 'command', 'type=submit commandfor=x', 'type=BUTTON', 'type=bogus commandfor=x', 'type=reset command=x', 'type=bogus', 'popovertarget=x']) {
      body.innerHTML = '<button ' + attrs + '></button>'; r.types.push([attrs, q('button').type]);
    }
    const t = document.createElement('button'); t.type = 'RESET'; r.typeSet = [t.getAttribute('type'), t.type];
    body.innerHTML = '<form><button id=c commandfor=x></button><button id=d command=x type=bogus></button><button id=e type=submit commandfor=x></button><button id=f></button></form>';
    r.willValidate = ['c', 'd', 'e', 'f'].map(id => q('#' + id).willValidate);
    body.innerHTML = '<div id=p></div><span id=s></span>';
    const p = q('#p'), s = q('#s'), live = document.createElement('button'); body.append(live);
    r.elements = [];
    for (const [property, attribute] of [['popoverTargetElement', 'popovertarget'], ['commandForElement', 'commandfor']]) {
      const row = [live[property]];
      live.setAttribute(attribute, 'p'); row.push(name(live[property]));
      live.setAttribute(attribute, 'missing'); row.push(live[property]);
      live[property] = s; row.push(live.getAttribute(attribute), name(live[property]));
      live.setAttribute(attribute, 'p'); row.push(name(live[property]));
      live[property] = s; s.remove(); row.push(live[property]); body.append(s); row.push(name(live[property]));
      live[property] = null; row.push(live.getAttribute(attribute), live[property]);
      live[property] = s; live[property] = undefined; row.push(live.getAttribute(attribute));
      row.push(caught(() => { live[property] = {}; }), live.getAttribute(attribute));
      live.removeAttribute(attribute);
      r.elements.push([property, row]);
    }
    const input = document.createElement('input'); body.append(input); input.setAttribute('popovertarget', 'p');
    r.input = [name(input.popoverTargetElement), input.popoverTargetAction];
    input.popoverTargetAction = 'SHOW'; r.input.push(input.getAttribute('popovertargetaction'), input.popoverTargetAction);
    input.popoverTargetElement = undefined; r.input.push(input.getAttribute('popovertarget'));
    r.input.push(caught(() => { input.popoverTargetAction = Symbol(); }));
    // Explicit elements honor tree scopes: ancestor scopes only, never into a shadow tree.
    body.innerHTML = '<div id=p popover></div><div id=host></div>';
    const root = q('#host').attachShadow({ mode: 'open' }); root.innerHTML = '<button id=sb></button><div id=sp popover></div>';
    const sb = root.querySelector('#sb'), scope = [];
    sb.setAttribute('popovertarget', 'p'); scope.push(sb.popoverTargetElement);
    sb.setAttribute('popovertarget', 'sp'); scope.push(name(sb.popoverTargetElement));
    sb.popoverTargetElement = q('#p'); scope.push(name(sb.popoverTargetElement));
    const lb = document.createElement('button'); body.append(lb); lb.commandForElement = root.querySelector('#sp'); scope.push(lb.commandForElement);
    const loose = document.createElement('button'); loose.popoverTargetElement = q('#p'); scope.push(loose.popoverTargetElement);
    r.scope = scope;
    out.reflection = r;
  }

  // Descriptors and receiver brands.
  {
    const r = { button: {}, input: {}, brands: {} };
    for (const key of ['command', 'commandForElement', 'popoverTargetAction', 'popoverTargetElement', 'type']) r.button[key] = describe(HTMLButtonElement.prototype, key);
    for (const key of ['popoverTargetAction', 'popoverTargetElement']) r.input[key] = describe(HTMLInputElement.prototype, key);
    for (const [proto, keys] of [[HTMLButtonElement.prototype, ['command', 'commandForElement', 'popoverTargetAction', 'popoverTargetElement', 'type']], [HTMLInputElement.prototype, ['popoverTargetAction', 'popoverTargetElement']]]) {
      for (const key of keys) {
        const d = Object.getOwnPropertyDescriptor(proto, key), rows = [];
        for (const receiver of [document.createElement('button'), document.createElement('input'), document.createElement('div'), document.createElementNS('urn:x', 'button'), {}, null]) rows.push(caught(() => d.get.call(receiver)), caught(() => d.set.call(receiver, null)));
        r.brands[(proto === HTMLButtonElement.prototype ? 'button.' : 'input.') + key] = rows;
      }
    }
    r.inPrototype = [['HTMLButtonElement', 'command'], ['HTMLButtonElement', 'commandForElement'], ['HTMLInputElement', 'popoverTargetElement'], ['HTMLInputElement', 'command'], ['HTMLInputElement', 'commandForElement'], ['HTMLElement', 'popoverTargetElement']].map(([iface, key]) => Object.hasOwn(globalThis[iface].prototype, key));
    out.descriptors = r;
  }

  // CommandEvent interface.
  {
    const r = {}, btn = document.createElement('button');
    const e = new CommandEvent('command', { command: 1, source: null, bubbles: true, cancelable: true, composed: true });
    r.basic = [e.type, e.command, e.source, e.bubbles, e.cancelable, e.composed, e.isTrusted, Object.prototype.toString.call(e), e instanceof Event];
    const d = new CommandEvent('x'); r.defaults = [d.command, d.source, d.bubbles, d.cancelable, d.composed];
    r.withSource = new CommandEvent('x', { source: btn }).source === btn;
    r.errors = [caught(() => new CommandEvent()), caught(() => CommandEvent('x')), caught(() => new CommandEvent('x', { source: {} })), caught(() => new CommandEvent('x', { command: Symbol() })), caught(() => new CommandEvent('x', 5)), caught(() => new CommandEvent(Symbol()))];
    r.nullInit = new CommandEvent('x', null).command;
    const order = []; new CommandEvent('x', { get bubbles() { order.push('bubbles'); }, get cancelable() { order.push('cancelable'); }, get command() { order.push('command'); }, get composed() { order.push('composed'); }, get source() { order.push('source'); } });
    r.initOrder = order;
    r.members = Object.getOwnPropertyNames(CommandEvent.prototype);
    for (const key of ['source', 'command']) r[key] = [describe(CommandEvent.prototype, key), caught(() => Object.getOwnPropertyDescriptor(CommandEvent.prototype, key).get.call(new Event('x'))), caught(() => Object.getOwnPropertyDescriptor(CommandEvent.prototype, key).get.call(new ToggleEvent('x')))];
    const own = Object.getOwnPropertyDescriptor(window, 'CommandEvent');
    r.constructor = [CommandEvent.name, CommandEvent.length, String(CommandEvent), Object.getPrototypeOf(CommandEvent) === Event, Object.getPrototypeOf(CommandEvent.prototype) === Event.prototype, own.enumerable, own.writable, own.configurable, Object.getOwnPropertyDescriptor(CommandEvent, 'prototype').writable];
    const tag = Object.getOwnPropertyDescriptor(CommandEvent.prototype, Symbol.toStringTag);
    r.toStringTag = [tag.value, tag.writable, tag.enumerable, tag.configurable];
    class Sub extends CommandEvent {} const sub = new Sub('x', { command: '--s' }); r.subclass = [sub.command, sub instanceof CommandEvent, Object.getPrototypeOf(sub) === Sub.prototype];
    out.commandEvent = r;
  }

  // CommandEvent/ToggleEvent source retargets against the current target (or out of shadows).
  {
    body.innerHTML = '<div id=light></div><div id=host></div>';
    const root = q('#host').attachShadow({ mode: 'open' }); root.innerHTML = '<button id=sb></button><div id=inner></div>';
    const sb = root.querySelector('#sb'), inner = root.querySelector('#inner'), light = q('#light'), r = {};
    for (const C of [CommandEvent, ToggleEvent]) {
      const make = () => new C('probe', { source: sb }), row = {};
      row.undispatched = name(make().source);
      let during;
      const a = make(); light.addEventListener('probe', event => { during = name(event.source); }, { once: true }); light.dispatchEvent(a); row.light = [during, name(a.source)];
      const b = make(); inner.addEventListener('probe', event => { during = name(event.source); }, { once: true }); inner.dispatchEvent(b); row.inner = [during, name(b.source)];
      const c = new C('probe', { source: sb, bubbles: true }); inner.addEventListener('probe', () => {}, { once: true }); root.addEventListener('probe', event => { during = name(event.source); }, { once: true }); inner.dispatchEvent(c); row.root = during;
      r[C.name] = row;
    }
    out.retarget = r;
  }

  // Activation: logs of click/command/beforetoggle/toggle/submit/reset at their targets.
  const record = ['click', 'command', 'beforetoggle', 'toggle', 'submit', 'reset'];
  async function run(label, html, act) {
    body.innerHTML = html;
    const log = [];
    for (const element of body.querySelectorAll('*')) for (const type of record) element.addEventListener(type, event => {
      if (event.currentTarget !== event.target) return;
      log.push([type, name(event.target), event.command, event.source === undefined ? 'undefined' : name(event.source), event.oldState, event.newState, event.isTrusted, event.cancelable, event.composed, event.bubbles, event.constructor.name]);
      if (type === 'submit') event.preventDefault();
    });
    try { act(); } catch (error) { log.push(['throw', error.name]); }
    const sync = log.length;
    await tick(); await tick();
    out.activation[label] = { log, sync, open: [...body.querySelectorAll('[popover]')].filter(element => element.matches(':popover-open')).map(element => element.id) };
  }
  out.activation = {};
  const click = selector => () => q(selector).click();
  await run('ptBasic', '<div id=p popover></div><button id=b popovertarget=p></button>', click('#b'));
  await run('ptShowOnOpen', '<div id=p popover></div><button id=b popovertarget=p popovertargetaction=show></button>', () => { q('#p').showPopover(); q('#b').click(); });
  await run('ptHideOnOpen', '<div id=p popover></div><button id=b popovertarget=p popovertargetaction=hide></button>', () => { q('#p').showPopover(); q('#b').click(); });
  await run('ptHideOnClosed', '<div id=p popover></div><button id=b popovertarget=p popovertargetaction=hide></button>', click('#b'));
  await run('ptToggleOpen', '<div id=p popover></div><button id=b popovertarget=p></button>', () => { q('#p').showPopover(); q('#b').click(); });
  await run('ptHint', '<div id=p popover=hint></div><button id=b popovertarget=p></button>', click('#b'));
  await run('ptManualShow', '<div id=p popover=manual></div><button id=b popovertarget=p popovertargetaction=show></button>', click('#b'));
  await run('ptFormDefault', '<div id=p popover></div><form id=f><button id=b popovertarget=p></button></form>', click('#b'));
  await run('ptFormButton', '<div id=p popover></div><form id=f><button id=b type=button popovertarget=p></button></form>', click('#b'));
  await run('ptFormReset', '<div id=p popover></div><form id=f><button id=b type=reset popovertarget=p></button></form>', click('#b'));
  await run('ptFormAttributeOwner', '<div id=p popover></div><form id=f></form><button id=b form=f popovertarget=p></button>', click('#b'));
  await run('ptNoFormSubmit', '<div id=p popover></div><button id=b type=submit popovertarget=p></button>', click('#b'));
  await run('ptAutoFormInvoker', '<div id=p popover></div><form id=f><button id=b command=bogus popovertarget=p></button></form>', click('#b'));
  for (const type of ['button', 'reset', 'submit', 'image', 'text', 'checkbox']) {
    await run('ptInput_' + type, '<div id=p popover></div><input id=b type=' + type + ' popovertarget=p>', click('#b'));
    await run('ptInputForm_' + type, '<div id=p popover></div><form id=f><input id=b type=' + type + ' popovertarget=p></form>', click('#b'));
  }
  await run('ptInputDisabled', '<div id=p popover></div><input type=button id=b disabled popovertarget=p>', click('#b'));
  await run('ptInputCommandfor', '<div id=p popover></div><input type=button id=b commandfor=p command=toggle-popover>', click('#b'));
  await run('ptDisabled', '<div id=p popover></div><button id=b disabled popovertarget=p></button>', click('#b'));
  await run('ptDisabledDispatch', '<div id=p popover></div><button id=b disabled popovertarget=p></button>', () => q('#b').dispatchEvent(new MouseEvent('click', { bubbles: true })));
  await run('ptFieldset', '<div id=p popover></div><fieldset disabled><button id=b popovertarget=p></button></fieldset>', click('#b'));
  await run('ptFieldsetLegend', '<div id=p popover></div><fieldset disabled><legend><button id=b popovertarget=p></button></legend></fieldset>', click('#b'));
  await run('ptNoPopover', '<div id=p></div><button id=b popovertarget=p></button>', click('#b'));
  await run('ptChild', '<div id=p popover></div><button id=b popovertarget=p><span id=s></span></button>', click('#s'));
  await run('ptAnchorChild', '<div id=p popover></div><button id=b popovertarget=p><a id=a>x</a></button>', click('#a'));
  await run('ptHrefChild', '<div id=p popover></div><button id=b popovertarget=p><a id=a href="#nowhere">x</a></button>', () => q('#a').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })));
  await run('ptPlainEvent', '<div id=p popover></div><button id=b popovertarget=p></button>', () => q('#b').dispatchEvent(new Event('click', { bubbles: true })));
  await run('ptMouseEvent', '<div id=p popover></div><button id=b popovertarget=p></button>', () => q('#b').dispatchEvent(new MouseEvent('click', { bubbles: true })));
  await run('ptPointerEvent', '<div id=p popover></div><button id=b popovertarget=p></button>', () => q('#b').dispatchEvent(new PointerEvent('click', { bubbles: true })));
  await run('ptOtherType', '<div id=p popover></div><button id=b popovertarget=p></button>', () => q('#b').dispatchEvent(new MouseEvent('mouseup', { bubbles: true })));
  await run('ptNonBubblingChild', '<div id=p popover></div><button id=b popovertarget=p><span id=s></span></button>', () => q('#s').dispatchEvent(new MouseEvent('click', { bubbles: false })));
  await run('ptClickPrevented', '<div id=p popover></div><button id=b popovertarget=p></button>', () => { q('#b').addEventListener('click', event => event.preventDefault()); q('#b').click(); });
  await run('ptInsidePopover', '<button id=b popovertarget=p><div id=p popover><span id=s></span></div></button>', () => { q('#p').showPopover(); q('#s').click(); });
  await run('ptNested', '<div id=a popover><button id=b popovertarget=c></button></div><div id=c popover></div>', () => { q('#a').showPopover(); q('#b').click(); });
  await run('ptSibling', '<div id=a popover></div><div id=c popover></div><button id=b popovertarget=c></button>', () => { q('#a').showPopover(); q('#b').click(); });
  await run('ptDisconnected', '<div id=p popover></div>', () => { const b = document.createElement('button'); b.popoverTargetElement = q('#p'); b.click(); });
  await run('ptBeforetoggleCancel', '<div id=p popover></div><button id=b popovertarget=p></button>', () => { q('#p').addEventListener('beforetoggle', event => event.preventDefault()); q('#b').click(); });
  await run('ptBeforetoggleRemoves', '<div id=p popover></div><button id=b popovertarget=p></button>', () => { q('#p').addEventListener('beforetoggle', event => event.target.remove()); q('#b').click(); });
  for (const command of ['toggle-popover', 'show-popover', 'hide-popover', 'Show-Popover', '--custom', 'bogus', '', 'show-modal', 'close', 'request-close'])
    await run('cmdPopover_' + command, '<div id=p popover></div><button id=b commandfor=p command="' + command + '"></button>', click('#b'));
  await run('cmdNoCommand', '<div id=p popover></div><button id=b commandfor=p></button>', click('#b'));
  await run('cmdHideOpen', '<div id=p popover></div><button id=b commandfor=p command=hide-popover></button>', () => { q('#p').showPopover(); q('#b').click(); });
  await run('cmdShowOpen', '<div id=p popover></div><button id=b commandfor=p command=show-popover></button>', () => { q('#p').showPopover(); q('#b').click(); });
  await run('cmdToggleOpen', '<div id=p popover></div><button id=b commandfor=p command=toggle-popover></button>', () => { q('#p').showPopover(); q('#b').click(); });
  for (const command of ['toggle-popover', '--custom', 'show-modal'])
    await run('cmdDiv_' + command, '<div id=p></div><button id=b commandfor=p command="' + command + '"></button>', click('#b'));
  await run('cmdSvgCustom', '<svg><g id=p></g></svg><button id=b commandfor=p command=--x></button>', click('#b'));
  await run('cmdSvgPopover', '<svg><g id=p></g></svg><button id=b commandfor=p command=toggle-popover></button>', click('#b'));
  await run('cmdMathCustom', '<math><mi id=p></mi></math><button id=b commandfor=p command=--x></button>', click('#b'));
  // Dialog command events fire; the dialog steps they would run are not observed here.
  for (const command of ['close', 'request-close', 'toggle-popover', '--x'])
    await run('cmdDialog_' + command, '<dialog id=d></dialog><button id=b commandfor=d command="' + command + '" value=v></button>', click('#b'));
  await run('cmdDialogShowModalCanceled', '<dialog id=d></dialog><button id=b commandfor=d command=show-modal></button>', () => { q('#d').addEventListener('command', event => event.preventDefault()); q('#b').click(); });
  await run('cmdCanceled', '<div id=p popover></div><button id=b commandfor=p command=toggle-popover></button>', () => { q('#p').addEventListener('command', event => event.preventDefault()); q('#b').click(); });
  await run('cmdRemoveTarget', '<div id=p popover></div><button id=b commandfor=p command=toggle-popover></button>', () => { q('#p').addEventListener('command', event => event.target.remove()); q('#b').click(); });
  await run('cmdRemovePopoverAttribute', '<div id=p popover></div><button id=b commandfor=p command=show-popover></button>', () => { q('#p').addEventListener('command', event => event.target.removeAttribute('popover')); q('#b').click(); });
  await run('cmdChangesCommand', '<div id=p popover></div><button id=b commandfor=p command=show-popover></button>', () => { q('#p').addEventListener('command', () => { q('#b').command = 'hide-popover'; }); q('#b').click(); });
  await run('cmdAndPt', '<div id=p popover></div><div id=r popover></div><button id=b commandfor=p command=toggle-popover popovertarget=r></button>', click('#b'));
  await run('cmdInvalidAndPt', '<div id=p popover></div><div id=r popover></div><button id=b commandfor=p command=bogus popovertarget=r></button>', click('#b'));
  await run('cmdMissingTargetAndPt', '<div id=r popover></div><button id=b commandfor=zz command=toggle-popover popovertarget=r></button>', click('#b'));
  await run('cmdInvalidForTargetAndPt', '<div id=p></div><div id=r popover></div><button id=b commandfor=p command=show-modal popovertarget=r></button>', click('#b'));
  await run('cmdCanceledAndPt', '<div id=p popover></div><div id=r popover></div><button id=b commandfor=p command=toggle-popover popovertarget=r></button>', () => { q('#p').addEventListener('command', event => event.preventDefault()); q('#b').click(); });
  await run('cmdCustomAndPt', '<div id=p popover></div><div id=r popover></div><button id=b commandfor=p command=--x popovertarget=r></button>', click('#b'));
  await run('cmdFormDefault', '<div id=p popover></div><form id=f><button id=b commandfor=p command=toggle-popover></button></form>', click('#b'));
  await run('cmdFormOnlyCommand', '<div id=p popover></div><form id=f><button id=b command=toggle-popover></button></form>', click('#b'));
  await run('cmdFormSubmit', '<div id=p popover></div><form id=f><button id=b type=submit commandfor=p command=toggle-popover></button></form>', click('#b'));
  await run('cmdFormInvalidType', '<div id=p popover></div><form id=f><button id=b type=bogus commandfor=p command=toggle-popover></button></form>', click('#b'));
  await run('cmdFormButton', '<div id=p popover></div><form id=f><button id=b type=button commandfor=p command=toggle-popover></button></form>', click('#b'));
  await run('cmdFormReset', '<div id=p popover></div><form id=f><button id=b type=reset commandfor=p command=toggle-popover></button></form>', click('#b'));
  await run('cmdNoFormReset', '<div id=p popover></div><button id=b commandfor=p command=toggle-popover type=reset></button>', click('#b'));
  await run('cmdNoFormSubmit', '<div id=p popover></div><button id=b commandfor=p command=toggle-popover type=submit></button>', click('#b'));
  await run('cmdDisabled', '<div id=p popover></div><button id=b disabled commandfor=p command=toggle-popover></button>', click('#b'));
  await run('cmdDisconnected', '<div id=p popover></div>', () => { const b = document.createElement('button'); b.command = 'toggle-popover'; b.commandForElement = q('#p'); b.click(); });
  await run('cmdChild', '<div id=p popover></div><button id=b commandfor=p command=toggle-popover><span id=s></span></button>', click('#s'));
  await run('cmdNested', '<div id=a popover><button id=b commandfor=c command=show-popover></button></div><div id=c popover></div>', () => { q('#a').showPopover(); q('#b').click(); });
  await run('cmdInsidePopover', '<button id=b commandfor=p command=toggle-popover><div id=p popover><span id=s></span></div></button>', () => { q('#p').showPopover(); q('#s').click(); });
  // Hosts are created after listeners attach: only the invoker's retargeted source is observed.
  const host = () => { const element = document.createElement('div'); element.id = 'host'; body.append(element); return element; };
  await run('cmdShadowButton', '<div id=p popover></div>', () => { const root = host().attachShadow({ mode: 'open' }); root.innerHTML = '<button id=b><span id=s></span></button>'; const b = root.querySelector('#b'); b.command = 'show-popover'; b.commandForElement = q('#p'); root.querySelector('#s').click(); });
  await run('ptShadowUncomposed', '<div id=p popover></div>', () => { const root = host().attachShadow({ mode: 'open' }); root.innerHTML = '<button id=b><span id=s></span></button>'; root.querySelector('#b').popoverTargetElement = q('#p'); root.querySelector('#s').dispatchEvent(new MouseEvent('click', { bubbles: true, composed: false })); });
  await run('ptLightButtonShadowPopover', '<div id=host></div><button id=b popovertarget=sp></button>', () => { const root = q('#host').attachShadow({ mode: 'open' }); root.innerHTML = '<div id=sp popover></div>'; q('#b').click(); });

  // Command event fields at the target and ordering against microtasks/tasks.
  {
    body.innerHTML = '<div id=p popover></div><button id=b commandfor=p command=--X></button>';
    const r = {};
    q('#p').addEventListener('command', event => { r.fields = [event.type, event.command, event.bubbles, event.cancelable, event.composed, event.isTrusted, event.eventPhase, name(event.target), name(event.currentTarget), name(event.source), event.defaultPrevented, Object.getPrototypeOf(event) === CommandEvent.prototype]; });
    let bubbled = false, handler = 0; body.addEventListener('command', () => { bubbled = true; }); q('#p').oncommand = () => { handler++; };
    q('#b').click(); r.bubbled = bubbled; r.handler = [handler, 'oncommand' in HTMLElement.prototype];
    body.innerHTML = '<div id=p popover></div><button id=b commandfor=p command=toggle-popover></button>';
    const order = [];
    q('#b').addEventListener('click', () => { order.push('click'); Promise.resolve().then(() => order.push('micro-click')); });
    q('#p').addEventListener('command', () => { order.push('command'); Promise.resolve().then(() => order.push('micro-command')); });
    q('#p').addEventListener('beforetoggle', () => { order.push('beforetoggle'); Promise.resolve().then(() => order.push('micro-beforetoggle')); });
    q('#p').addEventListener('toggle', () => order.push('toggle'));
    q('#b').click(); order.push('after-click'); await tick(); await tick();
    r.order = order;
    out.events = r;
  }
  // click() is a no-op on disabled form controls (but not fieldsets, outputs or objects).
  {
    const r = {};
    for (const tag of ['button', 'input', 'select', 'textarea', 'fieldset', 'output', 'option', 'optgroup', 'object', 'div']) {
      const count = (parent, element) => { let calls = 0; element.addEventListener('click', () => calls++); element.click(); return calls; };
      const own = document.createElement(tag); own.setAttribute('disabled', '');
      const fieldset = document.createElement('fieldset'), inner = document.createElement(tag); fieldset.setAttribute('disabled', ''); fieldset.append(inner);
      const outer = document.createElement('fieldset'), legend = document.createElement('legend'), legendChild = document.createElement(tag); outer.setAttribute('disabled', ''); outer.append(legend); legend.append(legendChild);
      const second = document.createElement('fieldset'), other = document.createElement('legend'), late = document.createElement('legend'), lateChild = document.createElement(tag); second.setAttribute('disabled', ''); second.append(other, late); late.append(lateChild);
      const select = document.createElement('select'), option = document.createElement(tag); select.disabled = true; select.append(option);
      r[tag] = [count(null, own), count(fieldset, inner), count(outer, legendChild), count(second, lateChild), count(select, option)];
    }
    out.clickDisabled = r;
  }
  body.innerHTML = '';
  return out;
})()
