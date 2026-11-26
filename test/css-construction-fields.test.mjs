import assert from 'node:assert/strict';
import test from 'node:test';
import { Text, Comment, Element, Document, Node, CharacterData } from '../src/runtime/dom.mjs';
import * as runtime from '../src/runtime/index.mjs';
import { cssConstructionFieldCollision as check } from '../src/runtime/css-construction-fields.mjs';

test('shared prototypes are rechecked, preserving first collision order and restoration', () => {
  assert.equal(Object.hasOwn(runtime, 'CharacterData'), false);
  assert.equal(check(), undefined);
  let reads = 0;
  const accessor = {configurable:true, get(){reads++;}};
  try {
    Object.defineProperty(Node.prototype, '__version', accessor);
    Object.defineProperty(CharacterData.prototype, '_data', accessor);
    assert.equal(check(), '_data');
    delete CharacterData.prototype._data;
    assert.equal(check(), '__version');
    Object.defineProperty(Element.prototype, 'localName', accessor);
    assert.equal(check(), 'localName');
  } finally {
    delete Element.prototype.localName;
    delete Node.prototype.__version;
    delete CharacterData.prototype._data;
  }
  assert.equal(reads, 0);
  assert.equal(check(), undefined);
});

test('new ancestry is walked on every read, including before guard initialization', async () => {
  const before = Object.getPrototypeOf(Text.prototype);
  const inserted = Object.create(before);
  const parent = Object.getPrototypeOf(Document.prototype);
  let cold;
  let reads = 0;
  try {
    Object.setPrototypeOf(Text.prototype, inserted);
    Object.defineProperty(inserted, '_data', {configurable:true, get(){reads++;}});
    cold = (await import('../src/runtime/css-construction-fields.mjs?changed-chain')).cssConstructionFieldCollision;
    assert.equal(cold(), '_data');
    assert.equal(check(), '_data');
    delete inserted._data;
    assert.equal(cold(), undefined);
    assert.equal(check(), undefined);
    Object.setPrototypeOf(Document.prototype, Comment.prototype);
    Object.defineProperty(Comment.prototype, 'defaultView', {configurable:true,value:undefined});
    assert.equal(check(), 'defaultView');
  } finally {
    delete Comment.prototype.defaultView;
    Object.setPrototypeOf(Document.prototype, parent);
    Object.setPrototypeOf(Text.prototype, before);
  }
  assert.equal(reads, 0);
  assert.equal(cold(), undefined);
  Object.defineProperty(Element.prototype, '__sheet', {configurable:true, value:undefined});
  try { assert.equal(cold(), '__sheet'); }
  finally { delete Element.prototype.__sheet; }
  assert.equal(check(), undefined);
});
