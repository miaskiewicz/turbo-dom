(async () => {
  // Popover method exceptions and nested calls (miaskiewicz/turbo-dom#139): Chrome 156's
  // "Failed to execute ..." messages, a show / toggle from a beforetoggle during a show (or
  // after a nested hide) throws, a nested hide during a hide runs without events, and
  // toggle tasks coalesce from the first old state.
  const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
  const run = f => { try { const v = f(); return v === undefined ? 'undefined' : v; } catch (e) { return [e.name, e.message]; } };
  const out = {};
  const scenario = async (name, body) => {
    document.body.innerHTML = '<div id=p popover>p</div>';
    const p = document.getElementById('p'), log = [];
    p.addEventListener('toggle', event => log.push([event.oldState, event.newState]));
    p.addEventListener('beforetoggle', event => log.push(['before', event.newState]));
    const result = body(p);
    await wait(20);
    out[name] = [result, log, p.matches(':popover-open')];
  };
  await scenario('showHide', p => { p.showPopover(); p.hidePopover(); });
  await scenario('showHideShow', p => { p.showPopover(); p.hidePopover(); p.showPopover(); });
  await scenario('nestedHide', p => {
    p.showPopover();
    let inner = null;
    const listener = event => { if (event.newState === 'closed') inner = [run(() => p.togglePopover()), p.matches(':popover-open'), run(() => p.togglePopover(true)), run(() => p.togglePopover(false)), run(() => p.showPopover()), run(() => p.hidePopover())]; };
    p.addEventListener('beforetoggle', listener);
    const result = run(() => p.hidePopover());
    p.removeEventListener('beforetoggle', listener);
    return [result, inner];
  });
  await scenario('nestedShow', p => {
    let inner = null;
    const listener = event => { if (event.newState === 'open') inner = [run(() => p.hidePopover()), run(() => p.togglePopover(false)), run(() => p.togglePopover(true)), run(() => p.showPopover())]; };
    p.addEventListener('beforetoggle', listener);
    const result = run(() => p.showPopover());
    p.removeEventListener('beforetoggle', listener);
    return [result, inner];
  });
  document.body.innerHTML = '<div id=p popover>p</div>';
  const p = document.getElementById('p');
  p.showPopover();
  await wait(20);
  const log = [];
  p.addEventListener('toggle', event => log.push([event.oldState, event.newState]));
  p.hidePopover(); p.showPopover();
  await wait(20);
  out.hideShow = log.splice(0);
  p.hidePopover(); p.showPopover(); p.hidePopover();
  await wait(20);
  out.hideShowHide = log.splice(0);
  const detached = document.createElement('div');
  detached.popover = 'auto';
  const plain = document.createElement('div');
  out.detached = [run(() => detached.showPopover()), run(() => detached.togglePopover()), run(() => detached.togglePopover(true)), run(() => detached.togglePopover(false)), run(() => detached.hidePopover())];
  out.notPopover = [run(() => plain.showPopover()), run(() => plain.hidePopover()), run(() => plain.togglePopover()), run(() => plain.togglePopover(false))];
  document.body.append(plain);
  out.notPopoverConnected = [run(() => plain.showPopover()), run(() => plain.hidePopover()), run(() => plain.togglePopover()), run(() => plain.togglePopover(false))];
  document.body.innerHTML = '';
  return out;
})()
