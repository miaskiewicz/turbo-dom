// DOM pre-insert / replace validity (Chrome 156): inserting a node into itself or one of
// its descendants throws HierarchyRequestError and leaves the tree unchanged.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

const { document } = createEnvironment('');
function mk() {
  const root = document.createElement('section');
  root.innerHTML = '<div id=p><a></a><b></b><i></i></div>';
  const p = root.firstChild;
  return [root, p, ...p.children];
}
// Chrome 156: every case throws HierarchyRequestError and p.outerHTML stays put.
const CASES = {
  insertAdjacentElement: (p, a) => a.insertAdjacentElement('afterbegin', p),
  appendSelf: (p) => p.appendChild(p),
  replaceWith: (p, a, b) => b.replaceWith(p),
  before: (p, a) => a.before(p),
  append: (p, a) => a.append(p),
  replaceChild: (p, a) => p.replaceChild(p, a),
  replaceChildren: (p, a) => a.replaceChildren(p),
  after: (p, a) => a.after(p),
  prepend: (p, a) => a.prepend(p),
  insertBefore: (p, a) => a.insertBefore(p, null),
};

for (const [name, run] of Object.entries(CASES)) {
  test(`${name}: inserting an ancestor throws HierarchyRequestError, tree unchanged`, () => {
    const [root, p, a, b] = mk();
    assert.throws(() => run(p, a, b), { name: 'HierarchyRequestError' });
    assert.equal(p.parentNode, root);
    assert.equal(p.outerHTML, '<div id="p"><a></a><b></b><i></i></div>');
  });
}

test('a shadow root cannot take an ancestor of its host', () => {
  const [root, p] = mk();
  const host = p.appendChild(document.createElement('span'));
  assert.throws(() => host.attachShadow({ mode: 'open' }).appendChild(p), { name: 'HierarchyRequestError' });
  assert.equal(p.parentNode, root);
});

test('a fragment cannot be inserted into one of its own children', () => {
  const frag = document.createDocumentFragment();
  const c = frag.appendChild(document.createElement('span'));
  assert.throws(() => c.appendChild(frag), { name: 'HierarchyRequestError' });
  assert.equal(c.parentNode, frag);
  assert.equal(frag.childNodes.length, 1);
});
