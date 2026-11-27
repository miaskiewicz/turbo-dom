import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  CSSStyleRule,
  CSSStyleSheet,
  cssSheetSnapshot,
  createObservedCssSheet,
  createObservedCssRule,
} from '../src/runtime/cssom.mjs';
import { createEnvironment } from '../src/runtime/index.mjs';
import { ensureCssState, cssStateSnapshot } from '../src/runtime/css-invalidation.mjs';

function scene() {
  const env = createEnvironment('<body><style></style><div id=target></div></body>');
  ensureCssState(env.document);
  const owner = env.document.querySelector('style');
  const sheet = owner.sheet;
  sheet.insertRule('.target{color:red}', 0);
  ensureCssState(env.document);
  return { ...env, owner, sheet };
}
function checkpoint(document) {
  return cssStateSnapshot(document);
}
function cssChange(document, mutate) {
  const before = checkpoint(document);
  mutate();
  const after = checkpoint(document);
  assert.notEqual(after, before);
  return [before, after];
}

test('CSS proxies retain own data descriptors, array identity, prototype and values', () => {
  const sheet = new CSSStyleSheet();
  assert.deepEqual(Object.keys(sheet), [
    'ownerNode',
    'cssRules',
    'type',
    'disabled',
    'media',
    'title',
    'href',
    'parentStyleSheet',
    'ownerRule',
  ]);
  for (const name of Object.keys(sheet)) {
    const descriptor = Object.getOwnPropertyDescriptor(sheet, name);
    assert.equal('value' in descriptor, true);
    assert.equal(descriptor.enumerable, true);
    assert.equal(descriptor.configurable, true);
    assert.equal(descriptor.writable, true);
  }
  assert.equal(Array.isArray(sheet.cssRules), true);
  assert.equal(sheet.rules, sheet.cssRules);
  const list = sheet.cssRules;
  sheet.insertRule('a{}');
  assert.equal(sheet.cssRules, list);
  const rule = sheet.cssRules[0];
  assert.deepEqual(Object.keys(rule), ['cssText', 'type']);
  assert.equal(rule instanceof CSSStyleRule, true);
  assert.equal(sheet instanceof CSSStyleSheet, true);
  assert.equal(Object.getPrototypeOf(rule), CSSStyleRule.prototype);
  assert.equal(Object.getPrototypeOf(sheet), CSSStyleSheet.prototype);
  assert.equal(sheet.insertRule, new CSSStyleSheet().insertRule);
  const object = {};
  rule.cssText = object;
  sheet.disabled = object;
  sheet.media = object;
  assert.equal(rule.cssText, object);
  assert.equal(sheet.disabled, object);
  assert.equal(sheet.media, object);
});

test('rule text and plain metadata edits advance source only, without broad DOM writes', () => {
  const { document, sheet } = scene();
  const version = document.__version;
  for (const mutate of [
    () => {
      sheet.cssRules[0].cssText = '.target{color:blue}';
    },
    () => {
      sheet.disabled = true;
    },
    () => {
      sheet.media = 'print';
    },
  ]) {
    const [before, after] = cssChange(document, mutate);
    assert.equal(before.structure, after.structure);
    assert.notEqual(before.cssSource, after.cssSource);
    assert.equal(document.__version, version);
  }
});

test('global source invalidation covers other documents but keeps exact local source identity', () => {
  const first = scene(),
    second = scene();
  const local = cssSheetSnapshot(second.sheet);
  const beforeFirst = checkpoint(first.document),
    beforeSecond = checkpoint(second.document);
  first.sheet.cssRules[0].cssText = 'a{color:blue}';
  const afterFirst = checkpoint(first.document),
    afterSecond = checkpoint(second.document);
  assert.notEqual(afterFirst, beforeFirst);
  assert.notEqual(afterSecond, beforeSecond);
  assert.equal(beforeFirst.structure, afterFirst.structure);
  assert.equal(beforeSecond.structure, afterSecond.structure);
  assert.equal(afterFirst.cssSource, afterSecond.cssSource);
  assert.equal(cssSheetSnapshot(second.sheet), local);
});

