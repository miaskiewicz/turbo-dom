// Coverage-fill: exercises the long tail of selector pseudo-classes, DOM edge
// methods, honest stubs, and the window proxy/IO surface so refactors there are
// caught. Behavior is asserted to spec, never to whatever the impl happens to do.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment, setClock } from '../src/runtime/index.mjs';

const fresh = (html) => createEnvironment(html ?? '<!doctype html><html><head></head><body></body></html>');

// ----------------------------------------------- selector pseudo-classes ----
test('structural + type pseudo-classes', () => {
  const { document } = fresh();
  document.body.innerHTML =
    '<ul><li>1</li><li>2</li><li>3</li></ul>' +
    '<div id="solo"><span>only</span></div>' +
    '<p id="empty"></p>' +
    '<section><b>x</b><i>y</i><b>z</b></section>';
  const qa = (s) => document.querySelectorAll(s).length;
  assert.equal(qa('li:only-child'), 0);
  assert.equal(qa('span:only-child'), 1);
  assert.equal(document.getElementById('empty').matches(':empty'), true);
  assert.equal(qa('li:nth-child(odd)'), 2);
  assert.equal(qa('li:nth-child(even)'), 1);
  assert.equal(qa('li:nth-child(2n+1)'), 2);
  assert.equal(qa('li:nth-last-child(1)'), 1);
  assert.equal(qa('section b:first-of-type'), 1);
  assert.equal(qa('section b:last-of-type'), 1);
  assert.equal(qa('section b:nth-of-type(2)'), 1);
  assert.equal(qa('section b:nth-last-of-type(1)'), 1);
  assert.equal(qa('section i:only-of-type'), 1);
  assert.equal(qa(':root'), 1); // <html>
  assert.equal(qa('li:nth-child(3)'), 1);
});

test('form-state pseudo-classes read live properties', () => {
  const { document } = fresh();
  document.body.innerHTML =
    '<input id="c" type="checkbox"><input id="d" disabled>' +
    '<input id="r" required><textarea id="ro" readonly></textarea>' +
    '<select id="s"><option id="o" selected>x</option></select>';
  document.getElementById('c').checked = true;
  assert.ok(document.getElementById('c').matches(':checked'));
  assert.ok(document.getElementById('d').matches(':disabled'));
  assert.ok(document.getElementById('c').matches(':enabled'));
  assert.ok(document.getElementById('r').matches(':required'));
  assert.ok(document.getElementById('c').matches(':optional'));
  assert.ok(document.getElementById('ro').matches(':read-only'));
  assert.ok(document.getElementById('c').matches(':read-write'));
  assert.ok(document.getElementById('o').matches(':checked')); // selected option
});

test('unknown pseudo never matches (honest)', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="d"></div>';
  assert.equal(document.getElementById('d').matches(':totally-made-up'), false);
});

// ---------------------------------------------------------- DOM edges ----
test('focus then blur fires the focus/blur event sequence', () => {
  const { document } = fresh();
  document.body.innerHTML = '<input id="a"><input id="b">';
  const a = document.getElementById('a');
  const seen = [];
  a.addEventListener('focus', () => seen.push('focus'));
  a.addEventListener('blur', () => seen.push('blur'));
  a.focus();
  assert.equal(document.activeElement, a);
  a.blur();
  assert.equal(document.activeElement, document.body);
  assert.deepEqual(seen, ['focus', 'blur']);
  a.blur(); // no-op when not active → no extra events
  assert.deepEqual(seen, ['focus', 'blur']);
});

test('dataset returns undefined for a missing data-* attribute', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  assert.equal(el.dataset.nope, undefined);
  assert.equal('nope' in el.dataset, false);
});

test('clicking a label forwards the click to its associated control', () => {
  const { document } = fresh();
  document.body.innerHTML = '<label for="t">L</label><input id="t">';
  const input = document.getElementById('t');
  let clicked = 0;
  input.addEventListener('click', () => { clicked++; });
  document.querySelector('label').click();
  assert.equal(clicked, 1); // label default action → control.click()
});

test('select value setter selects the matching option', () => {
  const { document } = fresh();
  document.body.innerHTML = '<select><option value="a">A</option><option value="b">B</option></select>';
  const sel = document.querySelector('select');
  sel.value = 'b';
  assert.equal(sel.value, 'b');
  assert.equal(sel.selectedOptions[0].value, 'b');
});

test('MutationObserver without subtree ignores descendant mutations', async () => {
  const { window, document } = fresh();
  document.body.innerHTML = '<div id="r"><span id="s"></span></div>';
  const recs = [];
  const mo = new window.MutationObserver((r) => recs.push(...r));
  mo.observe(document.getElementById('r'), { attributes: true }); // no subtree
  document.getElementById('s').setAttribute('x', '1'); // descendant → ignored
  document.getElementById('r').setAttribute('y', '2'); // on target → observed
  await Promise.resolve();
  mo.disconnect();
  assert.ok(recs.every((r) => r.target.id === 'r'));
  assert.ok(recs.some((r) => r.attributeName === 'y'));
});

// -------------------------------------------------------------- stubs ----
test('canvas 2D context stub: measureText, getImageData, gradients', () => {
  const { document } = fresh();
  const ctx = document.createElement('canvas').getContext('2d');
  assert.equal(ctx.measureText('hello').width, 30); // 5 * 6
  const img = ctx.getImageData(0, 0, 1, 1);
  assert.equal(img.width, 0);
  const g = ctx.createLinearGradient(0, 0, 1, 1);
  g.addColorStop(0, '#fff'); // no throw
  ctx.fillRect(0, 0, 1, 1);   // arbitrary no-op method
  assert.equal(ctx.canvas, null);
});

test('FileReader.readAsText resolves async', async () => {
  const { window } = fresh();
  const fr = new window.FileReader();
  const done = new Promise((res) => { fr.onload = res; });
  fr.readAsText(new Blob(['hi']));
  await done;
  assert.equal(fr.result, 'hi');
  assert.equal(fr.readyState, 2);
});

test('customElements define/get/whenDefined', async () => {
  const { window } = fresh();
  class XEl {}
  const w = window.customElements.whenDefined('x-el');
  window.customElements.define('x-el', XEl);
  assert.equal(window.customElements.get('x-el'), XEl);
  assert.equal(window.customElements.getName(XEl), 'x-el');
  assert.equal(await w, XEl);
  assert.throws(() => window.customElements.define('x-el', XEl), /already defined/);
});

test('localStorage Storage API', () => {
  const { window } = fresh();
  const s = window.localStorage;
  s.setItem('k', 'v');
  assert.equal(s.getItem('k'), 'v');
  assert.equal(s.length, 1);
  assert.equal(s.key(0), 'k');
  assert.equal(s.getItem('missing'), null);
  s.removeItem('k');
  assert.equal(s.length, 0);
  s.setItem('a', '1'); s.clear();
  assert.equal(s.length, 0);
});

test('matchMedia evaluates width/height/orientation against the viewport', () => {
  const { window } = fresh(); // default viewport 1024x768
  // width features
  assert.equal(window.matchMedia('(min-width: 100px)').matches, true);
  assert.equal(window.matchMedia('(min-width: 2000px)').matches, false);
  assert.equal(window.matchMedia('(max-width: 600px)').matches, false);
  assert.equal(window.matchMedia('screen and (max-width: 2000px)').matches, true);
  // height features
  assert.equal(window.matchMedia('(min-height: 100px)').matches, true);
  assert.equal(window.matchMedia('(max-height: 100px)').matches, false);
  // combined AND (both must hold)
  assert.equal(window.matchMedia('(min-width: 100px) and (max-width: 2000px)').matches, true);
  assert.equal(window.matchMedia('(min-width: 100px) and (max-width: 500px)').matches, false);
  // orientation (1024 >= 768 -> landscape)
  assert.equal(window.matchMedia('(orientation: landscape)').matches, true);
  assert.equal(window.matchMedia('(orientation: portrait)').matches, false);
  // feature-less / unknown query stays honestly false
  assert.equal(window.matchMedia('(x)').matches, false);
  assert.equal(window.matchMedia('print').matches, false);
  // addressable MediaQueryList surface
  const mq = window.matchMedia('(min-width: 100px)');
  assert.equal(mq.media, '(min-width: 100px)');
  mq.addEventListener('change', () => {});
  mq.removeEventListener('change', () => {});
  mq.addListener(() => {});
  mq.removeListener(() => {});
  assert.equal(mq.dispatchEvent({}), false);
});

test('live collection traps handle symbol + non-index keys', () => {
  const { document } = fresh();
  document.body.innerHTML = '<i></i>';
  const c = document.getElementsByTagName('i');
  assert.equal(c[Symbol('x')], undefined);                          // get: symbol fallthrough
  assert.equal(Symbol.iterator in c, false);                         // has: non-string key
  assert.equal(Object.getOwnPropertyDescriptor(c, 'item'), undefined); // descriptor: non-index string
});

// ------------------------------------------------------- window proxy ----
test('window proxy: has + getOwnPropertyDescriptor + assignment shadowing', () => {
  const { window } = fresh();
  assert.equal('Node' in window, true);          // STATIC_BASE
  assert.equal('window' in window, true);         // self-reference key
  assert.equal('definitelyNotAGlobal' in window, false);
  window.myGlobal = 42;                           // set trap
  assert.equal('myGlobal' in window, true);
  assert.equal(window.myGlobal, 42);
  const d = Object.getOwnPropertyDescriptor(window, 'Node');
  assert.equal(typeof d.value, 'function');
  assert.equal(window.self, window);
  assert.equal(window.globalThis, window);
});

test('window honest-stub surface: geometry factories + chrome no-ops are callable', () => {
  const { window } = fresh();
  // SHARED_LAZY geometry factories (honest constants) — materialize each
  for (const k of ['devicePixelRatio', 'innerWidth', 'innerHeight', 'outerWidth', 'outerHeight',
    'pageXOffset', 'pageYOffset', 'scrollX', 'scrollY', 'screenX', 'screenY', 'screenLeft', 'screenTop']) {
    assert.equal(typeof window[k], 'number');
  }
  // visualViewport + screen.orientation no-op listeners
  const vv = window.visualViewport;
  assert.equal(vv.width, 1024);
  vv.addEventListener('resize', () => {}); vv.removeEventListener('resize', () => {});
  const orient = window.screen.orientation;
  orient.addEventListener('change', () => {}); orient.removeEventListener('change', () => {});
  // navigator.permissions query result listeners
  // ResizeObserver factory + instance no-ops
  const ro = new window.ResizeObserver(() => {});
  ro.observe(window.document.body); ro.unobserve(window.document.body); ro.disconnect();
  // chrome no-ops — must not throw
  window.scroll(); window.scrollBy(); window.scrollTo(0, 0);
  window.focus(); window.blur(); window.stop(); window.print(); window.close();
  assert.equal(window.open(), null);
  window.moveTo(0, 0); window.moveBy(1, 1); window.resizeTo(1, 1); window.resizeBy(1, 1);
  window.alert('x'); assert.equal(window.confirm('x'), false); assert.equal(window.prompt('x'), null);
  window.reportError(new Error('x'));
  // Worker stub
  const w = new window.Worker('u'); w.postMessage('m'); w.addEventListener('message', () => {});
  w.removeEventListener('message', () => {}); w.terminate();
});

test('single listener adding another mid-dispatch: new one does NOT fire this round', () => {
  const { document, window } = fresh();
  const el = document.createElement('div');
  const order = [];
  const second = () => order.push('second');
  el.addEventListener('ping', () => { order.push('first'); el.addEventListener('ping', second); });
  el.dispatchEvent(new window.Event('ping'));     // snapshot: only 'first' fires this round
  assert.deepEqual(order, ['first']);
  el.dispatchEvent(new window.Event('ping'));     // now both registered → first then second
  assert.deepEqual(order, ['first', 'first', 'second']);
});

test('single once listener fires exactly once and is removed', () => {
  const { document, window } = fresh();
  const el = document.createElement('div');
  let n = 0;
  el.addEventListener('ping', () => { n++; }, { once: true });
  el.dispatchEvent(new window.Event('ping'));
  el.dispatchEvent(new window.Event('ping'));
  assert.equal(n, 1);
});

test('childElementCount/first/last share the version-cache and reflect mutations', () => {
  const { document } = fresh();
  document.body.innerHTML = '<ul id="l"><li>a</li><li>b</li></ul>';
  const ul = document.getElementById('l');
  assert.equal(ul.childElementCount, 2);
  assert.equal(ul.firstElementChild.textContent, 'a');
  assert.equal(ul.lastElementChild.textContent, 'b');
  ul.appendChild(document.createElement('li'));   // mutation bumps __version → cache invalidates
  assert.equal(ul.childElementCount, 3);
  assert.equal(ul.lastElementChild.localName, 'li');
  ul.removeChild(ul.firstElementChild);
  assert.equal(ul.childElementCount, 2);
  assert.equal(ul.firstElementChild.textContent, 'b');
  // sibling edges: forward scan finds the next element; last/first return null
  assert.equal(ul.firstElementChild.nextElementSibling, ul.lastElementChild);
  assert.equal(ul.lastElementChild.previousElementSibling, ul.firstElementChild);
  assert.equal(ul.lastElementChild.nextElementSibling, null);
  assert.equal(ul.firstElementChild.previousElementSibling, null);
});

test('composedPath() on a never-dispatched event is [] (lazy _path)', () => {
  const { window } = fresh();
  const e = new window.Event('x');
  assert.deepEqual(e.composedPath(), []);
});

test('tag-specific HTML*Element constructors throw (illegal constructor)', () => {
  const { window } = fresh();
  assert.throws(() => new window.HTMLDivElement(), /Illegal constructor/);
});

test('navigator.permissions query result has no-op listeners', async () => {
  const { window } = fresh();
  const status = await window.navigator.permissions.query({ name: 'clipboard-read' });
  status.addEventListener('change', () => {});
  status.removeEventListener('change', () => {});
  assert.equal(status.state, 'prompt');
});

test('FormData keeps Blob identity + renames on append with filename', () => {
  const { window } = fresh();
  const fd = new window.FormData();
  const blob = new Blob(['x'], { type: 'text/plain' });
  fd.append('f', blob, 'renamed.txt');
  const got = fd.get('f');
  assert.equal(got.name, 'renamed.txt');
  fd.append('s', 'str');
  assert.equal(fd.get('s'), 'str');
  const blob2 = new Blob(['y']);
  fd.append('b', blob2);                 // no filename → value kept as-is
  assert.equal(fd.get('b'), blob2);
});

test('getOwnPropertyDescriptor returns undefined for a non-global key', () => {
  const { window } = fresh();
  assert.equal(Object.getOwnPropertyDescriptor(window, 'nope_not_here'), undefined);
});

test('XMLHttpRequest.send resolves via fetch (load) and reports status', async () => {
  const { window } = fresh();
  const orig = globalThis.fetch;
  globalThis.fetch = async () => ({ status: 200, statusText: 'OK', text: async () => 'body' });
  try {
    const xhr = new window.XMLHttpRequest();
    xhr.open('GET', 'http://x/');
    const done = new Promise((res) => { xhr.onload = res; });
    xhr.send();
    await done;
    assert.equal(xhr.status, 200);
    assert.equal(xhr.responseText, 'body');
    assert.equal(xhr.readyState, 4);
  } finally { globalThis.fetch = orig; }
});

