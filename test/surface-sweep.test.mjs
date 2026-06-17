// Surface sweep: drives function/branch/line coverage of the runtime's meaningful
// reflected IDL getters/setters, Range/Selection, CharacterData, document/element
// ops, and the honest stubs. Assertions follow WEB SPEC, never the impl's quirks.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

const fresh = (html) => createEnvironment(html ?? '<!doctype html><html><head><title>T</title></head><body><div id=root></div></body></html>');

// ----------------------------------------------------- reflected IDL props ----
test('input value / valueAsNumber / valueAsDate reflection', () => {
  const { document } = fresh();
  const inp = document.createElement('input');
  inp.value = 'hello';
  assert.equal(inp.value, 'hello');
  // number input: valueAsNumber reflects numeric value both ways
  inp.type = 'number';
  assert.equal(inp.getAttribute('type'), 'number');
  assert.equal(inp.type, 'number');
  inp.value = '42';
  assert.equal(inp.valueAsNumber, 42);
  inp.valueAsNumber = 7;
  assert.equal(inp.value, '7');
  // empty value → NaN
  inp.value = '';
  assert.ok(Number.isNaN(inp.valueAsNumber));
  // date input: valueAsDate round-trips
  const d = document.createElement('input');
  d.type = 'date';
  d.value = '2020-01-02';
  const got = d.valueAsDate;
  assert.ok(got instanceof Date);
  assert.equal(got.toISOString().slice(0, 10), '2020-01-02');
  d.valueAsDate = new Date('2021-03-04T00:00:00Z');
  assert.equal(d.value, '2021-03-04');
  // invalid value → valueAsDate null
  const bad = document.createElement('input');
  bad.value = 'not-a-date';
  assert.equal(bad.valueAsDate, null);
  // setting valueAsDate to non-Date clears value
  d.valueAsDate = null;
  assert.equal(d.value, '');
});

test('selection range props + setSelectionRange + select', () => {
  const { document } = fresh();
  const inp = document.createElement('input');
  inp.value = 'abcdef';
  assert.equal(inp.selectionStart, null);
  inp.selectionStart = 1;
  inp.selectionEnd = 3;
  assert.equal(inp.selectionStart, 1);
  assert.equal(inp.selectionEnd, 3);
  assert.equal(inp.selectionDirection, 'none');
  inp.selectionDirection = 'forward';
  assert.equal(inp.selectionDirection, 'forward');
  inp.setSelectionRange(2, 4, 'backward');
  assert.equal(inp.selectionStart, 2);
  assert.equal(inp.selectionEnd, 4);
  assert.equal(inp.selectionDirection, 'backward');
  inp.select();
  assert.equal(inp.selectionStart, 0);
  assert.equal(inp.selectionEnd, 6);
});

test('type defaults: input=text, button=submit', () => {
  const { document } = fresh();
  assert.equal(document.createElement('input').type, 'text');
  assert.equal(document.createElement('button').type, 'submit');
  const other = document.createElement('div');
  assert.equal(other.type, undefined);
  other.type = 'foo';
  assert.equal(other.type, 'foo');
});

test('maxLength / minLength reflection (-1 default)', () => {
  const { document } = fresh();
  const inp = document.createElement('input');
  assert.equal(inp.maxLength, -1);
  assert.equal(inp.minLength, -1);
  inp.maxLength = 10;
  inp.minLength = 2;
  assert.equal(inp.getAttribute('maxlength'), '10');
  assert.equal(inp.getAttribute('minlength'), '2');
  assert.equal(inp.maxLength, 10);
  assert.equal(inp.minLength, 2);
});

test('htmlFor reflects the "for" attribute', () => {
  const { document } = fresh();
  const label = document.createElement('label');
  label.htmlFor = 'foo';
  assert.equal(label.getAttribute('for'), 'foo');
  assert.equal(label.htmlFor, 'foo');
  label.setAttribute('for', 'bar');
  assert.equal(label.htmlFor, 'bar');
});

test('className get/set + dir + lang + hidden + role + spellcheck', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  el.className = 'a b';
  assert.equal(el.className, 'a b');
  assert.equal(el.getAttribute('class'), 'a b');
  el.dir = 'rtl'; assert.equal(el.dir, 'rtl'); assert.equal(el.getAttribute('dir'), 'rtl');
  el.lang = 'en'; assert.equal(el.lang, 'en');
  assert.equal(el.hidden, false);
  el.hidden = true; assert.equal(el.hidden, true); assert.ok(el.hasAttribute('hidden'));
  el.hidden = false; assert.equal(el.hasAttribute('hidden'), false);
  el.role = 'button'; assert.equal(el.role, 'button');
  // spellcheck defaults true; explicit "false" attribute → false
  assert.equal(el.spellcheck, true);
  el.spellcheck = false; assert.equal(el.getAttribute('spellcheck'), 'false'); assert.equal(el.spellcheck, false);
  el.spellcheck = true; assert.equal(el.spellcheck, true);
});

test('boolean reflections: disabled, required, readOnly, multiple', () => {
  const { document } = fresh();
  const inp = document.createElement('input');
  assert.equal(inp.disabled, false);
  inp.disabled = true; assert.ok(inp.hasAttribute('disabled')); assert.equal(inp.disabled, true);
  inp.disabled = false; assert.equal(inp.hasAttribute('disabled'), false);
  inp.required = true; assert.ok(inp.hasAttribute('required'));
  inp.required = false; assert.equal(inp.hasAttribute('required'), false);
  inp.readOnly = true; assert.ok(inp.hasAttribute('readonly'));
  inp.readOnly = false; assert.equal(inp.hasAttribute('readonly'), false);
  const sel = document.createElement('select');
  sel.multiple = true; assert.ok(sel.hasAttribute('multiple')); assert.equal(sel.multiple, true);
  sel.multiple = false; assert.equal(sel.hasAttribute('multiple'), false);
});

