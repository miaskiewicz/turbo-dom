(async () => {
  const run = f => { try { const v = f(); return v === undefined ? 'undefined' : v; } catch (e) { return [e.name, e.message]; } };
  const settle = async f => { let p; try { p = f(); } catch (e) { return ['throw', e.name, e.message]; } try { const v = await p; return ['ok', v === undefined ? 'undefined' : v]; } catch (e) { return ['reject', e.name, e.message]; } };
  const wait = ms => new Promise(r => setTimeout(r, ms));
  const tag = n => n == null ? null : n === document ? 'document' : n === window ? 'window' : n.id || n.nodeName;
  const describe = (o, k) => { const x = Object.getOwnPropertyDescriptor(o, k); if (!x) return null; return { kind: 'value' in x ? typeof x.value : 'accessor', get: x.get ? [x.get.name, x.get.length] : null, set: x.set ? [x.set.name, x.set.length] : null, name: x.value && x.value.name, length: x.value && x.value.length, e: x.enumerable, c: x.configurable, w: x.writable === undefined ? null : x.writable }; };
  const out = {};
  const L = [];

  // Activation behavior runs only for MouseEvent clicks (isTrusted is irrelevant); the
  // activation target is the target or, for bubbling events, its nearest such ancestor.
  const fixture = '<form id=form><input id=cb type=checkbox><input id=radio type=radio name=r><input id=text name=t value=orig><button id=submit>s</button><button id=reset type=reset>r</button><input id=isubmit type=submit><input id=ireset type=reset><input id=ibutton type=button><button id=dsubmit disabled>d</button></form>' +
    '<label id=label for=lcb>label<span id=lspan>s</span></label><input id=lcb type=checkbox>' +
    '<label id=wrap>wrap<input id=wcb type=checkbox><span id=wspan>w</span><a id=wa href=#wa>link</a><button id=wbutton type=button>b</button></label>' +
    '<label id=dlabel for=dcb>disabled</label><input id=dcb type=checkbox disabled>' +
    '<a id=anchor href=#frag>a<span id=aspan>s</span></a><a id=nohref>n<span id=nspan>s</span></a>' +
    '<details id=details><summary id=summary>sum<span id=sspan>s</span><button id=sbutton type=button>b</button></summary>body</details>' +
    '<details id=second><summary>first</summary><summary id=other>second</summary></details><summary id=loose>loose</summary>';
  const kinds = {
    event: () => new Event('click', { bubbles: true, cancelable: true }),
    custom: () => new CustomEvent('click', { bubbles: true, cancelable: true }),
    keyboard: () => new KeyboardEvent('click', { bubbles: true, cancelable: true }),
    mouse: () => new MouseEvent('click', { bubbles: true, cancelable: true }),
    mouseNoBubble: () => new MouseEvent('click', { cancelable: true }),
    pointer: () => new PointerEvent('click', { bubbles: true, cancelable: true }),
    method: null,
    mouseCanceled: () => new MouseEvent('click', { bubbles: true, cancelable: true }),
  };
  const targets = ['cb', 'dcb', 'radio', 'submit', 'dsubmit', 'reset', 'isubmit', 'ireset', 'ibutton', 'label', 'lspan', 'wrap', 'wspan', 'wa', 'wbutton', 'dlabel', 'anchor', 'aspan', 'nspan', 'summary', 'sspan', 'sbutton', 'other', 'loose'];
  // A bare binding realm has no Location/History; fragment navigation is then unobservable.
  const hash = () => typeof location === 'object' ? location.hash : '';
  const resetURL = () => { if (typeof history === 'object') history.replaceState(null, '', location.href.split('#')[0]); };
  out.synthetic = {};
  for (const [kind, make] of Object.entries(kinds)) {
    for (const id of targets) {
      resetURL();
      const root = document.body.appendChild(document.createElement('div'));
      root.innerHTML = fixture;
      const $ = name => root.querySelector('#' + name);
      L.length = 0;
      const form = $('form');
      form.addEventListener('submit', e => { e.preventDefault(); L.push('submit:' + tag(e.submitter)); });
      form.addEventListener('reset', () => L.push('reset'));
      for (const type of ['click', 'input', 'change', 'toggle']) root.addEventListener(type, e => { if (e.target.id !== id || type !== 'click') L.push([type, tag(e.target), e.isTrusted, e.constructor.name]); }, true);
      if (kind === 'mouseCanceled') root.addEventListener('click', e => { if (e.target.id === id) e.preventDefault(); });
      $('text').value = 'changed';
      const target = $(id);
      const returned = make ? target.dispatchEvent(make()) : target.click();
      const sync = { cb: $('cb').checked, radio: $('radio').checked, text: $('text').value, lcb: $('lcb').checked, wcb: $('wcb').checked, dcb: $('dcb').checked, hash: hash(), details: $('details').open, second: $('second').open, active: tag(document.activeElement) };
      await wait(10);
      out.synthetic[kind + ':' + id] = { returned, sync, hashLater: hash(), log: L.slice() };
      root.remove();
    }
  }
  resetURL();

  out.shape = Object.fromEntries([
    ['inputPicker', HTMLInputElement.prototype, 'showPicker'], ['selectPicker', HTMLSelectElement.prototype, 'showPicker'],
    ['requestFullscreen', Element.prototype, 'requestFullscreen'], ['webkitRequestFullscreen', Element.prototype, 'webkitRequestFullscreen'], ['webkitRequestFullScreen', Element.prototype, 'webkitRequestFullScreen'],
    ['onfullscreenchange', Element.prototype, 'onfullscreenchange'], ['onfullscreenerror', Element.prototype, 'onfullscreenerror'],
    ['onwebkitfullscreenchange', Element.prototype, 'onwebkitfullscreenchange'], ['onwebkitfullscreenerror', Element.prototype, 'onwebkitfullscreenerror'],
    ['documentOnfullscreenchange', Document.prototype, 'onfullscreenchange'], ['documentOnfullscreenerror', Document.prototype, 'onfullscreenerror'],
    ['documentOnwebkitfullscreenchange', Document.prototype, 'onwebkitfullscreenchange'],
    ['exitFullscreen', Document.prototype, 'exitFullscreen'], ['fullscreenElement', Document.prototype, 'fullscreenElement'],
  ].map(([name, owner, key]) => [name, describe(owner, key)]));

  // No transient activation exists in this realm, so every gate below reports Chrome's refusal.
  const box = document.body.appendChild(document.createElement('div'));
  box.id = 'box';
  const pickers = document.body.appendChild(document.createElement('div'));
  pickers.innerHTML = '<input id=text><input id=date type=date><input id=color type=color><input id=file type=file><input id=checkbox type=checkbox><input id=hidden type=hidden><input id=range type=range><input id=number type=number readonly><input id=dtext disabled><input id=rtext readonly><input id=rcheckbox type=checkbox readonly><input id=rdate type=date readonly><input id=rcolor type=color readonly><input id=rfile type=file readonly><input id=dfile type=file disabled><input id=dhidden type=hidden disabled><fieldset disabled><legend><input id=ltext></legend><input id=ftext><select id=fselect><option>a</select></fieldset><select id=select><option>a</select><select id=dselect disabled><option>a</select><select id=multi multiple><option>a</select><input id=nonetext style=display:none><select id=noneselect style=display:none><option>a</select>';
  out.picker = {};
  for (const element of pickers.querySelectorAll('input,select')) out.picker[element.id] = run(() => element.showPicker());
  out.picker.detachedInput = run(() => document.createElement('input').showPicker());
  out.picker.detachedSelect = run(() => document.createElement('select').showPicker());
  out.picker.inputBrand = run(() => HTMLInputElement.prototype.showPicker.call(document.createElement('select')));
  out.picker.selectBrand = run(() => HTMLSelectElement.prototype.showPicker.call({}));
  out.picker.extraArgument = run(() => pickers.querySelector('#dtext').showPicker(1, 2));
  pickers.remove();

  const events = [];
  for (const type of ['fullscreenchange', 'fullscreenerror', 'webkitfullscreenchange', 'webkitfullscreenerror'])
    document.addEventListener(type, e => events.push([type, tag(e.target), e.isTrusted, e.cancelable, e.bubbles, e.composed]), true);
  box.onfullscreenerror = e => events.push(['handler', tag(e.target)]);
  const order = [];
  const pending = box.requestFullscreen();
  order.push(pending instanceof Promise);
  const plain = settle(() => pending).then(value => { order.push('settled'); return value; });
  order.push('sync');
  out.fullscreen = {
    plain: await plain,
    brand: await settle(() => Element.prototype.requestFullscreen.call({})),
    text: await settle(() => Element.prototype.requestFullscreen.call(document.createTextNode('x'))),
    navigationUI: await settle(() => box.requestFullscreen({ navigationUI: 'bogus' })),
    primitive: await settle(() => box.requestFullscreen(5)),
    screen: await settle(() => box.requestFullscreen({ screen: 1 })),
    hide: await settle(() => box.requestFullscreen({ navigationUI: 'hide', screen: undefined })),
    nullOptions: await settle(() => box.requestFullscreen(null)),
    detached: await settle(() => document.createElement('div').requestFullscreen()),
    detachedBadUI: await settle(() => document.createElement('div').requestFullscreen({ navigationUI: 'x' })),
    dialog: await settle(() => document.body.appendChild(document.createElement('dialog')).requestFullscreen()),
    rect: await settle(() => document.body.appendChild(document.createElementNS('http://www.w3.org/2000/svg', 'rect')).requestFullscreen()),
    svg: await settle(() => document.body.appendChild(document.createElementNS('http://www.w3.org/2000/svg', 'svg')).requestFullscreen()),
    math: await settle(() => document.body.appendChild(document.createElementNS('http://www.w3.org/1998/Math/MathML', 'math')).requestFullscreen()),
    inert: await settle(() => document.implementation.createHTMLDocument('').body.requestFullscreen()),
    webkit: run(() => box.webkitRequestFullscreen()),
    webkitScreen: run(() => box.webkitRequestFullScreen()),
    exit: await settle(() => document.exitFullscreen()),
    webkitExit: run(() => document.webkitExitFullscreen()),
    element: tag(document.fullscreenElement),
  };
  await wait(50);
  out.fullscreen.order = order;
  out.fullscreen.events = events;
  return out;
})()