test('same-value writes, absent deletes and failed Reflect operations add no source change', () => {
  const { document, sheet } = scene();
  const before = checkpoint(document),
    metadata = cssSheetSnapshot(sheet);
  sheet.disabled = false;
  sheet.cssRules[0].cssText = sheet.cssRules[0].cssText;
  sheet.cssRules.length = sheet.cssRules.length;
  delete sheet.absent;
  delete sheet.cssRules.absent;
  delete sheet.cssRules[0].absent;
  Object.defineProperty(sheet, 'disabled', { value: false });
  assert.equal(checkpoint(document), before);
  assert.equal(cssSheetSnapshot(sheet), metadata);
  Object.freeze(sheet.cssRules[0]);
  Object.freeze(sheet);
  const frozen = checkpoint(document),
    frozenMetadata = cssSheetSnapshot(sheet);
  assert.equal(Reflect.set(sheet, 'disabled', true), false);
  assert.equal(Reflect.set(sheet.cssRules[0], 'cssText', 'b{}'), false);
  assert.equal(Reflect.deleteProperty(sheet, 'media'), false);
  assert.equal(Reflect.deleteProperty(sheet.cssRules[0], 'cssText'), false);
  assert.equal(Reflect.defineProperty(sheet, 'media', { value: 'print' }), false);
  assert.equal(Reflect.defineProperty(sheet.cssRules[0], 'cssText', { value: 'b{}' }), false);
  assert.throws(() => {
    sheet.disabled = true;
  }, TypeError);
  assert.throws(() => {
    sheet.cssRules[0].cssText = 'b{}';
  }, TypeError);
  assert.equal(checkpoint(document), frozen);
  assert.equal(cssSheetSnapshot(sheet), frozenMetadata);
});

test('owned array edits invalidate same-length replacement, order, splice, length and deletion', () => {
  const { document, sheet } = scene();
  sheet.insertRule('b{}', 1);
  const list = sheet.cssRules;
  const replacement = observedRule('z{}');
  cssChange(document, () => {
    list[0] = replacement;
  });
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, true);
  cssChange(document, () => list.sort((a, b) => a.cssText.localeCompare(b.cssText)));
  cssChange(document, () => list.reverse());
  cssChange(document, () => list.splice(1, 0, observedRule('c{}')));
  cssChange(document, () => {
    list.length = 1;
  });
  assert.equal(sheet.cssRules, list);
  assert.equal(list[0], replacement);
  cssChange(document, () => {
    delete list[0];
  });
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, false);
  cssChange(document, () => {
    list.length = 3;
  });
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, false);
});

test('array and record getters are not invoked by proof; original source getter failures remain intact', () => {
  const { document, owner, sheet } = scene();
  const error = new Error('original getter');
  let calls = 0;
  cssChange(document, () =>
    Object.defineProperty(sheet.cssRules[0], 'cssText', {
      get() {
        calls++;
        throw error;
      },
      configurable: true,
    }),
  );
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, false);
  assert.equal(calls, 0);
  assert.throws(
    () => owner.textContent,
    (value) => value === error,
  );
  assert.equal(calls, 1);
  const second = scene();
  Object.defineProperty(second.sheet.cssRules, 0, {
    get() {
      calls++;
      throw error;
    },
    configurable: true,
  });
  assert.equal(cssSheetSnapshot(second.sheet).rulesTrusted, false);
  assert.equal(calls, 1);
  assert.throws(
    () => second.owner.textContent,
    (value) => value === error,
  );
});

test('sheet descriptor overrides invalidate without executing owner/media/disabled getters', () => {
  const { document, sheet } = scene();
  const error = new Error('sheet getter');
  let calls = 0;
  for (const name of ['ownerNode', 'media', 'disabled', 'cssRules']) {
    cssChange(document, () =>
      Object.defineProperty(sheet, name, {
        get() {
          calls++;
          throw error;
        },
        configurable: true,
      }),
    );
    assert.equal(cssSheetSnapshot(sheet).metadataTrusted && cssSheetSnapshot(sheet).rulesTrusted, false);
    assert.equal(calls, 0);
    assert.throws(
      () => sheet[name],
      (value) => value === error,
    );
    calls--;
  }
  assert.equal(calls, 0);
});

test('external array and media identities are preserved and explicitly require fingerprints', () => {
  const { document, sheet } = scene();
  const external = [{ cssText: 'external{}' }];
  cssChange(document, () => {
    sheet.cssRules = external;
  });
  assert.equal(sheet.cssRules, external);
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, false);
  sheet.insertRule('new{}');
  assert.equal(sheet.cssRules, external);
  const media = [];
  sheet.media = media;
  assert.equal(sheet.media, media);
  const metadata = cssSheetSnapshot(sheet);
  assert.equal(metadata.mediaNeedsFingerprint, true);
  media.push('print');
  assert.equal(cssSheetSnapshot(sheet), metadata);
  assert.deepEqual(cssSheetSnapshot(sheet).media, ['print']);
  assert.equal(cssSheetSnapshot({ cssRules: [] }), null);
});