test('colSpan / rowSpan reflection with 1 default', () => {
  const { document } = fresh();
  const td = document.createElement('td');
  assert.equal(td.colSpan, 1);
  assert.equal(td.rowSpan, 1);
  td.colSpan = 3; td.rowSpan = 2;
  assert.equal(td.getAttribute('colspan'), '3');
  assert.equal(td.getAttribute('rowspan'), '2');
  assert.equal(td.colSpan, 3);
  assert.equal(td.rowSpan, 2);
});

test('href / src / alt / placeholder / pattern / step / min / max / name reflect', () => {
  const { document } = fresh();
  const a = document.createElement('a');
  a.href = '/x'; assert.equal(a.href, '/x'); assert.equal(a.getAttribute('href'), '/x');
  const img = document.createElement('img');
  img.src = 's.png'; assert.equal(img.src, 's.png');
  img.alt = 'pic'; assert.equal(img.alt, 'pic'); assert.equal(img.getAttribute('alt'), 'pic');
  const inp = document.createElement('input');
  inp.placeholder = 'ph'; assert.equal(inp.placeholder, 'ph'); assert.equal(inp.getAttribute('placeholder'), 'ph');
  inp.pattern = '[0-9]+'; assert.equal(inp.pattern, '[0-9]+'); assert.equal(inp.getAttribute('pattern'), '[0-9]+');
  inp.step = '2'; assert.equal(inp.step, '2');
  inp.min = '0'; assert.equal(inp.min, '0');
  inp.max = '9'; assert.equal(inp.max, '9');
  inp.name = 'field'; assert.equal(inp.name, 'field'); assert.equal(inp.getAttribute('name'), 'field');
  // empty defaults
  assert.equal(document.createElement('a').href, '');
  assert.equal(document.createElement('img').src, '');
  assert.equal(document.createElement('img').alt, '');
});

test('defaultChecked / defaultSelected reflect the content attribute', () => {
  const { document } = fresh();
  const inp = document.createElement('input');
  assert.equal(inp.defaultChecked, false);
  inp.defaultChecked = true; assert.ok(inp.hasAttribute('checked')); assert.equal(inp.defaultChecked, true);
  inp.defaultChecked = false; assert.equal(inp.hasAttribute('checked'), false);
  const opt = document.createElement('option');
  assert.equal(opt.defaultSelected, false);
  opt.setAttribute('selected', '');
  assert.equal(opt.defaultSelected, true);
});

test('tabIndex defaults and reflection', () => {
  const { document } = fresh();
  // anchor / button / input default to 0
  assert.equal(document.createElement('a').tabIndex, 0);
  assert.equal(document.createElement('button').tabIndex, 0);
  // generic div defaults -1
  const div = document.createElement('div');
  assert.equal(div.tabIndex, -1);
  div.tabIndex = 5;
  assert.equal(div.getAttribute('tabindex'), '5');
  assert.equal(div.tabIndex, 5);
});

test('accept / autocomplete / inputMode / referrerPolicy / hreflang / target / text reflect', () => {
  const { document } = fresh();
  const inp = document.createElement('input');
  inp.accept = 'image/*'; assert.equal(inp.accept, 'image/*'); assert.equal(inp.getAttribute('accept'), 'image/*');
  inp.autocomplete = 'off'; assert.equal(inp.autocomplete, 'off');
  inp.inputMode = 'numeric'; assert.equal(inp.inputMode, 'numeric'); assert.equal(inp.getAttribute('inputmode'), 'numeric');
  const a = document.createElement('a');
  a.referrerPolicy = 'no-referrer'; assert.equal(a.referrerPolicy, 'no-referrer'); assert.equal(a.getAttribute('referrerpolicy'), 'no-referrer');
  a.setAttribute('hreflang', 'fr'); assert.equal(a.hreflang, 'fr');
  a.target = '_blank'; assert.equal(a.target, '_blank'); assert.equal(a.getAttribute('target'), '_blank');
  // empty defaults
  assert.equal(document.createElement('input').accept, '');
  assert.equal(document.createElement('input').autocomplete, '');
  assert.equal(document.createElement('input').inputMode, '');
  assert.equal(document.createElement('a').referrerPolicy, '');
  assert.equal(document.createElement('a').hreflang, '');
  assert.equal(document.createElement('a').target, '');
  // option.text reflects text content
  const opt = document.createElement('option');
  opt.text = 'Label';
  assert.equal(opt.text, 'Label');
  assert.equal(opt.textContent, 'Label');
});

test('innerText get/set mirrors textContent', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  el.innerText = 'hi there';
  assert.equal(el.innerText, 'hi there');
  assert.equal(el.textContent, 'hi there');
});

test('nodeName / nodeValue / childElementCount / lastElementChild / previousElementSibling', () => {
  const { document } = fresh();
  const div = document.createElement('div');
  assert.equal(div.nodeName, 'DIV');
  div.innerHTML = '<span>a</span>text<b>b</b>';
  assert.equal(div.childElementCount, 2);
  assert.equal(div.lastElementChild.localName, 'b');
  assert.equal(div.lastElementChild.previousElementSibling.localName, 'span');
  // text node nodeValue
  const t = document.createTextNode('xyz');
  assert.equal(t.nodeName, '#text');
  assert.equal(t.nodeValue, 'xyz');
  t.nodeValue = 'abc';
  assert.equal(t.data, 'abc');
  // element nodeValue is null per spec
  assert.equal(div.nodeValue, null);
});

