import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

const HTML = `<!doctype html><html><head><title>t</title></head><body>
  <div id="app" class="card big" data-role="main">
    <h1>Title</h1>
    <ul class="list"><li>a</li><li class="sel">b</li><li>c</li></ul>
    <input type="text" value="hi">
  </div>
</body></html>`;

const fresh = () => createEnvironment(HTML);

// ---------------------------------------------------------- Layer 2 ----
test('identity memoization: same node === across accesses', () => {
  const { document } = fresh();
  const a = document.getElementById('app');
  const b = document.querySelector('#app');
  assert.equal(a, b);
  assert.equal(document.getElementById('app'), a);
});

test('WeakMap keying survives mutations around the node', () => {
  const { document } = fresh();
  const app = document.getElementById('app');
  const wm = new WeakMap();
  wm.set(app, 'tag');
  app.appendChild(document.createElement('span'));
  app.setAttribute('x', '1');
  assert.equal(wm.get(document.getElementById('app')), 'tag');
});

test('live childNodes / children reflect mutation immediately', () => {
  const { document } = fresh();
  const ul = document.querySelector('ul');
  const kids = ul.children;
  assert.equal(kids.length, 3);
  ul.appendChild(document.createElement('li'));
  assert.equal(kids.length, 4); // same live collection sees the new child
  ul.removeChild(ul.firstElementChild);
  assert.equal(kids.length, 3);
});

test('live getElementsByClassName updates on class change', () => {
  const { document } = fresh();
  const sel = document.getElementsByClassName('sel');
  assert.equal(sel.length, 1);
  document.querySelector('h1').classList.add('sel');
  assert.equal(sel.length, 2);
});

test('COW: reading buffer-backed then mutating is consistent', () => {
  const { document } = fresh();
  const li = document.querySelectorAll('li')[1];
  assert.equal(li.textContent, 'b');
  li.textContent = 'B!';
  assert.equal(document.querySelectorAll('li')[1].textContent, 'B!');
});

test('textContent get/set', () => {
  const { document } = fresh();
  const app = document.getElementById('app');
  assert.ok(app.textContent.includes('Title'));
  app.textContent = 'gone';
  assert.equal(app.textContent, 'gone');
  assert.equal(app.children.length, 0);
});

test('innerHTML get + set (reparse)', () => {
  const { document } = fresh();
  const app = document.getElementById('app');
  app.innerHTML = '<p class="x">hi</p><p>yo</p>';
  assert.equal(app.children.length, 2);
  assert.equal(app.querySelector('.x').textContent, 'hi');
  assert.equal(app.innerHTML, '<p class="x">hi</p><p>yo</p>');
});

test('innerHTML get on unmutated buffer-backed subtree (lazy __attrs)', () => {
  const { document } = fresh();
  const app = document.getElementById('app'); // never mutated → children keep lazy __attrs
  const html = app.innerHTML;                  // serializes buffer-backed els with __attrs undefined
  assert.ok(html.includes('<li class="sel">b</li>'));
  assert.ok(html.includes('<input type="text" value="hi">'));
});

test('insertBefore / replaceChild / cloneNode(deep)', () => {
  const { document } = fresh();
  const ul = document.querySelector('ul');
  const first = ul.firstElementChild;
  const n = document.createElement('li'); n.textContent = 'z';
  ul.insertBefore(n, first);
  assert.equal(ul.firstElementChild.textContent, 'z');
  const clone = ul.cloneNode(true);
  assert.equal(clone.children.length, ul.children.length);
  assert.notEqual(clone, ul);
});

test('classList + attribute reflection', () => {
  const { document } = fresh();
  const app = document.getElementById('app');
  assert.ok(app.classList.contains('card'));
  app.classList.toggle('card');
  assert.ok(!app.classList.contains('card'));
  app.id = 'changed';
  assert.equal(app.getAttribute('id'), 'changed');
});