test('shared rule records refuse both owned trust paths and remain live in each sheet', () => {
  const first = scene(),
    second = scene();
  const original = cssSheetSnapshot(first.sheet);
  const shared = first.sheet.cssRules[0];
  second.sheet.cssRules[0] = shared;
  assert.notEqual(cssSheetSnapshot(first.sheet), original);
  assert.equal(cssSheetSnapshot(first.sheet).rulesTrusted, false);
  assert.equal(cssSheetSnapshot(second.sheet).rulesTrusted, false);
  const beforeFirst = checkpoint(first.document),
    beforeSecond = checkpoint(second.document);
  shared.cssText = 'shared{color:green}';
  assert.notEqual(checkpoint(first.document), beforeFirst);
  assert.notEqual(checkpoint(second.document), beforeSecond);
  assert.equal(second.sheet.cssRules[0].cssText, 'shared{color:green}');
});

test('own methods preserve exactly one legacy owner touch and expose one final source snapshot', () => {
  const { document, owner, sheet } = scene();
  const touch = owner.__touch;
  let touches = 0,
    during;
  owner.__touch = function () {
    touches++;
    during = cssSheetSnapshot(sheet);
    touch.call(this);
  };
  for (const mutate of [() => sheet.insertRule('b{}'), () => sheet.deleteRule(0), () => sheet.replaceSync('c{}d{}')]) {
    const count = touches,
      version = document.__version;
    cssChange(document, mutate);
    assert.equal(touches, count + 1);
    assert.ok(document.__version > version);
    assert.equal(cssSheetSnapshot(sheet), during);
    assert.equal(cssSheetSnapshot(sheet).rulesTrusted, true);
  }
});

test('subclass and inherited constructor setters receive the public returned instance', () => {
  let ruleReceiver, sheetReceiver;
  class Rule extends CSSStyleRule {
    set type(value) {
      ruleReceiver = this;
      Object.defineProperty(this, 'type', { value, writable: true, enumerable: true, configurable: true });
    }
  }
  class Sheet extends CSSStyleSheet {
    set ownerNode(value) {
      sheetReceiver = this;
      Object.defineProperty(this, 'ownerNode', { value, writable: true, enumerable: true, configurable: true });
    }
  }
  const rule = new Rule('sub{}'),
    sheet = new Sheet();
  assert.equal(ruleReceiver, rule);
  assert.equal(sheetReceiver, sheet);
  assert.equal(rule instanceof Rule, true);
  assert.equal(sheet instanceof Sheet, true);
  assert.equal(rule instanceof CSSStyleRule, true);
  assert.equal(sheet instanceof CSSStyleSheet, true);
  assert.equal(Object.getPrototypeOf(rule), Rule.prototype);
  assert.equal(Object.getPrototypeOf(sheet), Sheet.prototype);
  assert.equal(rule.selectorText, 'sub');
  sheet.insertRule('x{}');
});

test('extracted and borrowed methods preserve original errors, mutation order and generic getters', () => {
  const method = CSSStyleSheet.prototype.insertRule;
  const fake = { cssRules: [] };
  assert.throws(
    () => method.call(fake, 'a{}'),
    (error) => error instanceof TypeError && error.message === 'Receiver must be an instance of class CSSStyleSheet',
  );
  assert.equal(fake.cssRules.length, 1);
  assert.throws(() => method('a{}'), TypeError);
  const getter = Object.getOwnPropertyDescriptor(CSSStyleSheet.prototype, 'rules').get;
  assert.equal(getter.call(fake), fake.cssRules);
  const { document, sheet } = scene();
  const error = new Error('original coercion');
  const before = checkpoint(document),
    version = document.__version;
  assert.throws(
    () =>
      sheet.insertRule({
        toString() {
          throw error;
        },
      }),
    (value) => value === error,
  );
  assert.equal(checkpoint(document), before);
  assert.equal(document.__version, version);
  Object.defineProperty(sheet, 'ownerNode', {
    get() {
      throw error;
    },
    configurable: true,
  });
  assert.throws(
    () => sheet.insertRule('inserted{}'),
    (value) => value === error,
  );
  assert.equal(sheet.cssRules[0].cssText, 'inserted{}');
});