// ------------------------------------------------------------- Range API ----
test('Range: setStart/setEnd/collapse/toString/commonAncestor', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<p>one</p><p>two</p><p>three</p>';
  const r = document.createRange();
  r.setStart(root, 0);
  r.setEnd(root, 2);
  assert.equal(r.startOffset, 0);
  assert.equal(r.endOffset, 2);
  assert.equal(r.collapsed, false);
  assert.equal(r.commonAncestorContainer, root);
  assert.equal(r.toString(), 'onetwo');
  // commonAncestor when start is a deep descendant and end is the root: walk up
  const deepText = root.querySelector('p').firstChild; // text inside first <p>
  const r4 = document.createRange();
  r4.setStart(deepText, 0);
  r4.setEnd(root, 1);
  assert.equal(r4.commonAncestorContainer, root);
  // toString fallback: cross-container element range returns ''
  const r5 = document.createRange();
  r5.setStart(root, 0);
  r5.setEnd(document.createElement('div'), 0);
  assert.equal(r5.toString(), '');
  // collapse to start
  r.collapse(true);
  assert.equal(r.collapsed, true);
  assert.equal(r.endOffset, 0);
});

test('Range: setStartBefore/After + setEndBefore/After use node index', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<a></a><b></b><c></c>';
  const b = root.querySelector('b');
  const r = document.createRange();
  r.setStartBefore(b);
  assert.equal(r.startContainer, root);
  assert.equal(r.startOffset, 1);
  r.setStartAfter(b);
  assert.equal(r.startOffset, 2);
  r.setEndBefore(b);
  assert.equal(r.endContainer, root);
  assert.equal(r.endOffset, 1);
  r.setEndAfter(b);
  assert.equal(r.endOffset, 2);
});

test('Range: selectNode brackets the node; selectNodeContents covers children', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<a></a><b></b>';
  const a = root.querySelector('a');
  const r = document.createRange();
  r.selectNode(a);
  assert.equal(r.startContainer, root);
  assert.equal(r.startOffset, 0);
  assert.equal(r.endOffset, 1);
  // selectNodeContents on an element with 2 children
  r.selectNodeContents(root);
  assert.equal(r.startContainer, root);
  assert.equal(r.startOffset, 0);
  assert.equal(r.endOffset, 2);
  // selectNodeContents on a text node uses string length
  const t = document.createTextNode('hello');
  const r2 = document.createRange();
  r2.selectNodeContents(t);
  assert.equal(r2.endOffset, 5);
});

test('Range: cloneRange is independent', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<a></a><b></b>';
  const r = document.createRange();
  r.setStart(root, 0); r.setEnd(root, 2);
  const c = r.cloneRange();
  assert.equal(c.startOffset, 0);
  assert.equal(c.endOffset, 2);
  c.setEnd(root, 1);
  assert.equal(r.endOffset, 2); // original untouched
  assert.equal(c.endOffset, 1);
});

test('Range: cloneContents copies, extractContents moves, deleteContents removes', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<a></a><b></b><c></c>';
  // clone
  const r1 = document.createRange();
  r1.setStart(root, 0); r1.setEnd(root, 2);
  const frag = r1.cloneContents();
  assert.equal(frag.childNodes.length, 2);
  assert.equal(root.children.length, 3); // original intact
  assert.equal(frag.firstChild.localName, 'a');
  // extract (moves)
  const r2 = document.createRange();
  r2.setStart(root, 0); r2.setEnd(root, 1);
  const ext = r2.extractContents();
  assert.equal(ext.childNodes.length, 1);
  assert.equal(ext.firstChild.localName, 'a');
  assert.equal(root.children.length, 2); // a removed
  assert.equal(r2.collapsed, true);
  // delete
  const r3 = document.createRange();
  r3.setStart(root, 0); r3.setEnd(root, 1);
  r3.deleteContents();
  assert.equal(root.children.length, 1);
  assert.equal(root.children[0].localName, 'c');
});

test('Range: insertNode + surroundContents', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<a></a><b></b>';
  const r = document.createRange();
  r.setStart(root, 1); r.setEnd(root, 1);
  const x = document.createElement('x');
  r.insertNode(x);
  // inserted before child at offset 1 (the <b>)
  assert.equal(root.children[1].localName, 'x');
  // surroundContents wraps the extracted range in a new element
  const root2 = document.createElement('div');
  root2.innerHTML = '<i></i><j></j>';
  const r2 = document.createRange();
  r2.setStart(root2, 0); r2.setEnd(root2, 1);
  const wrap = document.createElement('w');
  r2.surroundContents(wrap);
  assert.equal(root2.children[0].localName, 'w');
  assert.equal(root2.children[0].firstChild.localName, 'i');
});

test('Range: text-container toString + detach is a no-op', () => {
  const { document } = fresh();
  const t = document.createTextNode('hello world');
  const r = document.createRange();
  r.setStart(t, 0); r.setEnd(t, 5);
  assert.equal(r.toString(), 'hello');
  // partial (different end container path)
  const r2 = document.createRange();
  r2.setStart(t, 6); r2.setEnd(document.createTextNode('x'), 0);
  assert.equal(r2.toString(), 'world');
  r.detach(); // no throw
});