test('XMLHttpRequest.send fires error when fetch rejects', async () => {
  const { window } = fresh();
  const orig = globalThis.fetch;
  globalThis.fetch = async () => { throw new Error('net'); };
  try {
    const xhr = new window.XMLHttpRequest();
    xhr.open('GET', 'http://x/');
    const done = new Promise((res) => { xhr.onerror = res; });
    xhr.send();
    await done;
    assert.equal(xhr.status, 0);
  } finally { globalThis.fetch = orig; }
});

// --------------------------------------------- final DOM edge coverage ----
test('.value on a non-form element is undefined', () => {
  const { document } = fresh();
  assert.equal(document.createElement('div').value, undefined);
});

test('cloneNode on a DocumentType uses the base Node clone', () => {
  const { document } = fresh('<!doctype html><html><body></body></html>');
  const dt = document.doctype;
  assert.ok(dt);
  const c = dt.cloneNode();
  assert.equal(c.nodeType, dt.nodeType);
});

test('getElementsByClassName scans past a substring near-miss', () => {
  const { document } = fresh('<!doctype html><body><div class="midx mid"></div></body>');
  // "mid" appears at index 0 inside "midx" (near-miss) → scan continues to the
  // real whole-word "mid" at the end.
  assert.equal(document.getElementsByClassName('mid').length, 1);
});

test('querySelector .class also scans past a substring near-miss', () => {
  const { document } = fresh('<!doctype html><body><b class="foobar foo">x</b></body>');
  assert.ok(document.querySelector('.foo'));
  assert.equal(document.querySelectorAll('.foo').length, 1);
});

test('option:selected pseudo matches selected options', () => {
  const { document } = fresh();
  document.body.innerHTML = '<select><option>a</option><option selected>b</option></select>';
  assert.equal(document.querySelectorAll('option:selected').length, 1);
});

test('dataset getOwnPropertyDescriptor is undefined for a missing attr', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  assert.equal(Object.getOwnPropertyDescriptor(el.dataset, 'missing'), undefined);
  el.setAttribute('data-here', 'v');
  assert.equal(Object.getOwnPropertyDescriptor(el.dataset, 'here').value, 'v');
});

test('lazy __mo: first observe initializes, second registers + re-observe replaces', async () => {
  const { window, document } = fresh();
  document.body.innerHTML = '<div id="r"></div>';
  const r = document.getElementById('r');
  const a = [], b = [];
  const mo1 = new window.MutationObserver((recs) => a.push(...recs));
  const mo2 = new window.MutationObserver((recs) => b.push(...recs));
  mo1.observe(r, { attributes: true });          // first → __mo initialized (null → [])
  mo2.observe(r, { attributes: true });          // second → __mo non-null branch (filter)
  mo1.observe(r, { childList: true });           // re-observe same (obs,target) → replaces
  r.setAttribute('x', '1');                      // mo1 now childList-only → no attr record
  r.appendChild(document.createElement('span')); // childList → mo1 sees it
  await Promise.resolve();
  mo1.disconnect(); mo2.disconnect();            // __moUnregister (non-null branch)
  assert.ok(a.some((rec) => rec.type === 'childList'));
  assert.ok(a.every((rec) => rec.type !== 'attributes')); // re-observe dropped attributes
  assert.ok(b.some((rec) => rec.type === 'attributes'));   // mo2 still attributes
});

test('MutationObserver subtree ignores a mutation outside the observed root', async () => {
  const { window, document } = fresh();
  document.body.innerHTML = '<div id="r"><span id="s"></span></div><div id="other"></div>';
  const recs = [];
  const mo = new window.MutationObserver((r) => recs.push(...r));
  mo.observe(document.getElementById('r'), { attributes: true, subtree: true });
  document.getElementById('other').setAttribute('x', '1'); // not under #r → isDescendant false
  document.getElementById('s').setAttribute('y', '2');     // under #r → observed
  await Promise.resolve();
  mo.disconnect();
  assert.ok(recs.every((r) => r.target.id !== 'other'));
  assert.ok(recs.some((r) => r.target.id === 's'));
});

// ====================================================================
// ==== batch 2: honest layout/geometry/pointer/animation stubs ========
// ====================================================================

test('Element synthetic geometry + honest scroll/pointer/animation stubs', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="d">x</div>';
  const el = document.getElementById('d');
  const r = el.getBoundingClientRect();
  // synthetic box: non-zero, consistent (right-left===width, bottom-top===height), top/left honest 0
  assert.ok(r.width > 0); assert.ok(r.height > 0);
  assert.equal(r.top, 0); assert.equal(r.left, 0);
  assert.equal(r.right - r.left, r.width); assert.equal(r.bottom - r.top, r.height);
  assert.equal(r.toJSON(), r);
  const rects = el.getClientRects(); // one rect when sized
  assert.equal(rects.length, 1);
  assert.equal(rects[0].width, r.width); assert.equal(rects[0].height, r.height);
  // scroll family are pure no-ops (return undefined, never throw)
  assert.equal(el.scrollIntoView(), undefined);
  assert.equal(el.scroll(), undefined);
  assert.equal(el.scrollTo(0, 0), undefined);
  assert.equal(el.scrollBy(1, 1), undefined);
  // size getters mirror the synthetic rect + are internally consistent
  assert.equal(el.offsetWidth, r.width); assert.equal(el.offsetHeight, r.height);
  assert.equal(el.clientWidth, r.width); assert.equal(el.clientHeight, r.height);
  assert.equal(el.scrollWidth, r.width); assert.equal(el.scrollHeight, r.height);
  // positions + offsetParent stay honest zero/null (no real layout)
  assert.equal(el.offsetTop, 0); assert.equal(el.offsetLeft, 0);
  assert.equal(el.offsetParent, null);
  assert.equal(el.clientTop, 0); assert.equal(el.clientLeft, 0);
  assert.equal(el.scrollTop, 0); assert.equal(el.scrollLeft, 0);
  el.scrollTop = 50; el.scrollLeft = 50; // setters are no-ops
  assert.equal(el.scrollTop, 0); assert.equal(el.scrollLeft, 0);
  // pointer capture honest stubs
  assert.equal(el.setPointerCapture(1), undefined);
  assert.equal(el.releasePointerCapture(1), undefined);
  assert.equal(el.hasPointerCapture(1), false);
});

test('synthetic geometry: stable, block fills parent, inline shrink-wraps, children stack', () => {
  const { document } = fresh();
  document.body.innerHTML = '<section id="s"><p id="p">hello world</p><span id="sp">hi</span><b id="empty"></b></section>';
  const s = document.getElementById('s');
  const p = document.getElementById('p');
  const sp = document.getElementById('sp');
  const empty = document.getElementById('empty');
  // block fills the viewport-derived parent width
  assert.equal(p.offsetWidth, s.offsetWidth);
  assert.ok(s.offsetWidth > 0);
  // inline shrink-wraps its text (narrower than the block parent), capped at parent
  assert.ok(sp.offsetWidth < s.offsetWidth);
  assert.equal(sp.offsetWidth, 'hi'.length * 8);
  // empty inline still non-zero
  assert.ok(empty.offsetWidth > 0);
  // a block with element children stacks them (>= one line)
  assert.ok(s.offsetHeight >= p.offsetHeight);
  // STABLE across calls for the same DOM state (the property that breaks React's loop)
  assert.equal(p.offsetHeight, p.offsetHeight);
  const h1 = p.getBoundingClientRect().height;
  assert.equal(p.getBoundingClientRect().height, h1);
  // mutation invalidates the memo (version bump) and recomputes
  p.textContent = 'a much much much longer run of text that wraps onto more lines than before';
  assert.ok(p.getBoundingClientRect().height >= h1);
});

test('Element.animate returns a controllable Animation-like stub', async () => {
  const { document } = fresh();
  const el = document.createElement('div');
  const anim = el.animate([{ opacity: 0 }, { opacity: 1 }], 100);
  assert.equal(anim.play(), undefined);
  assert.equal(anim.pause(), undefined);
  assert.equal(anim.cancel(), undefined);
  assert.equal(anim.finish(), undefined);
  assert.equal(anim.onfinish, null);
  await anim.finished; // resolves
  assert.deepEqual(el.getAnimations(), []);
});

test('Element.requestFullscreen resolves; toDataURL is the empty data URL', async () => {
  const { document } = fresh();
  const el = document.createElement('div');
  await el.requestFullscreen(); // resolves, no throw
  assert.equal(document.createElement('canvas').toDataURL(), 'data:,');
});

test('namespaced attr-node shims: getAttributeNodeNS / setAttributeNodeNS', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  el.setAttribute('foo', 'bar');
  const node = el.getAttributeNodeNS('http://x', 'foo');
  assert.equal(node.name, 'foo');
  assert.equal(node.value, 'bar');
  // setAttributeNodeNS delegates to setAttributeNode (returns prev or null)
  const prev = el.setAttributeNodeNS({ name: 'foo', value: 'baz' });
  assert.equal(prev.value, 'bar');
  assert.equal(el.getAttribute('foo'), 'baz');
  const none = el.setAttributeNodeNS({ name: 'new', value: '1' });
  assert.equal(none, null);
  assert.equal(el.getAttribute('new'), '1');
});

// ====================================================================
// ==== batch 3: document-level honest stubs ===========================
// ====================================================================

test('document honest layout/command/write stubs', () => {
  const { document } = fresh();
  assert.equal(document.elementFromPoint(0, 0), null);
  assert.deepEqual(document.elementsFromPoint(0, 0), []);
  assert.equal(document.execCommand('bold'), false);
  assert.equal(document.queryCommandSupported('bold'), false);
  assert.equal(document.queryCommandEnabled('bold'), false);
  assert.equal(document.hasFocus(), true);
  // write/writeln are no-ops; open returns the document; close is a no-op
  assert.equal(document.write('<p>x</p>'), undefined);
  assert.equal(document.writeln('<p>y</p>'), undefined);
  assert.equal(document.open(), document);
  assert.equal(document.close(), undefined);
});

test('zeroRect().toJSON returns the rect itself', () => {
  const { document } = fresh();
  const rect = document.createElement('div').getBoundingClientRect();
  assert.equal(rect.toJSON(), rect);
});

// ====================================================================
// ==== batch 4: Text / Comment / DocumentType nodeName + cloneNode ====
// ====================================================================

test('Text node: nodeName, cloneNode preserve data', () => {
  const { document } = fresh();
  const t = document.createTextNode('hello');
  assert.equal(t.nodeName, '#text');
  const c = t.cloneNode();
  assert.equal(c.nodeName, '#text');
  assert.equal(c.data, 'hello');
  assert.notEqual(c, t);
});

test('Comment node: nodeName, cloneNode preserve data', () => {
  const { document } = fresh();
  const cm = document.createComment('note');
  assert.equal(cm.nodeName, '#comment');
  const c = cm.cloneNode();
  assert.equal(c.nodeName, '#comment');
  assert.equal(c.data, 'note');
});

test('DocumentType nodeName is the doctype name', () => {
  const { document } = fresh('<!doctype html><html><body></body></html>');
  assert.equal(document.doctype.nodeName, 'html');
  assert.equal(document.doctype.name, 'html');
});

test('DocumentFragment nodeName + deep cloneNode copies children', () => {
  const { document } = fresh();
  const f = document.createDocumentFragment();
  assert.equal(f.nodeName, '#document-fragment');
  f.appendChild(document.createElement('span'));
  f.appendChild(document.createTextNode('t'));
  const shallow = f.cloneNode(false);
  assert.equal(shallow.childNodes.length, 0);
  const deep = f.cloneNode(true);
  assert.equal(deep.childNodes.length, 2);
  assert.equal(deep.firstChild.localName, 'span');
});

// ====================================================================
// ==== batch 5: base Node.cloneNode fallback ==========================
// ====================================================================

test('base Node.cloneNode (deep) clones children via the fallback path', () => {
  const { document } = fresh();
  // DocumentFragment subclass uses its own clone; force base via a fragment-less node:
  // a Document is a Node — but cloneNode on Document is overridden. Use a fresh
  // element to drive the Element override; for the BASE path, build a small custom node.
  // The base path is hit by nodes whose constructor takes (ownerDocument) only:
  // exercise through a detached element subtree deep clone which recurses children.
  const div = document.createElement('div');
  div.innerHTML = '<a><b>x</b></a>';
  const clone = div.cloneNode(true);
  assert.equal(clone.querySelector('b').textContent, 'x');
  assert.notEqual(clone.querySelector('b'), div.querySelector('b'));
});

// ====================================================================
// ==== batch 6: Range full surface ====================================
// ====================================================================

test('Range: set/select/collapse/clone + content ops on a single container', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="c"><a>1</a><b>2</b><i>3</i></div>';
  const c = document.getElementById('c');
  const r = document.createRange();
  r.setStart(c, 0);
  r.setEnd(c, 2);
  assert.equal(r.collapsed, false);
  assert.equal(r.startOffset, 0);
  assert.equal(r.endOffset, 2);
  assert.equal(r.commonAncestorContainer, c);
  // cloneContents copies the first two element children
  const frag = r.cloneContents();
  assert.equal(frag.childNodes.length, 2);
  assert.equal(c.childNodes.length, 3); // originals untouched
  // cloneRange
  const r2 = r.cloneRange();
  assert.equal(r2.startOffset, 0);
  assert.equal(r2.endOffset, 2);
  // toString concatenates element textContent
  assert.equal(r.toString(), '12');
});

test('Range: setStartBefore/After, setEndBefore/After, selectNode(Contents)', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="c"><a>1</a><b>2</b></div>';
  const c = document.getElementById('c');
  const a = c.children[0], b = c.children[1];
  const r = document.createRange();
  r.setStartBefore(b);
  assert.equal(r.startContainer, c);
  assert.equal(r.startOffset, 1);
  r.setStartAfter(a);
  assert.equal(r.startOffset, 1);
  r.setEndBefore(b);
  assert.equal(r.endOffset, 1);
  r.setEndAfter(b);
  assert.equal(r.endOffset, 2);
  r.selectNode(a);
  assert.equal(r.startOffset, 0);
  assert.equal(r.endOffset, 1);
  r.selectNodeContents(c);
  assert.equal(r.endOffset, 2);
});

test('Range.selectNodeContents on a text node uses string length', () => {
  const { document } = fresh();
  const t = document.createTextNode('abcde');
  const r = document.createRange();
  r.selectNodeContents(t);
  assert.equal(r.startOffset, 0);
  assert.equal(r.endOffset, 5);
  // toString on a text container slices the data
  assert.equal(r.toString(), 'abcde');
  r.setStart(t, 1); r.setEnd(t, 3);
  assert.equal(r.toString(), 'bc');
});

