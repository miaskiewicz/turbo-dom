import { test } from 'node:test';
import assert from 'node:assert/strict';
import { _internal } from '../src/runtime/selectors.mjs';
import { createEnvironment } from '../src/runtime/index.mjs';

const { parseSelectorList, parseComplex } = _internal;

// Parse a single complex selector (no top-level comma) and return its AST.
const one = (sel) => parseSelectorList(sel)[0];
// Build a fresh compound AST literal with only the fields a test cares about.
const cmp = (o = {}) => ({ tag: null, id: null, classes: [], attrs: [], pseudos: [], ...o });
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
  // lenient: a trailing bare type after a full compound is dropped (≡ div[id=x])
  assert.deepEqual(one('div[id=x]y'), cx(cmp({ tag: 'div', attrs: [attr('id', 'equals', 'x')] })));
});

// ── Leniency the matcher relies on ───────────────────────────────────────────
test('lenient div[id=x]y matches a <div id="x"> (no throw)', () => {
  const { document } = createEnvironment('<!doctype html><body><div id="x">hi</div></body>');
  assert.equal(document.querySelectorAll('div[id=x]y').length, 1);
  assert.equal(document.querySelector('div[id=x]y').textContent, 'hi');
});

test('redundant leading type / * is dropped (first wins)', () => {
  // a second type after a class is ignored; `*` only fills an unset tag
  assert.deepEqual(one('.foo*'), cx(cmp({ tag: '*', classes: ['foo'] })));
  assert.deepEqual(one('*'), cx(cmp({ tag: '*' })));
  assert.deepEqual(one('DIV'), cx(cmp({ tag: 'div' }))); // type lowercased
  assert.deepEqual(one('div*'), cx(cmp({ tag: 'div' }))); // redundant * dropped
});

// ── Throw cases preserved exactly ────────────────────────────────────────────
test('malformed selectors still throw SyntaxError', () => {
  assert.throws(() => parseSelectorList('>'), SyntaxError); // leading combinator ⇒ empty compound
  assert.throws(() => parseSelectorList('div!p'), SyntaxError); // unexpected char
  assert.throws(() => parseSelectorList('[href'), SyntaxError); // unterminated attribute
  assert.throws(() => parseSelectorList('a > > b'), SyntaxError); // empty compound between combinators
  assert.throws(() => parseSelectorList('a + > b'), SyntaxError);
});

// ── Empty / comma / whitespace edges (no throw — empties are dropped) ─────────
// SHAPE-ONLY UPDATE: a Complex is now always non-empty. parseComplexTokens returns
// null for an empty segment and parseSelectorList filters the nulls, so an empty /
// all-whitespace / comma-only segment yields NO complex rather than an empty
// `{ compounds:[], combinators:[] }`. (The old empty pair could only ever throw if
// matched, so no querySelectorAll/matches COUNT changes — see the matcher tests.)
test('empty complex selectors are dropped (no throw)', () => {
  assert.deepEqual(parseSelectorList(''), []);
  assert.deepEqual(parseSelectorList('   '), []); // all-whitespace ⇒ no complex
  assert.deepEqual(parseSelectorList(','), []); // two empty segments, both dropped
  assert.equal(parseSelectorList('a,,b').length, 2); // the empty middle segment is dropped
  assert.deepEqual(parseSelectorList('a,,b'), [cx(cmp({ tag: 'a' })), cx(cmp({ tag: 'b' }))]);
});

test('leading/trailing/repeated whitespace is trimmed; all whitespace kinds work', () => {
  assert.deepEqual(one('  a  b  '), cx(cmp({ tag: 'a' }), seg(' ', cmp({ tag: 'b' }))));
  assert.deepEqual(one(' div '), cx(cmp({ tag: 'div' })));
  // tab + newline behave as descendant whitespace just like a space
  assert.deepEqual(one('a\tb'), cx(cmp({ tag: 'a' }), seg(' ', cmp({ tag: 'b' }))));
  assert.deepEqual(one('a\r\n>\fb'), cx(cmp({ tag: 'a' }), seg('>', cmp({ tag: 'b' }))));
});

test('dangling trailing combinator is dropped by result (a > ≡ a)', () => {
  // SHAPE-ONLY UPDATE: the old desynced `{compounds:[a], combinators:['>']}` (one
  // compound, one stray combinator) is now the honest `{first:a, rest:[]}`. The
  // combinator is only paired once a trailing compound is read, so the dangling `>`
  // simply has nothing to attach to. Behavior is unchanged: `a >` matches like `a`.
  assert.deepEqual(one('a >'), cx(cmp({ tag: 'a' })));
  const { document } = createEnvironment('<!doctype html><body><a>x</a><a>y</a></body>');
  // selects the SAME nodes (by reference) as plain `a`
  const dangling = document.querySelectorAll('a >');
  const plain = document.querySelectorAll('a');
  assert.equal(dangling.length, 2);
  assert.equal(dangling.length, plain.length);
  for (let i = 0; i < plain.length; i++) assert.equal(dangling[i], plain[i]);
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
  // bare (unquoted) empty value after '='
  assert.deepEqual(one('[a=]').first.attrs, [attr('a', 'equals', '')]);
  // bare (unquoted) multi-char value is NOT quote-stripped
  assert.deepEqual(one('[href=index.html]').first.attrs, [attr('href', 'equals', 'index.html')]);
  // a ']' inside a quoted value must NOT close the attribute
  assert.deepEqual(one('[a="]"]').first.attrs, [attr('a', 'equals', ']')]);
  // value that opens with a quote but doesn't end with one is left intact (not stripped)
  assert.deepEqual(one('[x="a"b]').first.attrs, [attr('x', 'equals', '"a"b')]);
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
  assert.deepEqual(one(':nth-child()').first.pseudos, [{ name: 'nth-child', arg: '' }]);
  assert.deepEqual(one(':not(.x)').first.pseudos, [{ name: 'not', arg: '.x' }]);
  // a quoted ')' inside the arg does not terminate it (double- and single-quoted)
  assert.deepEqual(one(':not([title=")"])').first.pseudos, [{ name: 'not', arg: '[title=")"]' }]);
  assert.deepEqual(one(":not([title=')'])").first.pseudos, [{ name: 'not', arg: "[title=')']" }]);
  // double colon + empty pseudo name are tolerated (unknown ⇒ never matches)
  assert.deepEqual(one('div::before').first.pseudos, [
    { name: '', arg: null },
    { name: 'before', arg: null },
  ]);
  // unterminated pseudo arg: take everything after '(' (lenient, no throw)
  assert.deepEqual(one(':not(.x').first.pseudos, [{ name: 'not', arg: '.x' }]);
});

// ── id / class edges + selector list + cache ────────────────────────────────
test('id/class names, empty names, comma list and parse cache', () => {
  assert.deepEqual(one('#app').first.id, 'app');
  assert.deepEqual(one('.x.y').first.classes, ['x', 'y']);
  assert.equal(one('#').first.id, ''); // empty id name tolerated
  assert.deepEqual(one('.').first.classes, ['']); // empty class name tolerated
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
  // an empty selector string parses to null (filtered out of a list)
  assert.equal(parseComplex('   '), null);
});