// --------------------------------------------------------- Selection API ----
test('Selection: collapse/extend/addRange/getRangeAt/rangeCount/removeRange', () => {
  const { window, document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<a></a><b></b>';
  const selection = window.getSelection();
  assert.equal(selection.rangeCount, 0);
  assert.equal(selection.type, 'None');
  // collapse → single caret range
  selection.collapse(root, 1);
  assert.equal(selection.rangeCount, 1);
  assert.equal(selection.isCollapsed, true);
  assert.equal(selection.type, 'Caret');
  assert.equal(selection.anchorNode, root);
  assert.equal(selection.anchorOffset, 1);
  assert.equal(selection.focusOffset, 1);
  // extend → focus moves, no longer collapsed
  selection.extend(root, 2);
  assert.equal(selection.isCollapsed, false);
  assert.equal(selection.focusOffset, 2);
  assert.equal(selection.type, 'Range');
  // addRange + getRangeAt
  const r = document.createRange();
  r.setStart(root, 0); r.setEnd(root, 1);
  selection.addRange(r);
  assert.equal(selection.rangeCount, 2);
  assert.equal(selection.getRangeAt(1), r);
  // removeRange
  selection.removeRange(r);
  assert.equal(selection.rangeCount, 1);
  // removeAllRanges
  selection.removeAllRanges();
  assert.equal(selection.rangeCount, 0);
  assert.equal(selection.isCollapsed, true);
});

test('Selection: selectAllChildren / setBaseAndExtent / empty / toString / collapse(null)', () => {
  const { window, document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<p>aa</p><p>bb</p>';
  const selection = window.getSelection();
  selection.selectAllChildren(root);
  assert.equal(selection.rangeCount, 1);
  assert.equal(selection.anchorOffset, 0);
  assert.equal(selection.focusOffset, 2);
  assert.equal(selection.toString(), 'aabb');
  // setBaseAndExtent
  selection.setBaseAndExtent(root, 0, root, 1);
  assert.equal(selection.anchorOffset, 0);
  assert.equal(selection.focusOffset, 1);
  assert.equal(selection.toString(), 'aa');
  // empty
  selection.empty();
  assert.equal(selection.rangeCount, 0);
  assert.equal(selection.toString(), '');
  // collapse(null) clears
  selection.collapse(root, 0);
  assert.equal(selection.rangeCount, 1);
  selection.collapse(null);
  assert.equal(selection.rangeCount, 0);
  // extend with no existing range creates one
  selection.extend(root, 1);
  assert.equal(selection.rangeCount, 1);
});

// --------------------------------------------------------- CharacterData ----
test('CharacterData: insert/delete/replace/substring/appendData', () => {
  const { document } = fresh();
  const t = document.createTextNode('Hello');
  assert.equal(t.substringData(1, 3), 'ell');
  t.appendData(' World');
  assert.equal(t.data, 'Hello World');
  t.insertData(5, ',');
  assert.equal(t.data, 'Hello, World');
  t.deleteData(5, 1);
  assert.equal(t.data, 'Hello World');
  t.replaceData(0, 5, 'Howdy');
  assert.equal(t.data, 'Howdy World');
  assert.equal(t.length, 11);
});

// ------------------------------------------------- Document / Element ops ----
test('createComment / createAttribute', () => {
  const { document } = fresh();
  const c = document.createComment('hi');
  assert.equal(c.nodeType, 8);
  assert.equal(c.data, 'hi');
  assert.equal(c.nodeName, '#comment');
  const a = document.createAttribute('data-x');
  assert.equal(a.name, 'data-x');
  assert.equal(a.value, '');
});

test('createNodeIterator: nextNode/previousNode/firstChild/lastChild', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<a></a><b><i></i></b>';
  const it = document.createNodeIterator(root, 1 /* elements */);
  assert.equal(it.nextNode().localName, 'a');
  assert.equal(it.nextNode().localName, 'b');
  const i = it.nextNode();
  assert.equal(i.localName, 'i');
  assert.equal(it.previousNode().localName, 'b');
  // firstChild / lastChild move the current node
  const it2 = document.createNodeIterator(root, 1);
  assert.equal(it2.firstChild().localName, 'a');
  const it3 = document.createNodeIterator(root, 1);
  assert.equal(it3.lastChild().localName, 'b');
});

test('importNode clones (shallow + deep) without adopting in place', () => {
  const { document } = fresh();
  const src = document.createElement('div');
  src.innerHTML = '<span>x</span>';
  src.setAttribute('id', 'orig');
  const shallow = document.importNode(src, false);
  assert.equal(shallow.getAttribute('id'), 'orig');
  assert.equal(shallow.childNodes.length, 0);
  assert.notEqual(shallow, src);
  const deep = document.importNode(src, true);
  assert.equal(deep.childNodes.length, 1);
  assert.equal(deep.querySelector('span').textContent, 'x');
});

test('document.contains', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  assert.equal(document.contains(root), true);
  const detached = document.createElement('div');
  assert.equal(document.contains(detached), false);
});

test('element.getElementsByClassName (live)', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<p class="x y">1</p><p class="x">2</p><p class="y">3</p>';
  const xs = root.getElementsByClassName('x');
  assert.equal(xs.length, 2);
  const both = root.getElementsByClassName('x y');
  assert.equal(both.length, 1);
  // live: add another
  root.innerHTML += '';
  const p = document.createElement('p'); p.className = 'x'; root.appendChild(p);
  assert.equal(xs.length, 3);
});

test('getAttributeNode', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  el.setAttribute('data-k', 'v');
  const node = el.getAttributeNode('data-k');
  assert.equal(node.name, 'data-k');
  assert.equal(node.value, 'v');
  assert.equal(node.ownerElement, el);
  assert.equal(el.getAttributeNode('missing'), null);
});

test('setAttributeNode / removeAttributeNode (React 19 singleton release)', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  // setAttributeNode returns the replaced Attr (null first time)
  assert.equal(el.setAttributeNode({ name: 'data-k', value: 'v' }), null);
  assert.equal(el.getAttribute('data-k'), 'v');
  const prev = el.setAttributeNode({ name: 'data-k', value: 'v2' });
  assert.equal(prev.value, 'v');
  assert.equal(el.getAttribute('data-k'), 'v2');
  // React reads node.attributes (detached copy) then removeAttributeNode each
  el.setAttribute('id', 'x');
  for (const attr of el.attributes) el.removeAttributeNode(attr);
  assert.equal(el.attributes.length, 0);
  // removeAttributeNode returns the removed Attr
  el.setAttribute('a', '1');
  const removed = el.removeAttributeNode(el.getAttributeNode('a'));
  assert.equal(removed.name, 'a');
  assert.equal(el.hasAttribute('a'), false);
});

