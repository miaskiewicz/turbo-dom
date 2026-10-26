(() => {
  'use strict';
  // Interface shapes (sorted own keys + descriptors) and behaviour of the form-control
  // prototypes, HTMLOptionsCollection, the CustomEvent/PopStateEvent/HashChangeEvent family and
  // HTMLHyperlinkElementUtils. Key ORDER is deliberately not compared.
  const run = (f) => { try { return ['ok', f()]; } catch (error) { return ['throw', error.name]; } };
  const fn = (f) => (typeof f === 'function' ? { name: f.name, length: f.length, ctor: (() => { try { Reflect.construct(String, [], f); return true; } catch { return false; } })(), proto: Object.hasOwn(f, 'prototype') } : typeof f);
  const desc = (o, k) => {
    const d = Object.getOwnPropertyDescriptor(o, k);
    if (!d) return null;
    const r = { c: d.configurable, e: d.enumerable };
    if ('value' in d) { r.w = d.writable; r.v = typeof d.value === 'function' ? fn(d.value) : typeof d.value === 'object' ? typeof d.value : d.value; }
    else { r.get = fn(d.get); r.set = fn(d.set); }
    return r;
  };
  const shape = (o) => Object.fromEntries(Reflect.ownKeys(o).map(String).sort().map((k) => [k, desc(o, Reflect.ownKeys(o).find((x) => String(x) === k))]));
  const iface = (name) => {
    const C = globalThis[name];
    if (typeof C !== 'function') return null;
    return { ctor: shape(C), proto: shape(C.prototype), parent: Object.getPrototypeOf(C).name, protoParent: Object.prototype.toString.call(Object.getPrototypeOf(C.prototype)), global: desc(globalThis, name), construct: run(() => new C('x')) };
  };
  const out = { shapes: {} };
  for (const name of ['HTMLLabelElement', 'HTMLOptionElement', 'HTMLSelectElement', 'HTMLTextAreaElement', 'HTMLOptionsCollection', 'CustomEvent', 'PopStateEvent', 'HashChangeEvent', 'ProgressEvent', 'CloseEvent'])
    out.shapes[name] = iface(name);
  const urlNames = ['href', 'origin', 'protocol', 'username', 'password', 'host', 'hostname', 'port', 'pathname', 'search', 'hash', 'toString'];
  for (const name of ['HTMLAnchorElement', 'HTMLAreaElement']) {
    const P = globalThis[name].prototype;
    out.shapes[name + '.url'] = Object.fromEntries(urlNames.map((k) => [k, desc(P, k)]));
  }

  const el = (tag, attrs, ...kids) => { const e = document.createElement(tag); for (const [k, v] of Object.entries(attrs || {})) e.setAttribute(k, v); e.append(...kids); return e; };
  const opt = (text, attrs) => el('option', attrs, text);
  const texts = (c) => Array.from({ length: c.length }, (_, i) => (c[i] ? c[i].textContent : String(c[i])));
  // HTMLOptionsCollection behaviour.
  const options = {};
  {
    const s = el('select', { name: 'pick' }, opt('a', { id: 'ia' }), opt('b', { name: 'nb' }), el('optgroup', {}, opt('c')));
    document.body.append(s);
    const c = s.options;
    options.identity = [c === s.options, c instanceof HTMLOptionsCollection, c instanceof HTMLCollection, Object.prototype.toString.call(c)];
    options.initial = [c.length, texts(c), c.selectedIndex, s.value];
    options.item = [c.item(1) === c[1], c.item(9), run(() => c.item())[0], c.namedItem('ia') === c[0], c.namedItem('nb') === c[1], c.namedItem('zz'), c.ia === c[0], c.nb === c[1]];
    options.keys = [Object.keys(c), Reflect.ownKeys(c).map(String), Object.getOwnPropertyNames(c)];
    options.indexDesc = [desc(c, 0), desc(c, 'ia'), desc(c, 5), Object.hasOwn(c, 2), 2 in c, 3 in c];
    options.iterate = [Array.from(c, (o) => o.textContent), [...c.entries ? ['entries'] : []], c[Symbol.iterator] === Array.prototype.values, typeof c.forEach];
    c.length = 5;
    options.grow = [c.length, texts(c), s.children.length, c[4].parentNode === s, c.selectedIndex];
    c.length = 2;
    options.shrink = [c.length, texts(c), s.children.length];
    c[3] = opt('d');
    options.setPad = [c.length, texts(c)];
    c[0] = opt('z');
    options.setReplace = [c.length, texts(c)];
    c[1] = null;
    options.setNull = [c.length, texts(c)];
    options.setBad = run(() => { c[0] = document.createElement('div'); });
    options.defineAccessor = run(() => Object.defineProperty(c, 0, { get() { return 1; } }));
    options.defineValue = [run(() => Object.defineProperty(c, 6, { value: opt('q') })), c.length, texts(c)];
    options.deleteIndex = [run(() => delete c[0]), (() => { 'use strict'; return run(() => delete c[0]); })(), c.length];
    options.namedSet = [run(() => { c.foo = 1; return c.foo; }), Object.hasOwn(c, 'foo')];
    c.add(opt('e'));
    c.add(opt('f'), 0);
    c.add(opt('g'), c[2]);
    options.add = [texts(c), run(() => c.add(document.createElement('div')))[1], run(() => c.add())[1], run(() => c.add(opt('h'), document.createElement('span')))[1]];
    c.selectedIndex = 2;
    options.selectedIndex = [c.selectedIndex, s.selectedIndex, s.value, c[2].selected];
    c.selectedIndex = -1;
    options.unselect = [c.selectedIndex, s.selectedIndex, s.value];
    c.remove(0);
    c.remove(99);
    c.remove(-1);
    options.remove = [texts(c), run(() => c.remove())[1], c.length];
    const sel = s.selectedOptions;
    s.multiple = true;
    c[0].selected = true;
    c[1].selected = true;
    options.selectedOptions = [sel === s.selectedOptions, sel instanceof HTMLCollection, Object.prototype.toString.call(sel), sel.length, sel[0] === c[0], sel.item(1) === c[1]];
    const P = HTMLOptionsCollection.prototype;
    options.brand = ['length', 'selectedIndex'].map((k) => run(() => Object.getOwnPropertyDescriptor(P, k).get.call(document.body.children))[0])
      .concat([run(() => P.add.call(s, opt('x')))[0], run(() => P.remove.call({}, 0))[0], run(() => HTMLCollection.prototype.item.call(c, 0) === c[0])[1], run(() => Object.getOwnPropertyDescriptor(HTMLCollection.prototype, 'length').get.call(c))[1]]);
    s.remove();
  }
  out.options = options;
  // <option> members.
  {
    const o = el('option', {}, '  hello \n  ', el('b', {}, ' big\tword '), el('script', {}, 'skip'), ' end ');
    const s = el('select', {}, opt('first'), el('optgroup', {}, o));
    const f = el('form', { id: 'f1' }, s);
    document.body.append(f);
    const lone = opt('lone');
    out.option = {
      text: o.text, value: o.value, label: o.label, index: o.index, form: o.form === f, loneIndex: lone.index, loneForm: lone.form,
      labelAttr: (() => { o.setAttribute('label', ''); return [o.label, o.text]; })(),
      setText: (() => { o.text = ' set  me '; return [o.text, o.childNodes.length]; })(),
      setLabel: (() => { o.label = 'L'; return o.getAttribute('label'); })(),
      setValue: (() => { o.value = 9; return [o.value, o.getAttribute('value')]; })(),
      datalist: (() => { const x = opt('dl'); el('datalist', {}, x); return [x.index, x.form]; })(),
      extras: ['name', 'placeholder', 'accept', 'min', 'max', 'step', 'pattern', 'autocomplete', 'multiple', 'required', 'readOnly'].filter((k) => k in o),
    };
    f.remove();
  }
  // <label> members.
  {
    const input = el('input', { id: 'lab-in' });
    const f = el('form', {}, input);
    const l1 = el('label', { for: 'lab-in' });
    const l2 = el('label', {}, el('output'));
    const l3 = el('label', {}, el('meter'));
    const l4 = el('label');
    document.body.append(f, l1, l2, l3, l4);
    const f2 = el('form', { id: 'f2' });
    document.body.append(f2);
    l2.firstChild.setAttribute('form', 'f2');
    out.label = {
      form: [l1.form === f, l2.form === f2, l3.form, l4.form, l1.control === input],
      htmlFor: (() => { l4.htmlFor = 'x'; return [l4.htmlFor, l4.getAttribute('for')]; })(),
      extras: ['value', 'name', 'placeholder', 'accept', 'min', 'max', 'step', 'pattern', 'autocomplete', 'disabled', 'multiple', 'required', 'readOnly'].filter((k) => k in l1),
      brand: run(() => Object.getOwnPropertyDescriptor(HTMLLabelElement.prototype, 'form').get.call(input))[0],
    };
    for (const node of [f, l1, l2, l3, l4, f2]) node.remove();
  }
  {
    const t = el('textarea');
    const s = el('select');
    out.controlExtras = {
      textarea: ['accept', 'max', 'min', 'multiple', 'pattern', 'step'].filter((k) => k in t),
      select: ['accept', 'max', 'min', 'pattern', 'placeholder', 'readOnly', 'step'].filter((k) => k in s),
    };
  }
  // Event interfaces with dictionary members.
  {
    const c = new CustomEvent('x', { detail: { a: 1 }, bubbles: true });
    const c2 = new CustomEvent('y');
    const init = run(() => { c2.initCustomEvent('z', true, true, 5); return [c2.type, c2.bubbles, c2.cancelable, c2.detail]; });
    const p = new PopStateEvent('popstate', { state: { s: 1 } });
    const h = new HashChangeEvent('hashchange', { oldURL: 'a', newURL: 7 });
    const proto = (e) => [Object.getOwnPropertyNames(e), e instanceof Event];
    out.events = {
      custom: [c.detail.a, c.bubbles, c2.detail, init, proto(c), run(() => new CustomEvent())[1], run(() => CustomEvent('x'))[1], run(() => Object.getOwnPropertyDescriptor(CustomEvent.prototype, 'detail').get.call(new Event('q')))[1], run(() => c.initCustomEvent())[1]],
      pop: [p.state.s, new PopStateEvent('p').state, p.hasUAVisualTransition, proto(p), run(() => new PopStateEvent())[1], run(() => Object.getOwnPropertyDescriptor(PopStateEvent.prototype, 'state').get.call(c))[1]],
      progress: [new ProgressEvent('p', { lengthComputable: 1, loaded: 5.7, total: -1 }), new ProgressEvent('p', { loaded: 2 ** 53, total: '3' }), new ProgressEvent('p')].map((e) => [e.lengthComputable, e.loaded, e.total]).concat([run(() => new ProgressEvent('p', { total: NaN }))[1], run(() => new ProgressEvent('p', { loaded: Infinity }))[1], run(() => new ProgressEvent())[1], run(() => Object.getOwnPropertyDescriptor(ProgressEvent.prototype, 'loaded').get.call(p))[1]]),
      close: [new CloseEvent('close', { wasClean: 'yes', code: 70000, reason: 4 }), new CloseEvent('close')].map((e) => [e.wasClean, e.code, e.reason]).concat([run(() => new CloseEvent())[1], run(() => Object.getOwnPropertyDescriptor(CloseEvent.prototype, 'code').get.call(p))[1]]),
      hash: [h.oldURL, h.newURL, new HashChangeEvent('h').oldURL, proto(h), run(() => new HashChangeEvent())[1], run(() => Object.getOwnPropertyDescriptor(HashChangeEvent.prototype, 'newURL').get.call(p))[1]],
    };
  }
  // HTMLHyperlinkElementUtils.
  {
    const a = el('a', { href: 'https://user:pw@example.com:8080/p/q?x=1#frag' });
    const r = {};
    r.read = urlNames.slice(0, -1).map((k) => a[k]);
    r.string = [String(a), a.toString()];
    for (const [k, v] of [['protocol', 'http'], ['host', 'other.test:81'], ['hostname', 'h.test'], ['port', '99'], ['pathname', 'a b'], ['search', 'y=2'], ['hash', 'h2'], ['username', 'u'], ['password', 'p']]) { a[k] = v; r[k] = a.href; }
    r.originSet = run(() => { 'use strict'; a.origin = 'x'; });
    const none = el('a');
    r.noHref = urlNames.slice(0, -1).map((k) => none[k]);
    none.hash = 'z';
    r.noHrefSet = none.hasAttribute('href');
    const bad = el('a', { href: 'http://[bad' });
    r.bad = urlNames.slice(0, -1).map((k) => bad[k]);
    bad.hash = 'q';
    r.badSet = bad.getAttribute('href');
    const rel = el('a', { href: '/rel?a#b' });
    r.relative = [rel.origin === location.origin, rel.pathname, rel.search, rel.hash];
    const area = el('area', { href: 'https://example.org/z' });
    r.area = [area.host, area.pathname, String(area)];
    r.brand = run(() => Object.getOwnPropertyDescriptor(HTMLAnchorElement.prototype, 'host').get.call(area));
    out.hyperlink = r;
  }
  return out;
})()