test('inline style CSSOM (honest, inline-only)', () => {
  const { document } = fresh();
  const app = document.getElementById('app');
  app.style.color = 'red';
  assert.equal(app.style.color, 'red');
  assert.ok(app.getAttribute('style').includes('color'));
});

// ---------------------------------------------------------- Events ----
test('event capture → target → bubble order', () => {
  const { document, window } = fresh();
  const app = document.getElementById('app');
  const h1 = document.querySelector('h1');
  const order = [];
  document.body.addEventListener('x', () => order.push('body-capture'), true);
  app.addEventListener('x', () => order.push('app-capture'), true);
  h1.addEventListener('x', () => order.push('target'));
  app.addEventListener('x', () => order.push('app-bubble'));
  document.body.addEventListener('x', () => order.push('body-bubble'));
  h1.dispatchEvent(new window.Event('x', { bubbles: true }));
  assert.deepEqual(order, ['body-capture', 'app-capture', 'target', 'app-bubble', 'body-bubble']);
});

test('stopPropagation / stopImmediatePropagation / preventDefault', () => {
  const { document, window } = fresh();
  const app = document.getElementById('app');
  const h1 = document.querySelector('h1');
  let bubbledToApp = false;
  app.addEventListener('y', () => { bubbledToApp = true; });
  h1.addEventListener('y', (e) => e.stopPropagation());
  h1.dispatchEvent(new window.Event('y', { bubbles: true }));
  assert.equal(bubbledToApp, false);

  let count = 0;
  h1.addEventListener('z', (e) => { count++; e.stopImmediatePropagation(); });
  h1.addEventListener('z', () => { count++; });
  h1.dispatchEvent(new window.Event('z'));
  assert.equal(count, 1);

  const e = new window.Event('w', { cancelable: true });
  h1.addEventListener('w', (ev) => ev.preventDefault());
  const ret = h1.dispatchEvent(e);
  assert.equal(ret, false);
  assert.equal(e.defaultPrevented, true);
});

test('composedPath + once', () => {
  const { document, window } = fresh();
  const h1 = document.querySelector('h1');
  let path;
  h1.addEventListener('p', (e) => { path = e.composedPath(); });
  h1.dispatchEvent(new window.Event('p', { bubbles: true }));
  assert.equal(path[0], h1);
  assert.ok(path.includes(document.getElementById('app')));

  let n = 0;
  h1.addEventListener('o', () => n++, { once: true });
  h1.dispatchEvent(new window.Event('o'));
  h1.dispatchEvent(new window.Event('o'));
  assert.equal(n, 1);
});

// -------------------------------------------------------- Selectors ----
test('selector engine: combinators, attr, :not, :first-child', () => {
  const { document } = fresh();
  assert.equal(document.querySelectorAll('ul > li').length, 3);
  assert.equal(document.querySelector('li.sel').textContent, 'b');
  assert.equal(document.querySelectorAll('[data-role="main"]').length, 1);
  assert.equal(document.querySelector('li:first-child').textContent, 'a');
  assert.equal(document.querySelectorAll('li:not(.sel)').length, 2);
  assert.equal(document.querySelector('li + li.sel').textContent, 'b');
  assert.ok(document.getElementById('app').matches('div.card[data-role]'));
  assert.equal(document.querySelector('h1').closest('#app').id, 'app');
});

// --------------------------------------------------- Layer 3 window ----
test('lazy window: nothing materialized until touched', () => {
  const env = fresh();
  assert.deepEqual(env.touched(), []);
  void env.document; // eager, not lazy
  assert.deepEqual(env.touched(), []);
  env.window.matchMedia('(min-width: 1px)');
  void env.window.localStorage;
  assert.deepEqual(env.touched().sort(), ['localStorage', 'matchMedia']);
});