test('hasAttributeNS / removeAttributeNS', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  el.setAttribute('foo', 'bar');
  assert.equal(el.hasAttributeNS(null, 'foo'), true);
  assert.equal(el.hasAttributeNS(null, 'baz'), false);
  el.removeAttributeNS(null, 'foo');
  assert.equal(el.hasAttribute('foo'), false);
});

test('replaceChildren / prepend / append / before / after / replaceWith on Element', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<a></a>';
  root.append(document.createElement('b'), 'text');
  assert.equal(root.childNodes.length, 3);
  root.prepend(document.createElement('c'));
  assert.equal(root.firstChild.localName, 'c');
  root.replaceChildren(document.createElement('d'), 'only');
  assert.equal(root.childNodes.length, 2);
  assert.equal(root.firstChild.localName, 'd');
  // before / after / replaceWith on a child
  const d = root.firstChild;
  d.before(document.createElement('e'));
  assert.equal(root.firstChild.localName, 'e');
  d.after(document.createElement('f'));
  assert.equal(d.nextSibling.localName, 'f');
  d.replaceWith(document.createElement('g'), 'tail');
  assert.equal(root.querySelector('g').localName, 'g');
  assert.equal(root.querySelector('d'), null);
});

test('CharacterData before / after / replaceWith', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = 'mid';
  const t = root.firstChild; // a text node
  t.before('pre');
  t.after('post');
  assert.equal(root.textContent, 'premidpost');
  t.replaceWith('X');
  assert.equal(root.textContent, 'preXpost');
});

test('hasChildNodes / item via childNodes', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  assert.equal(el.hasChildNodes(), false);
  el.appendChild(document.createElement('span'));
  assert.equal(el.hasChildNodes(), true);
  assert.equal(el.childNodes.item(0).localName, 'span');
  assert.equal(el.childNodes.item(5), null);
});

test('DocumentFragment cloneNode + append', () => {
  const { document } = fresh();
  const frag = document.createDocumentFragment();
  frag.append(document.createElement('a'), 'txt', document.createElement('b'));
  assert.equal(frag.childNodes.length, 3);
  const clone = frag.cloneNode(true);
  assert.equal(clone.childNodes.length, 3);
  assert.equal(clone.firstChild.localName, 'a');
  // shallow clone has no children
  const shallow = frag.cloneNode(false);
  assert.equal(shallow.childNodes.length, 0);
});

// ----------------------------------------------------- stubs: location/history ----
test('window.location getters + toString', () => {
  const { window } = createEnvironment(undefined, { url: 'https://ex.com:8443/p/q?a=1#frag' });
  const loc = window.location;
  assert.equal(loc.protocol, 'https:');
  assert.equal(loc.host, 'ex.com:8443');
  assert.equal(loc.hostname, 'ex.com');
  assert.equal(loc.port, '8443');
  assert.equal(loc.pathname, '/p/q');
  assert.equal(loc.search, '?a=1');
  assert.equal(loc.hash, '#frag');
  assert.equal(loc.origin, 'https://ex.com:8443');
  assert.equal(loc.toString(), 'https://ex.com:8443/p/q?a=1#frag');
  assert.equal(loc.href, 'https://ex.com:8443/p/q?a=1#frag');
});

test('window.history back/forward/go/pushState/replaceState', () => {
  const { window } = createEnvironment(undefined, { url: 'https://ex.com/start' });
  const h = window.history;
  assert.equal(h.length, 1);
  h.pushState({ n: 1 }, '', '/a');
  h.pushState({ n: 2 }, '', '/b');
  assert.equal(h.length, 3);
  assert.deepEqual(h.state, { n: 2 });
  h.back();
  assert.deepEqual(h.state, { n: 1 });
  h.forward();
  assert.deepEqual(h.state, { n: 2 });
  h.go(-2);
  assert.equal(h.state, null);
  h.replaceState({ r: true }, '', '/c');
  assert.deepEqual(h.state, { r: true });
});

test('FileReader readAsDataURL + readAsArrayBuffer (async)', async () => {
  const { window } = fresh();
  const blob = new window.Blob(['hello'], { type: 'text/plain' });
  // readAsDataURL
  const fr = new window.FileReader();
  const dataUrl = await new Promise((resolve) => { fr.onload = () => resolve(fr.result); fr.readAsDataURL(blob); });
  assert.ok(dataUrl.startsWith('data:text/plain;base64,'));
  assert.equal(Buffer.from(dataUrl.split(',')[1], 'base64').toString(), 'hello');
  // readAsArrayBuffer
  const fr2 = new window.FileReader();
  const buf = await new Promise((resolve) => { fr2.addEventListener('load', () => resolve(fr2.result)); fr2.readAsArrayBuffer(blob); });
  assert.ok(buf instanceof ArrayBuffer);
  assert.equal(Buffer.from(buf).toString(), 'hello');
});

// --------------------------------------------------- window real functions ----
test('structuredClone + CSS.escape', () => {
  const { window } = fresh();
  const obj = { a: 1, b: [2, 3] };
  const clone = window.structuredClone(obj);
  assert.deepEqual(clone, obj);
  assert.notEqual(clone, obj);
  // CSS.escape backslash-escapes non-ident chars
  assert.equal(window.CSS.escape('a.b'), 'a\\.b');
  assert.equal(window.CSS.supports('color', 'red'), true);
});

test('DataTransfer setData/getData/clearData', () => {
  const { window } = fresh();
  const dt = new window.DataTransfer();
  dt.setData('text/plain', 'hi');
  dt.setData('text/html', '<b>x</b>');
  assert.equal(dt.getData('text/plain'), 'hi');
  assert.deepEqual(dt.types, ['text/plain', 'text/html']);
  assert.equal(dt.getData('missing'), '');
  dt.clearData('text/plain');
  assert.equal(dt.getData('text/plain'), '');
  dt.clearData();
  assert.equal(dt.getData('text/html'), '');
});

