// Edges of the Chrome 156 selector grammar and matching rules in src/runtime/selectors.mjs
// that the Chrome control (test/selectors-chrome.test.mjs) does not reach: escapes, string
// and attribute syntax, pseudo arity and placement, :scope, :defined with a registry,
// :dir() auto directionality, foreign (SVG) elements (miaskiewicz/turbo-dom#18).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';
import { _internal, matchesSelector } from '../src/runtime/selectors.mjs';

const { parseSelectorList, parseAnB, ruleSelectors, specificity, compareSpecificity } = _internal;
const one = (sel) => parseSelectorList(sel)[0].first;
const bad = (sel) => assert.throws(() => parseSelectorList(sel), SyntaxError, JSON.stringify(sel));
const env = (body) => createEnvironment(`<!doctype html><html><head></head><body>${body}</body></html>`);

test('CSS escapes decode in identifiers (hex, terminators, invalid code points, end of input)', () => {
  assert.deepEqual(one('.md\\3a\r\nflex').classes, ['md:flex']); // CRLF ends a hex escape
  assert.deepEqual(one('.a\\:b').classes, ['a:b']); // non-hex escape
  assert.deepEqual(one('#\\31 23').id, '123'); // space ends a hex escape
  assert.deepEqual(one('.\\0').classes, ['�']); // NUL
  assert.deepEqual(one('.\\110000').classes, ['�']); // beyond Unicode
  assert.deepEqual(one('.\\d800').classes, ['�']); // surrogate
  assert.deepEqual(one('.a\\').classes, ['a�']); // a trailing backslash
  assert.deepEqual(one('.\\😀').classes, ['😀']); // an escaped astral code point
  assert.deepEqual(one(`.a${String.fromCharCode(0)}b`).classes, ['a�b']);
  assert.deepEqual(one('.-\\31').classes, ['-1']);
  assert.deepEqual(one('.--').classes, ['--']);
  bad('.a\\\nb'); // a backslash before a newline is not an escape
  bad('.\\\n');
  bad('.-1');
  bad('.-');
});

test('quoted attribute values: escapes and line continuations', () => {
  assert.deepEqual(one('[a="b\\\nc"]').attrs[0].match.value, 'bc');
  assert.deepEqual(one('[a="b\\\r\nc"]').attrs[0].match.value, 'bc');
  assert.deepEqual(one('[a="b\\\rc"]').attrs[0].match.value, 'bc');
  assert.deepEqual(one('[a="b\\\fc"]').attrs[0].match.value, 'bc');
  assert.deepEqual(one('[a="\\41 x"]').attrs[0].match.value, 'Ax');
  assert.deepEqual(one("[a='it\\'s']").attrs[0].match.value, "it's");
  assert.deepEqual(one('[a="]\\"" i]').attrs[0], { name: 'a', match: { op: 'equals', value: ']"' }, ci: true });
  assert.deepEqual(one('[a="x" I]').attrs[0].ci, true);
  for (const s of ['[|=x]', '[a b]', '[a=b i x]', '[a ~ = b]', '[a="b"c]', '[a=b/*', '[a/*x]', '[*|]', '[|]']) bad(s);
  // comments outside quotes are whitespace; inside quotes they are text
  assert.deepEqual(one('[a="/*x*/"]').attrs[0].match.value, '/*x*/');
  bad('[a=\\/*x*/ i]'); // an escaped `/` does not open a comment
});

test('type selectors, namespace prefixes and stray characters', () => {
  assert.deepEqual(one('*|*'), { tag: '*', ns: 'any', id: null, classes: [], attrs: [], pseudos: [], pe: null });
  assert.deepEqual(one('|p').ns, 'none');
  assert.deepEqual(one('_p').tag, '_p');
  assert.deepEqual(one('-p').tag, '-p');
  for (const s of ['|', '*|', 'a|p', '*|.x', '.x|p', 'p!', '!p', 'a ,', '> p', 'a >> b', 'a + > b', 'a ~ ~ b']) bad(s);
  // a whitespace run before a combinator is not a second combinator
  assert.equal(parseSelectorList('a  >  b')[0].rest[0].combinator, '>');
  assert.equal(parseSelectorList('a\tb')[0].rest[0].combinator, ' ');
});