test('window self-references + storage isolation', () => {
  const env = fresh();
  assert.equal(env.window.window, env.window);
  assert.equal(env.window.self, env.window);
  env.window.localStorage.setItem('a', '1');
  assert.equal(env.window.localStorage.getItem('a'), '1');
  assert.equal(env.window.sessionStorage.getItem('a'), null);
});

test('history co-materializes with location and shares it', () => {
  const env = fresh();
  env.window.history.pushState({ n: 1 }, '', '/next');
  assert.equal(env.window.history.state.n, 1);
  assert.ok(env.touched().includes('location'));
});

// ---------------------------------------------------- Layer 4 honest ----
test('honest stubs: no plausible lies', () => {
  const env = fresh();
  const app = env.document.getElementById('app');
  assert.deepEqual(app.getBoundingClientRect().width, 0);
  assert.equal(env.window.getComputedStyle(app).color, ''); // honest empty, not "rgb(0,0,0)"
  assert.equal(env.window.matchMedia('(x)').matches, false);
});

// ----------------------------------------------------- Layer 5 reset ----
test('reset drops overlay + cache, keeps working', () => {
  const env = fresh();
  const app1 = env.document.getElementById('app');
  app1.textContent = 'mutated';
  env.window.localStorage.setItem('k', 'v');
  env.reset();
  // buffer reused → pristine structure back, overlay gone
  const app2 = env.document.getElementById('app');
  assert.ok(app2.textContent.includes('Title'));
  assert.notEqual(app1, app2);             // cache cleared → fresh handles
  assert.deepEqual(env.touched(), []);     // globals dropped
});

test('reset with new html', () => {
  const env = fresh();
  env.reset('<!doctype html><body><main>new page</main></body>');
  assert.equal(env.document.querySelector('main').textContent, 'new page');
  assert.equal(env.document.getElementById('app'), null);
});

// ---------------------------------------------- regression batch (1-7) ----
const env = (html) => createEnvironment('<!doctype html><body>' + html + '</body>');

// 1. normalize() must invalidate caches + notify observers (same class as v0.1.28)
test('normalize() bumps version so cached queries invalidate', () => {
  const { document } = env('<p id=p>a</p>');
  const p = document.getElementById('p');
  p.appendChild(document.createTextNode('b'));
  document.querySelectorAll('p');          // warm version-keyed cache
  const v1 = document.__version;
  p.normalize();
  assert.equal(p.childNodes.length, 1);    // merged
  assert.notEqual(document.__version, v1); // cache invalidated
});

test('normalize() notifies MutationObserver', async () => {
  const { document, window } = env('<p id=p>a</p>');
  const p = document.getElementById('p');
  p.appendChild(document.createTextNode('b'));
  let records = 0;
  const mo = new window.MutationObserver((recs) => { records += recs.length; });
  mo.observe(p, { childList: true });
  p.normalize();
  await Promise.resolve(); await Promise.resolve();
  mo.disconnect();
  assert.ok(records > 0);
});

// 2. checkbox/radio click() must fire input + change (activation default action)
test('checkbox.click() fires input + change as default action', () => {
  const { document } = env('<input id=c type=checkbox>');
  const c = document.getElementById('c');
  const seen = [];
  c.addEventListener('input', () => seen.push('input'));
  c.addEventListener('change', () => seen.push('change'));
  c.click();
  assert.equal(c.checked, true);
  assert.deepEqual(seen, ['input', 'change']);
});

test('preventDefault on checkbox click suppresses toggle AND change', () => {
  const { document } = env('<input id=c type=checkbox>');
  const c = document.getElementById('c');
  let changed = false;
  c.addEventListener('click', (e) => e.preventDefault());
  c.addEventListener('change', () => { changed = true; });
  c.click();
  assert.equal(c.checked, false);
  assert.equal(changed, false);
});