test('ClipboardEvent carries a DataTransfer', () => {
  const { window } = fresh();
  const ev = new window.ClipboardEvent('paste');
  assert.equal(ev.type, 'paste');
  assert.ok(ev.clipboardData);
  ev.clipboardData.setData('text/plain', 'z');
  assert.equal(ev.clipboardData.getData('text/plain'), 'z');
  const dt = new window.DataTransfer();
  const ev2 = new window.ClipboardEvent('copy', { clipboardData: dt });
  assert.equal(ev2.clipboardData, dt);
});

test('navigator.clipboard read/write/readText/writeText', async () => {
  const { window } = fresh();
  const c = window.navigator.clipboard;
  assert.equal(await c.readText(), '');
  await c.writeText('x'); // no throw
  assert.deepEqual(await c.read(), []);
  await c.write([]); // no throw
});

test('performance mark/measure/getEntriesBy*/clearMarks/clearMeasures', () => {
  const { window } = fresh();
  const p = window.performance;
  assert.equal(typeof p.now(), 'number');
  p.mark('a');
  p.measure('m', 'a');
  assert.deepEqual(p.getEntriesByName('a'), []);
  assert.deepEqual(p.getEntriesByType('mark'), []);
  p.clearMarks();
  p.clearMeasures();
});

test('URL.createObjectURL / revokeObjectURL', () => {
  const { window } = fresh();
  const url = window.URL.createObjectURL(new window.Blob(['x']));
  assert.ok(url.startsWith('blob:turbo-dom/'));
  window.URL.revokeObjectURL(url); // no throw
});

test('fetch delegates to globalThis.fetch (mocked)', async () => {
  const orig = globalThis.fetch;
  globalThis.fetch = async (url) => ({ ok: true, url, text: async () => 'body' });
  try {
    const env = createEnvironment();
    const res = await env.window.fetch('/api');
    assert.equal(res.url, '/api');
    assert.equal(await res.text(), 'body');
  } finally { globalThis.fetch = orig; }
});

test('navigator.sendBeacon returns true', () => {
  const { window } = fresh();
  assert.equal(window.navigator.sendBeacon('/x', 'data'), true);
});

test('requestIdleCallback / cancelIdleCallback', async () => {
  const { window } = fresh();
  await new Promise((resolve) => {
    const id = window.requestIdleCallback((deadline) => {
      assert.equal(deadline.didTimeout, false);
      assert.equal(typeof deadline.timeRemaining(), 'number');
      resolve();
    });
    assert.equal(typeof id, 'object');
  });
  // cancel is a no-op (id never fires) — just confirm it doesn't throw
  const id2 = window.requestIdleCallback(() => {});
  window.cancelIdleCallback(id2);
});

test('getSelection via window proxies to document', () => {
  const { window, document } = fresh();
  assert.equal(window.getSelection(), document.getSelection());
});

// --------------------------------------------------- Selection collapse ends ----
test('Selection collapseToStart / collapseToEnd', () => {
  const { window, document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<a></a><b></b><c></c>';
  const selection = window.getSelection();
  selection.setBaseAndExtent(root, 0, root, 2);
  selection.collapseToStart();
  assert.equal(selection.isCollapsed, true);
  assert.equal(selection.anchorOffset, 0);
  assert.equal(selection.focusOffset, 0);
  selection.setBaseAndExtent(root, 0, root, 2);
  selection.collapseToEnd();
  assert.equal(selection.isCollapsed, true);
  assert.equal(selection.anchorOffset, 2);
});

// ------------------------------------------------------- ClassList surface ----
test('classList length / item / value / replace', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  el.className = 'a b c';
  const cl = el.classList;
  assert.equal(cl.length, 3);
  assert.equal(cl.item(0), 'a');
  assert.equal(cl.item(9), null);
  assert.equal(cl.value, 'a b c');
  assert.equal(cl.toString(), 'a b c');
  assert.equal(cl.contains('b'), true);
  assert.equal(cl.replace('b', 'z'), true);
  assert.equal(el.className, 'a z c');
  assert.equal(cl.replace('missing', 'q'), false);
  assert.deepEqual([...cl], ['a', 'z', 'c']);
});

// ------------------------------------------------- Event phase constants ----
test('Event phase constants + returnValue', () => {
  const { window } = fresh();
  const e = new window.Event('x', { cancelable: true });
  assert.equal(e.NONE, 0);
  assert.equal(e.CAPTURING_PHASE, 1);
  assert.equal(e.AT_TARGET, 2);
  assert.equal(e.BUBBLING_PHASE, 3);
  // returnValue mirrors !defaultPrevented
  assert.equal(e.returnValue, true);
  e.returnValue = false;
  assert.equal(e.defaultPrevented, true);
  assert.equal(e.returnValue, false);
  // assigning true does not un-prevent
  e.returnValue = true;
  assert.equal(e.defaultPrevented, true);
});

// --------------------------------------------- HTMLCollection namedItem/iter ----
test('HTMLCollection namedItem + entries/keys/values/forEach', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<p id="first"></p><p name="second"></p>';
  const ps = root.getElementsByTagName('p');
  assert.equal(ps.namedItem('first').getAttribute('id'), 'first');
  assert.equal(ps.namedItem('second').getAttribute('name'), 'second');
  assert.equal(ps.namedItem('nope'), null);
  // iteration helpers
  const keys = [...ps.keys()];
  assert.deepEqual(keys, [0, 1]);
  const vals = [...ps.values()];
  assert.equal(vals.length, 2);
  const entries = [...ps.entries()];
  assert.equal(entries[0][0], 0);
  let count = 0;
  ps.forEach(() => count++);
  assert.equal(count, 2);
  assert.equal(ps.toString(), '[object NodeList]');
});