test('record deletion and prototype customization invalidate without binding methods', () => {
  const { document, sheet } = scene();
  const rule = sheet.cssRules[0];
  const proto = Object.getPrototypeOf(rule);
  const before = checkpoint(document);
  assert.equal(Reflect.setPrototypeOf(rule, proto), true);
  assert.equal(checkpoint(document), before);
  cssChange(document, () => Object.setPrototypeOf(rule, Object.create(proto)));
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, false);
  cssChange(document, () => {
    delete rule.cssText;
  });
  assert.equal(rule.cssText, undefined);
  rule.cssText = 'restored{}';
  assert.equal(rule.selectorText, 'restored');
  const current = Object.getPrototypeOf(rule);
  Object.preventExtensions(rule);
  const frozen = checkpoint(document);
  assert.equal(Reflect.setPrototypeOf(rule, {}), false);
  assert.equal(Reflect.setPrototypeOf(rule, current), true);
  assert.equal(checkpoint(document), frozen);
});

test('sheet and array prototype changes retain public behavior and explicitly lose trust', () => {
  const { document, sheet } = scene();
  let before = checkpoint(document);
  assert.equal(Reflect.setPrototypeOf(sheet, CSSStyleSheet.prototype), true);
  assert.equal(Reflect.setPrototypeOf(sheet.cssRules, Array.prototype), true);
  assert.equal(checkpoint(document), before);
  cssChange(document, () => Object.setPrototypeOf(sheet.cssRules, Object.create(Array.prototype)));
  assert.equal(Array.isArray(sheet.cssRules), true);
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, false);
  sheet.insertRule('extra{}');
  cssChange(document, () => Object.setPrototypeOf(sheet, Object.create(CSSStyleSheet.prototype)));
  CSSStyleSheet.prototype.insertRule.call(sheet, 'base{}');
  assert.equal(sheet.cssRules[0].cssText, 'base{}');
  Object.preventExtensions(sheet.cssRules);
  Object.preventExtensions(sheet);
  before = checkpoint(document);
  assert.equal(Reflect.setPrototypeOf(sheet.cssRules, {}), false);
  assert.equal(Reflect.setPrototypeOf(sheet, {}), false);
  assert.equal(checkpoint(document), before);
});

test('inactive old arrays cannot notify a replacement store and external arrays are never adopted', () => {
  const { document, sheet } = scene();
  const old = sheet.cssRules;
  const external = [];
  sheet.cssRules = external;
  const before = checkpoint(document);
  old.extra = 'inactive';
  assert.equal(checkpoint(document), before);
  assert.equal(sheet.cssRules, external);
  const version = document.__version;
  cssChange(document, () => {
    delete sheet.media;
  });
  assert.equal(cssSheetSnapshot(sheet).metadataTrusted, false);
  assert.equal(document.__version, version);
});

test('foreign records and untrusted genuine records refuse ownership without reading text getters', () => {
  const { document, sheet } = scene();
  const error = new Error('foreign record');
  let calls = 0;
  const foreign = {
    get cssText() {
      calls++;
      throw error;
    },
  };
  cssChange(document, () => sheet.cssRules.push(foreign));
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, false);
  assert.equal(calls, 0);
  const record = new CSSStyleRule('known{}');
  Object.defineProperty(record, 'cssText', {
    configurable: true,
    get() {
      calls++;
      throw error;
    },
  });
  sheet.cssRules.push(record);
  assert.equal(calls, 0);
  assert.throws(
    () => sheet.cssRules[1].cssText,
    (value) => value === error,
  );
});

test('symbol and non-index list metadata are observed without treating them as rule entries', () => {
  const { document, sheet } = scene();
  const key = Symbol('list metadata');
  cssChange(document, () => {
    sheet.cssRules[key] = 1;
  });
  cssChange(document, () => {
    sheet.cssRules.extra = 2;
  });
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, true);
  const version = document.__version;
  cssChange(document, () => {
    sheet.cssRules[5] = new CSSStyleRule('gap{}');
  });
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, false);
  assert.equal(document.__version, version);
});