test('radio.click() fires change on the newly-checked radio only', () => {
  const { document } = env('<form><input id=r1 type=radio name=g><input id=r2 type=radio name=g></form>');
  const r1 = document.getElementById('r1'), r2 = document.getElementById('r2');
  let c1 = 0, c2 = 0;
  r1.addEventListener('change', () => c1++);
  r2.addEventListener('change', () => c2++);
  r1.click();
  assert.equal(r1.checked, true);
  assert.equal(c1, 1); assert.equal(c2, 0);
});

// 3. form.reset() must restore control defaults
test('form.reset() restores control defaults', () => {
  const { document } = env('<form id=f><input id=i value=orig><input id=c type=checkbox checked><textarea id=t>def</textarea><select id=s><option value=a>a</option><option value=b selected>b</option></select></form>');
  const f = document.getElementById('f');
  document.getElementById('i').value = 'changed';
  document.getElementById('c').checked = false;
  document.getElementById('t').value = 'changed';
  document.getElementById('s').value = 'a';
  f.reset();
  assert.equal(document.getElementById('i').value, 'orig');
  assert.equal(document.getElementById('c').checked, true);
  assert.equal(document.getElementById('t').value, 'def');
  assert.equal(document.getElementById('s').value, 'b');
});

test('form.reset() abortable via preventDefault', () => {
  const { document } = env('<form id=f><input id=i value=orig></form>');
  const f = document.getElementById('f');
  document.getElementById('i').value = 'changed';
  f.addEventListener('reset', (e) => e.preventDefault());
  f.reset();
  assert.equal(document.getElementById('i').value, 'changed'); // not reset
});

// 4. previously-missing tag-classes are defined → instanceof returns false, not throw
test('instanceof: missing tag-classes defined and match by tag', () => {
  const { document, window } = env('<table id=t></table><div id=d></div>');
  const t = document.getElementById('t'), d = document.getElementById('d');
  for (const name of ['HTMLTableElement','HTMLTableRowElement','HTMLOListElement','HTMLFieldSetElement','HTMLDialogElement','HTMLDetailsElement','HTMLVideoElement','HTMLAudioElement','HTMLProgressElement','HTMLPreElement']) {
    assert.equal(typeof window[name], 'function', name + ' should be defined');
  }
  assert.ok(t instanceof window.HTMLTableElement);
  assert.ok(!(d instanceof window.HTMLTableElement)); // no longer throws
});

// 5. adoptNode must materialize a buffer-backed node off its old document
test('adoptNode materializes buffer-backed node off old document', () => {
  const a = env('<ul id=u><li class=x>hi</li></ul>');
  const b = env('<div id=host></div>');
  const li = a.document.querySelector('li.x');
  const adopted = b.document.adoptNode(li);
  b.document.getElementById('host').appendChild(adopted);
  assert.equal(adopted.ownerDocument, b.document);
  assert.equal(adopted.getAttribute('class'), 'x'); // attrs survived re-home
  assert.equal(adopted.textContent, 'hi');          // children survived
  assert.equal(b.document.querySelector('.x'), adopted);
});

// 6. parse cache survives churn past its bound and stays correct
test('parse cache stays correct after >64 distinct shells (LRU, no wholesale clear)', () => {
  const a = env('<b>keep</b>');
  assert.equal(a.document.querySelector('b').textContent, 'keep');
  for (let i = 0; i < 80; i++) env('<i>' + i + '</i>');
  const c = env('<b>keep</b>');
  assert.equal(c.document.querySelector('b').textContent, 'keep');
});

// 7. style longhand reads fall back to a single-token shorthand
test('style longhand reads fall back to single-token shorthand', () => {
  const { document } = env('<div id=d></div>');
  const d = document.getElementById('d');
  d.style.background = 'red';
  assert.equal(d.style.backgroundColor, 'red');
  assert.equal(d.style.getPropertyValue('background-color'), 'red');
  d.style.margin = '5px';
  assert.equal(d.style.marginTop, '5px');
  d.style.padding = '2px 4px'; // multi-token → no single-longhand fallback (honest)
  assert.equal(d.style.paddingTop, '');
});

