(() => {
  'use strict';
  // turbo-dom#66: the DOM event path across shadow trees. An event dispatched in a shadow tree
  // crosses its root into the host when composed (stops at the root otherwise); every listener
  // sees event.target (and relatedTarget) retargeted against its own tree, hosts are AT_TARGET,
  // composedPath() hides closed shadow trees the listener cannot see, a node assigned to a slot
  // passes through the slot, propagation stops where the retargeted relatedTarget is the host,
  // and after dispatch target / relatedTarget are cleared when they stay inside a shadow tree.
  const roots = new Map();
  const label = (n) => n == null ? null : n === window ? 'W' : n === document ? 'D' : roots.has(n) ? roots.get(n) : n.id || n.nodeName;
  const $ = (id) => document.getElementById(id);
  const attach = (host, mode, html) => { const root = host.attachShadow({ mode }); root.innerHTML = html; roots.set(root, 'SR:' + host.id); return root; };
  const open = attach($('host'), 'open', '<div id="sa"><span id="st">t</span><slot id="slot"></slot></div>');
  const closed = attach($('chost'), 'closed', '<div id="ca"><b id="ct">c</b><div id="nhost"></div></div>');
  const nested = attach(closed.getElementById('nhost'), 'open', '<i id="nt">n</i>');
  const detachedHost = document.createElement('div');
  detachedHost.id = 'dhost';
  const detached = attach(detachedHost, 'open', '<p id="dt">d</p>');
  const find = (id) => $(id) || open.getElementById(id) || closed.getElementById(id) || nested.getElementById(id) || detached.getElementById(id);
  const nodes = [window, document, document.documentElement, document.body, $('outer'), $('host'), open, find('sa'), find('st'), find('slot'), $('light'), $('chost'), closed, find('ca'), find('ct'), find('nhost'), nested, find('nt'), detachedHost, detached, find('dt')];
  let log = [], stopAt = null;
  const record = (e) => {
    log.push([label(e.currentTarget), e.eventPhase, label(e.target), 'relatedTarget' in e ? label(e.relatedTarget) : '-', e.composedPath().map(label).join(' ')].join('|'));
    if (stopAt === e.currentTarget) e.stopPropagation();
  };
  for (const node of nodes) for (const type of ['x', 'mouseover', 'focusin']) { node.addEventListener(type, record, true); node.addEventListener(type, record); }
  const run = (target, make, stop) => {
    log = []; stopAt = stop || null;
    const event = make();
    const result = target.dispatchEvent(event);
    return { log, result, after: [label(event.target), 'relatedTarget' in event ? label(event.relatedTarget) : '-', event.composedPath().length, event.eventPhase, label(event.currentTarget)] };
  };
  const x = (bubbles, composed) => () => new Event('x', { bubbles, composed });
  const out = {};
  out.composedBubbles = run(find('st'), x(true, true));
  out.uncomposed = run(find('st'), x(true, false));
  out.composedNoBubble = run(find('st'), x(false, true));
  out.slotted = run($('light'), x(true, true));
  out.slottedUncomposed = run($('light'), x(true, false));
  out.closed = run(find('ct'), x(true, true));
  out.closedUncomposed = run(find('ct'), x(true, false));
  out.nested = run(find('nt'), x(true, true));
  out.atRoot = run(open, x(true, true));
  out.atHost = run($('host'), x(true, true));
  out.stoppedAtHost = run(find('st'), x(true, true), $('host'));
  out.stoppedAtRoot = run(find('st'), x(true, true), open);
  out.detached = run(find('dt'), x(true, true));
  const mouse = (related) => () => new MouseEvent('mouseover', { bubbles: true, composed: true, relatedTarget: related });
  out.relatedSameTree = run(find('st'), mouse(find('sa')));
  out.relatedOutside = run(find('st'), mouse($('outer')));
  out.relatedFromOutside = run($('outer'), mouse(find('st')));
  out.relatedClosed = run(find('ct'), mouse($('light')));
  out.relatedIsHost = run(find('st'), mouse($('host')));
  const focus = (related) => () => new FocusEvent('focusin', { bubbles: true, composed: true, relatedTarget: related });
  out.focusRelatedClosed = run($('light'), focus(find('ct')));
  out.focusRelatedNested = run(find('ct'), focus(find('nt')));
  return out;
})();
