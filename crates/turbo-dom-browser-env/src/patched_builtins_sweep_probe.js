(async () => {
  // miaskiewicz/turbo-dom#199: the captured-intrinsics rule, checked mechanically. The page
  // replaces every configurable member of every global constructor, its prototype and every
  // global namespace object (JS builtins and WebIDL interfaces alike) with a recording thrower,
  // then calls every member of every interface prototype on a pooled instance (getters, setters,
  // methods with a few argument vectors) and every constructor. Output: per member, the patched
  // builtins it reached. Chrome 156.0.8078.47 (headless, localhost) is the control; it reaches
  // only what the specs make observable (argument conversions), so any extra entry is the engine
  // calling a page-patchable builtin. The probe itself only uses originals captured up front, and
  // no syntax that reaches builtins (no for-of, spread, regexps, or object-to-string coercion).
  const A = Reflect.apply, gopd = Object.getOwnPropertyDescriptor, def = Object.defineProperty, own = Reflect.ownKeys;
  const getProto = Object.getPrototypeOf, Err = Error, construct = Reflect.construct, isProto = Object.prototype.isPrototypeOf;
  const symDesc = gopd(Symbol.prototype, 'description').get, g = globalThis, doc = document;
  const fn = (o, k) => { for (; o !== null; o = getProto(o)) { const d = gopd(o, k); if (d) return d.value; } };
  const call = (o, k, args) => A(fn(o, k), o, args);

  // Interfaces whose members would navigate, block, open windows or reset the document.
  const SKIP_IFACE = { __proto__: null, Window: 1, Location: 1, History: 1, Navigator: 1, WorkerNavigator: 1, Document_open: 1 };
  const SKIP_MEMBER = { __proto__: null, 'Document.open': 1, 'Document.write': 1, 'Document.writeln': 1, 'Document.close': 1,
    'Document.execCommand': 1, 'HTMLFormElement.submit': 1, 'HTMLFormElement.requestSubmit': 1, 'Element.requestFullscreen': 1,
    'Element.requestPointerLock': 1, 'HTMLDialogElement.showModal': 1, 'HTMLInputElement.showPicker': 1, 'HTMLSelectElement.showPicker': 1,
    'HTMLIFrameElement.src': 1, 'HTMLIFrameElement.srcdoc': 1, 'HTMLFrameElement.src': 1, 'HTMLObjectElement.data': 1, 'HTMLEmbedElement.src': 1,
    'HTMLAnchorElement.click': 1, 'HTMLAreaElement.click': 1, 'HTMLElement.click': 1, 'Element.setHTMLUnsafe': 1, 'HTMLMediaElement.play': 1,
    'HTMLMediaElement.load': 1, 'HTMLMediaElement.src': 1, 'HTMLSourceElement.src': 1, 'HTMLScriptElement.src': 1, 'HTMLScriptElement.text': 1,
    'HTMLScriptElement.textContent': 1, 'HTMLScriptElement.innerText': 1, 'HTMLLinkElement.href': 1, 'HTMLImageElement.src': 1, 'HTMLImageElement.srcset': 1,
    'HTMLBaseElement.href': 1, 'HTMLMetaElement.content': 1, 'HTMLMetaElement.httpEquiv': 1 };
  // ECMAScript builtins: their own members convert arguments observably by design.
  const ES = { __proto__: null };
  const esNames = ['Object', 'Function', 'Array', 'Number', 'Boolean', 'String', 'Symbol', 'Date', 'Promise', 'RegExp', 'Error', 'AggregateError', 'EvalError',
    'RangeError', 'ReferenceError', 'SyntaxError', 'TypeError', 'URIError', 'ArrayBuffer', 'SharedArrayBuffer', 'Uint8Array', 'Int8Array', 'Uint16Array',
    'Int16Array', 'Uint32Array', 'Int32Array', 'Float16Array', 'Float32Array', 'Float64Array', 'Uint8ClampedArray', 'BigUint64Array', 'BigInt64Array', 'DataView',
    'Map', 'BigInt', 'Set', 'WeakMap', 'WeakSet', 'Proxy', 'WeakRef', 'FinalizationRegistry', 'Iterator', 'DisposableStack', 'AsyncDisposableStack', 'SuppressedError'];
  for (let i = 0; i < esNames.length; i++) ES[esNames[i]] = 1;
  // Conversion hooks every engine reaches by spec when an argument is an object.
  const CONVERSION = { __proto__: null, 'Object.prototype.toString': 1, 'Object.prototype.valueOf': 1, 'String.prototype.toString': 1 };
  const SKIP_CTOR = { __proto__: null, Worker: 1, SharedWorker: 1, EventSource: 1, WebSocket: 1, WebTransport: 1, XMLHttpRequest: 1, Notification: 1,
    Audio: 1, AudioContext: 1, OfflineAudioContext: 1, RTCPeerConnection: 1, PaymentRequest: 1, BroadcastChannel: 1, Option: 1, Image: 1,
    Function: 1, AsyncFunction: 1, Proxy: 1, Window: 1, Document: 1, XMLDocument: 1, SpeechSynthesisUtterance: 1, MediaRecorder: 1, VideoEncoder: 1,
    VideoDecoder: 1, AudioEncoder: 1, AudioDecoder: 1, ImageDecoder: 1, IdleDetector: 1, PresentationRequest: 1, Sanitizer: 1 };

  // The instance pool: the document, one connected element per tag, the other node kinds,
  // the objects the DOM hands out, and whatever each global constructor builds from simple args.
  const host = doc.body.appendChild(doc.createElement('div'));
  const pool = [], add = make => { try { const made = make(); if (made !== null && made !== undefined) pool[pool.length] = made; } catch (e) {} };
  const seeds = [() => doc, () => doc.documentElement, () => doc.body, () => doc.head, () => doc.implementation, () => doc.createTextNode('t'),
    () => doc.createComment('c'), () => doc.createDocumentFragment(), () => doc.createAttribute('a'), () => doc.createRange(), () => doc.getSelection(),
    () => doc.createTreeWalker(doc), () => doc.createNodeIterator(doc), () => doc.childNodes, () => doc.children, () => host.attributes, () => host.classList,
    () => host.style, () => getComputedStyle(host), () => doc.styleSheets, () => doc.timeline, () => doc.fonts, () => host.dataset, () => doc.doctype,
    () => doc.createProcessingInstruction('p', 'q'), () => new DOMParser().parseFromString('<a/>', 'application/xml'), () => host.attachShadow({ mode: 'open' })];
  for (let i = 0; i < seeds.length; i++) add(seeds[i]);
  const tags = 'a abbr address area article aside audio b base bdi bdo blockquote body br button canvas caption cite code col colgroup data datalist dd del details dfn dialog div dl dt em embed fieldset figcaption figure footer form h1 head header hgroup hr html i iframe img input ins kbd label legend li link main map mark menu meta meter nav noscript object ol optgroup option output p picture pre progress q rp rt ruby s samp search section select slot small source span strong style sub summary sup table tbody td template textarea tfoot th thead time title tr track u ul var video wbr frame frameset marquee font dir applet';
  let tag = '';
  for (let i = 0; i <= tags.length; i++) {
    const c = i < tags.length ? tags[i] : ' ';
    if (c !== ' ') { tag += c; continue; }
    const el = doc.createElement(tag); tag = '';
    if (el.localName !== 'html' && el.localName !== 'body' && el.localName !== 'head' && el.localName !== 'base' && el.localName !== 'meta') host.appendChild(el);
    pool[pool.length] = el;
  }
  const svgTags = ['svg', 'g', 'rect', 'circle', 'path', 'text', 'use', 'image', 'foreignObject', 'linearGradient', 'filter', 'animate', 'polygon', 'line', 'tspan', 'marker', 'symbol', 'clipPath', 'mask', 'pattern'];
  const svg = host.appendChild(doc.createElementNS('http://www.w3.org/2000/svg', 'svg'));
  for (let i = 0; i < svgTags.length; i++) { const el = doc.createElementNS('http://www.w3.org/2000/svg', svgTags[i]); svg.appendChild(el); pool[pool.length] = el; }
  const math = host.appendChild(doc.createElementNS('http://www.w3.org/1998/Math/MathML', 'math'));
  pool[pool.length] = math;
  const input = host.appendChild(doc.createElement('input'));
  add(() => input.validity); add(() => input.labels); add(() => doc.createElement('select').options);
  add(() => host.animate([{ opacity: 0 }, { opacity: 1 }], 1000));
  add(() => doc.evaluate('.', doc, null, 0, null));
  add(() => host.attributeStyleMap); add(() => host.computedStyleMap());
  add(() => doc.createElement('canvas').getContext('2d'));
  try {
  const sheet = new CSSStyleSheet(); sheet.replaceSync('@media print { a { color: red } } b { color: blue } @font-face { font-family: f } @keyframes k { from { opacity: 0 } }');
  pool[pool.length] = sheet; pool[pool.length] = sheet.cssRules;
  for (let i = 0; i < sheet.cssRules.length; i++) pool[pool.length] = sheet.cssRules[i];
  add(() => sheet.cssRules[0].media); add(() => sheet.cssRules[1].style);
  } catch (e) {}
  const names = own(g), ctors = [];
  for (let i = 0; i < names.length; i++) {
    const name = names[i];
    if (typeof name !== 'string' || SKIP_CTOR[name] || ES[name] || (name[0] === '_' && name[1] === '_')) continue;
    const d = gopd(g, name), C = d && d.value;
    if (typeof C !== 'function' || !C.prototype) continue;
    ctors[ctors.length] = [name, C];
    const argSets = [[], ['x'], ['x', 1], [1, 1]];
    for (let a = 0; a < argSets.length; a++) {
      try { const made = construct(C, argSets[a]); if (made !== null && typeof made === 'object') { pool[pool.length] = made; break; } } catch (e) {}
    }
  }
  try { pool[pool.length] = new MutationObserver(function () {}); } catch (e) {}

  // Every member to exercise: interface name, prototype, key, descriptor, an instance.
  const members = [];
  for (let i = 0; i < ctors.length; i++) {
    const name = ctors[i][0], P = ctors[i][1].prototype;
    if (SKIP_IFACE[name] || ES[name] || P === Object.prototype || P === Function.prototype) continue;
    let instance;
    for (let j = 0; j < pool.length; j++) { const o = pool[j]; if (o !== null && o !== undefined && getProto(o) === P) { instance = o; break; } }
    if (instance === undefined) for (let j = 0; j < pool.length; j++) { const o = pool[j]; if (o !== null && o !== undefined && (typeof o === 'object') && A(isProto, P, [o])) { instance = o; break; } }
    if (instance === undefined) continue;
    const keys = own(P);
    for (let k = 0; k < keys.length; k++) {
      const key = keys[k];
      if (typeof key !== 'string' || key === 'constructor') continue;
      if (SKIP_MEMBER[name + '.' + key]) continue;
      const desc = gopd(P, key);
      members[members.length] = [name + '.' + key, instance, desc];
    }
  }

  // Patch every configurable member with a recording thrower.
  const hits = { __proto__: null }; let current = '(setup)';
  const record = n => { let set = hits[current]; if (set === undefined) set = hits[current] = { __proto__: null }; set[n] = 1; };
  const thrower = n => function () { record(n); throw new Err('patched ' + n); };
  const saved = []; let count = 0;
  const skip = { __proto__: null, constructor: 1, prototype: 1, length: 1, name: 1, caller: 1, arguments: 1 };
  skip['__proto__'] = 1;
  const patch = (o, label) => {
    const ks = own(o);
    for (let i = 0; i < ks.length; i++) {
      const k = ks[i];
      if (typeof k === 'string' && skip[k]) continue;
      const d = gopd(o, k); if (!d.configurable) continue;
      const n = label + '.' + (typeof k === 'symbol' ? '@@' + A(symDesc, k, []) : k);
      if (CONVERSION[n]) continue;
      if (typeof d.value === 'function') { saved[saved.length] = [o, k, d]; count++; def(o, k, { value: thrower(n), writable: true, configurable: true, enumerable: d.enumerable }); }
      else if (d.get || d.set) { saved[saved.length] = [o, k, d]; count++; def(o, k, { get: thrower('get ' + n), set: thrower('set ' + n), configurable: true, enumerable: d.enumerable }); }
    }
  };
  const targets = [];
  for (let i = 0; i < names.length; i++) {
    const name = names[i];
    if (typeof name !== 'string' || name === 'globalThis' || name === 'window' || name === 'self' || name === 'document' || name === 'frames' || name === 'top' || name === 'parent') continue;
    const d = gopd(g, name), v = d && d.value;
    if (typeof v === 'function') { targets[targets.length] = [v, name]; const p = gopd(v, 'prototype'); if (p && p.value !== null && typeof p.value === 'object') targets[targets.length] = [p.value, name + '.prototype']; }
    else if (v !== null && typeof v === 'object' && name[0] >= 'A' && name[0] <= 'Z') targets[targets.length] = [v, name];
  }
  // Prototypes the globals do not name: iterators, generators, async functions, typed arrays.
  const iterProto = getProto(getProto([][Symbol.iterator]()));
  targets[targets.length] = [iterProto, '%IteratorPrototype%'];
  targets[targets.length] = [getProto([][Symbol.iterator]()), '%ArrayIteratorPrototype%'];
  targets[targets.length] = [getProto(new Map()[Symbol.iterator]()), '%MapIteratorPrototype%'];
  targets[targets.length] = [getProto(new Set()[Symbol.iterator]()), '%SetIteratorPrototype%'];
  targets[targets.length] = [getProto(''[Symbol.iterator]()), '%StringIteratorPrototype%'];
  targets[targets.length] = [getProto(Int8Array), '%TypedArray%'];
  targets[targets.length] = [getProto(Int8Array.prototype), '%TypedArray%.prototype'];
  // Let the caller attach to this promise before anything is patched.
  await 0;
  const out = { __proto__: null };
  try {
    for (let i = 0; i < targets.length; i++) patch(targets[i][0], targets[i][1]);
    current = '(patched)';
    const argSets = [[], ['x'], ['x', 'y'], [host, input], [1, 'x']];
    for (let i = 0; i < members.length; i++) {
      const name = members[i][0], instance = members[i][1], desc = members[i][2];
      current = name;
      if (desc.get) {
        let value; try { value = A(desc.get, instance, []); } catch (e) {}
        if (desc.set) { try { A(desc.set, instance, [value]); } catch (e) {} try { A(desc.set, instance, ['x']); } catch (e) {} }
      } else if (typeof desc.value === 'function') {
        for (let a = 0; a < argSets.length; a++) { try { A(desc.value, instance, argSets[a]); } catch (e) {} }
      }
      await 0; await 0;
    }
    for (let i = 0; i < ctors.length; i++) {
      current = 'new ' + ctors[i][0];
      for (let a = 0; a < 3; a++) { try { construct(ctors[i][1], a === 0 ? [] : a === 1 ? ['x'] : ['x', 1]); } catch (e) {} }
      await 0; await 0;
    }
    current = '(after)';
    await 0; await 0;
  } finally {
    for (let i = saved.length - 1; i >= 0; i--) def(saved[i][0], saved[i][1], saved[i][2]);
  }
  host.remove();
  out.hits = hits;
  out.patched = count > 2000 && members.length > 800;
  return out;
})()