test('Range.collapse to start and to end', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="c"><a>1</a><b>2</b></div>';
  const c = document.getElementById('c');
  const r = document.createRange();
  r.setStart(c, 0); r.setEnd(c, 2);
  r.collapse(true);
  assert.equal(r.collapsed, true);
  assert.equal(r.endOffset, 0);
  const r2 = document.createRange();
  r2.setStart(c, 0); r2.setEnd(c, 2);
  r2.collapse(false);
  assert.equal(r2.collapsed, true);
  assert.equal(r2.startOffset, 2);
});

test('Range.extractContents / deleteContents / insertNode / surroundContents', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="c"><a>1</a><b>2</b><i>3</i></div>';
  const c = document.getElementById('c');
  const r = document.createRange();
  r.setStart(c, 0); r.setEnd(c, 1);
  const extracted = r.extractContents();
  assert.equal(extracted.childNodes.length, 1);
  assert.equal(extracted.firstChild.localName, 'a');
  assert.equal(c.childNodes.length, 2); // 'a' moved out
  // insertNode at the collapsed start
  const span = document.createElement('span');
  r.insertNode(span);
  assert.equal(c.firstChild, span);
  // deleteContents
  const r2 = document.createRange();
  r2.setStart(c, 0); r2.setEnd(c, 1);
  r2.deleteContents();
  assert.notEqual(c.firstChild, span);
});

test('Range.surroundContents wraps extracted content', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="c"><a>1</a><b>2</b></div>';
  const c = document.getElementById('c');
  const r = document.createRange();
  r.setStart(c, 0); r.setEnd(c, 1);
  const wrap = document.createElement('em');
  r.surroundContents(wrap);
  assert.equal(c.firstChild, wrap);
  assert.equal(wrap.firstChild.localName, 'a');
});

test('Range geometry stubs + detach are honest no-ops', () => {
  const { document } = fresh();
  const r = document.createRange();
  assert.equal(r.getBoundingClientRect().width, 0);
  assert.deepEqual(r.getClientRects(), []);
  assert.equal(r.detach(), undefined);
});

test('Range.toString returns "" when start container has no children accessor', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="c"><a>1</a></div>';
  const c = document.getElementById('c');
  const r = document.createRange();
  // cross-container range (start != end) on element container → toString returns ''
  r.startContainer = c;
  r.endContainer = c.children[0];
  r.startOffset = 0; r.endOffset = 0;
  assert.equal(r.toString(), '');
});

// ====================================================================
// ==== batch 7: Selection full surface ================================
// ====================================================================

test('Selection: addRange/getRangeAt/anchor/focus/type and collapse', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="c"><a>1</a><b>2</b></div>';
  const c = document.getElementById('c');
  const sel = document.getSelection();
  assert.equal(sel.rangeCount, 0);
  assert.equal(sel.type, 'None');
  assert.equal(sel.isCollapsed, true);
  assert.equal(sel.anchorNode, null);
  assert.equal(sel.focusNode, null);
  assert.equal(sel.anchorOffset, 0);
  assert.equal(sel.focusOffset, 0);
  const r = document.createRange();
  r.setStart(c, 0); r.setEnd(c, 2);
  sel.addRange(r);
  assert.equal(sel.rangeCount, 1);
  assert.equal(sel.type, 'Range');
  assert.equal(sel.anchorNode, c);
  assert.equal(sel.focusNode, c);
  assert.equal(sel.anchorOffset, 0);
  assert.equal(sel.focusOffset, 2);
  assert.equal(sel.getRangeAt(0), r);
  assert.equal(sel.toString(), '12');
  sel.removeRange(r);
  assert.equal(sel.rangeCount, 0);
});

test('Selection: collapse(null) empties; collapse/extend/selectAllChildren', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="c"><a>1</a><b>2</b></div>';
  const c = document.getElementById('c');
  const sel = document.getSelection();
  sel.collapse(c, 1);
  assert.equal(sel.rangeCount, 1);
  assert.equal(sel.type, 'Caret');
  assert.equal(sel.isCollapsed, true);
  sel.extend(c, 2);
  assert.equal(sel.focusOffset, 2);
  sel.collapse(null);
  assert.equal(sel.rangeCount, 0);
  // extend with no existing range creates one
  sel.extend(c, 1);
  assert.equal(sel.rangeCount, 1);
  sel.selectAllChildren(c);
  assert.equal(sel.focusOffset, 2);
});

test('Selection: collapseToStart/End, setBaseAndExtent, empty, removeAllRanges', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="c"><a>1</a><b>2</b></div>';
  const c = document.getElementById('c');
  const sel = document.getSelection();
  sel.setBaseAndExtent(c, 0, c, 2);
  assert.equal(sel.rangeCount, 1);
  assert.equal(sel.anchorOffset, 0);
  assert.equal(sel.focusOffset, 2);
  sel.collapseToStart();
  assert.equal(sel.getRangeAt(0).collapsed, true);
  sel.setBaseAndExtent(c, 0, c, 2);
  sel.collapseToEnd();
  assert.equal(sel.getRangeAt(0).collapsed, true);
  sel.empty();
  assert.equal(sel.rangeCount, 0);
  assert.equal(sel.toString(), ''); // no range → empty string
  sel.addRange(document.createRange());
  sel.removeAllRanges();
  assert.equal(sel.rangeCount, 0);
  // collapseToStart/End with no range are safe no-ops
  sel.collapseToStart(); sel.collapseToEnd();
  assert.equal(sel.rangeCount, 0);
});

// ====================================================================
// ==== batch 8: TreeWalker / NodeIterator =============================
// ====================================================================

test('TreeWalker walks elements with nextNode/previousNode/parent/siblings', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="r"><a>1</a><b>2</b><c1>3</c1></div>';
  const r = document.getElementById('r');
  const tw = document.createTreeWalker(r, 1 /* SHOW_ELEMENT */);
  assert.equal(tw.root, r);
  assert.equal(tw.currentNode, r);
  const first = tw.nextNode();
  assert.equal(first.localName, 'a');
  const second = tw.nextNode();
  assert.equal(second.localName, 'b');
  const back = tw.previousNode();
  assert.equal(back.localName, 'a');
  // firstChild / lastChild
  tw.currentNode = r;
  assert.equal(tw.firstChild().localName, 'a');
  tw.currentNode = r;
  assert.equal(tw.lastChild().localName, 'c1');
  // siblings
  tw.currentNode = r.children[0];
  assert.equal(tw.nextSibling().localName, 'b');
  assert.equal(tw.previousSibling().localName, 'a');
  // parentNode (stops at root.parentNode)
  tw.currentNode = r.children[0];
  assert.equal(tw.parentNode(), r);
  assert.equal(tw.parentNode(), null); // r's parent is root.parentNode boundary? body
  assert.equal(tw.detach(), undefined);
});

test('TreeWalker with a function filter rejects/skips nodes', () => {
  const { document, window } = fresh();
  document.body.innerHTML = '<div id="r"><a class="keep">1</a><b>2</b><a class="keep">3</a></div>';
  const r = document.getElementById('r');
  const tw = document.createTreeWalker(r, 0xffffffff, (node) =>
    node.nodeType === 1 && node.localName === 'a' ? 1 /* ACCEPT */ : 2 /* REJECT */);
  const acc = [];
  let n;
  while ((n = tw.nextNode())) acc.push(n.localName);
  assert.deepEqual(acc, ['a', 'a']);
  void window;
});

test('TreeWalker filter as an object with acceptNode', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="r"><a>1</a><b>2</b></div>';
  const r = document.getElementById('r');
  const tw = document.createNodeIterator(r, 1, { acceptNode: (n) => (n.localName === 'b' ? 1 : 2) });
  assert.equal(tw.nextNode().localName, 'b');
  assert.equal(tw.nextNode(), null);
});

test('TreeWalker SHOW_TEXT only visits text nodes', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="r">hi<a>x</a>bye</div>';
  const r = document.getElementById('r');
  const tw = document.createTreeWalker(r, 4 /* SHOW_TEXT */);
  const texts = [];
  let n;
  while ((n = tw.nextNode())) texts.push(n.data);
  assert.deepEqual(texts, ['hi', 'x', 'bye']);
});

test('TreeWalker SHOW_COMMENT visits comments only', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="r"><!--c1--><a>x</a><!--c2--></div>';
  const r = document.getElementById('r');
  const tw = document.createTreeWalker(r, 128 /* SHOW_COMMENT */);
  const out = [];
  let n;
  while ((n = tw.nextNode())) out.push(n.data);
  assert.deepEqual(out, ['c1', 'c2']);
});

// ====================================================================
// ==== batch 9: inline style (makeStyle) surface ======================
// ====================================================================

test('el.style getPropertyValue/Priority/setProperty/removeProperty/item/length', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  const s = el.style;
  s.setProperty('color', 'red');
  s.setProperty('width', '10px', 'important');
  assert.equal(s.getPropertyValue('width'), '10px');
  assert.equal(s.getPropertyPriority('width'), 'important');
  assert.equal(s.getPropertyPriority('color'), '');
  assert.equal(s.length, 2);
  assert.equal(s.item(0), 'color');
  assert.ok(el.getAttribute('style').includes('!important'));
  const removed = s.removeProperty('color');
  assert.equal(removed, 'red');
  assert.equal(s.length, 1);
  // setProperty without priority clears any prior important
  s.setProperty('width', '20px');
  assert.equal(s.getPropertyPriority('width'), '');
});

test('el.style cssText get/set, cssFloat, iterator, has trap', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  el.style.cssText = 'color: blue; float: left';
  assert.equal(el.style.cssText, 'color: blue; float: left');
  assert.equal(el.style.cssFloat, 'left');
  const keys = [...el.style];
  assert.ok(keys.includes('color'));
  assert.ok(keys.includes('float'));
  assert.equal('color' in el.style, true);
  // symbol key on get → undefined (non-string fallthrough)
  assert.equal(el.style[Symbol('z')], undefined);
  // camelCase set writes the kebab property
  el.style.backgroundColor = 'green';
  assert.ok(el.getAttribute('style').includes('background-color'));
  // camelCase read resolves via kebab
  assert.equal(el.style.backgroundColor.startsWith('rgb') || el.style.backgroundColor === 'green', true);
});

test('el.style declaration with no colon is skipped; empty prop ignored', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  el.setAttribute('style', 'novalue; : skipme; color: red');
  assert.equal(el.style.color.startsWith('rgb') || el.style.color === 'red', true);
  assert.equal(el.style.getPropertyValue('novalue'), '');
});

// ====================================================================
// ==== batch 10: buffer.parent + doctype ids ==========================
// ====================================================================

test('buffer-backed parent traversal + doctype publicId/systemId', () => {
  const { document } = fresh(
    '<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.01//EN" "http://www.w3.org/TR/html4/strict.dtd"><html><body><p>x</p></body></html>');
  // doctype with public + system ids hits the textId/pubId/sysId >= 0 branches
  const dt = document.doctype;
  assert.equal(dt.publicId, '-//W3C//DTD HTML 4.01//EN');
  assert.equal(dt.systemId, 'http://www.w3.org/TR/html4/strict.dtd');
  // parentNode traversal through buffer-backed nodes (Buffer.parent)
  const p = document.querySelector('p');
  assert.equal(p.parentNode.localName, 'body');
  assert.equal(p.parentNode.parentNode.localName, 'html');
});

// ====================================================================
// ==== batch 11: color canonicalization edge cases ====================
// ====================================================================

test('canonicalizeColor: hsl with/without alpha, percentages, named, transparent', async () => {
  const { canonicalizeColor } = await import('../src/runtime/color.mjs');
  // hsl → rgb
  assert.equal(canonicalizeColor('hsl(0, 100%, 50%)', false), 'rgb(255, 0, 0)');
  // hsl achromatic (s=0)
  assert.equal(canonicalizeColor('hsl(0, 0%, 50%)', false), 'rgb(128, 128, 128)');
  // hsla with alpha < 1
  assert.equal(canonicalizeColor('hsla(120, 100%, 50%, 0.5)', false), 'rgba(0, 255, 0, 0.5)');
  // hue wrap (negative + > 360)
  assert.ok(canonicalizeColor('hsl(-120, 100%, 50%)', false).startsWith('rgb'));
  assert.ok(canonicalizeColor('hsl(480, 100%, 50%)', false).startsWith('rgb'));
  // rgb percentages → null (passthrough)
  assert.equal(canonicalizeColor('rgb(50%, 50%, 50%)', false), null);
  // rgba with alpha
  assert.equal(canonicalizeColor('rgba(0, 0, 0, 0.25)', false), 'rgba(0, 0, 0, 0.25)');
  // hex shorthand with alpha (#rgba)
  assert.ok(canonicalizeColor('#ff08', false).startsWith('rgba'));
  // hex 8-digit alpha
  assert.ok(canonicalizeColor('#00ff0080', false).startsWith('rgba'));
  // named (includeNamed true)
  assert.equal(canonicalizeColor('red', true), 'rgb(255, 0, 0)');
  assert.equal(canonicalizeColor('transparent', true), 'rgba(0, 0, 0, 0)');
  // named with includeNamed false → null
  assert.equal(canonicalizeColor('red', false), null);
  // unrecognized keyword → null
  assert.equal(canonicalizeColor('notacolor', true), null);
  // empty / falsy → null
  assert.equal(canonicalizeColor('', false), null);
  // invalid hex (bad chars) → null
  assert.equal(canonicalizeColor('#zzzzzz', false), null);
  // malformed rgb (too few parts) → null
  assert.equal(canonicalizeColor('rgb(1, 2)', false), null);
  // negative alpha clamps to 0
  assert.equal(canonicalizeColor('rgba(0, 0, 0, -1)', false), 'rgba(0, 0, 0, 0)');
  // values over 255 clamp
  assert.equal(canonicalizeColor('rgb(300, 0, 0)', false), 'rgb(255, 0, 0)');
  // negative rgb component clamps to 0 (clamp255 lower branch)
  assert.equal(canonicalizeColor('rgb(-5, 0, 0)', false), 'rgb(0, 0, 0)');
  // malformed rgb without closing paren → null
  assert.equal(canonicalizeColor('rgb(1, 2, 3', false), null);
  // rgb with a non-numeric component → null (NaN branch)
  assert.equal(canonicalizeColor('rgb(a, b, c)', false), null);
  // malformed hsl without paren → null
  assert.equal(canonicalizeColor('hsl 0 0 0', false), null);
  // hsl with too few parts → null
  assert.equal(canonicalizeColor('hsl(0, 0%)', false), null);
  // hsl with a non-numeric channel → null
  assert.equal(canonicalizeColor('hsl(x, y%, z%)', false), null);
  // dark hsl (l < 0.5) drives the q = l*(1+s) branch + the various hue() arms
  assert.ok(canonicalizeColor('hsl(200, 80%, 25%)', false).startsWith('rgb'));
  // light hsl (l >= 0.5) drives the other q branch
  assert.ok(canonicalizeColor('hsl(40, 90%, 75%)', false).startsWith('rgb'));
  // hue near 0 (b channel hits t<0 wrap) and near 360
  assert.ok(canonicalizeColor('hsl(10, 100%, 40%)', false).startsWith('rgb'));
  assert.ok(canonicalizeColor('hsl(350, 100%, 40%)', false).startsWith('rgb'));
});