test('pseudo-class arity, arguments and nesting', () => {
  for (const s of [':hover(1)', ':host-context', ':nth-last-of-type(of p)', ':dir()', ':lang(1)', ':state(a b)', ':frob(x)', ':not()', ':has()', ':nth-child(1 of)', '::slotted(a, b)', ':before(x)', '::before::after', 'p::before > span', ':has(> :not(:has(a)))']) bad(s);
  // a forgiving :is() inside :has() drops its items with a nested :has() (Chrome 156)
  assert.equal(one(':has(:is(p, :has(a)))').pseudos[0].parsed.rel[0].complex.first.pseudos[0].parsed.list.length, 1);
  // forgiving lists drop invalid items (a fully invalid list matches nothing)
  assert.equal(one(':is(:bogus, p)').pseudos[0].parsed.list.length, 1);
  assert.equal(one(':where(:bogus)').pseudos[0].parsed.list.length, 0);
  assert.equal(one(':-webkit-any(p, ::before)').pseudos[0].parsed.list.length, 1);
  assert.deepEqual(one(':nth-child(2n+1 of .a, .b)').pseudos[0].parsed.of.length, 2);
  assert.equal(one(':has(> a, + b, ~ c, d)').pseudos[0].parsed.rel.map((r) => r.combinator).join(''), '>+~ ');
  assert.equal(one(':state(on)').pseudos[0].parsed.v, 'on');
  assert.equal(one(':host(.x)').pseudos[0].parsed.never, true);
  assert.equal(one('::cue(b)').pe.name, 'cue');
  assert.equal(one('::-webkit-foo').pe.name, '-webkit-foo');
});

test('An+B grammar', () => {
  assert.deepEqual(parseAnB(' -n+3 '), { a: -1, b: 3 });
  assert.deepEqual(parseAnB('+5'), { a: 0, b: 5 });
  assert.deepEqual(parseAnB('-5'), { a: 0, b: -5 });
  assert.deepEqual(parseAnB('2n- 1'), { a: 2, b: -1 });
  assert.deepEqual(parseAnB('N'), { a: 1, b: 0 });
  assert.deepEqual(parseAnB('EVEN'), { a: 2, b: 0 });
  for (const s of ['', '+', 'n-', '2n+', '+ 2n', '2 n', '2.5n', '2n+-1', 'n1', '2n 1 ']) assert.equal(parseAnB(s), null, s);
});

test('ruleSelectors splits a valid list at top-level commas and drops an invalid one', () => {
  assert.deepEqual(ruleSelectors(':is(a, b), [t="x,y"], c\\,d'), [':is(a, b)', '[t="x,y"]', 'c\\,d']);
  assert.deepEqual(ruleSelectors('a, :bogus'), []);
  assert.deepEqual(ruleSelectors('p::before, :host(.x), ::slotted(span)'), ['p::before', ':host(.x)', '::slotted(span)']);
});

test('specificity follows Selectors 4 (mirrors the Rust specificity_follows_selectors_4)', () => {
  const rows = {
    '*': [0, 0, 0], '*|*': [0, 0, 0], '|p': [0, 0, 1], 'div p.a[x]:hover': [0, 3, 2], '#a .b': [1, 1, 0],
    ':where(#a, .b) p': [0, 0, 1], ':is(#a, .b)': [1, 0, 0], ':is(.b, :bogus)': [0, 1, 0],
    ':not(div, .a.b)': [0, 2, 0], ':not(div)': [0, 0, 1], 'p:has(> #x, .y)': [1, 0, 1],
    ':nth-child(2 of #a, .b)': [1, 1, 0], ':nth-last-child(odd of p)': [0, 1, 1], ':nth-child(2)': [0, 1, 0],
    'p::before': [0, 0, 2], 'p:before': [0, 0, 2], 'slot::slotted(.a)': [0, 1, 2], ':-webkit-any(#a, p)': [0, 1, 0],
    ':scope': [0, 1, 0], ':FIRST-CHILD': [0, 1, 0], '.\\31 0': [0, 1, 0], '.md\\:x': [0, 1, 0],
    '[title="#t"]': [0, 1, 0], 'div :is(i)': [0, 0, 2], ':host(.x)': [0, 1, 0], 'p::-webkit-scrollbar:hover': [0, 1, 2],
  };
  for (const [sel, want] of Object.entries(rows)) assert.deepEqual(specificity(sel), want, sel);
  assert.ok(compareSpecificity([1, 0, 0], [0, 99, 99]) > 0);
  assert.ok(compareSpecificity([0, 1, 2], [0, 1, 2]) === 0);
  assert.ok(compareSpecificity([0, 0, 1], [0, 0, 2]) < 0);
});

