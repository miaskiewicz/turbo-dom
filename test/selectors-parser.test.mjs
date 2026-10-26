import { test } from 'node:test';
import assert from 'node:assert/strict';
import { _internal } from '../src/runtime/selectors.mjs';
import { createEnvironment } from '../src/runtime/index.mjs';

const { parseSelectorList, parseComplex } = _internal;

// Parse a single complex selector (no top-level comma) and return its AST.
const one = (sel) => parseSelectorList(sel)[0];
// Build a fresh compound AST literal with only the fields a test cares about.
const cmp = (o = {}) => ({ tag: null, ns: null, id: null, classes: [], attrs: [], pseudos: [], pe: null, ...o });
// Build a Complex AST literal: a leftmost compound + (combinator, compound) tail
// pairs. Mirrors the "parse, don't validate" shape `{ first, rest }` the matcher
// consumes — no parallel compounds[]/combinators[] arrays that can desync.
const cx = (first, ...rest) => ({ first, rest });
const seg = (combinator, compound) => ({ combinator, compound });
// Attr AST literals: presence carries NO value key; valued folds { op, value }.
const present = (name) => ({ name, match: { op: 'present' } });
const attr = (name, op, value) => ({ name, match: { op, value } });

// ── Parity corpus (mirrors the parallel Rust selector-parser effort) ─────────
// Each entry encodes a tricky tokenization/grammar case; we assert the exact AST
// the unchanged matcher consumes.
test('parity corpus: compounds, combinators, attrs, pseudos', () => {
  assert.deepEqual(one('div.card'), cx(cmp({ tag: 'div', classes: ['card'] })));
  assert.deepEqual(one('.grid .t'), cx(cmp({ classes: ['grid'] }), seg(' ', cmp({ classes: ['t'] }))));
  assert.deepEqual(one('main > div'), cx(cmp({ tag: 'main' }), seg('>', cmp({ tag: 'div' }))));
  assert.deepEqual(one('a[href]'), cx(cmp({ tag: 'a', attrs: [present('href')] })));
  assert.deepEqual(one('a[data-k=v]'), cx(cmp({ tag: 'a', attrs: [attr('data-k', 'equals', 'v')] })));
  assert.deepEqual(one("[data-k='v']"), cx(cmp({ attrs: [attr('data-k', 'equals', 'v')] })));
  assert.deepEqual(one("a[href^='/docs']"), cx(cmp({ tag: 'a', attrs: [attr('href', 'prefix', '/docs')] })));
  assert.deepEqual(one("a[data-x*='oba']"), cx(cmp({ tag: 'a', attrs: [attr('data-x', 'substr', 'oba')] })));
  assert.deepEqual(one("a[class~='primary']"), cx(cmp({ tag: 'a', attrs: [attr('class', 'includes', 'primary')] })));
  assert.deepEqual(one("a[lang|='en']"), cx(cmp({ tag: 'a', attrs: [attr('lang', 'dash', 'en')] })));
  // value containing spaces (quotes must protect the whitespace from tokenizing)
  assert.deepEqual(one('svg[viewBox="0 0 10 10"]'), cx(cmp({ tag: 'svg', attrs: [attr('viewBox', 'equals', '0 0 10 10')] })));
  // An+B with interior whitespace preserved in the raw arg
  assert.deepEqual(one('li:nth-child(2n + 1)'), cx(cmp({ tag: 'li', pseudos: [{ name: 'nth-child', arg: '2n + 1' }] })));
  // nested parens inside :not survive intact for the recursive re-parse
  assert.deepEqual(one(':not(:nth-child(2))'), cx(cmp({ pseudos: [{ name: 'not', arg: ':nth-child(2)' }] })));
  assert.deepEqual(
    one('.a > .b .c'),
    cx(cmp({ classes: ['a'] }), seg('>', cmp({ classes: ['b'] })), seg(' ', cmp({ classes: ['c'] }))),
  );
  assert.deepEqual(one('p + i'), cx(cmp({ tag: 'p' }), seg('+', cmp({ tag: 'i' }))));
  assert.deepEqual(one('p ~ b'), cx(cmp({ tag: 'p' }), seg('~', cmp({ tag: 'b' }))));
  // namespace prefixes and escaped / non-ASCII identifiers (decoded)
  assert.deepEqual(one('*|p'), cx(cmp({ tag: 'p', ns: 'any' })));
  assert.deepEqual(one('|*'), cx(cmp({ tag: '*', ns: 'none' })));
  assert.deepEqual(one('.md\\:flex #\\31 23 .café'), cx(cmp({ classes: ['md:flex'] }), seg(' ', cmp({ id: '123' })), seg(' ', cmp({ classes: ['café'] }))));
});

