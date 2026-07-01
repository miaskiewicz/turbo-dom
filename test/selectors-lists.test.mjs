import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

// :is() / :where() / :not() selector-list pseudo-classes (parity with the Rust
// rtdom port). Each argument is a comma-separated list of complex selectors,
// anchored at the element under test.
test(':is() matches if any complex in the list matches', () => {
  const { document } = createEnvironment(
    '<!doctype html><body><div class="a">1</div><div class="b">2</div><div class="c">3</div></body>',
  );
  assert.equal(document.querySelectorAll(':is(.a, .b)').length, 2);
  assert.equal(document.querySelectorAll(':is(.c)').length, 1); // single-item degenerate
  assert.equal(document.querySelectorAll(':is(.nope)').length, 0);
});

test(':is() anchors the list at the element and composes with combinators', () => {
  const { document } = createEnvironment(
    '<!doctype html><body>' +
      '<div class="x"><span class="y">hit</span></div>' +
      '<div class="z"><span class="y">miss</span></div>' +
      '<section><p class="t">a</p></section><p class="t">b</p>' +
      '</body>',
  );
  // div:is(.x) .y -> only the .y inside the div.x
  const r = document.querySelectorAll('div:is(.x) .y');
  assert.equal(r.length, 1);
  assert.equal(r[0].textContent, 'hit');
  // a complex selector inside :is()
  const r2 = document.querySelectorAll(':is(section .t, .nope)');
  assert.equal(r2.length, 1);
  assert.equal(r2[0].textContent, 'a');
});

test(':where() matches identically to :is()', () => {
  const { document } = createEnvironment(
    '<!doctype html><body><i class="a">1</i><i class="b">2</i><i>3</i></body>',
  );
  assert.equal(document.querySelectorAll(':where(.a,.b)').length, 2);
  assert.equal(document.querySelectorAll('i:where(.a)').length, 1);
});

test(':not() takes a selector list (none of the list may match)', () => {
  const { document } = createEnvironment(
    '<!doctype html><body><ul>' +
      '<li class="a">1</li><li class="b">2</li><li class="c">3</li>' +
      '</ul></body>',
  );
  const r = document.querySelectorAll('li:not(.a, .b)');
  assert.equal(r.length, 1);
  assert.equal(r[0].textContent, '3');
  // tag list: none of these <li> are div/span -> all three survive
  assert.equal(document.querySelectorAll('li:not(div, span)').length, 3);
  // single-item :not still works (degenerate list)
  assert.equal(document.querySelectorAll('li:not(.a)').length, 2);
});

test('matches() honors :is/:where/:not on a single element', () => {
  const { document } = createEnvironment(
    '<!doctype html><body><p class="lead big">x</p></body>',
  );
  const p = document.querySelector('p');
  assert.equal(p.matches(':is(.lead, .nope)'), true);
  assert.equal(p.matches(':where(.nope, .big)'), true);
  assert.equal(p.matches(':not(.lead, .big)'), false);
  assert.equal(p.matches(':not(.x, .y)'), true);
});

// :has() relational pseudo-class (parity with the Rust rtdom port). Each relative
// selector may begin with a combinator (>, +, ~) or none (= descendant).
test(':has() descendant: element with a matching descendant', () => {
  const { document } = createEnvironment(
    '<!doctype html><body>' +
      '<div id="hit"><span class="x">y</span></div>' +
      '<div id="miss"><span>z</span></div>' +
      '</body>',
  );
  const r = document.querySelectorAll('div:has(.x)');
  assert.equal(r.length, 1);
  assert.equal(r[0].getAttribute('id'), 'hit');
  // negative: nothing has a .absent descendant
  assert.equal(document.querySelectorAll('div:has(.absent)').length, 0);
});

test(':has(> S) child combinator only reaches direct children', () => {
  const { document } = createEnvironment(
    '<!doctype html><body>' +
      '<ul id="a"><li class="active">1</li></ul>' +
      '<ul id="b"><li>2</li></ul>' +
      '<ul id="c"><div><li class="active">deep</li></div></ul>' +
      '</body>',
  );
  const r = document.querySelectorAll('ul:has(> li.active)');
  assert.equal(r.length, 1);
  assert.equal(r[0].getAttribute('id'), 'a'); // c's li.active is a grandchild
  // descendant form reaches the grandchild → a and c
  assert.equal(document.querySelectorAll('ul:has(li.active)').length, 2);
});

test(':has(+ S) adjacent sibling', () => {
  const { document } = createEnvironment(
    '<!doctype html><body>' +
      '<section><h2 id="p">t</h2><p>para</p></section>' +
      '<section><h2 id="q">t2</h2><div>not-p</div></section>' +
      '</body>',
  );
  const r = document.querySelectorAll('h2:has(+ p)');
  assert.equal(r.length, 1);
  assert.equal(r[0].getAttribute('id'), 'p');
});