// ------------------------------ report categories B / C / E (real gaps) ----
// B. SVG interface globals exist (setup mocks defineProperty on their prototypes)
test('SVG element globals are defined', () => {
  const { window } = env('<svg></svg>');
  for (const n of ['SVGSVGElement','SVGElement','SVGPathElement','SVGGElement','SVGRectElement','SVGCircleElement','SVGTextElement','SVGUseElement']) {
    assert.equal(typeof window[n], 'function', n + ' must be defined');
  }
  // the exact pattern that crashed MapCard setup
  assert.doesNotThrow(() => Object.defineProperty(window.SVGSVGElement.prototype, '__viewBoxMock', { value: 1, configurable: true }));
});

// C. case-insensitive HTML attribute access + camelCase IDL reflection
test('getAttribute is case-insensitive for HTML elements', () => {
  const { document } = env('<div id=d tabindex=0></div>');
  const d = document.getElementById('d');
  assert.equal(d.getAttribute('tabIndex'), '0');   // camelCase query hits lowercased attr
  assert.equal(d.getAttribute('TABINDEX'), '0');
  assert.ok(d.hasAttribute('TabIndex'));
});

test('camelCase IDL properties reflect to lowercased attributes', () => {
  const { document } = env('<table><tr><td id=c>x</td></tr></table><input id=i>');
  const c = document.getElementById('c'), i = document.getElementById('i');
  c.colSpan = 3;
  assert.equal(c.getAttribute('colspan'), '3');
  i.inputMode = 'decimal';
  assert.equal(i.getAttribute('inputmode'), 'decimal');
  i.spellcheck = false;
  assert.equal(i.getAttribute('spellcheck'), 'false');
  i.autocomplete = 'off';
  assert.equal(i.getAttribute('autocomplete'), 'off');
});

// E. accept / min / max reflect
test('accept / min / max reflect to attributes', () => {
  const { document } = env('<input id=i>');
  const i = document.getElementById('i');
  i.accept = 'image/png,image/jpeg';
  i.min = '2024-01-01'; i.max = '2024-12-31';
  assert.equal(i.getAttribute('accept'), 'image/png,image/jpeg');
  assert.equal(i.getAttribute('min'), '2024-01-01');
  assert.equal(i.getAttribute('max'), '2024-12-31');
});

// ------------------------------- partial computed-style cascade (A / D) ----
test('getComputedStyle resolves injected <style> class rules', () => {
  const { document, window } = env('<style>.box{color:rgb(1,2,3);font-size:24px}</style><div id=d class=box></div>');
  const cs = window.getComputedStyle(document.getElementById('d'));
  assert.equal(cs.color, 'rgb(1,2,3)');
  assert.equal(cs.fontSize, '24px');
  assert.equal(cs.getPropertyValue('font-size'), '24px');
});

test('cascade: specificity + source order + inline overlay', () => {
  const { document, window } = env('<style>.a{color:red}#x{color:green}.a{color:blue}</style><p id=x class=a style="color:black"></p>');
  // inline beats everything
  assert.equal(window.getComputedStyle(document.getElementById('x')).color, 'black');
  const { document: d2, window: w2 } = env('<style>.a{color:red}#x{color:green}.a{color:blue}</style><p id=x class=a></p>');
  // #id (higher specificity) wins over the later .a rule
  assert.equal(w2.getComputedStyle(d2.getElementById('x')).color, 'green');
});

test('cascade is honest: unmatched property + no-stylesheet reads empty', () => {
  const { document, window } = env('<style>.box{color:red}</style><div id=d class=box></div><div id=e></div>');
  assert.equal(window.getComputedStyle(document.getElementById('d')).background, ''); // not set anywhere
  assert.equal(window.getComputedStyle(document.getElementById('e')).color, '');      // matches no rule
});