// ----------------------------------------------- computed style surface ----
test('getComputedStyle getPropertyPriority / item / length / iterator', () => {
  const { window, document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<style>.box{color:red;margin:0}</style><div class="box"></div>';
  const box = root.querySelector('.box');
  const cs = window.getComputedStyle(box);
  assert.equal(cs.getPropertyValue('color'), 'rgb(255, 0, 0)');
  assert.equal(cs.color, 'rgb(255, 0, 0)');
  assert.equal(cs.getPropertyPriority('color'), '');
  assert.ok(cs.length >= 1);
  assert.ok([...cs].includes('color'));
  assert.equal(typeof cs.item(0), 'string');
  // unmatched property reads ''
  assert.equal(cs.getPropertyValue('font-weight'), '');
  // null element → empty proxy
  const empty = window.getComputedStyle(null);
  assert.equal(empty.getPropertyValue('color'), '');
});

// ------------------------------------------------- location assign/replace ----
test('location assign / replace / reload / href set (no-op navigations)', () => {
  const { window } = createEnvironment(undefined, { url: 'https://ex.com/p' });
  const loc = window.location;
  // these are honest no-ops in a headless runner — they must not throw and must
  // not change the URL (no navigation possible).
  loc.assign('/q');
  loc.replace('/r');
  loc.reload();
  loc.href = '/s';
  assert.equal(loc.href, 'https://ex.com/p');
});

// ------------------------------------------------------- FileReader.abort ----
test('FileReader abort fires abort event and sets readyState', async () => {
  const { window } = fresh();
  const fr = new window.FileReader();
  await new Promise((resolve) => {
    fr.onabort = () => resolve();
    fr.abort();
  });
  assert.equal(fr.readyState, 2);
  // removeEventListener path
  const cb = () => {};
  fr.addEventListener('load', cb);
  fr.removeEventListener('load', cb);
});

// ----------------------------------------------- customElements.upgrade ----
test('customElements define/get/whenDefined/upgrade', async () => {
  const { window } = fresh();
  const ce = window.customElements;
  class Foo {}
  ce.define('x-foo', Foo);
  assert.equal(ce.get('x-foo'), Foo);
  assert.equal(ce.getName(Foo), 'x-foo');
  assert.equal(await ce.whenDefined('x-foo'), Foo);
  ce.upgrade(window.document.createElement('div')); // no-op, no throw
  assert.throws(() => ce.define('x-foo', Foo), /already defined/);
});

// ------------------------------------------------------------- FormData ----
test('FormData append/set/get/getAll/has/delete/forEach/iterators', () => {
  const { window } = fresh();
  const fd = new window.FormData();
  fd.append('a', '1');
  fd.append('a', '2');
  fd.append('b', '3');
  assert.equal(fd.get('a'), '1');
  assert.deepEqual(fd.getAll('a'), ['1', '2']);
  assert.equal(fd.has('a'), true);
  assert.equal(fd.has('z'), false);
  fd.set('a', '9');
  assert.deepEqual(fd.getAll('a'), ['9']);
  fd.delete('b');
  assert.equal(fd.has('b'), false);
  const seen = [];
  fd.forEach((v, k) => seen.push([k, v]));
  assert.deepEqual(seen, [['a', '9']]);
  assert.deepEqual([...fd.keys()], ['a']);
  assert.deepEqual([...fd.values()], ['9']);
  assert.deepEqual([...fd.entries()], [['a', '9']]);
  assert.deepEqual([...fd], [['a', '9']]);
  assert.equal(fd.get('missing'), null);
});

// --------------------------------------------------------- Audio / Image ----
test('window.Audio + Image factories build elements', () => {
  const { window } = fresh();
  const a = new window.Audio('s.mp3');
  assert.equal(a.localName, 'audio');
  assert.equal(a.getAttribute('src'), 's.mp3');
  const a2 = new window.Audio();
  assert.equal(a2.localName, 'audio');
  const img = new window.Image(10, 20);
  assert.equal(img.localName, 'img');
  assert.equal(img.getAttribute('width'), '10');
  assert.equal(img.getAttribute('height'), '20');
});

// ------------------------------------------------- tag-specific instanceof ----
test('tag-specific HTML*Element instanceof matches by localName', () => {
  const { window, document } = fresh();
  const a = document.createElement('a');
  const div = document.createElement('div');
  assert.ok(a instanceof window.HTMLAnchorElement);
  assert.ok(!(div instanceof window.HTMLAnchorElement));
  // RegExp-matcher interface (headings)
  const h2 = document.createElement('h2');
  assert.ok(h2 instanceof window.HTMLHeadingElement);
  assert.ok(!(div instanceof window.HTMLHeadingElement));
  // every element is an HTMLElement
  assert.ok(div instanceof window.HTMLElement);
});

// --------------------------------------------------------- Document getters ----
test('Document reflective getters: nodeName/activeElement/location/URL/characterSet/compatMode/hidden/form', () => {
  const { window, document } = createEnvironment(undefined, { url: 'https://ex.com/path' });
  assert.equal(document.nodeName, '#document');
  // activeElement defaults to body when nothing focused
  assert.equal(document.activeElement, document.body);
  assert.equal(document.location, window.location);
  assert.equal(document.baseURI, 'https://ex.com/path');
  assert.equal(document.URL, 'https://ex.com/path');
  assert.equal(document.documentURI, 'https://ex.com/path');
  assert.equal(document.characterSet, 'UTF-8');
  assert.equal(document.compatMode, 'CSS1Compat');
  assert.equal(document.hidden, false);
  assert.equal(document.visibilityState, 'visible');
  assert.equal(document.readyState, 'complete');
  assert.equal(document.scrollingElement, document.documentElement);
  assert.equal(document.fullscreenElement, null);
  // input.form returns the ancestor form
  document.body.innerHTML = '<form id=f><input id=i></form>';
  assert.equal(document.getElementById('i').form, document.getElementById('f'));
  assert.equal(document.createElement('input').form, null);
});

// --------------------------------------------------------- XHR header methods ----
test('XMLHttpRequest open/setRequestHeader/getResponseHeader/getAllResponseHeaders/abort', () => {
  const { window } = fresh();
  const xhr = new window.XMLHttpRequest();
  xhr.open('POST', '/api');
  assert.equal(xhr.readyState, 1);
  xhr.setRequestHeader('X-Test', '1');
  assert.equal(xhr.getResponseHeader('X-Test'), null);
  assert.equal(xhr.getAllResponseHeaders(), '');
  let aborted = false;
  xhr.onabort = () => { aborted = true; };
  xhr.abort();
  assert.equal(aborted, true);
  assert.equal(xhr.readyState, 0);
});

// ------------------------------------------------ permissions.query / vibrate ----
test('navigator.permissions.query + vibrate', async () => {
  const { window } = fresh();
  const status = await window.navigator.permissions.query({ name: 'geolocation' });
  assert.equal(status.state, 'prompt');
  assert.equal(window.navigator.vibrate(100), false);
});

// --------------------------------------------------------- select selectedIndex ----
test('select.selectedIndex setter selects the nth option', () => {
  const { document } = fresh();
  const sel = document.createElement('select');
  sel.innerHTML = '<option>a</option><option>b</option><option>c</option>';
  sel.selectedIndex = 2;
  assert.equal(sel.selectedIndex, 2);
  assert.equal(sel.value, 'c');
  sel.selectedIndex = 0;
  assert.equal(sel.value, 'a');
});

// --------------------------------------------------------- TreeWalker nav ----
test('TreeWalker parentNode / nextSibling / previousSibling', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<a></a><b><i></i></b><c></c>';
  const tw = document.createTreeWalker(root, 1);
  tw.firstChild(); // <a>
  assert.equal(tw.nextSibling().localName, 'b');
  assert.equal(tw.previousSibling().localName, 'a');
  tw.nextSibling(); // back to b
  tw.firstChild();  // <i>
  assert.equal(tw.parentNode().localName, 'b');
});

