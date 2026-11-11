(() => {
  const out = {};
  const tag = value => Object.prototype.toString.call(value);
  const shape = value => value === null ? null : value === undefined ? 'undefined' : typeof value === 'object' || typeof value === 'function' ? tag(value) : value;
  const run = body => { try { return { value: shape(body()) }; } catch (error) { return { name: error.name, message: error.message, sameRealm: error instanceof Error }; } };
  const describe = (object, key) => {
    const descriptor = Object.getOwnPropertyDescriptor(object, key);
    if (!descriptor) return null;
    const result = {};
    for (const field of Object.keys(descriptor)) {
      const value = descriptor[field];
      result[field] = typeof value === 'function' ? { name: value.name, length: value.length } : field === 'value' ? shape(value) : value;
    }
    return result;
  };
  const ids = list => Array.from(list, node => node.id || node.localName);
  document.head.replaceChildren();
  const all = document.all;
  out.interface = {
    global: describe(globalThis, 'HTMLAllCollection'),
    keys: Reflect.ownKeys(HTMLAllCollection.prototype).map(String),
    members: Object.fromEntries(Reflect.ownKeys(HTMLAllCollection.prototype).map(key => [String(key), describe(HTMLAllCollection.prototype, key)])),
    prototypeParent: Object.getPrototypeOf(HTMLAllCollection.prototype) === Object.prototype,
    constructorParent: Object.getPrototypeOf(HTMLAllCollection) === Function.prototype,
    prototypeDescriptor: describe(HTMLAllCollection, 'prototype'),
    iteratorIsArrayValues: HTMLAllCollection.prototype[Symbol.iterator] === Array.prototype.values,
    construct: run(() => new HTMLAllCollection()),
    call: run(() => HTMLAllCollection()),
    documentAll: describe(Document.prototype, 'all'),
    ownAll: Object.getOwnPropertyNames(document).includes('all'),
  };
  out.undetectable = {
    typeof: typeof all, looseNull: all == null, looseUndefined: all == undefined, strictUndefined: all === undefined, strictNull: all === null,
    truthy: !!all, conditional: all ? 'truthy' : 'falsy', nullish: (all ?? 'fallback') === all, optional: all?.length, tag: tag(all),
    instance: all instanceof HTMLAllCollection, prototype: Object.getPrototypeOf(all) === HTMLAllCollection.prototype,
    same: document.all === all, typeofItem: typeof all.item, json: run(() => JSON.stringify(all)), construct: run(() => new all(0)),
    assign: run(() => { document.all = 5; return document.all === all; }),
    illegalGetter: run(() => Object.getOwnPropertyDescriptor(Document.prototype, 'all').get.call(document.body)),
    illegalPlainGetter: run(() => Object.getOwnPropertyDescriptor(Document.prototype, 'all').get.call({})),
    illegalLength: run(() => Object.getOwnPropertyDescriptor(HTMLAllCollection.prototype, 'length').get.call(document.body.children)),
    illegalItem: run(() => HTMLAllCollection.prototype.item.call(document.body.children, 0)),
    illegalNamed: run(() => HTMLAllCollection.prototype.namedItem.call({}, 'x')),
  };
  out.initial = { length: all.length, keys: Reflect.ownKeys(all).map(String), tags: Array.from(all, node => node.localName) };
  document.body.innerHTML = '<div id="first"></div><span name="ignored"></span><span name="ignored" id="spanned"></span><a name="anchor"></a><form name="form"></form><img name="image"><input name="field"><p id="7"></p><a name="dup"></a><img name="dup" id="dup-image"><div id="dup"></div><meta name="meta"><b id="item"></b><i id="length"></i><object name="object"></object><select name="select"></select><textarea name="textarea"></textarea><button name="button"></button><map name="map"></map><embed name="embed"><iframe name="frame"></iframe>';
  out.live = {
    length: all.length, ownKeys: Reflect.ownKeys(all).map(String), keys: Object.keys(all), forIn: (() => { const keys = []; for (const key in all) keys.push(key); return keys; })(),
    first: all.first && all.first.id, spanned: all.spanned && all.spanned.id, ignored: shape(all.ignored),
    anchor: shape(all.anchor), form: shape(all.form), image: shape(all.image), field: shape(all.field), meta: shape(all.meta), object: shape(all.object), select: shape(all.select), textarea: shape(all.textarea), button: shape(all.button), map: shape(all.map), embed: shape(all.embed), frame: shape(all.frame),
    seven: all[7] && all[7].localName, sevenString: all['7'] && all['7'].localName,
    namedSeven: run(() => all.namedItem('7').localName), itemSeven: run(() => all.item('7').localName), callSeven: run(() => all('7').localName),
    item: typeof all.item, itemId: run(() => all.namedItem('item').localName), lengthId: run(() => all.namedItem('length').localName), lengthValue: typeof all.length,
    zero: all[0] && all[0].localName, out: shape(all[9999]), negative: shape(all[-1]),
    descriptorIndex: describe(all, 0), descriptorNamed: describe(all, 'first'), descriptorMissing: describe(all, 'missing'), descriptorItem: describe(all, 'item'),
    hasNamed: 'first' in all, hasIndex: 0 in all, hasOut: 9999 in all, hasOwnLength: Object.prototype.hasOwnProperty.call(all, 'length'), hasOwnNamed: Object.prototype.hasOwnProperty.call(all, 'first'),
  };
  const duplicate = all.dup, duplicateAgain = all.namedItem('dup');
  out.duplicates = {
    tag: tag(duplicate), length: duplicate.length, ids: ids(duplicate), same: duplicate === all.dup, sameNamed: duplicate === duplicateAgain, callSame: all('dup') === duplicate, itemSame: all.item('dup') === duplicate,
    proto: Object.getPrototypeOf(duplicate) === HTMLCollection.prototype, namedOfNamed: shape(duplicate.namedItem('dup')), image: shape(duplicate['dup-image']),
  };
  document.body.lastElementChild.previousElementSibling.remove();
  document.querySelector('#dup').remove();
  out.duplicates.afterRemove = { length: duplicate.length, ids: ids(duplicate), named: shape(all.dup), namedLength: all.dup.length };
  document.querySelector('img[name=dup]').remove();
  out.duplicates.single = { tag: shape(all.dup), retained: duplicate.length };
  out.calls = {
    none: run(() => all()), undefined: run(() => all(undefined)), null: run(() => all(null)), zero: run(() => all(0).localName), zeroString: run(() => all('0').localName),
    float: run(() => all(0.5)), negative: run(() => all(-1)), named: run(() => all('first').id), extra: run(() => all('first', 0).id), symbol: run(() => all(Symbol('x'))),
    missing: run(() => all('missing')), applyReceiver: run(() => Reflect.apply(all, null, ['first']).id), callReceiver: run(() => all.call({}, 0).localName), bound: run(() => Function.prototype.bind.call(all, null)(0).localName),
    item: { none: run(() => all.item()), undefined: run(() => all.item(undefined)), zero: run(() => all.item(0).localName), named: run(() => all.item('first').id), symbol: run(() => all.item(Symbol('x'))) },
    namedItem: { none: run(() => all.namedItem()), undefined: run(() => all.namedItem(undefined)), empty: run(() => all.namedItem('')), number: run(() => all.namedItem(0)), symbol: run(() => all.namedItem(Symbol('x'))) },
  };
  // Strict-mode rejections are compared by type and effect: Blink words them from
  // PropertyCallbackInfo::ShouldThrowOnError, which Rusty V8 cannot observe, so
  // V8 reports the same TypeError in its own words.
  const thrown = body => { try { body(); return null; } catch (error) { return { name: error.name, sameRealm: error instanceof Error, retained: all[0] === document.documentElement && all.first === document.getElementById('first') }; } };
  out.mutation = {
    reflectSetIndex: run(() => { 'use strict'; return Reflect.set(all, 0, 1); }), reflectSetNamed: run(() => { 'use strict'; return Reflect.set(all, 'first', 1); }),
    reflectDeleteIndex: run(() => { 'use strict'; return Reflect.deleteProperty(all, 0); }), reflectDeleteNamed: run(() => { 'use strict'; return Reflect.deleteProperty(all, 'first'); }),
    reflectDefineIndex: run(() => Reflect.defineProperty(all, 0, { value: 1 })), reflectDefineNamed: run(() => Reflect.defineProperty(all, 'first', { value: 1 })),
    setIndex: run(() => { all[0] = 1; return all[0] === 1; }),
    strictSetIndex: thrown(() => { 'use strict'; all[0] = 1; return all[0] === 1; }),
    setOutIndex: run(() => { all[9999] = 1; return all[9999]; }),
    strictSetOutIndex: thrown(() => { 'use strict'; all[9998] = 1; return all[9998]; }),
    defineIndex: run(() => { Object.defineProperty(all, 0, { value: 1 }); return all[0]; }),
    defineOutIndex: run(() => { Object.defineProperty(all, 9997, { value: 1, configurable: true }); return all[9997]; }),
    deleteIndex: run(() => delete all[0]), strictDeleteIndex: thrown(() => { 'use strict'; return delete all[0]; }), deleteOutIndex: run(() => delete all[9996]),
    setNamed: run(() => { all.first = 1; return all.first === 1; }),
    strictSetNamed: thrown(() => { 'use strict'; all.first = 1; return all.first === 1; }),
    defineNamed: run(() => { Object.defineProperty(all, 'first', { value: 1, configurable: true }); return all.first === 1; }),
    deleteNamed: run(() => delete all.first), strictDeleteNamed: thrown(() => { 'use strict'; return delete all.first; }),
    expando: run(() => { all.expando = 1; return [all.expando, Object.keys(all).includes('expando')].join(); }),
    defineExpando: run(() => { Object.defineProperty(all, 'defined', { value: 2, configurable: true, enumerable: true }); return all.defined; }),
    shadowExpando: run(() => { document.body.insertAdjacentHTML('beforeend', '<div id="expando"></div><div id="defined"></div>'); return [shape(all.expando), shape(all.defined)].join(); }),
    deleteExpando: run(() => [delete all.expando, delete all.defined, shape(all.expando), shape(all.defined)].join()),
    preventExtensions: run(() => Reflect.preventExtensions(all)),
  };
  const fresh = document.implementation.createHTMLDocument('inert');
  fresh.body.innerHTML = '<p id="inert"></p>';
  out.inert = { typeof: typeof fresh.all, length: fresh.all.length, distinct: fresh.all !== document.all, same: fresh.all === fresh.all, named: fresh.all.inert && fresh.all.inert.localName, call: run(() => fresh.all(0).localName), tags: Array.from(fresh.all, node => node.localName) };
  const xml = document.implementation.createDocument(null, 'root', null), leaf = xml.createElementNS(null, 'leaf');
  leaf.setAttribute('id', 'leaf');
  xml.documentElement.append(leaf);
  out.xml = { typeof: typeof xml.all, length: xml.all.length, named: shape(xml.all.leaf), tags: Array.from(xml.all, node => node.localName) };
  const template = document.createElement('template');
  template.innerHTML = '<p id="in-template"></p>';
  document.body.append(template);
  const host = document.createElement('div');
  document.body.append(host);
  host.attachShadow({ mode: 'open' }).innerHTML = '<p id="in-shadow"></p>';
  out.scope = { template: shape(all['in-template']), shadow: shape(all['in-shadow']), contentAll: shape(template.content.all) };
  // An independent realm keeps its own interface; the getter is not realm-bound.
  const child = globalThis.__legacyChildRealm || (() => { const frame = document.createElement('iframe'); document.body.append(frame); return frame.contentWindow; })();
  child.document.head.replaceChildren();
  const childAll = child.document.all;
  out.realm = {
    typeof: typeof childAll, distinctInterface: child.HTMLAllCollection !== HTMLAllCollection, childBrand: Object.getPrototypeOf(childAll) === child.HTMLAllCollection.prototype,
    crossGetter: Object.getOwnPropertyDescriptor(Document.prototype, 'all').get.call(child.document) === childAll,
    crossLength: Object.getOwnPropertyDescriptor(HTMLAllCollection.prototype, 'length').get.call(childAll),
    crossItem: run(() => HTMLAllCollection.prototype.item.call(childAll, 0).localName),
    childLoose: child.Function('value', 'return [typeof value, value == null, value === undefined]')(all).join(),
  };
  return out;
})();
