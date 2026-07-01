import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

// Mirrors the Rust rtdom regression (commit 3cdf298). The complex-selector matcher
// walks right-to-left; for a descendant (or general-sibling) combinator it must try
// EVERY candidate ancestor/sibling, not commit to the nearest one. `.a > .b .c` where
// the `.b` that is a direct child of `.a` is FARTHER from `.c` than another `.b` is the
// canonical failure of a greedy non-backtracking matcher.
test('selector matcher backtracks on mixed child/descendant combinators', () => {
  const { document } = createEnvironment(
    '<!doctype html><body>' +
      '<div class="a"><div class="b"><div class="x"><div class="b"><div class="c">c</div></div></div></div></div>' +
      '</body>',
  );
  // correct answer is 1 (the outer .b is a child of .a, and .c descends from it);
  // a greedy matcher picks the inner .b, fails the `> .a` check, and returns 0.
  assert.equal(document.querySelectorAll('.a > .b .c').length, 1);
});

// Exercises the remaining matchChain arms so the recursive matcher is fully covered:
// a descendant selector whose left compound matches no ancestor (the ' ' arm
// exhausts to false), and a general-sibling ('~') chain that matches after skipping
// a non-matching sibling, alongside one that finds no preceding match (exhausts).
test('matcher arms: failed descendant + general-sibling match and exhaust', () => {
  const { document } = createEnvironment(
    '<!doctype html><body>' +
      '<div><span class="c">x</span></div>' + //          .nomatch .c → no matching ancestor
      '<ul><li class="x"></li><li></li><li class="y"></li></ul>' + // .x ~ .y → skip middle li, match
      '<ol><li></li><li class="y"></li></ol>' + //         .x ~ .y → no .x sibling before → exhaust
      '</body>',
  );
  assert.equal(document.querySelectorAll('.nomatch .c').length, 0); // descendant arm exhausts → false
  assert.equal(document.querySelectorAll('.x ~ .y').length, 1); //    '~' arm: iterate-then-match; other exhausts
});

// Covers the short-circuit false-branches of the child ('>') and adjacent ('+')
// arms: no parent (detached), a non-element parent (the root element's parent is
// the document node), and no previous element sibling (a first child).
test('child/adjacent combinator edge branches', () => {
  const { document } = createEnvironment('<!doctype html><body><p>x</p></body>');
  // detached element → child combinator has no parent → no match (!parent branch)
  const orphan = document.createElement('span');
  assert.equal(orphan.matches('div > span'), false);
  // `* > html`: <html>'s parent is the document node (nodeType 9, not an element)
  assert.equal(document.querySelectorAll('* > html').length, 0);
  // `* + head`: <head> is the first element child of <html> → no previous element
  assert.equal(document.querySelectorAll('* + head').length, 0);
});