test('cascade resolves toBeVisible-style hiding (display/visibility)', () => {
  const { document, window } = env('<style>.hidden{display:none}.invis{visibility:hidden}</style><div id=a class=hidden></div><div id=b class=invis></div><div id=c></div>');
  assert.equal(window.getComputedStyle(document.getElementById('a')).display, 'none');
  assert.equal(window.getComputedStyle(document.getElementById('b')).visibility, 'hidden');
  assert.equal(window.getComputedStyle(document.getElementById('c')).display, ''); // unknown → honest empty
});

test('cascade invalidates on style mutation (version-keyed)', () => {
  const { document, window } = env('<style id=s>.box{color:red}</style><div id=d class=box></div>');
  const d = document.getElementById('d');
  assert.equal(window.getComputedStyle(d).color, 'red');
  document.getElementById('s').textContent = '.box{color:lime}';
  assert.equal(window.getComputedStyle(d).color, 'lime'); // rebuilt, not stale
});

test('cascade skips @media/@keyframes blocks (honest partial)', () => {
  const { document, window } = env('<style>.box{color:red}@media(min-width:1px){.box{color:blue}}</style><div id=d class=box></div>');
  assert.equal(window.getComputedStyle(document.getElementById('d')).color, 'red'); // media rule ignored, base applies
});

// --------------------------------- 0.1.30 cascade + sanitize follow-ups ----
// #3: invalid typed-input value sanitizes to '' (WHATWG), not retain prior
test('number input invalid value sanitizes to empty (not retain prior)', () => {
  const { document } = env('<input id=n type=number>');
  const n = document.getElementById('n');
  n.value = '0'; assert.equal(n.value, '0');
  n.value = '.'; assert.equal(n.value, '');   // was "0" retained before
  n.value = '5'; assert.equal(n.value, '5');
  n.value = '-'; assert.equal(n.value, '');
});

// #1: zero-length computed values px-normalize
test('cascade px-normalizes bare 0 for length properties', () => {
  const { document, window } = env('<style>.t{flex-basis:0;opacity:0;z-index:0;margin:0}</style><div id=d class=t></div>');
  const cs = window.getComputedStyle(document.getElementById('d'));
  assert.equal(cs.flexBasis, '0px');
  assert.equal(cs.marginTop, '0px');
  assert.equal(cs.opacity, '0');  // NOT a length → stays bare
  assert.equal(cs.zIndex, '0');   // NOT a length → stays bare
});

// #2: shorthand expands to longhands in computed style
test('cascade expands border/margin/padding shorthands to longhands', () => {
  const { document, window } = env('<style>.t{border:1px solid red;margin:5px 10px;padding:2px}</style><div id=d class=t></div>');
  const cs = window.getComputedStyle(document.getElementById('d'));
  assert.match(cs.borderWidth, /1px/);
  assert.equal(cs.borderStyle, 'solid');
  assert.equal(cs.borderColor, 'red');
  assert.equal(cs.marginTop, '5px');
  assert.equal(cs.marginRight, '10px');
  assert.equal(cs.marginBottom, '5px'); // 2-value: bottom=top
  assert.equal(cs.paddingLeft, '2px');  // 1-value: all sides
});

// #font-family: computed value normalizes comma spacing to ", " (emotion minifies)
test('cascade normalizes font-family comma spacing', () => {
  const { document, window } = env('<style>.t{font-family:"Open Sans",Inter,Roboto,"Helvetica Neue",Arial,sans-serif;color:rgb(1,2,3)}</style><div id=d class=t></div>');
  const cs = window.getComputedStyle(document.getElementById('d'));
  assert.equal(cs.fontFamily, '"Open Sans", Inter, Roboto, "Helvetica Neue", Arial, sans-serif');
  assert.equal(cs.color, 'rgb(1,2,3)'); // non-font-family commas untouched
});
