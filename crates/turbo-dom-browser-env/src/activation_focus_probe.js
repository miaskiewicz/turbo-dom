(async () => {
  // turbo-dom#188: focus event composition, the `:open` state a successful showPicker()
  // leaves on a control, and the dialog close-request (Esc) event order and focus fixup.
  // Trusted input goes through the collector's `__cdpInput` binding (see
  // activation_granted_probe.js); offline drivers use each step's `target`. File inputs are
  // left out: headless Chrome cancels their chooser at once (a `cancel` event), which
  // depends on the shell, not on the page.
  const waiters = {}; let next = 0;
  globalThis.__cdpInputDone = id => { const resolve = waiters[id]; delete waiters[id]; resolve(); };
  const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
  const input = async (...steps) => { await wait(30); await new Promise(resolve => { const id = ++next; waiters[id] = resolve; __cdpInput(JSON.stringify({ id, steps })); }); };
  const activate = target => input({ type: 'down', x: 20, y: 20, target }, { type: 'up', x: 20, y: 20, target });
  const escape = () => input({ type: 'key', key: 'Escape', code: 'Escape', keyCode: 27 });
  const tag = n => n == null ? null : n === document ? 'document' : n === window ? 'window' : n.id || n.nodeName;
  const run = f => { try { const v = f(); return v === undefined ? 'undefined' : v; } catch (e) { return [e.name, e.message]; } };
  let log = [];
  const record = event => log.push([event.type, tag(event.target), event.composed, event.bubbles]);
  for (const type of ['focus', 'blur', 'focusin', 'focusout', 'cancel', 'close', 'beforetoggle', 'toggle']) document.addEventListener(type, record, true);
  const $ = id => document.getElementById(id);
  const out = {};
  const scenario = async (name, html, body) => {
    if (document.activeElement && document.activeElement !== document.body) document.activeElement.blur();
    document.body.innerHTML = '<button id=b style="position:fixed;left:0;top:0;width:60px;height:60px;margin:0">b</button>' + html;
    await wait(30);
    log = [];
    const result = await body();
    await wait(30);
    out[name] = { result, log: log.slice() };
  };
  for (const [name, html] of [
    ['date', '<input id=t type=date>'], ['color', '<input id=t type=color>'], ['time', '<input id=t type=time>'],
    ['select', '<select id=t><option>a<option>b</select>'], ['text', '<input id=t>'],
  ]) {
    await scenario('picker-' + name, html, async () => {
      $('b').focus();
      await activate('b');
      const shown = run(() => $('t').showPicker());
      const open = run(() => $('t').matches(':open'));
      await wait(100);
      return { shown, open, openLater: run(() => $('t').matches(':open')), active: tag(document.activeElement) };
    });
  }
  await scenario('focus-move', '<input id=x><input id=y>', async () => { $('x').focus(); $('y').focus(); $('y').blur(); return tag(document.activeElement); });
  await scenario('escape-modal', '<dialog id=d><button id=inner>in</button></dialog>', async () => {
    $('d').showModal();
    const before = tag(document.activeElement);
    await escape();
    await wait(100);
    return { before, open: $('d').open, active: tag(document.activeElement) };
  });
  await scenario('escape-dialog-and-popover', '<dialog id=d>d<div id=p popover>p</div></dialog>', async () => {
    $('d').showModal(); $('p').showPopover();
    await escape();
    await wait(100);
    const first = { dialog: $('d').open, popover: $('p').matches(':popover-open') };
    await escape();
    await wait(100);
    return { first, dialog: $('d').open, popover: $('p').matches(':popover-open') };
  });
  await scenario('close-modal-method', '<dialog id=d><button id=inner>in</button></dialog>', async () => {
    $('d').showModal();
    const before = tag(document.activeElement);
    $('d').close();
    const sync = tag(document.activeElement);
    await wait(100);
    return { before, sync, active: tag(document.activeElement) };
  });
  return out;
})()