test(':has(.a .b) multi-compound relative complex is scoped to the element', () => {
  const { document } = createEnvironment(
    '<!doctype html><body>' +
      '<section id="in"><div class="a"><span class="b">hit</span></div></section>' +
      '<section id="out"><span class="b">noA</span></section>' +
      '<div class="a"><section id="split"><span class="b">aOutside</span></section></div>' +
      '</body>',
  );
  const r = document.querySelectorAll('section:has(.a .b)');
  // only `in` (both .a and .b inside); `split` has .b but its only .a ancestor
  // is OUTSIDE the section → scope cap rejects it.
  assert.equal(r.length, 1);
  assert.equal(r[0].getAttribute('id'), 'in');
});

test(':has(~ S) general sibling + relative selector list + empty arg', () => {
  const { document } = createEnvironment(
    '<!doctype html><body>' +
      '<div><h3 id="a">x</h3><span>s</span><p>after</p></div>' +
      '<div><h3 id="b">y</h3><span>only</span></div>' +
      '</body>',
  );
  const r = document.querySelectorAll('h3:has(~ p)');
  assert.equal(r.length, 1);
  assert.equal(r[0].getAttribute('id'), 'a');
  // a comma list of relative selectors: matches if ANY matches
  assert.equal(document.querySelectorAll('h3:has(~ p, ~ .none)').length, 1);
  // empty :has() never matches
  assert.equal(document.querySelectorAll('div:has()').length, 0);
});

test(':has() works through matches() on a single element', () => {
  const { document } = createEnvironment(
    '<!doctype html><body><article><img alt="x"></article><article><p>no img</p></article></body>',
  );
  const [withImg, without] = document.querySelectorAll('article');
  assert.equal(withImg.matches('article:has(img)'), true);
  assert.equal(without.matches('article:has(img)'), false);
});

test(':has() relative complex may itself contain combinators (scoped matcher arms)', () => {
  const { document } = createEnvironment(
    '<!doctype html><body>' +
      '<section id="s1"><div class="a"><span class="b">x</span></div></section>' + // .a > .b
      '<section id="s2"><div class="a"></div><span class="b">y</span></section>' +  // .a + .b
      '<section id="s3"><div class="a"></div><i>z</i><span class="b">w</span></section>' + // .a ~ .b (gap)
      '</body>',
  );
  // child combinator inside the relative complex
  const gt = document.querySelectorAll('section:has(.a > .b)');
  assert.equal(gt.length, 1);
  assert.equal(gt[0].getAttribute('id'), 's1');
  // adjacent combinator inside the relative complex
  const plus = document.querySelectorAll('section:has(.a + .b)');
  assert.equal(plus.length, 1);
  assert.equal(plus[0].getAttribute('id'), 's2');
  // general-sibling combinator inside the relative complex (matches s2 and s3)
  assert.equal(document.querySelectorAll('section:has(.a ~ .b)').length, 2);
});

test(':has(.a .b) ancestor walk iterates past a non-matching wrapper (within scope)', () => {
  const { document } = createEnvironment(
    '<!doctype html><body>' +
      // .b is nested two levels under .a, with a non-.a wrapper in between, so the
      // scoped ancestor walk must skip the wrapper and keep ascending (still < scope).
      '<section id="hit"><div class="a"><div class="wrap"><span class="b">x</span></div></div></section>' +
      '<section id="miss"><div class="wrap"><span class="b">y</span></div></section>' +
      '</body>',
  );
  const r = document.querySelectorAll('section:has(.a .b)');
  assert.equal(r.length, 1);
  assert.equal(r[0].getAttribute('id'), 'hit');
});

// Regression: a leading combinator constrains the HEAD compound of the relative
// complex, not the whole thing. Anchoring the entire complex at a child/sibling
// candidate made these return 0. (parity fix, both engines)
test(':has(> A B) — leading combinator binds head, multi-compound tail', () => {
  const { document } = createEnvironment(
    '<!doctype html><body>' +
      '<div id="u"><span class="a"><span class="b">hit</span></span></div>' +
      '<div id="v"><div><span class="a"><span class="b">deep</span></span></div></div>' +
      '</body>',
  );
  // u: .a is a direct child, .b a descendant of it → match
  assert.equal(document.querySelectorAll('#u:has(> .a .b)').length, 1);
  assert.equal(document.querySelectorAll('#u:has(> .a > .b)').length, 1);
  // v: .a is a grandchild, so `> .a …` must NOT match
  assert.equal(document.querySelectorAll('#v:has(> .a .b)').length, 0);
  // but the descendant form does match v
  assert.equal(document.querySelectorAll('#v:has(.a .b)').length, 1);
});

test(':has(+ A B) / :has(~ A B) — sibling combinator binds head', () => {
  const { document } = createEnvironment(
    '<!doctype html><body>' +
      '<h2 id="h">t</h2><div class="s"><span class="x">hit</span></div>' +
      '<p id="p">t</p><em>z</em><div class="s2"><span class="x">y</span></div>' +
      '</body>',
  );
  assert.equal(document.querySelectorAll('#h:has(+ div .x)').length, 1);
  assert.equal(document.querySelectorAll('#h:has(~ div .x)').length, 1);
  // #p's immediate next sibling is <em>, not the div → + fails, ~ succeeds
  assert.equal(document.querySelectorAll('#p:has(+ div .x)').length, 0);
  assert.equal(document.querySelectorAll('#p:has(~ div .x)').length, 1);
});