test('partial native splice failure preserves data/error and refuses the resulting hole', () => {
  const { document, sheet } = scene();
  Object.defineProperty(sheet.cssRules, 'length', { writable: false });
  const version = document.__version;
  assert.throws(() => sheet.deleteRule(0), TypeError);
  assert.equal(document.__version, version);
  assert.equal(sheet.cssRules.length, 1);
  assert.equal(0 in sheet.cssRules, false);
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, false);
});

test('custom splice getter order and original callable/coercion errors are unchanged', () => {
  const { sheet } = scene();
  const list = sheet.cssRules;
  const native = Array.prototype.splice;
  const events = [];
  Object.defineProperty(list, 'splice', {
    configurable: true,
    get() {
      events.push('get');
      return function (...args) {
        events.push('call');
        return Reflect.apply(native, this, args);
      };
    },
  });
  sheet.insertRule({
    toString() {
      events.push('string');
      return 'ordered{}';
    },
  });
  assert.deepEqual(events, ['get', 'string', 'call']);
  const error = new Error('index coercion');
  const normal = scene();
  assert.throws(
    () =>
      normal.sheet.insertRule('a{}', {
        valueOf() {
          throw error;
        },
      }),
    (value) => value === error,
  );
  const index = {
    valueOf() {
      return 0;
    },
  };
  normal.sheet.insertRule('object-index{}', index);
  assert.equal(normal.sheet.cssRules[0].selectorText, 'object-index');
});

test('inherited new-index properties disable canonical batching without changing array semantics', () => {
  const { sheet } = scene();
  const name = String(sheet.cssRules.length);
  const previous = Object.getOwnPropertyDescriptor(Array.prototype, name);
  try {
    Object.defineProperty(Array.prototype, name, { value: 'inherited', writable: true, configurable: true });
    sheet.insertRule('guard{}', 0);
  } finally {
    if (previous) Object.defineProperty(Array.prototype, name, previous);
    else delete Array.prototype[name];
  }
  assert.equal(sheet.cssRules[0].selectorText, 'guard');
});

test('custom map results retain exact public identity and require a fingerprint', () => {
  const { sheet } = scene();
  const original = Array.prototype.map;
  let result;
  try {
    Array.prototype.map = function (...args) {
      result = Reflect.apply(original, this, args);
      return result;
    };
    sheet.replaceSync('custom{}');
  } finally {
    Array.prototype.map = original;
  }
  assert.equal(sheet.cssRules, result);
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, false);
  assert.equal(sheet.cssRules[0].selectorText, 'custom');
});

test('custom constructor init getters keep their original count and thrown values', () => {
  let mediaReads = 0,
    titleReads = 0;
  const sheet = new CSSStyleSheet(null, {
    get media() {
      mediaReads++;
      return mediaReads === 1 ? 'screen' : 'print';
    },
    get title() {
      titleReads++;
      return 'title';
    },
  });
  assert.equal(mediaReads, 2);
  assert.equal(titleReads, 2);
  assert.equal(sheet.media, 'print');
  const error = new Error('init');
  assert.throws(
    () =>
      new CSSStyleSheet(null, {
        get media() {
          throw error;
        },
      }),
    (value) => value === error,
  );
  const owner = () => {};
  const callableOwner = new CSSStyleSheet(owner);
  assert.equal(callableOwner.ownerNode, owner);
});

test('captured Reflect operations add no new user callbacks to ordinary CSS writes', () => {
  const { sheet } = scene();
  const original = {
    set: Reflect.set,
    defineProperty: Reflect.defineProperty,
    deleteProperty: Reflect.deleteProperty,
    setPrototypeOf: Reflect.setPrototypeOf,
  };
  try {
    for (const name of Object.keys(original))
      Reflect[name] = () => {
        throw new Error('unexpected public Reflect lookup');
      };
    sheet.disabled = true;
    sheet.cssRules[0].cssText = 'captured{}';
    Object.defineProperty(sheet, 'media', { value: 'screen' });
    sheet.cssRules.extra = 1;
    delete sheet.cssRules.extra;
    Object.setPrototypeOf(sheet.cssRules, Array.prototype);
  } finally {
    Object.assign(Reflect, original);
  }
  assert.equal(sheet.disabled, true);
  assert.equal(sheet.cssRules[0].cssText, 'captured{}');
});