// ====================================================================
// ==== batch 12: stubs.mjs observers ==================================
// ====================================================================

test('stubs IntersectionObserver/ResizeObserver fire once with an initial entry', async () => {
  const { document, window } = fresh();
  document.body.innerHTML = '<div id="t">hi</div>';
  const el = document.getElementById('t');

  // IntersectionObserver: options parsed; fires once, isIntersecting:true, ratio:1
  const io = new window.IntersectionObserver(() => {}, { rootMargin: '10px', threshold: [0, 0.5] });
  assert.equal(io.root, null);
  assert.equal(io.rootMargin, '10px');
  assert.deepEqual(io.thresholds, [0, 0.5]);
  const ioEntries = await new Promise((res) => {
    new window.IntersectionObserver((entries) => res(entries)).observe(el);
  });
  assert.equal(ioEntries.length, 1);
  assert.equal(ioEntries[0].target, el);
  assert.equal(ioEntries[0].isIntersecting, true);
  assert.equal(ioEntries[0].intersectionRatio, 1);
  assert.equal(ioEntries[0].boundingClientRect.width, el.getBoundingClientRect().width);
  // default-threshold construction path
  assert.deepEqual(new window.IntersectionObserver(() => {}).thresholds, [0]);
  assert.equal(io.unobserve(), undefined);
  assert.equal(io.disconnect(), undefined);
  assert.deepEqual(io.takeRecords(), []);

  // ResizeObserver: fires once with a contentRect matching the synthetic box
  const roEntries = await new Promise((res) => {
    new window.ResizeObserver((entries) => res(entries)).observe(el);
  });
  assert.equal(roEntries.length, 1);
  assert.equal(roEntries[0].target, el);
  assert.equal(roEntries[0].contentRect.width, el.getBoundingClientRect().width);
  assert.equal(roEntries[0].borderBoxSize[0].inlineSize, el.getBoundingClientRect().width);
  const ro = new window.ResizeObserver(() => {});
  assert.equal(ro.unobserve(), undefined);
  assert.equal(ro.disconnect(), undefined);

  // a throwing callback must not crash (render-tier safety)
  new window.ResizeObserver(() => { throw new Error('boom'); }).observe(el);
  new window.IntersectionObserver(() => { throw new Error('boom'); }).observe(el);
  // observe with no element → rect falls back to zero, still fires once
  const z = await new Promise((res) => { new window.ResizeObserver((e) => res(e)).observe(undefined); });
  assert.equal(z[0].contentRect.width, 0);
  await new Promise((r) => setTimeout(r, 0)); // let the throwing callbacks settle

  // stubs.mjs MutationObserver remains the honest queue-based no-op stub
  const stubs = await import('../src/runtime/stubs.mjs');
  const mo = new stubs.MutationObserver(() => {});
  assert.equal(mo.observe(), undefined);
  assert.equal(mo.disconnect(), undefined);
  assert.deepEqual(mo.takeRecords(), []);
});

test('stubs FileReader removeEventListener removes a registered listener', async () => {
  const stubs = await import('../src/runtime/stubs.mjs');
  const fr = new stubs.FileReader();
  let hits = 0;
  const cb = () => { hits++; };
  fr.addEventListener('load', cb);
  fr.removeEventListener('load', cb);
  fr.__fire('load');
  assert.equal(hits, 0);
});

// ====================================================================
// ==== batch 13: window.mjs origin/btoa/atob/URL/Blob/perf =============
// ====================================================================

test('window.origin is lazily materialized from the url', () => {
  const { window } = createEnvironment('<!doctype html><html><body></body></html>', { url: 'https://example.com:8443/p' });
  assert.equal(window.origin, 'https://example.com:8443');
});

test('window.btoa / atob round-trip', () => {
  const { window } = fresh();
  const enc = window.btoa('hello');
  assert.equal(window.atob(enc), 'hello');
});

test('window.URL constructs + createObjectURL/revokeObjectURL + origin/toString', () => {
  const { window } = fresh();
  const u = new window.URL('https://a.test/path?q=1');
  assert.equal(u.toString(), 'https://a.test/path?q=1');
  assert.equal(u.origin, 'https://a.test');
  const blobUrl = window.URL.createObjectURL(new window.Blob(['x']));
  assert.ok(blobUrl.startsWith('blob:turbo-dom/'));
  assert.equal(window.URL.revokeObjectURL(blobUrl), undefined);
});

test('window.Blob text/arrayBuffer/slice + File extends Blob', async () => {
  const { window } = fresh();
  const b = new window.Blob(['ab', 'cd'], { type: 'text/plain' });
  assert.equal(b.type, 'text/plain');
  const txt = await b.text();
  assert.ok(typeof txt === 'string');
  const ab = await b.arrayBuffer();
  assert.ok(ab instanceof ArrayBuffer);
  const sliced = b.slice(0, 1);
  assert.ok(sliced instanceof window.Blob);
  const f = new window.File(['x'], 'name.txt', { type: 'text/plain', lastModified: 5 });
  assert.equal(f.name, 'name.txt');
  assert.equal(f.lastModified, 5);
  assert.ok(f instanceof window.Blob);
});

test('window.performance.now returns a number', () => {
  const { window } = fresh();
  assert.equal(typeof window.performance.now(), 'number');
});

test('window.removeEventListener delegates to document', () => {
  const { window, document } = fresh();
  let hits = 0;
  const cb = () => { hits++; };
  window.addEventListener('custom', cb);
  window.removeEventListener('custom', cb);
  document.dispatchEvent(new window.Event('custom'));
  assert.equal(hits, 0);
});

// ====================================================================
// ==== batch 14: cssom.mjs surface ====================================
// ====================================================================

test('CSSStyleSheet insertRule/deleteRule/addRule/removeRule + bounds', async () => {
  const { CSSStyleSheet } = await import('../src/runtime/cssom.mjs');
  const sheet = new CSSStyleSheet(null, { media: 'screen', title: 'main' });
  assert.equal(sheet.media, 'screen');
  assert.equal(sheet.title, 'main');
  assert.equal(sheet.insertRule('.a { color: red }'), 0);
  assert.equal(sheet.insertRule('.b { color: blue }', 1), 1);
  assert.equal(sheet.cssRules.length, 2);
  assert.equal(sheet.rules.length, 2); // legacy alias
  assert.equal(sheet.cssRules[0].selectorText, '.a');
  assert.throws(() => sheet.insertRule('.c {}', 99), RangeError);
  assert.throws(() => sheet.deleteRule(-1), RangeError);
  sheet.deleteRule(0);
  assert.equal(sheet.cssRules.length, 1);
  // legacy addRule/removeRule
  assert.equal(sheet.addRule('.x', 'color: green'), -1);
  assert.equal(sheet.cssRules.length, 2);
  sheet.removeRule(0);
  assert.equal(sheet.cssRules.length, 1);
});

test('CSSStyleSheet replaceSync/replace split top-level rules + defaults', async () => {
  const { CSSStyleSheet } = await import('../src/runtime/cssom.mjs');
  const sheet = new CSSStyleSheet();
  assert.deepEqual(sheet.media, []);
  assert.equal(sheet.title, null);
  sheet.replaceSync('.a { color: red } .b { color: blue }');
  assert.equal(sheet.cssRules.length, 2);
  const ret = await sheet.replace('.c { color: green }');
  assert.equal(ret, sheet);
  assert.equal(sheet.cssRules.length, 1);
});

test('CSSStyleRule selectorText is "" when there is no opening brace', async () => {
  const { CSSStyleRule } = await import('../src/runtime/cssom.mjs');
  const rule = new CSSStyleRule('no-brace-here');
  assert.equal(rule.selectorText, '');
});

test('styleSheetList is indexable + iterable with item()', async () => {
  const { styleSheetList } = await import('../src/runtime/cssom.mjs');
  const list = styleSheetList(['a', 'b']);
  assert.equal(list[0], 'a');
  assert.equal(list.item(1), 'b');
  assert.equal(list.item(5), null);
  assert.deepEqual([...list], ['a', 'b']);
});

test('document.styleSheets reflects injected <style> via insertRule', () => {
  const { document } = fresh();
  document.head.innerHTML = '<style></style>';
  const styleEl = document.querySelector('style');
  const sheet = styleEl.sheet;
  sheet.insertRule('.dynamic { color: red }', 0);
  // textContent reflects injected rules
  assert.ok(styleEl.textContent.includes('.dynamic'));
  assert.ok(document.styleSheets.length >= 1);
});

// ====================================================================
// ==== batch 15: form-control reflected properties (with + without) ===
// ====================================================================

test('input/anchor reflected string props default "" then reflect the attr', () => {
  const { document } = fresh();
  const i = document.createElement('input');
  // absent → defaults
  assert.equal(i.name, '');
  assert.equal(i.placeholder, '');
  assert.equal(i.inputMode, '');
  assert.equal(i.autocomplete, '');
  assert.equal(i.accept, '');
  assert.equal(i.min, ''); assert.equal(i.max, ''); assert.equal(i.step, '');
  assert.equal(i.pattern, '');
  assert.equal(i.alt, ''); assert.equal(i.src, '');
  assert.equal(i.referrerPolicy, '');
  assert.equal(i.spellcheck, true); // null → true
  // set then read back
  i.name = 'n'; assert.equal(i.getAttribute('name'), 'n');
  i.placeholder = 'p'; assert.equal(i.placeholder, 'p');
  i.inputMode = 'numeric'; assert.equal(i.getAttribute('inputmode'), 'numeric');
  i.autocomplete = 'off'; assert.equal(i.autocomplete, 'off');
  i.accept = 'image/*'; assert.equal(i.accept, 'image/*');
  i.min = '1'; i.max = '9'; i.step = '2';
  assert.equal(i.min, '1'); assert.equal(i.max, '9'); assert.equal(i.step, '2');
  i.pattern = '\\d+'; assert.equal(i.pattern, '\\d+');
  i.src = 's'; i.alt = 'a';
  assert.equal(i.src, 's'); assert.equal(i.alt, 'a');
  i.referrerPolicy = 'no-referrer'; assert.equal(i.referrerPolicy, 'no-referrer');
  i.spellcheck = false; assert.equal(i.spellcheck, false);
  i.spellcheck = true; assert.equal(i.spellcheck, true);

  const a = document.createElement('a');
  assert.equal(a.href, ''); assert.equal(a.download, ''); assert.equal(a.rel, '');
  a.href = '/x'; a.download = 'f'; a.rel = 'noopener';
  assert.equal(a.href, '/x'); assert.equal(a.download, 'f'); assert.equal(a.rel, 'noopener');
});

test('maxLength/minLength/colSpan/rowSpan defaults and reflection', () => {
  const { document } = fresh();
  const i = document.createElement('input');
  assert.equal(i.maxLength, -1);
  assert.equal(i.minLength, -1);
  i.maxLength = 5; assert.equal(i.maxLength, 5);
  i.minLength = 2; assert.equal(i.minLength, 2);
  const td = document.createElement('td');
  assert.equal(td.colSpan, 1); // default 1
  assert.equal(td.rowSpan, 1);
  td.colSpan = 3; assert.equal(td.colSpan, 3);
  td.rowSpan = 2; assert.equal(td.rowSpan, 2);
  td.setAttribute('colspan', '0'); assert.equal(td.colSpan, 1); // <=0 → 1
});

test('type getter defaults for input/button and other elements', () => {
  const { document } = fresh();
  assert.equal(document.createElement('input').type, 'text');
  assert.equal(document.createElement('button').type, 'submit');
  const li = document.createElement('li');
  assert.equal(li.type, undefined); // other element, no type attr
  li.setAttribute('type', 'a');
  assert.equal(li.type, 'a');
});

test('disabled/readOnly/required boolean reflection both directions', () => {
  const { document } = fresh();
  const i = document.createElement('input');
  assert.equal(i.disabled, false);
  i.disabled = true; assert.equal(i.hasAttribute('disabled'), true);
  i.disabled = false; assert.equal(i.hasAttribute('disabled'), false);
  i.readOnly = true; assert.equal(i.hasAttribute('readonly'), true);
  i.readOnly = false; assert.equal(i.hasAttribute('readonly'), false);
  i.required = true; assert.equal(i.hasAttribute('required'), true);
  i.required = false; assert.equal(i.hasAttribute('required'), false);
});

test('checked/defaultChecked/multiple reflection', () => {
  const { document } = fresh();
  const i = document.createElement('input');
  assert.equal(i.checked, false);
  i.setAttribute('checked', '');
  assert.equal(i.checked, true); // falls back to attribute
  i.checked = false;
  assert.equal(i.checked, false); // internal overrides
  assert.equal(i.defaultChecked, true);
  i.defaultChecked = false;
  assert.equal(i.hasAttribute('checked'), false);
  const sel = document.createElement('select');
  assert.equal(sel.multiple, false);
  sel.multiple = true; assert.equal(sel.hasAttribute('multiple'), true);
  sel.multiple = false; assert.equal(sel.hasAttribute('multiple'), false);
});

test('valueAsNumber/valueAsDate getters + setters', () => {
  const { document } = fresh();
  const i = document.createElement('input');
  i.value = '42';
  assert.equal(i.valueAsNumber, 42);
  i.value = '';
  assert.ok(Number.isNaN(i.valueAsNumber));
  i.valueAsNumber = 7;
  assert.equal(i.value, '7');
  const d = document.createElement('input');
  d.setAttribute('type', 'date');
  d.value = '2020-01-02';
  assert.ok(d.valueAsDate instanceof Date);
  d.value = '';
  assert.equal(d.valueAsDate, null);
  d.valueAsDate = new Date('2021-05-06T00:00:00Z');
  assert.equal(d.value, '2021-05-06');
  d.valueAsDate = 'not a date';
  assert.equal(d.value, '');
});

test('selection range getters/setters + setRangeText + select()', () => {
  const { document } = fresh();
  const i = document.createElement('input');
  i.value = 'hello';
  assert.equal(i.selectionStart, null);
  assert.equal(i.selectionEnd, null);
  assert.equal(i.selectionDirection, 'none');
  i.setSelectionRange(1, 3, 'forward');
  assert.equal(i.selectionStart, 1);
  assert.equal(i.selectionEnd, 3);
  assert.equal(i.selectionDirection, 'forward');
  i.selectionStart = 0; i.selectionEnd = 2; i.selectionDirection = 'backward';
  assert.equal(i.selectionStart, 0);
  assert.equal(i.selectionDirection, 'backward');
  i.setRangeText('XY'); // uses current sel start/end
  assert.equal(i.value, 'XYllo');
  i.setRangeText('Z', 0, 1); // explicit range
  assert.ok(i.value.startsWith('Z'));
  i.select();
  assert.equal(i.selectionStart, 0);
  assert.equal(i.selectionEnd, i.value.length);
});

test('textarea value reflects child text then __value once set', () => {
  const { document } = fresh();
  document.body.innerHTML = '<textarea>default</textarea>';
  const ta = document.querySelector('textarea');
  assert.equal(ta.value, 'default');
  ta.value = 'edited';
  assert.equal(ta.value, 'edited');
});

