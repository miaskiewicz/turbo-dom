(function () {
  const g = globalThis;
  const ns = g[Symbol.for('ts')];
  const native = g.__documentLegacyNative;
  delete g.__documentLegacyNative;
  const make = ns.makeNativeFunction, liveCollection = ns.formCollectionFactory.live;
  const apply = Reflect.apply, define = Object.defineProperty, ErrorType = TypeError, SetType = Set, MapType = Map;
  const setHas = Set.prototype.has, setAdd = Set.prototype.add, mapGet = Map.prototype.get, mapSet = Map.prototype.set, test = RegExp.prototype.test;
  const type = Object.getOwnPropertyDescriptor(g.Node.prototype, 'nodeType').get;
  const localName = Object.getOwnPropertyDescriptor(g.Element.prototype, 'localName').get;
  const namespace = Object.getOwnPropertyDescriptor(g.Element.prototype, 'namespaceURI').get;
  const getAttribute = g.Element.prototype.getAttribute;
  const html = 'http://www.w3.org/1999/xhtml';
  // V8's own ToString message; Web IDL operations prefix it with their context.
  const domString = (value, context) => {
    if (typeof value === 'symbol') throw new ErrorType(context + 'Cannot convert a Symbol value to a string');
    return `${value}`;
  };
  const isDocument = receiver => { try { return apply(type, receiver, []) === 9; } catch (error) { return false; } };
  const check = receiver => { if (!isDocument(receiver)) throw new ErrorType('Illegal invocation'); };
  const getter = (proto, name, read) => define(proto, name, { get: make(read, 'get ' + name, 0), enumerable: true, configurable: true });
  const method = (proto, name, body, arity) => define(proto, name, { value: make(body, name, arity), writable: true, enumerable: true, configurable: true });
  const own = (rows, value) => define(rows, rows.length, { value, writable: true, enumerable: true, configurable: true });
  const arrayIndex = /^(0|[1-9][0-9]*)$/;
  const indexOf = key => apply(test, arrayIndex, [key]) && +key < 4294967295 ? +key : -1;

  // HTMLAllCollection: the obsolete, undetectable, callable view of every element
  // in the document's tree, excluding shadow trees and template contents.
  const AllCollection = make(function () {
    throw new ErrorType(new.target ? "Failed to construct 'HTMLAllCollection': Illegal constructor" : 'Illegal constructor');
  }, 'HTMLAllCollection', 0, true, true, false);
  const allProto = AllCollection.prototype;
  delete allProto.constructor;
  const record = receiver => { const value = native('record', receiver); if (!value) throw new ErrorType('Illegal invocation'); return value; };
  // "all"-named elements: any id, plus the name attribute of these HTML elements.
  const nameable = { a: 1, button: 1, embed: 1, form: 1, frame: 1, frameset: 1, iframe: 1, img: 1, input: 1, map: 1, meta: 1, object: 1, select: 1, textarea: 1 };
  const nameAttribute = element => apply(namespace, element, []) === html && nameable[apply(localName, element, [])] === 1 ? apply(getAttribute, element, ['name']) : null;
  const namedRows = (rows, name) => {
    const result = [];
    if (name === '') return result;
    for (let index = 0; index < rows.length; index++) {
      const element = rows[index];
      if (apply(getAttribute, element, ['id']) === name || nameAttribute(element) === name) own(result, element);
    }
    return result;
  };
  function namedItem(state, name) {
    const matches = namedRows(state.rows(), name);
    if (!matches.length) return null;
    if (matches.length === 1) return matches[0];
    // Several matches share one live HTMLCollection per name, as Blink caches it.
    let list = apply(mapGet, state.named, [name]);
    if (!list) { list = liveCollection(() => namedRows(state.rows(), name), true); apply(mapSet, state.named, [name, list]); }
    return list;
  }
  const itemOf = (state, nameOrIndex) => { const index = indexOf(nameOrIndex); return index >= 0 ? state.rows()[index] || null : namedItem(state, nameOrIndex); };
  function names(rows) {
    const result = [], seen = new SetType();
    for (let index = 0; index < rows.length; index++) {
      const element = rows[index];
      for (const value of [apply(getAttribute, element, ['id']), nameAttribute(element)]) {
        if (value && !apply(setHas, seen, [value])) { apply(setAdd, seen, [value]); own(result, value); }
      }
    }
    return result;
  }
  function allFor(document) {
    let stamp = -1, cached = null;
    const state = { named: new MapType(), rows() { const next = native('stamp', document); if (next !== stamp) { cached = native('elements', document); stamp = next; } return cached; } };
    const named = function (operation, key) {
      if (operation === 'keys') return names(state.rows());
      if (key in allProto) return undefined;
      const found = namedItem(state, key);
      if (found === null) return undefined;
      if (operation === 'get') return found;
      if (operation === 'has') return true;
      if (operation === 'descriptor') return { value: found, writable: false, enumerable: false, configurable: true };
      return undefined;
    };
    const indexed = function (operation, key) {
      const rows = state.rows();
      if (operation === 'indices') { const indices = []; for (let index = 0; index < rows.length; index++) own(indices, index); return indices; }
      const index = indexOf(key);
      if (index < 0 || index >= rows.length) return undefined;
      if (operation === 'get') return rows[index];
      if (operation === 'has') return true;
      if (operation === 'descriptor') return { value: rows[index], writable: false, enumerable: true, configurable: true };
      return undefined;
    };
    const caller = function (nameOrIndex) { return arguments.length && nameOrIndex !== undefined ? itemOf(state, domString(nameOrIndex, '')) : null; };
    return native('collection', allProto, named, indexed, caller, state);
  }
  getter(allProto, 'length', function () { return record(this).rows().length; });
  method(allProto, 'item', function (nameOrIndex) {
    const state = record(this);
    return arguments.length && nameOrIndex !== undefined ? itemOf(state, domString(nameOrIndex, "Failed to execute 'item' on 'HTMLAllCollection': ")) : null;
  }, 0);
  method(allProto, 'namedItem', function (name) {
    const state = record(this);
    if (!arguments.length) throw new ErrorType("Failed to execute 'namedItem' on 'HTMLAllCollection': 1 argument required, but only 0 present.");
    return namedItem(state, domString(name, "Failed to execute 'namedItem' on 'HTMLAllCollection': "));
  }, 1);
  define(allProto, 'constructor', { value: AllCollection, writable: true, enumerable: false, configurable: true });
  define(allProto, Symbol.toStringTag, { value: 'HTMLAllCollection', configurable: true });
  define(allProto, Symbol.iterator, { value: Array.prototype.values, writable: true, configurable: true });
  define(AllCollection, 'prototype', { writable: false });
  define(g, 'HTMLAllCollection', { value: AllCollection, writable: true, configurable: true });
  native('register', allFor);
  getter(g.Document.prototype, 'all', function () { check(this); return native('all', this); });

  const proto = g.Document.prototype, DOMExceptionType = g.DOMException;
  const domainError = message => new DOMExceptionType("Failed to set the 'domain' property on 'Document': " + message, 'SecurityError');
  define(proto, 'domain', {
    get: make(function () { check(this); return native('domain', this); }, 'get domain', 0),
    set: make(function (value) {
      check(this);
      const refusal = native('setDomain', this, domString(value, "Failed to set the 'domain' property on 'Document': "));
      if (refusal !== undefined) throw domainError(refusal);
    }, 'set domain', 1),
    enumerable: true, configurable: true,
  });
  // The response's Last-Modified instant when the host supplied one, else now; in local time.
  const DateType = Date, now = Date.now, dateParts = ['getMonth', 'getDate', 'getFullYear', 'getHours', 'getMinutes', 'getSeconds'].map(name => Date.prototype[name]);
  const pad = (value, width) => { let text = '' + value; while (text.length < width) text = '0' + text; return text; };
  getter(proto, 'lastModified', function () {
    check(this);
    const fixed = native('lastModified', this), date = new DateType(fixed === undefined ? now() : fixed);
    const [month, day, year, hours, minutes, seconds] = dateParts.map(part => apply(part, date, []));
    return pad(month + 1, 2) + '/' + pad(day, 2) + '/' + pad(year, 4) + ' ' + pad(hours, 2) + ':' + pad(minutes, 2) + ':' + pad(seconds, 2);
  });
  // Fullscreen state (fullscreen/fullscreenElement/webkit exits) is user_activation_gates.js.
  // No element of this engine is ever picture-in-picture, and its documents are never
  // prerendered or discarded. Pointer lock is native state (pointer_events.rs).
  if (Object.getOwnPropertyDescriptor(g.document, 'fullscreenElement')) delete g.document.fullscreenElement;
  getter(proto, 'pointerLockElement', function () { check(this); return native('pointerLockElement', this); });
  getter(proto, 'pictureInPictureElement', function () { check(this); return null; });
  for (const name of ['prerendering', 'wasDiscarded']) getter(proto, name, function () { check(this); return false; });
  const reject = Promise.reject.bind(Promise);
  const rejecting = (name, refusal) => method(proto, name, function () {
    if (!isDocument(this)) return reject(new ErrorType("Failed to execute '" + name + "' on 'Document': Illegal invocation"));
    return reject(refusal("Failed to execute '" + name + "' on 'Document': "));
  }, 0);
  rejecting('exitPictureInPicture', prefix => new DOMExceptionType(prefix + 'There is no Picture-in-Picture element in this document.', 'InvalidStateError'));
  method(proto, 'exitPointerLock', function () { check(this); native('exitPointerLock', this); }, 0);
})();