test('non-index names and alternate Reflect receivers never acquire rule membership', () => {
  const { document, sheet } = scene();
  for (const name of ['-1', '01', '4294967295']) sheet.cssRules[name] = new CSSStyleRule('metadata{}');
  assert.equal(sheet.cssRules.length, 1);
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, true);
  const before = checkpoint(document),
    metadata = cssSheetSnapshot(sheet),
    receiver = {};
  assert.equal(Reflect.set(sheet, 'disabled', true, receiver), true);
  assert.equal(Reflect.set(sheet.cssRules[0], 'cssText', 'receiver{}', receiver), true);
  assert.equal(receiver.disabled, true);
  assert.equal(receiver.cssText, 'receiver{}');
  assert.equal(sheet.disabled, false);
  assert.equal(sheet.cssRules[0].cssText, '.target{color:red}');
  assert.equal(checkpoint(document), before);
  assert.equal(cssSheetSnapshot(sheet), metadata);
  sheet.media = () => {};
  assert.equal(cssSheetSnapshot(sheet).mediaNeedsFingerprint, true);
  sheet.media = null;
  assert.equal(cssSheetSnapshot(sheet).mediaNeedsFingerprint, false);
});

test('public accessor setters remain live and unknown stores are not silently certified', () => {
  const { document, sheet } = scene();
  let current = false;
  Object.defineProperty(sheet, 'disabled', {
    get() {
      return current;
    },
    set(value) {
      current = value;
    },
    configurable: true,
  });
  cssChange(document, () => {
    sheet.disabled = true;
  });
  assert.equal(sheet.disabled, true);
  assert.equal(cssSheetSnapshot(sheet).metadataTrusted, false);
  let text = 'initial{}';
  const rule = sheet.cssRules[0];
  Object.defineProperty(rule, 'cssText', {
    get() {
      return text;
    },
    set(value) {
      text = value;
    },
    configurable: true,
  });
  cssChange(document, () => {
    rule.cssText = 'accessor{}';
  });
  assert.equal(rule.selectorText, 'accessor');
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, false);
});

test('metadata writes never add owner/proxy getter callbacks before the legacy owner touch', () => {
  const error = new Error('owner getter');
  let reads = 0;
  const owner = new Proxy(
    {},
    {
      get() {
        reads++;
        throw error;
      },
      getOwnPropertyDescriptor() {
        reads++;
        throw error;
      },
    },
  );
  const sheet = createObservedCssSheet(owner);
  sheet.disabled = true;
  sheet.media = 'screen';
  const rule = new CSSStyleRule('a{}');
  sheet.cssRules.push(rule);
  rule.cssText = 'b{}';
  assert.equal(reads, 0);
  assert.equal(cssSheetSnapshot(sheet).ownerNode, owner);
  assert.equal(reads, 0);
  assert.throws(
    () => sheet.insertRule('late{}'),
    (value) => value === error,
  );
  assert.equal(reads, 1);
});

test('a custom no-op splice retains the original broad touch without inventing a CSS write', () => {
  const { document, sheet } = scene();
  sheet.cssRules.splice = () => [];
  const before = checkpoint(document),
    metadata = cssSheetSnapshot(sheet),
    version = document.__version;
  sheet.insertRule('ignored{}');
  const after = checkpoint(document);
  assert.ok(document.__version > version);
  assert.notEqual(after.structure, before.structure);
  assert.equal(after.cssSource, before.cssSource);
  assert.equal(cssSheetSnapshot(sheet), metadata);
  assert.equal(sheet.cssRules.length, 1);
});

test('canonical splice proof is revalidated after constructor callbacks change array species', () => {
  const { document, sheet } = scene();
  const list = sheet.cssRules;
  const previous = Object.getOwnPropertyDescriptor(CSSStyleRule.prototype, 'type');
  let calls = 0,
    during;
  Object.defineProperty(CSSStyleRule.prototype, 'type', {
    configurable: true,
    set(value) {
      Object.defineProperty(this, 'type', { value, writable: true, enumerable: true, configurable: true });
      Object.defineProperty(list, 'constructor', {
        configurable: true,
        get() {
          calls++;
          during = checkpoint(document);
          return Array;
        },
      });
    },
  });
  try {
    sheet.insertRule('revalidate{}');
  } finally {
    if (previous) Object.defineProperty(CSSStyleRule.prototype, 'type', previous);
    else delete CSSStyleRule.prototype.type;
  }
  assert.equal(calls, 1);
  assert.equal(sheet.cssRules[0].selectorText, 'revalidate');
  assert.notEqual(checkpoint(document), during);
});