// ── Chrome 156 grammar (miaskiewicz/turbo-dom#18): no leniency ──────────────
test('a type selector must come first in its compound (div[id=x]y is a SyntaxError)', () => {
  const { document } = createEnvironment('<!doctype html><body><div id="x">hi</div></body>');
  assert.throws(() => document.querySelectorAll('div[id=x]y'), SyntaxError);
  for (const bad of ['.foo*', 'div*', '[a]div', ':not(p)x']) assert.throws(() => parseSelectorList(bad), SyntaxError, bad);
  assert.deepEqual(one('*'), cx(cmp({ tag: '*' })));
  assert.deepEqual(one('DIV'), cx(cmp({ tag: 'div' }))); // type lowercased
});

// ── Throw cases preserved exactly ────────────────────────────────────────────
test('malformed selectors still throw SyntaxError', () => {
  assert.throws(() => parseSelectorList('>'), SyntaxError); // leading combinator ⇒ empty compound
  assert.throws(() => parseSelectorList('div!p'), SyntaxError); // unexpected char
  assert.throws(() => parseSelectorList('[href'), SyntaxError); // unterminated attribute
  assert.throws(() => parseSelectorList('a > > b'), SyntaxError); // empty compound between combinators
  assert.throws(() => parseSelectorList('a + > b'), SyntaxError);
});

// ── Empty / comma / whitespace edges: SyntaxErrors in Chrome 156 ─────────────
test('empty selectors and empty list items are SyntaxErrors', () => {
  for (const bad of ['', '   ', ',', 'a,,b', 'a,', ',a']) assert.throws(() => parseSelectorList(bad), SyntaxError, JSON.stringify(bad));
  assert.deepEqual(parseSelectorList('a, b'), [cx(cmp({ tag: 'a' })), cx(cmp({ tag: 'b' }))]);
});

test('leading/trailing/repeated whitespace is trimmed; all whitespace kinds work', () => {
  assert.deepEqual(one('  a  b  '), cx(cmp({ tag: 'a' }), seg(' ', cmp({ tag: 'b' }))));
  assert.deepEqual(one(' div '), cx(cmp({ tag: 'div' })));
  // tab + newline behave as descendant whitespace just like a space
  assert.deepEqual(one('a\tb'), cx(cmp({ tag: 'a' }), seg(' ', cmp({ tag: 'b' }))));
  assert.deepEqual(one('a\r\n>\fb'), cx(cmp({ tag: 'a' }), seg('>', cmp({ tag: 'b' }))));
});

test('a dangling trailing combinator is a SyntaxError (a >)', () => {
  const { document } = createEnvironment('<!doctype html><body><a>x</a><a>y</a></body>');
  assert.throws(() => document.querySelectorAll('a >'), SyntaxError);
  assert.throws(() => parseSelectorList('a ~'), SyntaxError);
  assert.equal(document.querySelectorAll('a').length, 2);
});

// ── Attribute interior parsing ───────────────────────────────────────────────
test('attribute parsing: presence, every operator, quote stripping, empty values', () => {
  assert.deepEqual(one('[a]').first.attrs, [present('a')]);
  // whitespace around name/op/value is tolerated and trimmed
  assert.deepEqual(one('[ data-k = v ]').first.attrs, [attr('data-k', 'equals', 'v')]);
  // empty quoted values
  assert.deepEqual(one('[title=""]').first.attrs, [attr('title', 'equals', '')]);
  assert.deepEqual(one("[title='']").first.attrs, [attr('title', 'equals', '')]);
  assert.deepEqual(one('[href^=""]').first.attrs, [attr('href', 'prefix', '')]);
  // an unquoted value is an identifier: empty, `index.html`, a number or trailing junk
  // after a string are SyntaxErrors (Chrome 156)
  for (const bad of ['[a=]', '[href=index.html]', '[a=1]', '[x="a"b]', '[1a]', '[ns|a]', '[a=b s]']) {
    assert.throws(() => parseSelectorList(bad), SyntaxError, bad);
  }
  assert.deepEqual(one('[href="index.html"]').first.attrs, [attr('href', 'equals', 'index.html')]);
  assert.deepEqual(one('[a=\\31]').first.attrs, [attr('a', 'equals', '1')]); // escaped ident
  // a ']' inside a quoted value must NOT close the attribute
  assert.deepEqual(one('[a="]"]').first.attrs, [attr('a', 'equals', ']')]);
  // the `i` modifier and the `*|` prefix
  assert.deepEqual(one('[a="B" i]').first.attrs, [{ ...attr('a', 'equals', 'B'), ci: true }]);
  assert.deepEqual(one('[*|a]').first.attrs, [{ ...present('a'), ns: 'any' }]);
  assert.deepEqual(one('[|a]').first.attrs, [present('a')]);
  // comments are allowed inside the brackets
  assert.deepEqual(one('[a/**/="b"/**/]').first.attrs, [attr('a', 'equals', 'b')]);
  assert.throws(() => parseSelectorList('[a/*="b"]'), SyntaxError);
  // multiple attributes in one compound
  assert.deepEqual(one('a[b][c]').first.attrs, [present('b'), present('c')]);
  // each operator folds to its named op (no value key on presence)
  assert.equal(one('[x~=y]').first.attrs[0].match.op, 'includes');
  assert.equal(one('[x|=y]').first.attrs[0].match.op, 'dash');
  assert.equal(one('[x$=y]').first.attrs[0].match.op, 'suffix');
  assert.deepEqual(one('[a]').first.attrs[0].match, { op: 'present' }); // presence has no value key
});