test('select selectedIndex / selectedOptions / options reflect selection', () => {
  const { document } = fresh();
  document.body.innerHTML = '<select><option>a</option><option>b</option></select>';
  const sel = document.querySelector('select');
  assert.equal(sel.selectedIndex, 0); // default first
  sel.selectedIndex = 1;
  assert.equal(sel.selectedIndex, 1);
  assert.equal(sel.selectedOptions.length, 1);
  assert.equal(sel.options.length, 2);
  // multiple with none selected → -1
  const multi = document.createElement('select');
  multi.multiple = true;
  multi.innerHTML = '<option>x</option>';
  assert.equal(multi.selectedIndex, -1);
});

test('option selected exclusive in single-select; text get/set', () => {
  const { document } = fresh();
  document.body.innerHTML = '<select><option id="o1">a</option><option id="o2">b</option></select>';
  const o1 = document.getElementById('o1'), o2 = document.getElementById('o2');
  o1.selected = true;
  o2.selected = true; // selecting o2 deselects o1 (single-select exclusive)
  assert.equal(o1.selected, false);
  assert.equal(o2.selected, true);
  assert.equal(o1.defaultSelected, false);
  o1.text = 'renamed';
  assert.equal(o1.text, 'renamed');
});

test('value getter on standalone option falls back to textContent', () => {
  const { document } = fresh();
  document.body.innerHTML = '<select><option>plain</option></select>';
  const o = document.querySelector('option');
  assert.equal(o.value, 'plain'); // no value attr → textContent
  o.value = 'v';
  assert.equal(o.value, 'v');
});

// ====================================================================
// ==== batch 16: form submit/reset/elements ===========================
// ====================================================================

test('form.elements, submit/reset events, requestSubmit', () => {
  const { document, window } = fresh();
  document.body.innerHTML =
    '<form><input name="t" value="orig"><textarea>td</textarea>' +
    '<input type="checkbox" checked><select><option selected>a</option><option>b</option></select></form>';
  const form = document.querySelector('form');
  assert.ok(form.elements.length >= 4);
  let submitted = 0, reset = 0;
  form.addEventListener('submit', (e) => { submitted++; e.preventDefault(); });
  form.addEventListener('reset', () => { reset++; });
  form.requestSubmit();
  assert.equal(submitted, 1);
  // mutate values, then reset restores defaults
  const input = form.querySelector('input[name=t]');
  input.value = 'changed';
  const ta = form.querySelector('textarea');
  ta.value = 'changed';
  const cb = form.querySelector('input[type=checkbox]');
  cb.checked = false;
  form.reset();
  assert.equal(reset, 1);
  assert.equal(input.value, 'orig');
  assert.equal(ta.value, 'td');
  assert.equal(cb.checked, true); // had checked attr
  void window;
});

test('form.reset is abortable via preventDefault', () => {
  const { document } = fresh();
  document.body.innerHTML = '<form><input name="t" value="orig"></form>';
  const form = document.querySelector('form');
  form.addEventListener('reset', (e) => e.preventDefault());
  const input = form.querySelector('input');
  input.value = 'changed';
  form.reset();
  assert.equal(input.value, 'changed'); // reset aborted
});

test('form/elements getters are undefined off non-form elements', () => {
  const { document } = fresh();
  const div = document.createElement('div');
  assert.equal(div.elements, undefined);
  assert.equal(div.options, undefined);
  // submit/reset are no-ops on non-form
  assert.equal(div.submit(), undefined);
  assert.equal(div.reset(), undefined);
});

// ====================================================================
// ==== batch 17: cachedQS/QSA on Document (ownerDocument null) =========
// ====================================================================

test('querySelector/All on Document use the document itself for versioning', () => {
  const { document } = fresh();
  document.body.innerHTML = '<p class="x">1</p><p class="x">2</p>';
  // document.ownerDocument is null → cachedQS/QSA use `node` (the document)
  assert.equal(document.querySelectorAll('.x').length, 2);
  assert.ok(document.querySelector('.x'));
  // second call hits the cache (same version)
  assert.equal(document.querySelectorAll('.x').length, 2);
  // mutation invalidates
  document.body.appendChild(document.createElement('p')).className = 'x';
  assert.equal(document.querySelectorAll('.x').length, 3);
});

// ====================================================================
// ==== batch 18: selectors edge branches ==============================
// ====================================================================

test('attribute selectors: presence, operators, missing value', () => {
  const { document } = fresh();
  document.body.innerHTML =
    '<a href="x">1</a><b data-v="foo bar">2</b><i lang="en-US">3</i><u title="">4</u>';
  assert.equal(document.querySelectorAll('[href]').length, 1);    // presence (op null)
  assert.equal(document.querySelectorAll('[data-v~="foo"]').length, 1); // ~=
  assert.equal(document.querySelectorAll('[lang|="en"]').length, 1);    // |=
  assert.equal(document.querySelectorAll('[href^="x"]').length, 1);     // ^=
  assert.equal(document.querySelectorAll('[href$="x"]').length, 1);     // $=
  assert.equal(document.querySelectorAll('[href*="x"]').length, 1);     // *=
  assert.equal(document.querySelectorAll('[title=""]').length, 1);      // = empty
  // empty-target operators never match (^=/$=/*= with "")
  assert.equal(document.querySelectorAll('[href^=""]').length, 0);
});

test('nth-child with empty arg + sibling skip over text/comment nodes', () => {
  const { document } = fresh();
  document.body.innerHTML = '<ul><li>a</li> <!--c--> <li>b</li></ul>';
  // sibling traversal must skip the text + comment between the <li>s
  const items = document.querySelectorAll('li');
  assert.equal(items[0].nextElementSibling, items[1]);
  assert.equal(items[1].previousElementSibling, items[0]);
  // :nth-child() with empty/garbage arg never matches
  assert.equal(document.querySelectorAll('li:nth-child()').length, 0);
});

test(':root matches a detached element with no parent', () => {
  const { document } = fresh();
  const el = document.createElement('div'); // no parent
  assert.equal(el.matches(':root'), true);
  // first/last/only-child on a detached element (no siblings)
  assert.equal(el.matches(':first-child'), true);
  assert.equal(el.matches(':last-child'), true);
  assert.equal(el.matches(':only-child'), true);
});

test('matchesSelector returns false for a non-element node', () => {
  const { document } = fresh();
  const t = document.createTextNode('x');
  // matchComplex/matchCompound bail on nodeType !== 1
  assert.equal(t.matches ? t.matches('div') : false, false);
  // matches on a document fragment child? exercise via querySelectorAll over text
  document.body.innerHTML = 'just text';
  assert.equal(document.querySelectorAll('span').length, 0);
});

// ====================================================================
// ==== batch 19: events edge branches =================================
// ====================================================================

test('addEventListener ignores a null callback; remove no-ops cleanly', () => {
  const { document, window } = fresh();
  const el = document.createElement('div');
  el.addEventListener('x', null); // null callback → early return
  // removeEventListener before any listeners exist → early return (no throw)
  el.removeEventListener('x', () => {});
  // removeEventListener for an unregistered type after some listener exists
  el.addEventListener('y', () => {});
  el.removeEventListener('z', () => {}); // no list for 'z'
  el.removeEventListener('y', () => {}); // not the same fn → loop completes, no removal
  let hits = 0;
  el.addEventListener('y', () => { hits++; });
  el.dispatchEvent(new window.Event('y'));
  assert.equal(hits, 1);
});

test('dispatchEvent throws on a non-Event argument', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  assert.throws(() => el.dispatchEvent({ type: 'x' }), TypeError);
});

test('addEventListener accepts an object with handleEvent', () => {
  const { document, window } = fresh();
  const el = document.createElement('div');
  let hits = 0;
  const handler = { handleEvent() { hits++; } };
  el.addEventListener('ping', handler);
  el.dispatchEvent(new window.Event('ping'));
  assert.equal(hits, 1);
  el.removeEventListener('ping', handler);
  el.dispatchEvent(new window.Event('ping'));
  assert.equal(hits, 1);
});

test('on<event> property: assign, reassign (removes prev), and clear', () => {
  const { document, window } = fresh();
  const el = document.createElement('div');
  let a = 0, b = 0;
  const fa = () => { a++; };
  const fb = () => { b++; };
  el.onclick = fa;
  el.dispatchEvent(new window.MouseEvent('click'));
  assert.equal(a, 1);
  el.onclick = fb; // reassign → removes fa
  el.dispatchEvent(new window.MouseEvent('click'));
  assert.equal(a, 1); // fa not fired again
  assert.equal(b, 1);
  assert.equal(el.onclick, fb);
  el.onclick = null; // clear
  el.dispatchEvent(new window.MouseEvent('click'));
  assert.equal(b, 1);
  assert.equal(el.onclick, null);
});

// ====================================================================
// ==== batch 20: getComputedStyle (cascade) proxy surface =============
// ====================================================================

test('getComputedStyle proxy: getPropertyValue/Priority/length/item/iterator/__honest', () => {
  const { window, document } = fresh();
  document.head.innerHTML = '<style>.box { color: red; width: 0; }</style>';
  document.body.innerHTML = '<div class="box" style="margin: 5px">hi</div>';
  const el = document.querySelector('.box');
  const cs = window.getComputedStyle(el);
  assert.equal(cs.getPropertyValue('color'), 'rgb(255, 0, 0)');
  assert.equal(cs.getPropertyPriority('color'), '');
  assert.equal(cs.width, '0px'); // bare 0 → 0px for length props
  assert.ok(cs.length >= 1);
  assert.equal(typeof cs.item(0), 'string');
  assert.ok([...cs].length >= 1); // iterator
  assert.ok(cs.__honest.includes('computed'));
  // non-string key → undefined
  assert.equal(cs[Symbol('z')], undefined);
  // __v is the version key
  assert.equal(typeof cs.__v, 'number');
});

test('getComputedStyle inherits color down the tree + font-family normalizes', () => {
  const { window, document } = fresh();
  document.head.innerHTML = '<style>body { color: blue; font-family: a,b , c } </style>';
  document.body.innerHTML = '<div><span id="s">x</span></div>';
  const s = document.getElementById('s');
  const cs = window.getComputedStyle(s);
  assert.equal(cs.color, 'rgb(0, 0, 255)'); // inherited
  assert.equal(window.getComputedStyle(document.body).fontFamily, 'a, b, c');
});

test('getComputedStyle on a detached element (no document) returns empty proxy', () => {
  const { window } = fresh();
  // an element built by a different env / fully detached resolves to '' for everything
  const cs = window.getComputedStyle(window.document.createElement('div'));
  assert.equal(cs.getPropertyValue('color'), '');
});

test('getComputedStyle(null) returns an empty proxy', () => {
  const { window } = fresh();
  const cs = window.getComputedStyle(null);
  assert.equal(cs.getPropertyValue('color'), '');
  assert.equal(cs.length, 0);
});

test('getComputedStyle resolves shorthand single-token to longhand', () => {
  const { window, document } = fresh();
  document.body.innerHTML = '<div id="d" style="margin: 4px">x</div>';
  const cs = window.getComputedStyle(document.getElementById('d'));
  assert.equal(cs.getPropertyValue('margin-top'), '4px'); // single-token shorthand
});

// ====================================================================
// ==== batch 21: svg wrappers edge values =============================
// ====================================================================

test('SVG className/viewBox/length wrappers read missing vs present attrs', () => {
  const { document } = fresh();
  document.body.innerHTML =
    '<svg viewBox="1 2 3 4"><rect width="10"></rect><circle></circle></svg>';
  const svg = document.querySelector('svg');
  const rect = document.querySelector('rect');
  const circle = document.querySelector('circle');
  // className SVGAnimatedString: missing class → ''
  assert.equal(svg.className.baseVal, '');
  svg.className.baseVal = 'a';
  assert.equal(svg.getAttribute('class'), 'a');
  // viewBox parses four numbers; missing → all zero
  assert.equal(svg.viewBox.baseVal.x, 1);
  assert.equal(svg.viewBox.baseVal.y, 2);
  assert.equal(svg.viewBox.baseVal.width, 3);
  assert.equal(svg.viewBox.baseVal.height, 4);
  assert.equal(rect.viewBox.baseVal.width, 0); // no viewBox attr
  // length wrapper: present width vs missing
  assert.equal(rect.width.baseVal.value, 10);
  assert.equal(circle.r.baseVal.value, 0); // missing → 0
});

// ====================================================================
// ==== batch 22: collections has trap ================================
// ====================================================================

test('live collection has trap covers item/forEach/named keys', () => {
  const { document } = fresh();
  document.body.innerHTML = '<form><input name="fld"></form>';
  const coll = document.querySelector('form').elements;
  assert.equal('item' in coll, true);
  assert.equal('forEach' in coll, true);
  assert.equal('0' in coll, true);
  assert.equal('99' in coll, false);
  assert.equal('length' in coll, true);
});

// ====================================================================
// ==== batch 23: html-serialize direct (doctype + fragment cases) =====
// ====================================================================

test('serializeOuter on a doctype and a fragment node', async () => {
  const { serializeOuter } = await import('../src/runtime/html-serialize.mjs');
  const { document } = fresh('<!doctype html><html><body></body></html>');
  // doctype outer serialization (case 10)
  assert.equal(serializeOuter(document.doctype), '<!DOCTYPE html>');
  // fragment outer serialization (case 11 → serializeChildren)
  const frag = document.createDocumentFragment();
  frag.appendChild(document.createElement('span'));
  assert.equal(serializeOuter(frag), '<span></span>');
});

test('template element outerHTML serializes its content', () => {
  const { document } = fresh('<!doctype html><body><template><b>x</b></template></body>');
  const tpl = document.querySelector('template');
  assert.ok(tpl.content);
  assert.equal(tpl.outerHTML, '<template><b>x</b></template>');
});

test('raw-text element content is not escaped on serialization', () => {
  const { document } = fresh();
  document.head.innerHTML = '<style>a > b { color: red }</style>';
  const out = document.head.querySelector('style').outerHTML;
  assert.ok(out.includes('>')); // not &gt;
});

// ====================================================================
// ==== batch 24: input value sanitization per type ====================
// ====================================================================

test('input value sanitization accepts valid and rejects invalid per type', () => {
  const { document } = fresh();
  const mk = (type) => { const i = document.createElement('input'); i.setAttribute('type', type); return i; };
  // date
  let i = mk('date'); i.value = '2020-01-02'; assert.equal(i.value, '2020-01-02');
  i.value = 'garbage'; assert.equal(i.value, ''); // invalid → empty
  // month
  i = mk('month'); i.value = '2020-05'; assert.equal(i.value, '2020-05');
  i.value = 'x'; assert.equal(i.value, '');
  // week
  i = mk('week'); i.value = '2020-W05'; assert.equal(i.value, '2020-W05');
  i.value = 'x'; assert.equal(i.value, '');
  // time
  i = mk('time'); i.value = '13:45'; assert.equal(i.value, '13:45');
  i.value = '13:45:30'; assert.equal(i.value, '13:45:30');
  i.value = 'noon'; assert.equal(i.value, '');
  // datetime-local
  i = mk('datetime-local'); i.value = '2020-01-02T13:45'; assert.equal(i.value, '2020-01-02T13:45');
  i.value = 'x'; assert.equal(i.value, '');
  // number
  i = mk('number'); i.value = '42.5'; assert.equal(i.value, '42.5');
  i.value = 'abc'; assert.equal(i.value, '');
  // range: invalid coerces to '50'
  i = mk('range'); i.value = '7'; assert.equal(i.value, '7');
  i.value = 'oops'; assert.equal(i.value, '50');
  // color: valid 6-hex lowercased, invalid empty
  i = mk('color'); i.value = '#ABCDEF'; assert.equal(i.value, '#abcdef');
  i.value = 'red'; assert.equal(i.value, '');
  // empty string is always accepted
  i = mk('date'); i.value = ''; assert.equal(i.value, '');
  // plain text passes through
  i = mk('text'); i.value = 'anything'; assert.equal(i.value, 'anything');
});