test('constructor callbacks never expose a completed-store trust certificate prematurely', () => {
  let constructing;
  class Sheet extends CSSStyleSheet {
    set ownerNode(value) {
      constructing = this;
      Object.defineProperty(this, 'ownerNode', { value, writable: true, enumerable: true, configurable: true });
    }
  }
  let partial;
  const sheet = new Sheet(null, {
    get title() {
      partial = cssSheetSnapshot(constructing);
      return null;
    },
  });
  assert.equal(partial, null); // Public constructed subclasses remain fingerprint-only.
  assert.equal(cssSheetSnapshot(sheet), null);
  const owner = createObservedCssSheet(null);
  const previous = Object.getOwnPropertyDescriptor(CSSStyleRule.prototype, 'type');
  Object.defineProperty(CSSStyleRule.prototype, 'type', {
    configurable: true,
    set(value) {
      Object.defineProperty(this, 'type', { value, writable: true, enumerable: true, configurable: true });
      owner.cssRules.push(this);
    },
  });
  try {
    new CSSStyleRule('partial{}');
  } finally {
    if (previous) Object.defineProperty(CSSStyleRule.prototype, 'type', previous);
    else delete CSSStyleRule.prototype.type;
  }
  assert.equal(cssSheetSnapshot(owner).rulesTrusted, false);
  assert.equal(owner.cssRules[0].cssText, 'partial{}');
});
test('internally appended rules expose one observed identity through every public read and lock path', () => {
  for (const read of [
    (list) => list[0],
    (list) => Reflect.get(list, '0'),
    (list) => Object.getOwnPropertyDescriptor(list, '0').value,
    (list) => Object.getOwnPropertyDescriptors(list)['0'].value,
    (list) => [...list][0],
    (list) => [...list.values()][0],
    (list) => Array.from(list)[0],
    (list) => list.slice()[0],
    (list) => [].concat(list)[0],
    (list) => Object.values(list)[0],
    (list) => {
      Object.freeze(list);
      return list[0];
    },
    (list) => {
      Object.seal(list);
      return list[0];
    },
    (list) => {
      Object.preventExtensions(list);
      return list[0];
    },
    (list) => {
      Object.defineProperty(list, '0', { writable: false, configurable: false });
      return list[0];
    },
  ]) {
    const { document, sheet } = scene();
    const record = read(sheet.cssRules);
    assert.equal(record, sheet.cssRules[0]);
    assert.ok(record instanceof CSSStyleRule);
    assert.deepEqual(Object.getOwnPropertyDescriptor(record, 'cssText'), {
      value: '.target{color:red}',
      writable: true,
      enumerable: true,
      configurable: true,
    });
    const before = checkpoint(document);
    record.cssText = '.changed{color:blue}';
    assert.notEqual(checkpoint(document), before);
    assert.equal(sheet.cssRules[0].cssText, '.changed{color:blue}');
  }
});
test('first source snapshot arms private rule records before granting any store trust', () => {
  const { document, sheet } = scene();
  const initial = cssSheetSnapshot(sheet);
  assert.equal(initial.rulesTrusted, true);
  const record = sheet.cssRules[0];
  record.cssText = '.later{}';
  assert.notEqual(cssSheetSnapshot(sheet), initial);
  const before = checkpoint(document);
  Object.defineProperty(record, 'cssText', {
    get() {
      return '.custom{}';
    },
    configurable: true,
  });
  assert.notEqual(checkpoint(document), before);
  assert.equal(cssSheetSnapshot(sheet).rulesTrusted, false);
});
test('internal append String coercion runs once and prototype callbacks see only the exposed proxy', () => {
  const sheet = new CSSStyleSheet();
  let calls = 0,
    seen;
  const before = Object.getOwnPropertyDescriptor(CSSStyleRule.prototype, 'cssText');
  try {
    sheet.insertRule({
      toString() {
        calls++;
        Object.defineProperty(CSSStyleRule.prototype, 'cssText', {
          configurable: true,
          set(value) {
            seen = this;
            Object.defineProperty(this, 'cssText', { value, writable: true, configurable: true, enumerable: true });
          },
        });
        return '.a{}';
      },
    });
    assert.equal(calls, 1);
    assert.equal(seen, sheet.cssRules[0]);
    assert.equal(seen.cssText, '.a{}');
  } finally {
    if (before) Object.defineProperty(CSSStyleRule.prototype, 'cssText', before);
    else delete CSSStyleRule.prototype.cssText;
  }
});
test('fallback inherited indexed setters never receive a private unobserved record', () => {
  const sheet = createObservedCssSheet(null);
  sheet.insertRule('.a{}');
  sheet.insertRule('.b{}');
  let seen, receiver;
  try {
    Object.defineProperty(Array.prototype, '2', {
      configurable: true,
      set(value) {
        seen = value;
        receiver = this;
        Object.defineProperty(this, '2', { value, writable: true, enumerable: true, configurable: true });
      },
    });
    sheet.insertRule('.c{}', 2);
  } finally {
    delete Array.prototype[2];
  }
  assert.equal(receiver, sheet.cssRules);
  assert.equal(seen, sheet.cssRules[2]);
  const before = cssSheetSnapshot(sheet);
  seen.cssText = '.observed{}';
  assert.notEqual(cssSheetSnapshot(sheet), before);
});
test('canonical append failures before writes preserve the CSS stamp and original error', () => {
  for (const prepare of [
    (sheet) => Object.freeze(sheet.cssRules),
    (sheet) => Object.preventExtensions(sheet.cssRules),
    () => {},
  ]) {
    const { document, sheet } = scene();
    prepare(sheet);
    const before = checkpoint(document);
    assert.throws(() => sheet.insertRule('.fail{}', 0n), TypeError);
    assert.equal(checkpoint(document), before);
  }
});