test(':scope is the queried element, the document element for a document, nothing in a fragment', () => {
  const { document } = env('<div id="d"><p id="p">x</p></div>');
  const d = document.getElementById('d');
  assert.equal(d.querySelectorAll(':scope > p').length, 1);
  assert.ok(d.matches(':scope'));
  assert.equal(document.querySelectorAll(':scope').length, 1);
  assert.equal(document.querySelector(':scope'), document.documentElement);
  const f = document.createDocumentFragment();
  f.append(document.createElement('p'));
  assert.equal(f.querySelectorAll(':scope').length, 0);
  assert.ok(d.closest(':scope') === d);
  // a stylesheet's :scope is :root
  assert.ok(matchesSelector(document.documentElement, ':scope', null));
  assert.ok(!matchesSelector(d, ':scope', null));
});

test(':defined consults the window registry; is= and autonomous names are undefined until defined', () => {
  const { window, document } = env('<x-def id="x"></x-def><x-undef id="u"></x-undef><button is="x-btn" id="b"></button><font-face id="f"></font-face>');
  const el = (id) => document.getElementById(id);
  assert.ok(!el('x').matches(':defined') && !el('b').matches(':defined'));
  assert.ok(el('f').matches(':defined'));
  window.customElements.define('x-def', class extends window.HTMLElement {});
  window.customElements.define('x-btn', class extends window.HTMLElement {});
  assert.ok(el('x').matches(':defined') && el('b').matches(':defined'));
  assert.ok(el('u').matches(':not(:defined)'));
});

test(':dir() auto directionality and inheritance edges', () => {
  const { document } = env(
    '<div dir="rtl" id="host"></div><div dir="auto" id="astral">\u{10900}x</div>' +
      '<div dir="auto" id="deep"><p><span>ש</span></p></div><textarea dir="auto" id="ta">abc</textarea>' +
      '<input dir="auto" type="checkbox" value="ש" id="cb"><input dir="auto" type="frob" value="ש" id="frob">' +
      '<input dir="auto" type="email" value="1" id="num"><div dir="auto" id="none">1 2</div><div dir="auto" id="cmt"><!--ש-->x</div>',
  );
  const el = (id) => document.getElementById(id);
  const host = el('host');
  const shadow = host.attachShadow({ mode: 'open' });
  shadow.innerHTML = '<span>s</span>';
  assert.ok(shadow.firstChild.matches(':dir(rtl)'), 'shadow content inherits the host');
  assert.ok(el('astral').matches(':dir(rtl)'), 'an astral RTL letter');
  assert.ok(el('deep').matches(':dir(rtl)'));
  assert.ok(el('ta').matches(':dir(ltr)'));
  assert.ok(el('cb').matches(':dir(ltr)'), 'a checkbox value is not text');
  assert.ok(el('frob').matches(':dir(rtl)'), 'an unknown input type is Text');
  assert.ok(el('num').matches(':dir(ltr)'), 'a non-empty value without a strong character');
  assert.ok(el('none').matches(':dir(ltr)'));
  assert.ok(el('cmt').matches(':dir(ltr)'));
  assert.ok(!host.matches(':dir(auto)') && !host.matches(':dir(foo)'));
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'g');
  host.append(svg);
  assert.ok(svg.matches(':dir(rtl)'), 'non-HTML elements inherit');
});

