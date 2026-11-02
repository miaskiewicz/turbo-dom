(async () => {
  // Trusted keys through the collector's `__cdpInput` binding (offline: the engine's host driver).
  let next = 0; const waiters = {};
  globalThis.__cdpInputDone = id => { const resolve = waiters[id]; delete waiters[id]; resolve(); };
  const input = (...steps) => new Promise(resolve => { const id = ++next; waiters[id] = resolve; __cdpInput(JSON.stringify({ id, steps })); });
  const codes = { ArrowRight: 39, ArrowLeft: 37, ArrowUp: 38, ArrowDown: 40, Home: 36, End: 35, PageDown: 34, a: 65 };
  const deep = () => { let active = document.activeElement; while (active && active.shadowRoot && active.shadowRoot.activeElement) active = active.shadowRoot.activeElement; return active; };
  const press = async (...keys) => {
    const out = [];
    for (const entry of keys) {
      const [mods, key] = entry.includes('+') ? entry.split('+') : ['', entry];
      const modifiers = (mods.includes('S') ? 8 : 0) | (mods.includes('C') ? 2 : 0) | (mods.includes('A') ? 1 : 0) | (mods.includes('M') ? 4 : 0);
      await input({ type: 'key', key, code: key, keyCode: codes[key], modifiers });
      const active = deep(); out.push(active.id || active.localName);
    }
    return out;
  };
  const out = {}, body = document.body;
  const caught = fn => { try { return fn(); } catch (error) { return 'E:' + error.name + ':' + error.message; } };
  const describe = (proto, key) => {
    const d = Object.getOwnPropertyDescriptor(proto, key);
    if (!d) return null;
    const fn = f => f ? [f.name, f.length, caught(() => new f())] : null;
    return [d.enumerable, d.configurable, fn(d.get), fn(d.set), 'value' in d];
  };
  const scenario = async (html, start, keys) => { body.innerHTML = html; document.getElementById(start).focus(); return press(...keys); };
  const R = 'ArrowRight', L = 'ArrowLeft', U = 'ArrowUp', D = 'ArrowDown';
  const three = '<button id=a>a</button><button id=b>b</button><button id=c>c</button>';

  // focusGroup ([PutForwards=value] DOMTokenList) and focusGroupStart (boolean) reflection.
  {
    const r = {};
    r.descriptors = ['HTMLElement', 'SVGElement', 'Element'].map(type => [type, describe(globalThis[type].prototype, 'focusGroup'), describe(globalThis[type].prototype, 'focusGroupStart')]);
    r.mathml = [typeof MathMLElement, typeof MathMLElement === 'function' ? [describe(MathMLElement.prototype, 'focusGroup'), describe(MathMLElement.prototype, 'focusGroupStart')] : null];
    const div = document.createElement('div'), list = div.focusGroup;
    r.list = [list instanceof DOMTokenList, list === div.focusGroup, list.length, list.value, String(list)];
    div.setAttribute('focusgroup', 'toolbar  wrap\tbogus'); r.parsed = [list.length, [...list], list.value, list.contains('wrap'), list.item(1)];
    div.focusGroup = 'Menu NOWRAP'; r.putForwards = [div.getAttribute('focusgroup'), [...list]];
    div.focusGroup = null; r.putNull = div.getAttribute('focusgroup');
    list.add('listbox'); list.remove('null'); r.mutate = div.getAttribute('focusgroup');
    r.supports = ['toolbar', 'tablist', 'radiogroup', 'listbox', 'menu', 'menubar', 'inline', 'block', 'wrap', 'nowrap', 'nomemory', 'none', 'grid', 'TOOLBAR', 'Wrap', 'flow', 'memory', 'manual', ''].map(token => list.supports(token));
    const getter = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'focusGroup').get;
    r.brands = [document.createElement('span'), document.createElementNS('http://www.w3.org/2000/svg', 'g'), document.createElementNS('urn:x', 'y'), document, {}].map(receiver => caught(() => getter.call(receiver).length));
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'g'); svg.setAttribute('focusgroup', 'menu');
    r.svg = [[...svg.focusGroup], svg.focusGroupStart, caught(() => Object.getOwnPropertyDescriptor(SVGElement.prototype, 'focusGroup').get.call(document.createElement('div')))];
    r.start = [];
    for (const value of [null, '', 'auto', 'first', 'FIRST', 'x']) { if (value === null) div.removeAttribute('focusgroupstart'); else div.setAttribute('focusgroupstart', value); r.start.push(div.focusGroupStart); }
    for (const value of [false, 'yes', 0, 1, null, {}]) { div.focusGroupStart = value; r.start.push(div.getAttribute('focusgroupstart')); }
    r.startBrand = caught(() => Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'focusGroupStart').set.call(svg, true));
    out.reflection = r;
  }

  // Behaviors, axes and wrapping.
  {
    const r = {}, sweep = [R, R, R, D, D, D, U, 'End', 'Home', 'PageDown'];
    for (const value of ['toolbar', 'tablist', 'radiogroup', 'listbox', 'menu', 'menubar', 'grid', '', 'bogus', 'inline', 'wrap', 'TOOLBAR', 'toolbar wrap', 'wrap toolbar', 'toolbar bogus', 'x toolbar', 'toolbar menu', 'none toolbar', 'toolbar none', 'toolbar block', 'toolbar inline block', 'menu inline', 'radiogroup inline', 'listbox nowrap', 'tablist nowrap', 'menu wrap', 'toolbar wrap nowrap'])
      r[value || '(empty)'] = await scenario(`<div focusgroup="${value}">${three}</div>`, 'a', sweep);
    r.homeEnd = await scenario('<div focusgroup=listbox>' + three + '</div>', 'b', ['Home', 'End', 'End', 'Home']);
    r.modifiers = await scenario('<div focusgroup=toolbar>' + three + '</div>', 'a', ['S+ArrowRight', 'C+ArrowRight', 'A+ArrowRight', 'M+ArrowRight', R, 'a']);
    out.behaviors = r;
  }

  // Items: sequential focusability, rendering, nesting, opt-out, shadow trees and slots.
  {
    const r = {};
    r.tabindex = await scenario('<div focusgroup=toolbar><button id=a>a</button><button id=b tabindex=-1>b</button><span id=s>s</span><a id=l href=#l>l</a><a id=nolink>n</a><div id=t tabindex=-1>t</div><div id=z tabindex=0>z</div><div id=p tabindex=2>p</div><button id=c>c</button></div>', 'a', [R, R, R, R, R, L, L, L, L]);
    r.disabled = await scenario('<div focusgroup=toolbar><button id=a>a</button><button id=b disabled>b</button><fieldset disabled><button id=f>f</button></fieldset><button id=c>c</button></div>', 'a', [R, L]);
    r.hidden = await scenario('<div focusgroup=toolbar><button id=a>a</button><button id=b hidden>b</button><button id=v style="visibility:hidden">v</button><div style=display:none><button id=d>d</button></div><button id=i inert>i</button><button id=c>c</button></div>', 'a', [R, L]);
    r.nested = await scenario('<div focusgroup=toolbar><button id=a>a</button><div><button id=b>b</button><span id=s tabindex=0>s</span></div><button id=c>c</button></div>', 'a', [R, R, R, R, L]);
    r.optOut = await scenario('<div focusgroup=toolbar><button id=a>a</button><div focusgroup=none><button id=b>b</button></div><button id=c>c</button></div>', 'a', [R, R, L]);
    r.insideOptOut = await scenario('<div focusgroup=toolbar><button id=a>a</button><div focusgroup=none><button id=b>b</button><button id=b2>b2</button></div><button id=c>c</button></div>', 'b', [R, L]);
    r.nestedGroup = await scenario('<div focusgroup=toolbar><button id=a>a</button><div focusgroup=menu><button id=m1>1</button><button id=m2>2</button></div><button id=c>c</button></div>', 'a', [R, R, D]);
    r.insideNestedGroup = await scenario('<div focusgroup=toolbar><button id=a>a</button><div focusgroup=menu><button id=m1>1</button><button id=m2>2</button></div><button id=c>c</button></div>', 'm1', [D, D, R, U]);
    r.ownerItem = await scenario('<div focusgroup=toolbar><button id=a focusgroup=toolbar>a</button><button id=b>b</button></div>', 'a', [R, L]);
    r.focusedOwner = await scenario('<div id=g tabindex=0 focusgroup=toolbar>' + three + '</div>', 'g', [R, D]);
    r.outside = await scenario('<button id=x>x</button><div focusgroup=toolbar>' + three + '</div>', 'x', [R]);
    r.svg = await scenario('<div focusgroup=toolbar><button id=a>a</button><svg><a id=s href=#x><rect width=10 height=10></rect></a></svg><button id=c>c</button></div>', 'a', [R, R, L, L]);
    r.shadow = await (async () => { body.innerHTML = '<div focusgroup=toolbar><button id=a>a</button><div id=h></div><button id=c>c</button></div>'; document.getElementById('h').attachShadow({ mode: 'open' }).innerHTML = '<button id=x>x</button><button id=y>y</button>'; document.getElementById('a').focus(); return press(R, R, R, L, L, L); })();
    r.slotted = await (async () => { body.innerHTML = '<div id=h focusgroup=toolbar><button id=a slot=s>a</button><button id=b>b</button></div>'; const root = document.getElementById('h').attachShadow({ mode: 'open' }); root.innerHTML = '<button id=x>x</button><slot></slot><slot name=s></slot>'; root.querySelector('#x').focus(); return press(R, R, R, L); })();
    r.shadowOwner = await (async () => { body.innerHTML = '<div id=h></div>'; const root = document.getElementById('h').attachShadow({ mode: 'open' }); root.innerHTML = '<div focusgroup=toolbar><button id=x>x</button><button id=y>y</button></div>'; root.querySelector('#x').focus(); return press(R, L); })();
    r.closedOwner = await (async () => { body.innerHTML = '<div id=h></div>'; const root = document.getElementById('h').attachShadow({ mode: 'closed' }); root.innerHTML = '<div focusgroup=menu><button id=x>x</button><button id=y>y</button></div>'; root.querySelector('#x').focus(); const seen = []; for (const key of [D, D, U]) { await press(key); seen.push(root.activeElement.id); } return seen; })();
    out.items = r;
  }

  // Controls that keep the keys, direction, and event order.
  {
    const r = {};
    r.consumers = await scenario('<div focusgroup=toolbar><button id=a>a</button><input type=checkbox id=k><input type=button id=i value=i><input type=radio id=o><button id=c>c</button></div>', 'a', [R, R, R, R]);
    r.text = await scenario('<div focusgroup=toolbar><button id=a>a</button><input id=t><button id=c>c</button></div>', 't', [R, L, 'Home', 'End']);
    r.textarea = await scenario('<div focusgroup=toolbar><button id=a>a</button><textarea id=t></textarea></div>', 't', [L, U]);
    r.select = await scenario('<div focusgroup=toolbar><button id=a>a</button><select id=s><option>1</option><option>2</option></select><button id=c>c</button></div>', 's', [R, L]);
    r.range = await scenario('<div focusgroup=toolbar><button id=a>a</button><input type=range id=r><button id=c>c</button></div>', 'r', [R, L]);
    r.editable = await scenario('<div focusgroup=toolbar><button id=a>a</button><div id=e contenteditable>e</div><button id=c>c</button></div>', 'e', [R, L]);
    r.rtl = await scenario('<div dir=rtl focusgroup=toolbar>' + three + '</div>', 'a', [R, L, L, R]);
    r.cssRtl = await scenario('<div style="direction:rtl" focusgroup=toolbar>' + three + '</div>', 'a', [R, L]);
    r.itemRtl = await scenario('<div focusgroup=toolbar><button id=a dir=rtl>a</button><button id=b>b</button></div>', 'a', [R, L, L]);
    r.innerRtl = await scenario('<div focusgroup=toolbar><div dir=rtl><button id=a>a</button><button id=b>b</button></div></div>', 'a', [R, L]);
    r.rtlBlock = await scenario('<div dir=rtl focusgroup=listbox>' + three + '</div>', 'a', [D, L, R, U]);
    r.dirAuto = await scenario('<div dir=auto focusgroup=toolbar><button id=a>\u05d0</button><button id=b>\u05d1</button></div>', 'a', [R, L]);
    r.dirAutoLatin = await scenario('<div dir=auto focusgroup=toolbar><button id=a>x</button><button id=b>\u05d1</button></div>', 'a', [R, L]);
    r.ltrInsideRtl = await scenario('<div dir=rtl focusgroup=toolbar><button id=a dir=ltr>a</button><button id=b>b</button></div>', 'a', [R, L]);
    r.vertical = await scenario('<div style="writing-mode:vertical-rl" focusgroup=toolbar>' + three + '</div>', 'a', [R, D, D, U]);
    r.prevented = await (async () => { body.innerHTML = '<div focusgroup=toolbar>' + three + '</div>'; document.getElementById('a').addEventListener('keydown', event => event.preventDefault()); document.getElementById('a').focus(); return press(R, R); })();
    r.preventedAncestor = await (async () => { body.innerHTML = '<div id=g focusgroup=toolbar>' + three + '</div>'; document.getElementById('g').addEventListener('keydown', event => event.preventDefault(), { once: true }); document.getElementById('a').focus(); return press(R, R); })();
    r.events = await (async () => {
      body.innerHTML = '<div focusgroup=toolbar>' + three + '</div>';
      const log = [], controller = new AbortController();
      for (const type of ['keydown', 'keyup', 'focus', 'blur', 'focusin', 'focusout']) document.addEventListener(type, event => log.push([type, event.target.id, event.isTrusted, event.defaultPrevented]), { capture: true, signal: controller.signal });
      document.addEventListener('keydown', event => log.push(['bubble', event.defaultPrevented, event.eventPhase]), { signal: controller.signal });
      document.getElementById('a').focus(); log.length = 0; await press(R); controller.abort();
      return log;
    })();
    out.keys = r;
  }
  return out;
})()