// ── Pseudo parsing ───────────────────────────────────────────────────────────
test('pseudo parsing: no-arg, arg, nested + quoted parens, unterminated arg', () => {
  assert.deepEqual(one(':first-child').first.pseudos, [{ name: 'first-child', arg: null }]);
  assert.deepEqual(one(':FIRST-CHILD').first.pseudos, [{ name: 'first-child', arg: null }]); // case-insensitive
  assert.deepEqual(one(':not(.x)').first.pseudos, [{ name: 'not', arg: '.x' }]);
  // a quoted ')' inside the arg does not terminate it (double- and single-quoted)
  assert.deepEqual(one(':not([title=")"])').first.pseudos, [{ name: 'not', arg: '[title=")"]' }]);
  assert.deepEqual(one(":not([title=')'])").first.pseudos, [{ name: 'not', arg: "[title=')']" }]);
  // a pseudo-element is kept apart from the pseudo-classes (and never matches)
  assert.deepEqual(one('div::before').first, { ...cmp({ tag: 'div' }), pe: { name: 'before', arg: null } });
  assert.deepEqual(one('p:after').first.pe, { name: 'after', arg: null }); // legacy single colon
  // an unterminated argument closes at the end of the selector (CSS)
  assert.deepEqual(one(':not(.x').first.pseudos, [{ name: 'not', arg: '.x' }]);
  // SyntaxErrors (Chrome 156): arity, An+B, unknown names, misplaced pseudo-elements
  for (const bad of [':nth-child()', ':nth-child', ':first-child(1)', ':is', ':bogus', '::bogus', ':selected', '::before.x', '::before span', ':not(::before)', ':has(:has(p))', ':lang(en, de)', ':dir("ltr")', '::before:hover', ':before(x)', '::slotted(a, b)', ':state', ':state(1)']) {
    assert.throws(() => parseSelectorList(bad), SyntaxError, bad);
  }
  for (const ok of ['::-webkit-scrollbar:hover', '::part(x):hover', '::selection:window-inactive', '::file-selector-button:focus', ':is(::before)', ':host', ':host(.x)', ':state(on)', '::slotted(p)', ':-webkit-any(p)']) {
    assert.ok(parseSelectorList(ok).length === 1, ok);
  }
});

// ── id / class edges + selector list + cache ────────────────────────────────
test('id/class names, empty names, comma list and parse cache', () => {
  assert.deepEqual(one('#app').first.id, 'app');
  assert.deepEqual(one('.x.y').first.classes, ['x', 'y']);
  // an id / class name is an identifier (Chrome 156)
  for (const bad of ['#', '.', '#1a', '.1a', 'p..x', 'p:']) assert.throws(() => parseSelectorList(bad), SyntaxError, bad);
  assert.deepEqual(one('.--a').first.classes, ['--a']);
  const list = parseSelectorList('h1, h2 , h3');
  assert.equal(list.length, 3);
  assert.equal(list[2].first.tag, 'h3');
  // cache: a second call returns the identical (cached) object
  assert.equal(parseSelectorList('.cached.sel'), parseSelectorList('.cached.sel'));
});

// ── the string-taking _internal.parseComplex wrapper ────────────────────────
test('_internal.parseComplex parses a single complex selector string', () => {
  assert.deepEqual(
    parseComplex('ul > li.active'),
    cx(cmp({ tag: 'ul' }), seg('>', cmp({ tag: 'li', classes: ['active'] }))),
  );
  // an empty selector string is a SyntaxError
  assert.throws(() => parseComplex('   '), SyntaxError);
});