test('foreign elements: case-insensitive types, prefixed attributes, SVG links', () => {
  const { document } = env('<svg><foreignObject id="fo"></foreignObject><a id="sa" xlink:href="#x"></a><a id="sn"></a><g id="g" class="c" data-x="1"></g></svg>');
  const fo = document.getElementById('fo');
  assert.ok(fo.matches('foreignobject') && fo.matches('FOREIGNOBJECT') && fo.matches('foreignObject'));
  assert.equal(document.querySelectorAll('foreignobject').length, 1);
  const sa = document.getElementById('sa');
  assert.ok(sa.matches(':link') && !document.getElementById('sn').matches(':any-link'));
  assert.ok(sa.matches('[*|href]') && !sa.matches('[href]') && !sa.matches('[|href]'));
  assert.ok(sa.matches('[*|href^="#" i]') && !sa.matches('[*|href="y"]'));
  assert.ok(document.getElementById('g').matches('g.c#g[data-x="1"]'));
  assert.ok(!document.getElementById('g').matches('|g') && document.getElementById('g').matches('*|g'));
});

test('attribute operators, the i modifier and the implicit case-insensitive HTML type', () => {
  const { document } = env('<input id="i" type="CheckBox" data-v="Ab cD-ef"><p id="p" data-v="x">p</p>');
  const i = document.getElementById('i');
  for (const s of ['[type=checkbox]', '[data-v~=Ab]', '[data-v$=ef]', '[data-v^="ab" i]', '[data-v$=EF i]', '[data-v*="B C" i]', '[data-v="ab cd-ef" i]']) {
    assert.ok(i.matches(s), s);
  }
  for (const s of ['[data-v~="b c"]', '[data-v~=""]', '[data-v^=""]', '[data-v$=""]', '[data-v*=""]', '[data-v|=cD]', '[data-v|=Ab]', '[data-v^=ab]']) {
    assert.ok(!i.matches(s), s);
  }
  assert.ok(!document.getElementById('p').matches('[type]'));
});

test('form state, :empty, :link and structural edges', () => {
  const { document } = env(
    '<fieldset disabled><legend><input id="l"></legend><input id="in"></fieldset><select disabled><option id="o">x</option></select>' +
      '<div contenteditable="FALSE"><span id="ro">x</span></div><div contenteditable="bogus"><b id="inh">x</b></div>' +
      '<input id="txt" readonly><textarea id="ta"></textarea><p id="e"><!--c--></p><p id="t">x</p>' +
      '<a id="a" href="/x">a</a><area id="ar"><map></map><ul><li class="x">1</li><li>2</li><li class="x">3</li></ul>',
  );
  const el = (id) => document.getElementById(id);
  assert.ok(el('l').matches(':enabled') && el('in').matches(':disabled') && el('o').matches(':disabled'));
  el('in').disabled = false;
  assert.ok(el('in').matches(':disabled'), 'the fieldset still disables it');
  assert.ok(el('ro').matches(':read-only') && el('inh').matches(':read-only'));
  assert.ok(el('txt').matches(':read-only') && el('ta').matches(':read-write'));
  assert.ok(el('e').matches(':empty') && !el('t').matches(':empty'));
  el('e').append(document.createTextNode(''));
  assert.ok(el('e').matches(':empty'), 'an empty text node does not count');
  assert.ok(el('a').matches(':any-link') && !el('ar').matches(':link'));
  assert.equal(document.querySelectorAll('li:nth-last-child(1 of .x)').length, 1);
  assert.equal(document.querySelectorAll('li:nth-child(even)').length, 1);
  assert.equal(document.querySelectorAll('li:last-of-type').length, 1);
  const lone = document.createElement('li');
  lone.className = 'x';
  assert.ok(lone.matches(':nth-child(1 of .x)') && lone.matches(':only-of-type') && !lone.matches(':nth-child(1 of .y)'));
  assert.ok(lone.matches(':first-of-type:last-child'));
});

test('pseudo-elements never match; :has() combinators and empty results', () => {
  const { document } = env('<div id="d"><h3 id="h">t</h3><p>a</p><span><i>x</i></span></div>');
  assert.equal(document.querySelectorAll('p::before').length, 0);
  assert.equal(document.querySelectorAll('h3:has(+ p)').length, 1);
  assert.equal(document.querySelectorAll('h3:has(~ span i)').length, 1);
  assert.equal(document.querySelectorAll('h3:has(+ span)').length, 0);
  assert.equal(document.querySelectorAll('div:has(> span > i)').length, 1);
  assert.equal(document.querySelectorAll('div:has(> i)').length, 0);
  assert.equal(document.querySelector('i:has(*)'), null);
  assert.equal(document.querySelectorAll(':is(h3, p) ~ span').length, 1);
});
