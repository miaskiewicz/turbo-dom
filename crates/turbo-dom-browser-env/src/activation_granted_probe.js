(async () => {
  // Activation-gated members WITH transient activation from trusted input: showPicker()
  // success and its consumption, the select rendering check, the fullscreen stack (nested
  // requests, re-requests, exit, prefixed forms, focus, removal while fullscreen or while a
  // request or exit is pending, shadow retargeting, :fullscreen) and details toggle events
  // (coalescing, unchanged states). Trusted input goes through the collector's `__cdpInput`
  // binding: {type:"down"|"up", x, y, target} presses or releases the mouse at x,y (offline
  // drivers use `target`, the element the point hits).
  const waiters = {}; let next = 0;
  globalThis.__cdpInputDone = id => { const resolve = waiters[id]; delete waiters[id]; resolve(); };
  const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
  const input = async (...steps) => { await wait(30); await new Promise(resolve => { const id = ++next; waiters[id] = resolve; __cdpInput(JSON.stringify({ id, steps })); }); };
  // A click at the top-left corner: the button, or the fullscreen element covering it.
  const activate = target => input({ type: 'down', x: 20, y: 20, target }, { type: 'up', x: 20, y: 20, target });
  const tag = n => n == null ? null : n === document ? 'document' : n === window ? 'window' : n.id || n.nodeName;
  const run = f => { try { const v = f(); return v === undefined ? 'undefined' : v; } catch (e) { return [e.name, e.message]; } };
  const settle = async p => { try { const v = await p; return ['ok', v === undefined ? 'undefined' : v]; } catch (e) { return ['reject', e.name, e.message]; } };
  let log = [];
  const types = ['fullscreenchange', 'fullscreenerror', 'webkitfullscreenchange', 'webkitfullscreenerror', 'toggle'];
  const record = event => log.push([event.type, tag(event.target)].concat(event.type === 'toggle' ? [event.oldState, event.newState] : []));
  for (const type of types) document.addEventListener(type, record, true);
  const $ = id => document.getElementById(id);
  const state = () => ({ element: tag(document.fullscreenElement), fullscreen: document.fullscreen, webkit: tag(document.webkitFullscreenElement), matches: [...document.querySelectorAll(':fullscreen')].map(tag), modal: [...document.querySelectorAll(':modal')].map(tag) });
  const out = {};
  const scenario = async (name, html, body) => {
    document.body.innerHTML = '<button id=b style="position:fixed;left:0;top:0;width:60px;height:60px;margin:0">b</button>' + html;
    log = [];
    const result = await body();
    await wait(30);
    out[name] = { result, log: log.slice() };
    while (document.fullscreenElement) { await document.exitFullscreen(); await wait(30); }
  };

  // showPicker succeeds once per activation: it consumes the activation.
  await scenario('inputPicker', '<input id=t>', async () => {
    await activate('b');
    return [run(() => $('t').showPicker()), run(() => $('t').showPicker())];
  });
  // A select that is not rendered refuses after the activation check, keeping activation.
  await scenario('selectNotRendered', '<select id=s1 style=display:none><option>1</select><div hidden><select id=s2><option>2</select></div><div style=display:none><select id=s3><option>3</select></div><input id=t>', async () => {
    await activate('b');
    return [run(() => $('s1').showPicker()), run(() => $('s2').showPicker()), run(() => $('s3').showPicker()), run(() => $('t').showPicker())];
  });

  // A request with activation resolves and consumes it; the change event follows.
  await scenario('request', '<div id=f>f<div id=g>g</div></div>', async () => {
    await activate('b');
    const promise = $('f').requestFullscreen();
    const sync = state();
    const result = await settle(promise);
    return [sync, result, state()];
  });
  // Nested requests stack; each exit pops one level.
  await scenario('nested', '<div id=f>f<div id=g>g</div></div>', async () => {
    await activate('b');
    await settle($('f').requestFullscreen());
    await activate('f');
    const inner = await settle($('g').requestFullscreen());
    const stacked = state();
    const first = await settle(document.exitFullscreen());
    const popped = state();
    const second = await settle(document.exitFullscreen());
    return [inner, stacked, first, popped, second, state(), await settle(document.exitFullscreen())];
  });
  // Re-requesting the current fullscreen element resolves without a change event; requesting
  // a lower stack entry moves it to the top.
  await scenario('rerequest', '<div id=f>f<div id=g>g</div></div>', async () => {
    await activate('b');
    await settle($('f').requestFullscreen());
    await activate('f');
    const again = await settle($('f').requestFullscreen());
    const same = state();
    await activate('f');
    await settle($('g').requestFullscreen());
    await activate('g');
    const lower = await settle($('f').requestFullscreen());
    return [again, same, lower, state(), await settle(document.exitFullscreen()), state()];
  });
  // Prefixed forms report through prefixed events only.
  await scenario('prefixed', '<div id=f>f</div>', async () => {
    await activate('b');
    const request = run(() => $('f').webkitRequestFullscreen());
    await wait(30);
    const entered = state();
    const exit = run(() => document.webkitExitFullscreen());
    await wait(30);
    const exited = state();
    await activate('b');
    run(() => $('f').webkitRequestFullScreen());
    await wait(30);
    const legacy = state();
    run(() => document.webkitCancelFullScreen());
    await wait(30);
    return [request, entered, exit, exited, legacy, state(), run(() => document.webkitExitFullscreen())];
  });
  // Entering fullscreen blurs the focused element outside the fullscreen element.
  await scenario('focus', '<input id=outside><div id=f><input id=inside></div>', async () => {
    $('outside').focus();
    await activate('b');
    $('outside').focus();
    await settle($('f').requestFullscreen());
    const outside = tag(document.activeElement);
    await document.exitFullscreen();
    $('inside').focus();
    await activate('b');
    $('inside').focus();
    await settle($('f').requestFullscreen());
    return [outside, tag(document.activeElement)];
  });
  // Removing the fullscreen element leaves the top layer at once; the change event targets
  // the document. Removing a lower stack entry fires nothing.
  await scenario('removeTop', '<div id=wrap><div id=f>f</div></div>', async () => {
    await activate('b');
    await settle($('f').requestFullscreen());
    $('f').remove();
    const removed = state();
    await wait(30);
    return [removed, state()];
  });
  await scenario('removePrefixedTop', '<div id=f>f</div>', async () => {
    await activate('b');
    $('f').webkitRequestFullscreen();
    await wait(30);
    $('f').remove();
    return [state()];
  });
  await scenario('removeChildren', '<div id=wrap><div id=f>f<div id=g>g</div></div></div>', async () => {
    await activate('b');
    await settle($('f').requestFullscreen());
    await activate('f');
    await settle($('g').requestFullscreen());
    $('f').replaceChildren();
    const inner = state();
    $('wrap').textContent = '';
    return [inner, state()];
  });
  await scenario('removeLower', '<div id=f>f</div><div id=g>g</div>', async () => {
    await activate('b');
    await settle($('f').requestFullscreen());
    await activate('f');
    await settle($('g').requestFullscreen());
    $('f').remove();
    return [state()];
  });
  // An element removed before the request task runs rejects it, firing the error at the
  // document.
  await scenario('removedWhilePending', '<div id=f>f</div>', async () => {
    await activate('b');
    const promise = $('f').requestFullscreen();
    $('f').remove();
    return [await settle(promise), state()];
  });
  // The fullscreen element removed before the exit task runs: exit still resolves.
  await scenario('removedWhileExiting', '<div id=f>f</div>', async () => {
    await activate('b');
    await settle($('f').requestFullscreen());
    const promise = document.exitFullscreen();
    $('f').remove();
    return [await settle(promise), state()];
  });
  // A fullscreen element in a shadow tree is retargeted to its host.
  await scenario('shadow', '<div id=host></div>', async () => {
    const root = $('host').attachShadow({ mode: 'open' });
    root.innerHTML = '<div id=inner>i</div>';
    await activate('b');
    await settle(root.getElementById('inner').requestFullscreen());
    const inside = [tag(root.fullscreenElement), tag(document.fullscreenElement)];
    $('host').remove();
    return [inside, state()];
  });

  // Details toggle events: a changed open state queues one toggle; changes within a task
  // coalesce into one event from the first old state; setting an unchanged state queues none.
  await scenario('details', '<details id=d><summary>s</summary>x</details><details id=e open><summary>s</summary>y</details>', async () => {
    const d = $('d'), e = $('e');
    d.open = true; d.open = false; d.open = true;
    e.setAttribute('open', 'again');
    await wait(10);
    d.open = true;
    d.setAttribute('open', 'x');
    d.open = false; d.open = true; d.open = false;
    await wait(10);
    e.open = false; e.open = true;
    await wait(10);
    return [d.open, e.open];
  });
  // A rendered select's picker opens with activation (and consumes it); this runs last since
  // Chrome then shows the picker.
  await scenario('selectRendered', '<select id=s><option>1</select>', async () => {
    await activate('b');
    return [run(() => $('s').showPicker()), run(() => $('s').showPicker())];
  });
  for (const type of types) document.removeEventListener(type, record, true);
  document.body.innerHTML = '';
  return out;
})()