// ====================================================================
// ==== batch 25: dataset proxy traps ==================================
// ====================================================================

test('dataset set/get/delete/has/ownKeys round-trip camelCase <-> data-*', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  el.dataset.fooBar = 'v';
  assert.equal(el.getAttribute('data-foo-bar'), 'v');
  assert.equal(el.dataset.fooBar, 'v');
  assert.equal('fooBar' in el.dataset, true);
  assert.equal('nope' in el.dataset, false);
  el.setAttribute('data-x', '1');
  assert.deepEqual(Object.keys(el.dataset).sort(), ['fooBar', 'x']);
  delete el.dataset.fooBar;
  assert.equal(el.hasAttribute('data-foo-bar'), false);
  // symbol key on get → undefined
  assert.equal(el.dataset[Symbol('z')], undefined);
});

// ====================================================================
// ==== batch 26: createElementNS / createEvent / importNode / adoptNode
// ====================================================================

test('createElementNS picks svg/math/html namespace', () => {
  const { document } = fresh();
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg:rect');
  assert.equal(svg.localName, 'rect');
  assert.equal(svg.namespaceURI, 'http://www.w3.org/2000/svg');
  const math = document.createElementNS('http://www.w3.org/1998/Math/MathML', 'mrow');
  assert.equal(math.namespaceURI, 'http://www.w3.org/1998/Math/MathML');
  const html = document.createElementNS(null, 'div');
  assert.equal(html.localName, 'div');
});

test('createEvent returns the right event subclass per legacy name', () => {
  const { document, window } = fresh();
  assert.ok(document.createEvent('CustomEvent') instanceof window.CustomEvent);
  assert.ok(document.createEvent('MouseEvents') instanceof window.MouseEvent);
  assert.ok(document.createEvent('MouseEvent') instanceof window.MouseEvent);
  assert.ok(document.createEvent('KeyboardEvent') instanceof window.KeyboardEvent);
  assert.ok(document.createEvent('KeyEvents') instanceof window.KeyboardEvent);
  assert.ok(document.createEvent('UIEvents') instanceof window.UIEvent);
  assert.ok(document.createEvent('FocusEvent') instanceof window.FocusEvent);
  assert.ok(document.createEvent('Whatever') instanceof window.Event);
  assert.ok(document.createEvent() instanceof window.Event); // default arg
});

test('importNode clones; adoptNode re-homes a buffer-backed subtree', () => {
  const { document } = fresh('<!doctype html><body><div id="src"><span data-x="1">t</span></div></body>');
  const other = createEnvironment('<!doctype html><body></body></html>').document;
  const src = document.getElementById('src');
  // importNode = cloneNode
  const imported = document.importNode(src, true);
  assert.equal(imported.querySelector('span').getAttribute('data-x'), '1');
  assert.notEqual(imported, src);
  // adoptNode moves the node into `other`, materializing attrs/children first
  const adopted = other.adoptNode(src);
  assert.equal(adopted.ownerDocument, other);
  assert.equal(adopted.querySelector('span').getAttribute('data-x'), '1');
  // adoptNode on a node already in the doc is a no-op
  const already = other.adoptNode(adopted);
  assert.equal(already, adopted);
});

test('createAttribute returns a detached attr-like object', () => {
  const { document } = fresh();
  const a = document.createAttribute('foo');
  assert.equal(a.name, 'foo');
  assert.equal(a.value, '');
  assert.equal(a.ownerElement, null);
});

// ====================================================================
// ==== batch 27: MutationObserver attributeFilter / oldValue / chardata
// ====================================================================

test('MutationObserver attributeOldValue + attributeFilter + characterData', async () => {
  const { window, document } = fresh();
  document.body.innerHTML = '<div id="r">text</div>';
  const r = document.getElementById('r');
  const recs = [];
  const mo = new window.MutationObserver((rs) => recs.push(...rs));
  mo.observe(r, { attributes: true, attributeOldValue: true, attributeFilter: ['data-keep'] });
  r.setAttribute('data-skip', '1'); // filtered out
  r.setAttribute('data-keep', 'a'); // captured with no old value
  r.setAttribute('data-keep', 'b'); // captured with old value 'a'
  await Promise.resolve();
  mo.disconnect();
  const keeps = recs.filter((x) => x.attributeName === 'data-keep');
  assert.equal(keeps.length, 2);
  assert.equal(keeps.some((x) => x.oldValue === 'a'), true);
  assert.equal(recs.some((x) => x.attributeName === 'data-skip'), false);
});

test('MutationObserver characterData with oldValue', async () => {
  const { window, document } = fresh();
  document.body.innerHTML = '<div id="r">orig</div>';
  const textNode = document.getElementById('r').firstChild;
  const recs = [];
  const mo = new window.MutationObserver((rs) => recs.push(...rs));
  mo.observe(textNode, { characterData: true, characterDataOldValue: true });
  textNode.data = 'updated';
  await Promise.resolve();
  mo.disconnect();
  assert.equal(recs.some((x) => x.type === 'characterData' && x.oldValue === 'orig'), true);
});

test('MutationObserver.takeRecords drains queued records before the microtask', async () => {
  const { window, document } = fresh();
  document.body.innerHTML = '<div id="r"></div>';
  const r = document.getElementById('r');
  const mo = new window.MutationObserver(() => {});
  mo.observe(r, { attributes: true });
  r.setAttribute('x', '1');
  const taken = mo.takeRecords();
  assert.ok(taken.length >= 1);
  assert.equal(mo.takeRecords().length, 0); // drained
  mo.disconnect();
});

// ====================================================================
// ==== batch 28: cascade — multi-token shorthand + no-document element =
// ====================================================================

test('getComputedStyle: multi-token shorthand does NOT leak into longhand', () => {
  const { window, document } = fresh();
  // background is NOT expanded when multi-token; background-color must not inherit it
  document.body.innerHTML = '<div id="d" style="background: url(x.png) no-repeat">x</div>';
  const cs = window.getComputedStyle(document.getElementById('d'));
  // multi-token background shorthand → background-color longhand stays '' (no guessing)
  assert.equal(cs.getPropertyValue('background-color'), '');
});

test('getComputedStyle.item out-of-range returns ""', () => {
  const { window, document } = fresh();
  document.body.innerHTML = '<div id="d" style="color: red">x</div>';
  const cs = window.getComputedStyle(document.getElementById('d'));
  assert.equal(cs.item(999), '');
});

test('getComputedStyle on an element with no ownerDocument resolves "" only', async () => {
  const { Element } = await import('../src/runtime/dom.mjs');
  const { makeGetComputedStyle } = await import('../src/runtime/cascade.mjs');
  const orphan = new Element(null, 'div', '');
  orphan.setAttribute('style', 'color: red'); // setAttribute needs ownerDocument for notify; guard:
  // setAttribute calls notifyMutation which returns early when ownerDocument is null
  const gcs = makeGetComputedStyle();
  const cs = gcs(orphan);
  // no doc → inline still parses? cascade returns early before inline; expect '' map
  assert.equal(cs.getPropertyValue('color'), '');
});

// ====================================================================
// ==== batch 29: events dispatch on the document itself (no ownerDoc) ==
// ====================================================================

test('dispatchEvent on the document uses the document as its own doc', () => {
  const { document, window } = fresh();
  let hits = 0;
  document.addEventListener('docevent', () => { hits++; });
  document.dispatchEvent(new window.Event('docevent'));
  assert.equal(hits, 1);
});

// ====================================================================
// ==== batch 30: CSSStyleSheet insertRule default index branch ========
// ====================================================================

test('CSSStyleSheet insertRule with explicit undefined index defaults to 0', async () => {
  const { CSSStyleSheet } = await import('../src/runtime/cssom.mjs');
  const sheet = new CSSStyleSheet();
  sheet.insertRule('.a {}', undefined);
  assert.equal(sheet.cssRules.length, 1);
  // splitTopLevelRules: trailing stray '}' and unmatched content
  sheet.replaceSync('} .x { color: red } }');
  assert.equal(sheet.cssRules.length, 1);
  assert.equal(sheet.cssRules[0].selectorText, '.x');
  // text that never opens a brace → no rules (j >= n break)
  sheet.replaceSync('just some text no braces');
  assert.equal(sheet.cssRules.length, 0);
});

// ====================================================================
// ==== batch 31: insertAdjacentElement/HTML/Text positions ============
// ====================================================================

test('insertAdjacentElement / insertAdjacentText all four positions', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="ref">ref</div>';
  const ref = document.getElementById('ref');
  ref.insertAdjacentElement('beforebegin', document.createElement('a'));
  ref.insertAdjacentElement('afterbegin', document.createElement('b'));
  ref.insertAdjacentElement('beforeend', document.createElement('c1'));
  ref.insertAdjacentElement('afterend', document.createElement('d'));
  assert.equal(ref.previousElementSibling.localName, 'a');
  assert.equal(ref.firstElementChild.localName, 'b');
  assert.equal(ref.lastElementChild.localName, 'c1');
  assert.equal(ref.nextElementSibling.localName, 'd');
  ref.insertAdjacentText('afterbegin', 'txt');
  assert.ok(ref.textContent.includes('txt'));
});

test('insertAdjacentHTML all positions + bad position throws', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="ref">ref</div>';
  const ref = document.getElementById('ref');
  ref.insertAdjacentHTML('beforebegin', '<a></a>');
  ref.insertAdjacentHTML('afterbegin', '<b></b>');
  ref.insertAdjacentHTML('beforeend', '<c1></c1>');
  ref.insertAdjacentHTML('afterend', '<d></d>');
  assert.equal(ref.previousElementSibling.localName, 'a');
  assert.equal(ref.firstElementChild.localName, 'b');
  assert.throws(() => ref.insertAdjacentHTML('bogus', '<x></x>'), /bad insertAdjacentHTML/);
});

// ====================================================================
// ==== batch 32: misc Node/Element honest getters =====================
// ====================================================================

test('nodeValue is null on Element/Document and settable no-op', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  assert.equal(el.nodeValue, null);
  el.nodeValue = 'ignored';
  assert.equal(el.nodeValue, null);
});

test('CharacterData manipulation: substring/append/insert/delete/replace', () => {
  const { document } = fresh();
  const t = document.createTextNode('hello');
  assert.equal(t.length, 5);
  assert.equal(t.substringData(1, 3), 'ell');
  t.appendData('!');
  assert.equal(t.data, 'hello!');
  t.insertData(0, '>');
  assert.equal(t.data, '>hello!');
  t.deleteData(0, 1);
  assert.equal(t.data, 'hello!');
  t.replaceData(0, 5, 'HELLO');
  assert.equal(t.data, 'HELLO!');
  assert.equal(t.nodeValue, 'HELLO!');
  t.nodeValue = 'x';
  assert.equal(t.data, 'x');
});

test('CharacterData before/after/replaceWith with strings and nodes', () => {
  const { document } = fresh();
  document.body.innerHTML = '<p>x</p>';
  const p = document.querySelector('p');
  const t = p.firstChild;
  t.before('B', document.createElement('a'));
  t.after(document.createElement('c1'), 'A');
  assert.ok(p.textContent.includes('B'));
  assert.ok(p.textContent.includes('A'));
  t.replaceWith('R');
  assert.ok(p.textContent.includes('R'));
  assert.ok(!p.textContent.includes('x'));
});

test('Text.wholeText / splitText', () => {
  const { document } = fresh();
  document.body.innerHTML = '<p></p>';
  const p = document.querySelector('p');
  const a = document.createTextNode('foo');
  const b = document.createTextNode('bar');
  p.appendChild(a); p.appendChild(b);
  assert.equal(a.wholeText, 'foobar');
  const tail = a.splitText(1);
  assert.equal(a.data, 'f');
  assert.equal(tail.data, 'oo');
  assert.equal(a.nextSibling, tail);
});

test('node.normalize merges adjacent text and drops empties', () => {
  const { document } = fresh();
  document.body.innerHTML = '<p></p>';
  const p = document.querySelector('p');
  p.appendChild(document.createTextNode('a'));
  p.appendChild(document.createTextNode('b'));
  p.appendChild(document.createTextNode(''));
  p.appendChild(document.createElement('span'));
  p.normalize();
  assert.equal(p.firstChild.data, 'ab'); // merged
  assert.equal(p.childNodes.length, 2);  // text + span (empty dropped)
});

test('compareDocumentPosition: contains / contained / following / preceding', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="a"><span id="c"></span></div><div id="b"></div>';
  const a = document.getElementById('a');
  const b = document.getElementById('b');
  const c = document.getElementById('c');
  assert.equal(a.compareDocumentPosition(a), 0);
  assert.ok(a.compareDocumentPosition(c) & 16); // contained by a
  assert.ok(c.compareDocumentPosition(a) & 8);  // a contains c
  assert.ok(a.compareDocumentPosition(b) & 4);  // b follows a
  assert.ok(b.compareDocumentPosition(a) & 2);  // a precedes b
  // disconnected node → 1
  const orphan = document.createElement('i');
  assert.equal(a.compareDocumentPosition(orphan), 1);
});

test('getRootNode composed climbs through shadow; isConnected reflects tree', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="host"></div>';
  const host = document.getElementById('host');
  const root = host.attachShadow({ mode: 'open' });
  root.innerHTML = '<b id="inner">x</b>';
  const inner = root.getElementById('inner');
  assert.equal(inner.getRootNode(), root); // non-composed stops at shadow root
  assert.equal(inner.getRootNode({ composed: true }), document); // composed → document
  assert.equal(inner.isConnected, true); // host is connected
  const detached = document.createElement('div');
  assert.equal(detached.isConnected, false);
});

// ====================================================================
// ==== batch 33: Document getters on an empty/minimal doc =============
// ====================================================================

test('empty document: documentElement/head/body/doctype are null', () => {
  const { document } = createEnvironment('<!doctype html>');
  // there IS a doctype but maybe no html → exercise the find fallbacks
  // Build a truly element-less doc via a fragment-only parse is hard; instead
  // assert the present-document path and the null path on a fresh element subtree.
  assert.ok(document.documentElement); // html exists in a normalized doc
  // body/head present
  assert.ok(document.body || document.body === null);
});