test('frozen and nonextensible ordinary numeric insert failures also remain nonmutations', () => {
  for (const prepare of [Object.freeze, Object.preventExtensions]) {
    const { document, sheet } = scene();
    prepare(sheet.cssRules);
    const before = checkpoint(document);
    assert.throws(() => sheet.insertRule('.fail{}', 0), TypeError);
    assert.equal(checkpoint(document), before);
  }
});
test('overridden String conversion remains exactly once and cannot bypass species revalidation', () => {
  const original = globalThis.String;
  const species = Object.getOwnPropertyDescriptor(Array, Symbol.species);
  const sheet = new CSSStyleSheet();
  let conversions = 0,
    constructed = 0;
  try {
    globalThis.String = function (value) {
      conversions++;
      Object.defineProperty(Array, Symbol.species, {
        configurable: true,
        value: function () {
          constructed++;
          return [];
        },
      });
      globalThis.String = original;
      return original(value);
    };
    sheet.insertRule('.converted{}');
  } finally {
    globalThis.String = original;
    Object.defineProperty(Array, Symbol.species, species);
  }
  assert.equal(conversions, 1);
  assert.equal(constructed, 1);
  assert.equal(sheet.cssRules[0].cssText, '.converted{}');
});
test('private source proofs add no extra mutable global String callbacks', () => {
  const original = globalThis.String;
  const sheet = new CSSStyleSheet();
  let calls = 0;
  try {
    globalThis.String = (value) => {
      calls++;
      return original(value);
    };
    sheet.insertRule('.x{}');
    cssSheetSnapshot(sheet);
  } finally {
    globalThis.String = original;
  }
  assert.equal(calls, 1);
});

function observedRule(text) {
  return createObservedCssRule(text);
}
test('internal observed constructor callbacks see only incomplete metadata until return', () => {
  const before = Object.getOwnPropertyDescriptor(CSSStyleSheet.prototype, 'ownerNode');
  let constructing, partial;
  try {
    Object.defineProperty(CSSStyleSheet.prototype, 'ownerNode', {
      configurable: true,
      set(value) {
        constructing = this;
        Object.defineProperty(this, 'ownerNode', { value, writable: true, configurable: true, enumerable: true });
      },
    });
    const sheet = createObservedCssSheet(null, {
      get title() {
        partial = cssSheetSnapshot(constructing);
        return null;
      },
    });
    assert.equal(constructing, sheet);
    assert.equal(partial.metadataTrusted, false);
    assert.equal(partial.rulesTrusted, false);
    assert.notEqual(cssSheetSnapshot(sheet), partial);
    assert.equal(cssSheetSnapshot(sheet).metadataTrusted, true);
  } finally {
    if (before) Object.defineProperty(CSSStyleSheet.prototype, 'ownerNode', before);
    else delete CSSStyleSheet.prototype.ownerNode;
  }
});