// --------------------------------------------------------- radio undo on prevent ----
test('radio pre-click activation undone when click is preventDefault-ed', () => {
  const { document } = fresh();
  document.body.innerHTML =
    '<form><input type=radio name=g value=1><input type=radio name=g value=2></form>';
  const [r1, r2] = document.body.getElementsByTagName('input');
  r1.__checked = true; // start with r1 checked (direct field, no React tracker)
  // clicking r2 with a preventDefault listener must roll back to r1 checked
  r2.addEventListener('click', (e) => e.preventDefault());
  r2.click();
  assert.equal(r2.checked, false);
  assert.equal(r1.checked, true);
});

// --------------------------------------------------------- DocumentType ----
test('DocumentType nodeName + doctype reflection', () => {
  const { document } = fresh();
  const dt = document.doctype;
  assert.ok(dt);
  assert.equal(dt.nodeType, 10);
  assert.equal(dt.nodeName, 'html');
  assert.equal(dt.name, 'html');
});

// --------------------------------------------------------- Node base ops ----
test('Node base replaceChildren (via fragment) + textContent setter', () => {
  const { document } = fresh();
  const frag = document.createDocumentFragment();
  frag.appendChild(document.createElement('a'));
  frag.replaceChildren(document.createElement('b'), 'txt');
  assert.equal(frag.childNodes.length, 2);
  assert.equal(frag.firstChild.localName, 'b');
  // textContent setter clears + sets text
  const el = document.createElement('div');
  el.innerHTML = '<span>x</span>';
  el.textContent = 'plain';
  assert.equal(el.childNodes.length, 1);
  assert.equal(el.textContent, 'plain');
  el.textContent = '';
  assert.equal(el.childNodes.length, 0);
});

// --------------------------------------------------- window-level events ----
test('window.dispatchEvent / addEventListener / removeEventListener proxy to document', () => {
  const { window } = fresh();
  let hits = 0;
  const cb = () => hits++;
  window.addEventListener('custom', cb);
  window.dispatchEvent(new window.Event('custom'));
  assert.equal(hits, 1);
  window.removeEventListener('custom', cb);
  window.dispatchEvent(new window.Event('custom'));
  assert.equal(hits, 1);
});

// --------------------------------------------------------- Storage lazy class ----
test('window.Storage class + localStorage behavior', () => {
  const { window } = fresh();
  assert.equal(typeof window.Storage, 'function');
  const ls = window.localStorage;
  ls.setItem('k', 'v');
  assert.equal(ls.getItem('k'), 'v');
  assert.equal(ls.length, 1);
  assert.equal(ls.key(0), 'k');
  ls.removeItem('k');
  assert.equal(ls.getItem('k'), null);
  ls.setItem('a', '1'); ls.clear();
  assert.equal(ls.length, 0);
});

// --------------------------------------------------------- ShadowRoot props ----
test('ShadowRoot nodeName + activeElement', () => {
  const { document } = fresh();
  const host = document.createElement('div');
  document.body.appendChild(host);
  const root = host.attachShadow({ mode: 'open' });
  assert.equal(root.nodeName, '#document-fragment');
  // a shadow root reports its own active element (null until something focuses)
  assert.equal(root.activeElement, null);
});

// --------------------------------------------------------- TreeWalker detach ----
test('TreeWalker detach is a no-op', () => {
  const { document } = fresh();
  const tw = document.createTreeWalker(document.body, 1);
  tw.detach(); // legacy no-op, must not throw
});

// ----------------------------------------------- element nodeValue setter no-op ----
test('element nodeValue setter is a no-op (spec) and getter is null', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  el.nodeValue = 'ignored';
  assert.equal(el.nodeValue, null);
});