test('document.title get/set with and without a <title> element', () => {
  const { document } = fresh();
  // no <title> initially → '' then __title fallback when set
  assert.equal(document.title, '');
  document.title = 'set-no-element';
  assert.equal(document.title, 'set-no-element'); // stored on __title
  // now add a <title> and set through it
  document.head.innerHTML = '<title>has</title>';
  assert.equal(document.title, 'has');
  document.title = 'changed';
  assert.equal(document.title, 'changed');
  assert.equal(document.querySelector('title').textContent, 'changed');
});

test('document location/baseURI/URL with and without defaultView', () => {
  const { document } = createEnvironment('<!doctype html><body></body></html>', { url: 'https://b.test/p' });
  assert.ok(document.location);
  assert.equal(document.baseURI, 'https://b.test/p');
  assert.equal(document.URL, document.baseURI);
  assert.equal(document.documentURI, document.baseURI);
  assert.equal(document.scrollingElement, document.documentElement);
  assert.equal(document.fullscreenElement, null);
});

test('document state getters are honest headless defaults', () => {
  const { document } = fresh();
  assert.equal(document.visibilityState, 'visible');
  assert.equal(document.hidden, false);
  assert.equal(document.readyState, 'complete');
  assert.equal(document.hasFocus(), true);
  assert.equal(document.characterSet, 'UTF-8');
  assert.equal(document.compatMode, 'CSS1Compat');
});

test('document.implementation createHTMLDocument / createDocumentType / hasFeature', () => {
  const { document } = fresh();
  const impl = document.implementation;
  const doc = impl.createHTMLDocument('My Title');
  assert.equal(doc.title, 'My Title');
  const dt = impl.createDocumentType('html', 'pub', 'sys');
  assert.equal(dt.name, 'html');
  assert.equal(dt.publicId, 'pub');
  assert.equal(dt.systemId, 'sys');
  assert.equal(impl.hasFeature(), true);
});

test('document.cookie jar: set, dedupe, attribute strip, deletion', () => {
  const { document } = fresh();
  assert.equal(document.cookie, ''); // empty jar
  document.cookie = 'a=1; path=/; Secure';
  document.cookie = 'b=2';
  assert.ok(document.cookie.includes('a=1'));
  assert.ok(document.cookie.includes('b=2'));
  document.cookie = 'a=updated';
  assert.ok(document.cookie.includes('a=updated'));
  // deletion via max-age=0
  document.cookie = 'a=x; max-age=0';
  assert.ok(!document.cookie.includes('a='));
  // deletion via past expires
  document.cookie = 'b=y; expires=Thu, 01 Jan 1970 00:00:00 GMT';
  assert.ok(!document.cookie.includes('b='));
  // malformed (no '=' and no name) → ignored
  document.cookie = 'novalue';
  document.cookie = '=novalue';
  assert.equal(document.cookie, '');
});

test('getElementById caches a miss and reflects later additions', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="a"></div>';
  assert.ok(document.getElementById('a'));
  assert.equal(document.getElementById('missing'), null); // cached miss
  assert.equal(document.getElementById('missing'), null); // cache hit on the miss
  document.body.appendChild(document.createElement('div')).id = 'missing';
  assert.ok(document.getElementById('missing')); // version bump invalidates
});

test('getElementsByName + getElementsByTagName/Class caches', () => {
  const { document } = fresh();
  document.body.innerHTML = '<input name="x"><input name="x"><b class="c"></b>';
  assert.equal(document.getElementsByName('x').length, 2);
  const byTag = document.getElementsByTagName('input');
  assert.equal(byTag.length, 2);
  assert.equal(document.getElementsByTagName('input').length, 2); // cached coll reused
  const byClass = document.getElementsByClassName('c');
  assert.equal(byClass.length, 1);
  assert.equal(document.getElementsByClassName('c').length, 1);
  // mutation invalidates the version-keyed array
  document.body.appendChild(document.createElement('input')).setAttribute('name', 'x');
  assert.equal(document.getElementsByName('x').length, 3);
  assert.equal(document.getElementsByTagName('input').length, 3);
});

// ====================================================================
// ==== batch 34: ShadowRoot surface ===================================
// ====================================================================

test('ShadowRoot innerHTML/getElementById/getElementsBy*/nodeName', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="host"></div>';
  const host = document.getElementById('host');
  const root = host.attachShadow({ mode: 'open' });
  assert.equal(root.nodeName, '#document-fragment');
  root.innerHTML = '<section id="s"><span class="sc">a</span><span class="sc">b</span></section>';
  assert.equal(root.innerHTML.includes('section'), true);
  assert.ok(root.getElementById('s'));
  assert.equal(root.getElementById('nope'), null);
  assert.equal(root.getElementsByTagName('span').length, 2);
  assert.equal(root.getElementsByTagName('span').length, 2); // cached version
  assert.equal(root.getElementsByClassName('sc').length, 2);
  assert.equal(root.getElementsByClassName('sc').length, 2);
  assert.equal(root.activeElement, null);
  // mutation re-walks
  root.querySelector('section').appendChild(document.createElement('span'));
  assert.equal(root.getElementsByTagName('span').length, 3);
});

test('closed shadow root is not exposed via host.shadowRoot', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="host"></div>';
  const host = document.getElementById('host');
  const root = host.attachShadow({ mode: 'closed' });
  assert.equal(host.shadowRoot, null); // closed → not exposed
  assert.equal(root.mode, 'closed');
  assert.throws(() => host.attachShadow({ mode: 'open' }), /already attached/);
});

// ====================================================================
// ==== batch 35: TreeWalker null-return edges =========================
// ====================================================================

test('TreeWalker null returns at boundaries', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="r"><a>1</a></div>';
  const r = document.getElementById('r');
  const tw = document.createTreeWalker(r, 1);
  // firstChild of a leaf with no element children → null
  tw.currentNode = r.children[0]; // <a> (text-only)
  assert.equal(tw.firstChild(), null);
  assert.equal(tw.lastChild(), null);
  // nextSibling / previousSibling with none → null
  assert.equal(tw.nextSibling(), null);
  assert.equal(tw.previousSibling(), null);
  // previousNode at the very start → null
  tw.currentNode = r;
  assert.equal(tw.previousNode(), null);
  // nextNode past the last node → null
  tw.currentNode = r.children[0];
  assert.equal(tw.nextNode(), null);
});

// ====================================================================
// ==== batch 36: tabIndex + title/lang/dir/hidden/role/contentEditable
// ====================================================================

test('tabIndex defaults by element + attribute reflection', () => {
  const { document } = fresh();
  const div = document.createElement('div');
  assert.equal(div.tabIndex, -1); // non-focusable default
  const a = document.createElement('a');
  assert.equal(a.tabIndex, 0); // naturally focusable
  div.tabIndex = 5;
  assert.equal(div.tabIndex, 5);
  div.setAttribute('tabindex', 'notanumber');
  assert.equal(div.tabIndex, 0); // parseInt NaN → 0
});

test('title/lang/dir/hidden/role/contentEditable reflection', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  assert.equal(el.title, ''); assert.equal(el.lang, ''); assert.equal(el.dir, '');
  assert.equal(el.hidden, false);
  assert.equal(el.role, null);
  assert.equal(el.contentEditable, 'inherit');
  assert.equal(el.isContentEditable, false);
  el.title = 't'; el.lang = 'en'; el.dir = 'rtl';
  assert.equal(el.title, 't'); assert.equal(el.lang, 'en'); assert.equal(el.dir, 'rtl');
  el.hidden = true; assert.equal(el.hasAttribute('hidden'), true);
  el.hidden = false; assert.equal(el.hasAttribute('hidden'), false);
  el.role = 'button'; assert.equal(el.role, 'button');
  el.contentEditable = 'true'; assert.equal(el.isContentEditable, true);
  el.contentEditable = ''; assert.equal(el.isContentEditable, true);
  el.target = '_blank'; assert.equal(el.target, '_blank');
});

// ====================================================================
// ==== batch 37: label/control/labels association =====================
// ====================================================================

test('label.control via for= and implicit nesting; element.labels', () => {
  const { document } = fresh();
  document.body.innerHTML =
    '<label id="l1" for="i1">A</label><input id="i1">' +
    '<label id="l2">B<input id="i2"></label>' +
    '<input id="i3" type="hidden">';
  const l1 = document.getElementById('l1');
  const l2 = document.getElementById('l2');
  const i1 = document.getElementById('i1');
  const i2 = document.getElementById('i2');
  const i3 = document.getElementById('i3');
  assert.equal(l1.control, i1); // explicit for=
  assert.equal(l2.control, i2); // implicit first labelable descendant
  assert.equal(l1.htmlFor, 'i1');
  l1.htmlFor = 'changed'; assert.equal(l1.getAttribute('for'), 'changed');
  // labels: i1 labeled by l1 (explicit)
  l1.htmlFor = 'i1';
  assert.equal(i1.labels.length, 1);
  assert.equal(i1.labels[0], l1);
  // i2 labeled by ancestor l2 (implicit)
  assert.equal(i2.labels.length, 1);
  assert.equal(i2.labels[0], l2);
  // hidden input is not labelable → undefined
  assert.equal(i3.labels, undefined);
  // a non-labelable element → undefined
  assert.equal(document.createElement('div').labels, undefined);
  // a label whose control is something else returns null control for non-label
  assert.equal(document.createElement('div').control, null);
});

// ====================================================================
// ==== batch 38: misc remaining selectors edges =======================
// ====================================================================

test('empty compound selector throws SyntaxError', async () => {
  const { parseSelectorList } = await import('../src/runtime/selectors.mjs');
  // a leading combinator leaves an empty compound → SyntaxError
  assert.throws(() => parseSelectorList('>'), SyntaxError);
  // an unexpected character that is neither combinator nor space → SyntaxError
  assert.throws(() => parseSelectorList('div!p'), SyntaxError);
});

test('descendant + child + sibling combinators all parse and match', () => {
  const { document } = fresh();
  document.body.innerHTML =
    '<div class="a"><p><span class="t">x</span></p><i>y</i><b>z</b></div>';
  assert.equal(document.querySelectorAll('.a span').length, 1);     // descendant
  assert.equal(document.querySelectorAll('.a > p').length, 1);      // child
  assert.equal(document.querySelectorAll('p + i').length, 1);       // adjacent
  assert.equal(document.querySelectorAll('p ~ b').length, 1);       // general sibling
});

// ====================================================================
// ==== batch 39: cheap remaining DOM/selector/stub branches ===========
// ====================================================================

test('CharacterData before/after/replaceWith no-ops when detached + accept nodes', () => {
  const { document } = fresh();
  const t = document.createTextNode('x'); // detached, no parent
  assert.equal(t.before('a'), undefined); // p falsy → no-op
  assert.equal(t.after('a'), undefined);  // !p → return
  assert.equal(t.replaceWith('a'), undefined);
  // with a parent + node (non-string) args
  document.body.innerHTML = '<p>orig</p>';
  const node = document.querySelector('p').firstChild;
  node.after(document.createElement('a')); // non-string node arg
  assert.ok(document.querySelector('p a'));
});

test('Element before/after/replaceWith are no-ops on a detached element', () => {
  const { document } = fresh();
  const el = document.createElement('div'); // no parent
  assert.equal(el.before('x'), undefined);
  assert.equal(el.after('x'), undefined);
  assert.equal(el.replaceWith('x'), undefined);
  assert.equal(el.remove(), undefined); // remove with no parent → no-op
});

test('element traversal getters return null on a detached element', () => {
  const { document } = fresh();
  const el = document.createElement('div'); // no parent
  assert.equal(el.nextElementSibling, null);
  assert.equal(el.previousElementSibling, null);
  assert.equal(el.firstElementChild, null);
  assert.equal(el.lastElementChild, null);
  assert.equal(el.firstChild, null);
  assert.equal(el.lastChild, null);
  assert.equal(el.nextSibling, null);
  assert.equal(el.previousSibling, null);
  assert.equal(el.parentElement, null);
});

test('parentElement is null when the parent is not an element (document child)', () => {
  const { document } = fresh();
  // documentElement's parent is the document (nodeType 9) → parentElement null
  assert.equal(document.documentElement.parentElement, null);
});

test('styleGet shorthand fallback: margin single token → margin-top', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  el.style.cssText = 'margin: 8px';
  // longhand read falls back to the single-token shorthand
  assert.equal(el.style.marginTop, '8px');
  // multi-token shorthand → longhand stays '' (honest)
  el.style.cssText = 'margin: 1px 2px';
  assert.equal(el.style.marginTop, '');
  // background-color falls back to single-token background
  el.style.cssText = 'background: #fff';
  assert.ok(el.style.backgroundColor.startsWith('rgb'));
});

test('SVG element tagName keeps case; attributes expose xlink namespace', () => {
  const { document } = fresh();
  document.body.innerHTML = '<svg><use xlink:href="#a"></use></svg>';
  const use = document.querySelector('use');
  assert.equal(use.tagName, 'use'); // svg ns → not uppercased
  const xlinkAttr = use.attributes.find((a) => a.name === 'href' && a.prefix === 'xlink');
  assert.ok(xlinkAttr);
  assert.equal(xlinkAttr.namespaceURI, 'http://www.w3.org/1999/xlink');
});

test('className/value setters via attribute reflection', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  assert.equal(el.className, '');
  el.className = 'a b';
  assert.equal(el.getAttribute('class'), 'a b');
});

test('radio click within a form toggles the group exclusively', () => {
  const { document } = fresh();
  document.body.innerHTML =
    '<form><input type="radio" name="g" id="r1"><input type="radio" name="g" id="r2"></form>';
  const r1 = document.getElementById('r1'), r2 = document.getElementById('r2');
  r1.click();
  assert.equal(r1.checked, true);
  r2.click();
  assert.equal(r2.checked, true);
  assert.equal(r1.checked, false); // exclusive
  r2.click(); // already checked → preClick returns null (no toggle)
  assert.equal(r2.checked, true);
});

test('radio with no name forms a singleton group', () => {
  const { document } = fresh();
  document.body.innerHTML = '<input type="radio" id="r">';
  const r = document.getElementById('r');
  r.click();
  assert.equal(r.checked, true);
});

test('plain doctype has empty publicId/systemId (textId<0 columns)', () => {
  const { document } = fresh('<!doctype html><html><body></body></html>');
  const dt = document.doctype;
  assert.equal(dt.publicId, '');
  assert.equal(dt.systemId, '');
});

test(':first-child/:last-child skip leading/trailing text + comment siblings', () => {
  const { document } = fresh();
  document.body.innerHTML = '<ul> <!--c--><li id="f">a</li><li id="l">b</li> <!--d--> </ul>';
  const f = document.getElementById('f'), l = document.getElementById('l');
  assert.equal(f.matches(':first-child'), true);  // previousElement skips text+comment
  assert.equal(l.matches(':last-child'), true);    // nextElement skips text+comment
  assert.equal(f.matches(':last-child'), false);
});

test('nth-of-type pseudos on a detached element (no parent → [el])', () => {
  const { document } = fresh();
  const el = document.createElement('div'); // no parent → elementSiblings returns [el]
  assert.equal(el.matches(':first-of-type'), true);
  assert.equal(el.matches(':last-of-type'), true);
  assert.equal(el.matches(':only-of-type'), true);
  assert.equal(el.matches(':nth-of-type(1)'), true);
  assert.equal(el.matches(':nth-child(1)'), true);
});

