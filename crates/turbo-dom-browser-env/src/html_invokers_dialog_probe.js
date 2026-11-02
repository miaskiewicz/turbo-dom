(async () => {
  // Dialog invokers: `commandRows` is the CommandEvent dispatch; `steps` the events the
  // show-modal/close/request-close dialog command steps then fire, with their sources.
  const out = {}, body = document.body, q = selector => body.querySelector(selector);
  const tick = () => new Promise(resolve => setTimeout(resolve, 0));
  const name = node => node === null ? null : node.id || node.localName;
  const cases = [
    ['showModalClosed', '<dialog id=d></dialog>', 'show-modal'],
    ['showModalOpen', '<dialog id=d open></dialog>', 'show-modal'],
    ['closeOpen', '<dialog id=d open></dialog>', 'close'],
    ['closeClosed', '<dialog id=d></dialog>', 'close'],
    ['requestCloseOpen', '<dialog id=d open></dialog>', 'request-close'],
    ['requestCloseClosed', '<dialog id=d></dialog>', 'request-close'],
    ['showModalPopoverDialog', '<dialog id=d popover></dialog>', 'show-modal'],
  ];
  for (const [label, html, command] of cases) {
    body.innerHTML = html + '<button id=b commandfor=d command=' + command + ' value=v></button>';
    const rows = [], steps = [];
    for (const type of ['command', 'cancel', 'beforetoggle', 'toggle', 'close']) q('#d').addEventListener(type, event => {
      if (type === 'command') rows.push([event.command, name(event.source), event.cancelable, event.composed, event.isTrusted]);
      steps.push([type, event.source === undefined ? 'undefined' : name(event.source)]);
    });
    q('#b').click(); await tick(); await tick();
    out[label] = { commandRows: rows, steps, open: q('#d').hasAttribute('open'), returnValue: 'returnValue' in q('#d') ? q('#d').returnValue : null };
  }
  body.innerHTML = '';
  return out;
})()
