(async () => {
  // Trusted input through the collector's `__cdpInput` binding (offline: the engine's host driver).
  let next = 0; const waiters = {}; globalThis.__cdpTargets = [];
  globalThis.__cdpInputDone = id => { const resolve = waiters[id]; delete waiters[id]; resolve(); };
  const input = (...steps) => new Promise(resolve => { const id = ++next; waiters[id] = resolve; __cdpInput(JSON.stringify({ id, steps })); });
  const hover = node => { const box = node.getBoundingClientRect(); __cdpTargets.push(node); return input({ type: 'move', target: __cdpTargets.length - 1, x: Math.round(box.left + box.width / 2), y: Math.round(box.top + box.height / 2) }); };
  const key = name => input({ type: 'key', key: name, code: name, keyCode: { Escape: 27 }[name] || 0 });
  const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
  const out = {}, body = document.body, q = selector => document.querySelector(selector);
  const name = node => node === null ? null : node === undefined ? 'undefined' : node.id || node.localName || node.nodeName;
  const caught = fn => { try { return fn(); } catch (error) { return 'E:' + error.name + ':' + error.message; } };
  const describe = (proto, key) => {
    const d = Object.getOwnPropertyDescriptor(proto, key);
    if (!d) return null;
    const fn = f => f ? [f.name, f.length, caught(() => new f())] : null;
    return [d.enumerable, d.configurable, fn(d.get), fn(d.set), 'value' in d];
  };
  // Hover and Escape cases wait on input round trips: long delays keep their margins wide.
  const SLOW = '<style>button,a{interest-delay:800ms 800ms}</style>';
  let log = [];
  const watch = (...nodes) => { for (const node of nodes) for (const type of ['interest', 'loseinterest', 'beforetoggle', 'toggle']) node.addEventListener(type, event => log.push([name(event.currentTarget), type, name(event.source), event.newState || ''])); };
  const state = (...nodes) => nodes.map(node => [node.matches(':interest-source'), node.matches(':interest-target'), node.matches(':popover-open')].map(Number).join(''));
  const mark = (label, ...nodes) => log.push([label, ...state(...nodes)]);
  const scenario = async (html, run) => {
    await hover(q('#park') || body);
    if (document.activeElement && document.activeElement !== body) document.activeElement.blur();
    body.innerHTML = '<div id=park style="position:fixed;top:520px;left:0;width:20px;height:20px"></div>' + html;
    await sleep(400);
    log = [];
    await run();
    return log;
  };

  // Surface: interestForElement reflection and the InterestEvent interface.
  {
    const r = {};
    r.descriptors = ['HTMLButtonElement', 'HTMLAnchorElement', 'HTMLAreaElement', 'HTMLInputElement', 'HTMLElement', 'Element'].map(type => [type, describe(globalThis[type].prototype, 'interestForElement')]);
    r.handlers = ['oninterest', 'onloseinterest'].map(key => key in HTMLElement.prototype || key in window);
    r.svgInterface = [typeof SVGAElement, typeof SVGAElement === 'function' ? describe(SVGAElement.prototype, 'interestForElement') : null];
    body.innerHTML = '<div id=p popover></div><span id=s></span>';
    r.reflection = [];
    for (const tag of ['button', 'a', 'area']) {
      const live = document.createElement(tag), row = [live.interestForElement]; body.append(live);
      live.setAttribute('interestfor', 'p'); row.push(name(live.interestForElement));
      live.setAttribute('interestfor', 'missing'); row.push(live.interestForElement);
      live.interestForElement = q('#s'); row.push(live.getAttribute('interestfor'), name(live.interestForElement));
      live.setAttribute('interestfor', 'p'); row.push(name(live.interestForElement));
      const span = q('#s'); live.interestForElement = span; span.remove(); row.push(live.interestForElement); body.append(span); row.push(name(live.interestForElement));
      live.interestForElement = null; row.push(live.getAttribute('interestfor'), live.interestForElement);
      row.push(caught(() => { live.interestForElement = {}; }), live.getAttribute('interestfor'));
      r.reflection.push([tag, row]);
    }
    const getter = Object.getOwnPropertyDescriptor(HTMLButtonElement.prototype, 'interestForElement').get;
    r.brands = [document.createElement('button'), document.createElement('a'), document.createElement('div'), document.createElementNS('urn:x', 'button'), {}, null].map(receiver => caught(() => name(getter.call(receiver))));
    const anchorGetter = Object.getOwnPropertyDescriptor(HTMLAnchorElement.prototype, 'interestForElement').get;
    r.anchorBrands = [document.createElement('a'), document.createElement('area'), document.createElement('button')].map(receiver => caught(() => name(anchorGetter.call(receiver))));
    r.event = [InterestEvent.length, InterestEvent.name, Object.getPrototypeOf(InterestEvent) === Event, Object.getPrototypeOf(InterestEvent.prototype) === Event.prototype, Object.getOwnPropertyNames(InterestEvent.prototype).sort(), describe(InterestEvent.prototype, 'source'), String(InterestEvent.prototype[Symbol.toStringTag])];
    const plain = new InterestEvent('interest');
    r.plain = [plain.type, plain.source, plain.bubbles, plain.cancelable, plain.composed, plain.isTrusted, Object.prototype.toString.call(plain)];
    const span = document.createElement('span');
    r.init = [name(new InterestEvent('x', { source: span, bubbles: true, cancelable: true, composed: true }).source), new InterestEvent('x', { source: null }).source, new InterestEvent('x', { source: undefined }).source, new InterestEvent('x', null).source, new InterestEvent(5).type];
    r.errors = [caught(() => new InterestEvent()), caught(() => InterestEvent('x')), caught(() => new InterestEvent('x', { source: {} })), caught(() => new InterestEvent('x', { source: document })), caught(() => new InterestEvent('x', 5)), caught(() => Object.getOwnPropertyDescriptor(InterestEvent.prototype, 'source').get.call(new Event('x')))];
    // A source inside a shadow tree is retargeted for listeners outside it.
    const host = document.createElement('div'); body.append(host);
    const inner = host.attachShadow({ mode: 'closed' }).appendChild(document.createElement('b')); inner.id = 'inner';
    const event = new InterestEvent('x', { source: inner });
    let seen = null; body.addEventListener('x', e => { seen = name(e.source); }, { once: true });
    body.dispatchEvent(event);
    r.retarget = [name(event.source), seen];
    out.surface = r;
  }

  // Programmatic focus()/blur(): CSS-timed delays, events, popover showing and state.
  {
    const r = {};
    r.defaults = await scenario('<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); q('#b').focus(); mark('focus', q('#b'), q('#p')); await sleep(250); mark('250', q('#b'), q('#p')); await sleep(400); mark('650', q('#b'), q('#p'));
      q('#b').blur(); mark('blur', q('#b'), q('#p')); await sleep(100); mark('100', q('#b'), q('#p')); await sleep(300); mark('400', q('#b'), q('#p'));
    });
    r.delays = await scenario('<style>#b{interest-delay:100ms 400ms}</style><button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); q('#b').focus(); await sleep(250); mark('250', q('#b'), q('#p')); q('#b').blur(); await sleep(250); mark('250', q('#b'), q('#p')); await sleep(300); mark('550', q('#b'), q('#p'));
    });
    r.longhands = await scenario('<style>#b{interest-delay-start:0s;interest-delay-end:0ms}</style><button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); q('#b').focus(); await sleep(100); mark('100', q('#b'), q('#p')); q('#b').blur(); await sleep(100); mark('100', q('#b'), q('#p'));
    });
    r.targetDelays = await scenario('<style>#p{interest-delay:0s 0s}</style><button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); q('#b').focus(); await sleep(250); mark('250', q('#b'), q('#p')); await sleep(400); mark('650', q('#b'), q('#p')); q('#b').blur(); await sleep(400);
    });
    r.cancelInterest = await scenario('<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); q('#p').addEventListener('interest', event => event.preventDefault()); q('#b').focus(); await sleep(650); mark('650', q('#b'), q('#p')); q('#b').blur(); await sleep(400); mark('400', q('#b'), q('#p'));
    });
    r.cancelLose = await scenario('<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); let cancel = true; q('#p').addEventListener('loseinterest', event => { if (cancel) event.preventDefault(); });
      q('#b').focus(); await sleep(650); q('#b').blur(); await sleep(400); mark('kept', q('#b'), q('#p'));
      q('#b').focus(); await sleep(650); mark('refocus', q('#b'), q('#p')); cancel = false; q('#b').blur(); await sleep(400); mark('lost', q('#b'), q('#p'));
    });
    r.plainTarget = await scenario('<button id=b interestfor=p>b</button><div id=p>plain</div>', async () => {
      watch(q('#p')); q('#b').focus(); await sleep(650); mark('650', q('#b'), q('#p')); q('#b').blur(); await sleep(400); mark('400', q('#b'), q('#p'));
    });
    for (const type of ['manual', 'hint', 'auto']) r['popover_' + type] = await scenario(`<div id=other popover>o</div><button id=b interestfor=p>b</button><div id=p popover=${type}>pop</div>`, async () => {
      q('#other').showPopover(); await sleep(50); watch(q('#p'), q('#other'));
      q('#b').focus(); await sleep(650); mark('shown', q('#b'), q('#p'), q('#other')); q('#b').blur(); await sleep(400); mark('blur', q('#b'), q('#p'), q('#other'));
    });
    r.openAlready = await scenario('<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      q('#p').showPopover(); await sleep(50); watch(q('#p')); q('#b').focus(); await sleep(650); mark('shown', q('#b'), q('#p')); q('#b').blur(); await sleep(400); mark('blur', q('#b'), q('#p'));
    });
    r.quickBlur = await scenario('<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); q('#b').focus(); await sleep(200); q('#b').blur(); await sleep(700); mark('end', q('#b'), q('#p'));
    });
    r.focusIntoTarget = await scenario('<button id=b interestfor=p>b</button><div id=p popover><button id=inner>i</button></div>', async () => {
      watch(q('#p')); q('#b').focus(); await sleep(650); q('#inner').focus(); await sleep(400); mark('inner', q('#b'), q('#p')); q('#inner').blur(); await sleep(400); mark('out', q('#b'), q('#p'));
    });
    r.removeAttribute = await scenario('<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); q('#b').focus(); await sleep(650); q('#b').removeAttribute('interestfor'); mark('removed', q('#b'), q('#p')); q('#b').blur(); await sleep(400); mark('400', q('#b'), q('#p'));
    });
    r.retargetAttribute = await scenario('<button id=b interestfor=p>b</button><div id=p popover>pop</div><div id=p2 popover>two</div>', async () => {
      watch(q('#p'), q('#p2')); q('#b').focus(); await sleep(650); q('#b').setAttribute('interestfor', 'p2'); mark('changed', q('#b'), q('#p'), q('#p2')); await sleep(650); mark('650', q('#b'), q('#p'), q('#p2'));
    });
    r.disconnectInvoker = await scenario('<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); const b = q('#b'); b.focus(); await sleep(650); b.remove(); mark('removed', b, q('#p')); await sleep(400); mark('400', b, q('#p'));
    });
    r.disconnectTarget = await scenario('<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      const p = q('#p'); watch(p); q('#b').focus(); await sleep(650); p.remove(); mark('removed', q('#b'), p); q('#b').blur(); await sleep(400); mark('400', q('#b'), p);
    });
    r.pendingDisconnect = await scenario('<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); const b = q('#b'); b.focus(); await sleep(200); b.remove(); await sleep(500); mark('600', b, q('#p'));
    });
    r.twoInvokers = await scenario('<button id=b1 interestfor=p>1</button><button id=b2 interestfor=p>2</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); q('#b1').focus(); await sleep(650); q('#b2').focus(); await sleep(100); mark('100', q('#b1'), q('#b2'), q('#p')); await sleep(550); mark('650', q('#b1'), q('#b2'), q('#p')); q('#b2').blur(); await sleep(400); mark('blur', q('#b1'), q('#b2'), q('#p'));
    });
    r.click = await scenario('<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); q('#b').click(); await sleep(650); mark('click', q('#b'), q('#p'));
    });
    r.anchor = await scenario('<a id=a href=#x interestfor=p>a</a><div id=p popover>pop</div>', async () => {
      watch(q('#p')); q('#a').focus(); await sleep(650); mark('a', q('#a'), q('#p')); q('#a').blur(); await sleep(400); mark('blur', q('#a'), q('#p'));
    });
    r.referenceTarget = await scenario('<button id=b interestfor=h>b</button><div id=h></div>', async () => {
      const root = q('#h').attachShadow({ mode: 'open', referenceTarget: 'p' }); root.innerHTML = '<div id=p popover>pop</div>';
      watch(root.querySelector('#p')); q('#b').focus(); await sleep(650); mark('shown', q('#b'), q('#h'), root.querySelector('#p')); q('#b').blur(); await sleep(400); mark('blur', q('#b'), root.querySelector('#p'));
    });
    r.event = await scenario('<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      for (const type of ['interest', 'loseinterest']) q('#p').addEventListener(type, event => log.push([event.constructor.name, event.type, event.bubbles, event.cancelable, event.composed, event.isTrusted, name(event.source), name(event.target), event.eventPhase, event.defaultPrevented]));
      q('#b').focus(); await sleep(650); q('#b').blur(); await sleep(400);
    });
    out.focus = r;
  }

  // Trusted hover over invokers and their targets.
  {
    const r = {};
    r.hover = await scenario(SLOW + '<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); await hover(q('#b')); mark('over', q('#b'), q('#p')); await sleep(300); mark('300', q('#b'), q('#p')); await sleep(800); mark('1100', q('#b'), q('#p'));
      await hover(q('#park')); await sleep(300); mark('300', q('#b'), q('#p')); await sleep(800); mark('1100', q('#b'), q('#p'));
    });
    r.ontoTarget = await scenario(SLOW + '<button id=b interestfor=p>b</button><div id=p popover style="position:fixed;inset:auto;top:200px;left:200px;width:50px;height:50px;margin:0">pop</div>', async () => {
      watch(q('#p')); await hover(q('#b')); await sleep(1100); await hover(q('#p')); await sleep(1100); mark('target', q('#b'), q('#p')); await hover(q('#park')); await sleep(1100); mark('left', q('#b'), q('#p'));
    });
    r.child = await scenario(SLOW + '<button id=b interestfor=p><span id=c>child</span></button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); await hover(q('#c')); await sleep(1100); mark('child', q('#b'), q('#p')); await hover(q('#b')); await sleep(1100); mark('button', q('#b'), q('#p')); await hover(q('#park')); await sleep(1100); mark('left', q('#b'), q('#p'));
    });
    r.quickLeave = await scenario(SLOW + '<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); await hover(q('#b')); await sleep(300); await hover(q('#park')); await sleep(1100); mark('end', q('#b'), q('#p'));
    });
    r.disabled = await scenario(SLOW + '<button id=b disabled interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); await hover(q('#b')); await sleep(1100); mark('1100', q('#b'), q('#p'));
    });
    r.hrefless = await scenario(SLOW + '<a id=a interestfor=p>no href</a><div id=p popover>pop</div>', async () => {
      watch(q('#p')); await hover(q('#a')); await sleep(1100); mark('1100', q('#a'), q('#p')); await hover(q('#park')); await sleep(1100);
    });
    r.svg = await scenario(SLOW + '<svg width=40 height=40><a id=s href=#y interestfor=p><rect width=40 height=40></rect></a></svg><div id=p popover>pop</div>', async () => {
      watch(q('#p')); await hover(q('#s').firstChild); await sleep(1100); mark('1100', q('#s'), q('#p')); await hover(q('#park')); await sleep(1100); mark('left', q('#s'), q('#p'));
    });
    r.hoverAndFocus = await scenario(SLOW + '<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); await hover(q('#b')); q('#b').focus(); await sleep(1100); await hover(q('#park')); await sleep(1100); mark('focused', q('#b'), q('#p')); q('#b').blur(); await sleep(1100); mark('blurred', q('#b'), q('#p'));
    });
    out.hover = r;
  }

  // Escape loses interest immediately.
  {
    const r = {};
    r.focused = await scenario(SLOW + '<button id=b interestfor=p>b</button><div id=p popover>pop</div>', async () => {
      watch(q('#p')); q('#b').focus(); await sleep(1100); await key('Escape'); mark('esc', q('#b'), q('#p')); await sleep(1100); mark('1100', q('#b'), q('#p')); log.push(q('#b') === document.activeElement);
    });
    r.hovered = await scenario(SLOW + '<button id=b interestfor=p>b</button><div id=p>plain</div>', async () => {
      watch(q('#p')); await hover(q('#b')); await sleep(1100); await key('Escape'); mark('esc', q('#b'), q('#p')); await hover(q('#park')); await sleep(1100);
    });
    r.prevented = await scenario(SLOW + '<button id=b interestfor=p>b</button><div id=p>plain</div>', async () => {
      watch(q('#p')); q('#b').addEventListener('keydown', event => event.preventDefault()); q('#b').focus(); await sleep(1100); await key('Escape'); mark('esc', q('#b'), q('#p')); q('#b').blur(); await sleep(1100);
    });
    r.pending = await scenario(SLOW + '<button id=b interestfor=p>b</button><div id=p>plain</div>', async () => {
      watch(q('#p')); q('#b').focus(); await sleep(300); await key('Escape'); await sleep(800); mark('1100', q('#b'), q('#p')); q('#b').blur(); await sleep(1100);
    });
    r.canceled = await scenario(SLOW + '<button id=b interestfor=p>b</button><div id=p>plain</div>', async () => {
      watch(q('#p')); q('#p').addEventListener('loseinterest', event => { log.push(['cancelable', event.cancelable]); event.preventDefault(); }, { once: true }); q('#b').focus(); await sleep(1100); await key('Escape'); mark('esc', q('#b'), q('#p')); q('#b').blur(); await sleep(1100); mark('blur', q('#b'), q('#p'));
    });
    out.escape = r;
  }

  // Selector state while interest is shown.
  {
    const r = {};
    r.selectors = await scenario('<button id=b interestfor=p>b</button><div id=p>plain</div><button id=other>o</button>', async () => {
      q('#b').focus(); await sleep(650);
      log.push([...document.querySelectorAll(':interest-source')].map(name), [...document.querySelectorAll(':interest-target')].map(name), [...document.querySelectorAll('button:not(:interest-source)')].map(name), q('#b').matches(':is(:interest-source, #none)'), name(q('#p').closest(':interest-target')));
      q('#b').blur(); await sleep(400);
      log.push([...document.querySelectorAll(':interest-source, :interest-target')].map(name));
    });
    out.selectors = r;
  }
  return out;
})()