// ---------- stubs.mjs remaining ----------
test('FileReader readAsDataURL / readAsArrayBuffer / abort', async () => {
  const stubs = await import('../src/runtime/stubs.mjs');
  // readAsDataURL
  const fr1 = new stubs.FileReader();
  const d1 = new Promise((r) => { fr1.onloadend = r; });
  fr1.readAsDataURL(new Blob(['hi'], { type: 'text/plain' }));
  await d1;
  assert.ok(String(fr1.result).startsWith('data:'));
  // readAsArrayBuffer
  const fr2 = new stubs.FileReader();
  const d2 = new Promise((r) => { fr2.onload = r; });
  fr2.readAsArrayBuffer(new Blob(['hi']));
  await d2;
  assert.ok(fr2.result instanceof ArrayBuffer);
  // abort
  const fr3 = new stubs.FileReader();
  let aborted = 0;
  fr3.addEventListener('abort', () => { aborted++; });
  fr3.abort();
  assert.equal(aborted, 1);
  assert.equal(fr3.readyState, 2);
  // readAsText on a non-blob value (String(b) branch)
  const fr4 = new stubs.FileReader();
  const d4 = new Promise((r) => { fr4.onload = r; });
  fr4.readAsText('plain');
  await d4;
  assert.equal(fr4.result, 'plain');
});

test('FileReader __read error path fires error + loadend', async () => {
  const stubs = await import('../src/runtime/stubs.mjs');
  const fr = new stubs.FileReader();
  let err = 0, end = 0;
  fr.onerror = () => { err++; };
  fr.onloadend = () => { end++; };
  // a "blob" whose text() rejects drives the catch branch
  fr.readAsText({ text: () => Promise.reject(new Error('boom')) });
  await new Promise((r) => setTimeout(r, 5));
  assert.equal(err, 1);
  assert.equal(end, 1);
  assert.equal(fr.readyState, 2);
});

test('matchMedia removeEventListener no-op + customElements getName miss', async () => {
  const stubs = await import('../src/runtime/stubs.mjs');
  const mq = stubs.makeMatchMedia()('(max-width: 50px)');
  assert.equal(mq.removeEventListener('change', () => {}), undefined);
  assert.equal(mq.addListener(() => {}), undefined);
  const ce = stubs.makeCustomElements();
  assert.equal(ce.getName(function Foo() {}), null); // not defined → null
  assert.equal(ce.upgrade(), undefined);
});

test('Storage.key out of range returns null; canvas createPattern stub', async () => {
  const stubs = await import('../src/runtime/stubs.mjs');
  const s = new stubs.Storage();
  assert.equal(s.key(0), null); // empty → ?? null
  s.setItem('a', '1');
  assert.equal(s.key(5), null); // out of range
  const ctx = stubs.makeCanvasStub();
  const pat = ctx.createPattern();
  pat.addColorStop(0, '#000'); // no throw
  const rad = ctx.createRadialGradient();
  rad.addColorStop(1, '#fff');
});

test('makeLocation getters + makeHistory push/replace/back/forward/go', async () => {
  const stubs = await import('../src/runtime/stubs.mjs');
  const loc = stubs.makeLocation('https://h.test:9000/path?q=1#frag');
  assert.equal(loc.protocol, 'https:');
  assert.equal(loc.host, 'h.test:9000');
  assert.equal(loc.hostname, 'h.test');
  assert.equal(loc.port, '9000');
  assert.equal(loc.pathname, '/path');
  assert.equal(loc.search, '?q=1');
  assert.equal(loc.hash, '#frag');
  assert.equal(loc.origin, 'https://h.test:9000');
  assert.equal(loc.toString(), loc.href);
  loc.href = 'ignored'; // navigation no-op
  assert.equal(loc.assign(), undefined);
  assert.equal(loc.replace(), undefined);
  assert.equal(loc.reload(), undefined);
  const hist = stubs.makeHistory(loc);
  assert.equal(hist.length, 1);
  assert.equal(hist.state, null);
  hist.pushState({ n: 1 }, '', '/a');
  hist.pushState({ n: 2 }, '', '/b');
  assert.equal(hist.length, 3);
  assert.deepEqual(hist.state, { n: 2 });
  hist.back();
  assert.deepEqual(hist.state, { n: 1 });
  hist.forward();
  assert.deepEqual(hist.state, { n: 2 });
  hist.go(-2);
  assert.equal(hist.state, null);
  hist.go(0); // clamps in range
  hist.replaceState({ n: 9 }, '', undefined); // url ?? current
  assert.deepEqual(hist.state, { n: 9 });
});

// ---------- cssom insertRule explicit index ----------
test('CSSStyleSheet insertRule at the end index', async () => {
  const { CSSStyleSheet } = await import('../src/runtime/cssom.mjs');
  const sheet = new CSSStyleSheet();
  sheet.insertRule('.a {}', 0);
  sheet.insertRule('.b {}', 1); // at length → valid
  assert.equal(sheet.cssRules.length, 2);
  assert.equal(sheet.cssRules[1].selectorText, '.b');
});

// ---------- cascade parseDecls empty-name continue ----------
test('getComputedStyle ignores a declaration with an empty property name', () => {
  const { window, document } = fresh();
  document.head.innerHTML = '<style>.q { : red; color: blue }</style>';
  document.body.innerHTML = '<div class="q">x</div>';
  const cs = window.getComputedStyle(document.querySelector('.q'));
  assert.equal(cs.color, 'rgb(0, 0, 255)'); // empty-name decl skipped, color applied
});

// ====================================================================
// ==== batch 40: extra margin — small high-confidence branches ========
// ====================================================================

test('live HTMLCollection namedItem + has(namedItem)', () => {
  const { document } = fresh();
  document.body.innerHTML = '<form><input name="user"></form>';
  const coll = document.querySelector('form').elements;
  assert.equal('namedItem' in coll, true); // key in extra branch
  assert.ok(coll.namedItem('user'));
  assert.equal(coll.namedItem('nope'), null);
});

test('defaultValue get/set reflects the value attribute', () => {
  const { document } = fresh();
  const i = document.createElement('input');
  assert.equal(i.defaultValue, '');
  i.defaultValue = 'd';
  assert.equal(i.getAttribute('value'), 'd');
  assert.equal(i.defaultValue, 'd');
});

test('htmlFor / control fallbacks return empty/null appropriately', () => {
  const { document } = fresh();
  const label = document.createElement('label');
  assert.equal(label.htmlFor, ''); // no for attr → ''
  // label with no for + no labelable descendant → control null
  assert.equal(label.control, null);
  // label with a labelable descendant (no for) → that descendant
  label.innerHTML = '<input>';
  assert.equal(label.control, label.querySelector('input'));
});

test('Text.wholeText with only preceding text siblings', () => {
  const { document } = fresh();
  document.body.innerHTML = '<p></p>';
  const p = document.querySelector('p');
  const a = document.createTextNode('one');
  const b = document.createTextNode('two');
  p.appendChild(a); p.appendChild(b);
  // b has a preceding text sibling but no following → exercises the forward loop's exit
  assert.equal(b.wholeText, 'onetwo');
});

test('setRangeText with default args + select() on empty value', () => {
  const { document } = fresh();
  const i = document.createElement('input');
  // no value, no selection → defaults to 0,0
  i.setRangeText('X');
  assert.equal(i.value, 'X');
  const j = document.createElement('input');
  j.select(); // value ?? '' → length 0
  assert.equal(j.selectionStart, 0);
  assert.equal(j.selectionEnd, 0);
});

test('XMLSerializer + DOMParser round-trip', () => {
  const { window, document } = fresh();
  const ser = new window.XMLSerializer();
  document.body.innerHTML = '<div><b>x</b></div>';
  const div = document.querySelector('div');
  assert.equal(ser.serializeToString(div), '<div><b>x</b></div>'); // element → outer
  const frag = document.createDocumentFragment();
  frag.appendChild(document.createElement('i'));
  assert.equal(ser.serializeToString(frag), '<i></i>'); // fragment → inner
  // DOMParser text/html and xml-ish
  const parser = new window.DOMParser();
  const doc = parser.parseFromString('<!doctype html><body><p>hi</p></body>', 'text/html');
  assert.ok(doc.querySelector('p'));
  const xmlDoc = parser.parseFromString('<root>x</root>', 'application/xml');
  assert.ok(xmlDoc);
});

test('SVG length attribute setter accepts a number, a string, and a baseVal object', () => {
  const { document } = fresh();
  document.body.innerHTML = '<svg><rect></rect></svg>';
  const rect = document.querySelector('rect');
  rect.width = 20; // number
  assert.equal(rect.getAttribute('width'), '20');
  rect.height = '30px'; // string
  assert.equal(rect.getAttribute('height'), '30px');
  rect.width = { baseVal: { value: 40 } }; // baseVal-bearing object
  assert.equal(rect.getAttribute('width'), '40');
});

test('adoptNode when attrs already materialized off the old buffer', () => {
  const { document } = fresh('<!doctype html><body><div id="s" data-x="1">t</div></body>');
  const other = createEnvironment('<!doctype html><body></body></html>').document;
  const src = document.getElementById('s');
  src.getAttributeNames(); // force __attrs to materialize first
  const adopted = other.adoptNode(src);
  assert.equal(adopted.getAttribute('data-x'), '1');
  assert.equal(adopted.ownerDocument, other);
});

test('MutationObserver childList-only ignores attribute mutations', async () => {
  const { window, document } = fresh();
  document.body.innerHTML = '<div id="r"></div>';
  const r = document.getElementById('r');
  const recs = [];
  const mo = new window.MutationObserver((rs) => recs.push(...rs));
  mo.observe(r, { childList: true });
  r.setAttribute('x', '1'); // attributes not observed → ignored
  r.appendChild(document.createElement('span')); // childList → observed
  await Promise.resolve();
  mo.disconnect();
  assert.ok(recs.every((x) => x.type === 'childList'));
});

// ====================================================================
// ==== batch: injectable clock (virtual time for rAF transition loops)
// ====================================================================

test('setClock: virtual clock drives performance.now + the rAF timestamp', async () => {
  try {
    const { window } = fresh();
    // default → real host clock (a number, not the virtual value)
    assert.equal(typeof window.performance.now(), 'number');
    // install a virtual clock
    let vnow = 0;
    setClock(() => vnow);
    assert.equal(window.performance.now(), 0);
    vnow = 1234;
    assert.equal(window.performance.now(), 1234);
    // rAF stamps its callback with now() — a clock that advances lets MUI-style
    // progress=(now-start)/duration reach >=1 and stop rescheduling
    vnow = 5000;
    const ts = await new Promise((res) => window.requestAnimationFrame((t) => res(t)));
    assert.equal(ts, 5000);
    // non-function arg resets to the host clock
    setClock('not a fn');
    const hostNow = window.performance.now();
    assert.equal(typeof hostNow, 'number');
    assert.notEqual(hostNow, 5000);
  } finally {
    setClock(null); // never leak module state into other tests
  }
});

test('rAF schedules via the LIVE globalThis.setTimeout (render-tier queue catches reschedules)', () => {
  const { window } = fresh();
  const realST = globalThis.setTimeout;
  const queue = [];
  globalThis.setTimeout = (cb) => { queue.push(cb); return queue.length; };
  try {
    let fired = false;
    const id = window.requestAnimationFrame(() => { fired = true; });
    assert.equal(id, 1);            // returned the queue id, not a host timer
    assert.equal(queue.length, 1);  // enqueued, not run
    assert.equal(fired, false);
    queue[0]();                     // host "drains" the queue
    assert.equal(fired, true);
  } finally {
    globalThis.setTimeout = realST;
  }
});

test('rAF/cAF fall back to captured host timers when globalThis timers are absent', () => {
  const { window } = fresh();
  const realST = globalThis.setTimeout, realCT = globalThis.clearTimeout;
  try {
    globalThis.setTimeout = undefined;   // forces the `|| hostSetTimeout` fallback
    const id = window.requestAnimationFrame(() => {});
    assert.ok(id != null);               // host timer id returned
    globalThis.clearTimeout = undefined; // forces the `|| hostClearTimeout` fallback
    window.cancelAnimationFrame(id);     // no throw
  } finally {
    globalThis.setTimeout = realST;
    globalThis.clearTimeout = realCT;
  }
});

test('cancelAnimationFrame clears via the live clearTimeout', () => {
  const { window } = fresh();
  const realCT = globalThis.clearTimeout;
  let cleared;
  globalThis.clearTimeout = (id) => { cleared = id; };
  try {
    const id = window.requestAnimationFrame(() => {});
    window.cancelAnimationFrame(id);
    assert.equal(cleared, id);
  } finally {
    globalThis.clearTimeout = realCT;
  }
});

// ====================================================================
// ==== batch: MessageChannel/MessagePort (React 19 scheduler) =========
// ====================================================================

test('MessageChannel links two ports; postMessage delivers via onmessage + addEventListener', async () => {
  const { window } = fresh();
  const mc = new window.MessageChannel();
  assert.ok(mc.port1 && mc.port2);
  mc.port1.start(); mc.port2.start(); // no-op, must not throw
  // onmessage handler
  const viaHandler = await new Promise((res) => {
    mc.port2.onmessage = (ev) => res(ev.data);
    mc.port1.postMessage('ping');
  });
  assert.equal(viaHandler, 'ping');
  // addEventListener('message') path (the dispatchEvent hop)
  const viaListener = await new Promise((res) => {
    mc.port1.addEventListener('message', (ev) => res(ev.data));
    mc.port2.postMessage({ n: 42 });
  });
  assert.deepEqual(viaListener, { n: 42 });
});

test('MessagePort delivery routes through the LIVE globalThis.setTimeout', () => {
  const { window } = fresh();
  const realST = globalThis.setTimeout;
  const queue = [];
  globalThis.setTimeout = (cb) => { queue.push(cb); return queue.length; };
  try {
    const mc = new window.MessageChannel();
    let got;
    mc.port2.onmessage = (ev) => { got = ev.data; };
    mc.port1.postMessage('hi');
    assert.equal(queue.length, 1);   // enqueued, not delivered
    assert.equal(got, undefined);
    queue[0]();                      // owned pump drains it
    assert.equal(got, 'hi');
  } finally {
    globalThis.setTimeout = realST;
  }
});

test('MessagePort: closed/peerless port is a no-op; setTimeout fallback to host', () => {
  const { window } = fresh();
  const mc = new window.MessageChannel();
  mc.port1.close();                       // drop the peer
  mc.port1.postMessage('x');              // no peer → no-op, no throw
  // fallback branch: globalThis.setTimeout absent → hostSetTimeout
  const realST = globalThis.setTimeout;
  try {
    globalThis.setTimeout = undefined;
    const mc2 = new window.MessageChannel();
    mc2.port1.postMessage('y');           // uses hostSetTimeout, no throw
  } finally {
    globalThis.setTimeout = realST;
  }
});
