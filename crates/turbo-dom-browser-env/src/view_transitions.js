// Same-document and element-scoped View Transitions as Chrome 156 ships them:
// Document/Element startViewTransition() and activeViewTransition, ViewTransition and its
// setlike ViewTransitionTypeSet. The lifecycle runs on the host's rendering updates: the old
// state is captured after a frame's animation callbacks, the update callback runs in a task
// while rendering stays suppressed, ready resolves once the new state is captured, and the
// default 0.25s cross-fade finishes in the first frame 250ms past the first frame after
// ready. There are no pixels: snapshots are the capture-time layout and view-transition-name
// checks, and no ::view-transition-* animation objects are exposed.
(function () {
  'use strict';
  const Native = globalThis.__viewTransitionNative; delete globalThis.__viewTransitionNative;
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction;
  const apply = Reflect.apply, define = Object.defineProperty, create = Object.create, getPrototypeOf = Object.getPrototypeOf;
  const PromiseValue = Promise, then = Promise.prototype.then, resolvePromise = Promise.resolve, rejectPromise = Promise.reject;
  const ErrorType = TypeError, StringValue = String, MapValue = Map, WeakMapValue = WeakMap;
  const weakGet = WeakMap.prototype.get, weakSet = WeakMap.prototype.set, weakDelete = WeakMap.prototype.delete;
  const mapSet = Map.prototype.set, mapDelete = Map.prototype.delete, mapHas = Map.prototype.has, mapClear = Map.prototype.clear, mapEntries = Map.prototype.entries, mapForEach = Map.prototype.forEach;
  const mapSize = Object.getOwnPropertyDescriptor(Map.prototype, 'size').get;
  const mapIteratorProto = getPrototypeOf(apply(mapEntries, new MapValue(), [])), mapNext = mapIteratorProto.next;
  const documentBrand = ns.domAccessor('nodeType', false, 'Document'), elementBrand = ns.domAccessor('nodeType', false, 'Element');
  const accessor = (proto, name) => Object.getOwnPropertyDescriptor(proto, name).get;
  const documentElementOf = accessor(g.Document.prototype, 'documentElement');
  const ownerDocumentOf = accessor(g.Node.prototype, 'ownerDocument');
  const connectedGetter = accessor(g.Node.prototype, 'isConnected');
  const iteratorKey = Symbol.iterator;
  const resolved = apply(resolvePromise, PromiseValue, []);
  const microtask = callback => { apply(then, resolved, [callback]); };
  const read = (map, key) => apply(weakGet, map, [key]);
  const PHASE_PENDING = 0, PHASE_CALLBACK = 1, PHASE_ANIMATING = 2, PHASE_DONE = 3;
  const DURATION = 250, TIMEOUT = 4000;
  const transitions = new WeakMapValue(), typeSets = new WeakMapValue(), iterators = new WeakMapValue();
  // Active transition per scope: the document for document-scoped transitions (including
  // ones started on its document element), else the scoping element.
  const active = new WeakMapValue();

  const illegal = () => new ErrorType('Illegal invocation');
  // DOMExceptions come from the realm's registered interface, like natively thrown ones.
  const exception = (message, name) => Native('exception', realmDocument, name, message);
  const invalid = detail => exception('Transition was aborted because of invalid state. ' + detail, 'InvalidStateError');
  const string = (value, prefix) => {
    if (typeof value === 'symbol') throw new ErrorType(prefix + 'Cannot convert a Symbol value to a string');
    return StringValue(value);
  };
  const required = (count, operation, owner) => {
    if (count < 1) throw new ErrorType("Failed to execute '" + operation + "' on '" + owner + "': 1 argument required, but only 0 present.");
  };
  const deferred = () => {
    const value = create(null);
    value.promise = new PromiseValue((resolve, reject) => { value.resolve = resolve; value.reject = reject; });
    return value;
  };
  // Tasks go to this realm's own trusted scheduler: a document without a browsing context
  // still queues its update callback on its relevant global's event loop. A host that never
  // captured one (the test runner) keeps the timer it installed before any page script.
  const realmDocument = g.document, installedTimer = g.setTimeout;
  const performanceObject = g.performance, performanceNow = performanceObject && performanceObject.now, dateNow = Date.now;
  const now = () => typeof performanceNow === 'function' ? apply(performanceNow, performanceObject, []) : dateNow();
  const task = (callback, delay) => {
    if (!Native('schedule', realmDocument, callback, delay || 0) && typeof installedTimer === 'function') apply(installedTimer, g, [callback, delay || 0]);
  };
  // Without a host rendering loop, frames are 16ms tasks stamped with the realm clock.
  const host = (record, command, argument) => Native('has-host', record.document) ? Native('host', record.document, command, argument) : undefined;
  const frame = (record, step) => {
    if (Native('has-host', record.document)) Native('host', record.document, 'frame', step);
    else task(() => step(now()), 16);
  };
  const typeList = record => {
    const values = [];
    apply(mapForEach, record.types.values, [(_generation, key) => { values[values.length] = key; }]);
    return values;
  };
  const publish = record => { if (read(active, record.scope) === record) Native('active', record.root, typeList(record)); };

  // https://drafts.csswg.org/css-view-transitions-1/#skip-the-view-transition
  function skip(record, reason) {
    if (record.phase === PHASE_DONE) return;
    const needsCallback = !record.invoked;
    record.phase = PHASE_DONE;
    record.invoked = true;
    if (read(active, record.scope) === record) {
      apply(weakDelete, active, [record.scope]);
      Native('active', record.root, null);
    }
    if (record.suppressing) { record.suppressing = false; host(record, 'suppress', false); }
    // ready rejects a microtask later; the update callback still runs, in a task.
    microtask(() => {
      record.ready.reject(reason);
      settleFinished(record);
    });
    if (needsCallback) task(() => invoke(record));
  }
  // finished follows updateCallbackDone once the transition is over.
  function settleFinished(record) {
    if (record.phase !== PHASE_DONE || record.callbackState === null) return;
    if (record.callbackState) record.finished.resolve(undefined);
    else record.finished.reject(record.callbackReason);
  }
  // https://drafts.csswg.org/css-view-transitions-1/#call-the-update-callback
  function invoke(record) {
    if (record.called) return;
    record.called = true;
    record.invoked = true;
    if (record.phase !== PHASE_DONE) {
      record.phase = PHASE_CALLBACK;
      task(() => {
        if (record.phase === PHASE_CALLBACK) skip(record, exception('Transition was aborted because of timeout in DOM update. DOM update timed out', 'TimeoutError'));
      }, TIMEOUT);
    }
    let result;
    try {
      result = record.callback === null ? undefined : apply(record.callback, undefined, []);
      result = apply(resolvePromise, PromiseValue, [result]);
    } catch (error) {
      result = apply(rejectPromise, PromiseValue, [error]);
    }
    apply(then, result, [() => {
      record.callbackState = true;
      record.updateCallbackDone.resolve(undefined);
      if (record.phase === PHASE_DONE) settleFinished(record);
      // Chrome captures the new state in a lifecycle update of its own, a few tasks out.
      else task(() => task(() => activate(record)));
    }, error => {
      record.callbackState = false;
      record.callbackReason = error;
      record.updateCallbackDone.reject(error);
      if (record.phase === PHASE_DONE) settleFinished(record);
      else skip(record, error);
    }]);
  }
  // Capture the old state after the frame's animation callbacks, then suppress rendering
  // and queue the update callback.
  function capture(record) {
    if (record.phase !== PHASE_PENDING || record.invoked) return;
    const status = Native('capture', record.root, record.scope === record.document);
    if (status === 'layout') return skip(record, invalid('Unsupported layout or style'));
    if (status === 'names') return skip(record, invalid('Snapshot capture failed'));
    if (record.scope === record.document) { record.suppressing = true; host(record, 'suppress', true); }
    task(() => { if (record.phase === PHASE_PENDING && !record.invoked) invoke(record); });
  }
  // https://drafts.csswg.org/css-view-transitions-1/#activate-view-transition
  function activate(record) {
    if (record.phase === PHASE_DONE) return;
    if (record.suppressing) { record.suppressing = false; host(record, 'suppress', false); }
    const status = Native('capture', record.root, record.scope === record.document);
    if (status === 'layout') return skip(record, invalid('Unsupported layout or style'));
    if (status === 'names') return skip(record, invalid('Animation start failed'));
    record.phase = PHASE_ANIMATING;
    record.ready.resolve(undefined);
    frame(record, timestamp => animate(record, timestamp));
  }
  // The pseudo-element animations start in the first frame after ready; the transition is
  // cleared in the frame they end once every waitUntil() promise has settled.
  function animate(record, timestamp) {
    if (record.phase !== PHASE_ANIMATING) return;
    if (record.start === null) record.start = timestamp;
    else if (timestamp - record.start >= DURATION) record.ended = true;
    if (!record.ended) return frame(record, next => animate(record, next));
    if (record.waiting === 0) finish(record);
  }
  function finish(record) {
    record.phase = PHASE_DONE;
    if (read(active, record.scope) === record) {
      apply(weakDelete, active, [record.scope]);
      Native('active', record.root, null);
    }
    settleFinished(record);
  }

  // (ViewTransitionUpdateCallback or ViewTransitionOptions), with sequence<DOMString>? types.
  function options(value, owner) {
    const prefix = "Failed to execute 'startViewTransition' on '" + owner + "': ";
    const result = create(null);
    result.callback = null; result.types = [];
    if (value === undefined || value === null) return result;
    if (typeof value === 'function') { result.callback = value; return result; }
    if (typeof value !== 'object') throw new ErrorType(prefix + 'Overload resolution failed.');
    const member = prefix + "Failed to read the '";
    const types = value.types;
    if (types !== undefined && types !== null) {
      const sequence = member + "types' property from 'ViewTransitionOptions': ";
      const method = typeof types === 'object' || typeof types === 'function' ? types[iteratorKey] : undefined;
      if (typeof method !== 'function') throw new ErrorType(sequence + 'The provided value cannot be converted to a sequence.');
      const iterator = apply(method, types, []);
      const next = iterator.next;
      for (;;) {
        const step = apply(next, iterator, []);
        if (step.done) break;
        result.types[result.types.length] = string(step.value, sequence);
      }
    }
    const update = value.update;
    if (update !== undefined && update !== null) {
      if (typeof update !== 'function') throw new ErrorType(member + "update' property from 'ViewTransitionOptions': The given value is not a function.");
      result.callback = update;
    }
    return result;
  }

  // Interface objects.
  const illegalConstructor = name => make(function () {
    throw new ErrorType(new.target ? "Failed to construct '" + name + "': Illegal constructor" : 'Illegal constructor');
  }, name, 0, true, true, false);
  const Transition = illegalConstructor('ViewTransition'), TypeSet = illegalConstructor('ViewTransitionTypeSet');
  const transitionProto = Transition.prototype, typeSetProto = TypeSet.prototype;
  const constructorDescriptor = Object.getOwnPropertyDescriptor(transitionProto, 'constructor');
  const typeSetConstructor = Object.getOwnPropertyDescriptor(typeSetProto, 'constructor');
  delete transitionProto.constructor; delete typeSetProto.constructor;
  define(Transition, 'prototype', { writable: false });
  define(TypeSet, 'prototype', { writable: false });
  const method = (proto, name, length, fn) => define(proto, name, { value: make(fn, name, length), writable: true, enumerable: true, configurable: true });
  const getter = (proto, name, fn) => define(proto, name, { get: make(fn, 'get ' + name, 0), enumerable: true, configurable: true });
  const transitionOf = receiver => { const record = read(transitions, receiver); if (!record) throw illegal(); return record; };
  for (const name of ['finished', 'ready', 'updateCallbackDone']) {
    getter(transitionProto, name, function () {
      const record = read(transitions, this);
      if (!record) return apply(rejectPromise, PromiseValue, [new ErrorType("Failed to read the '" + name + "' property from 'ViewTransition': Illegal invocation")]);
      return record[name].promise;
    });
  }
  getter(transitionProto, 'types', function () { return transitionOf(this).typeSet; });
  getter(transitionProto, 'transitionRoot', function () { return transitionOf(this).root; });
  method(transitionProto, 'skipTransition', 0, function () {
    skip(transitionOf(this), exception('Transition was skipped. skipTransition() called', 'AbortError'));
  });
  method(transitionProto, 'waitUntil', 1, function (promise) {
    const record = transitionOf(this);
    required(arguments.length, 'waitUntil', 'ViewTransition');
    const waited = apply(resolvePromise, PromiseValue, [promise]);
    if (record.phase === PHASE_DONE) return;
    record.waiting++;
    const settled = () => {
      if (--record.waiting !== 0 || record.phase !== PHASE_ANIMATING || !record.ended) return;
      frame(record, () => { if (record.phase === PHASE_ANIMATING && record.waiting === 0) finish(record); });
    };
    apply(then, waited, [settled, settled]);
  });
  define(transitionProto, 'constructor', constructorDescriptor);
  define(transitionProto, Symbol.toStringTag, { value: 'ViewTransition', configurable: true });

  // ViewTransitionTypeSet: setlike<DOMString>; iterators stay live across mutation.
  const setOf = receiver => { const record = read(typeSets, receiver); if (!record) throw illegal(); return record; };
  const typesPrefix = operation => "Failed to execute '" + operation + "' on 'ViewTransitionTypeSet': ";
  const iteratorProto = create(getPrototypeOf(mapIteratorProto));
  method(iteratorProto, 'next', 0, function () {
    const state = read(iterators, this);
    if (!state) throw illegal();
    const types = state.record.types;
    if (state.exhausted) {
      if (state.observed === types.generation) return { done: true };
      state.iterator = apply(mapEntries, types.values, []);
      state.exhausted = false;
    }
    for (;;) {
      const next = apply(mapNext, state.iterator, []);
      if (next.done) { state.exhausted = true; state.observed = types.generation; return { done: true }; }
      const key = next.value[0], generation = next.value[1];
      if (generation <= state.cursor) continue;
      state.cursor = generation;
      return { done: false, value: state.entries ? [key, key] : key };
    }
  });
  define(iteratorProto, Symbol.toStringTag, { value: 'ViewTransitionTypeSet Iterator', configurable: true });
  getter(typeSetProto, 'size', function () { return apply(mapSize, setOf(this).types.values, []); });
  method(typeSetProto, 'add', 1, function (value) {
    const record = setOf(this), types = record.types;
    required(arguments.length, 'add', 'ViewTransitionTypeSet');
    value = string(value, typesPrefix('add'));
    if (!apply(mapHas, types.values, [value])) { apply(mapSet, types.values, [value, ++types.generation]); publish(record); }
  });
  method(typeSetProto, 'clear', 0, function () {
    const record = setOf(this);
    apply(mapClear, record.types.values, []);
    publish(record);
  });
  method(typeSetProto, 'delete', 1, function (value) {
    const record = setOf(this);
    required(arguments.length, 'delete', 'ViewTransitionTypeSet');
    const removed = apply(mapDelete, record.types.values, [string(value, typesPrefix('delete'))]);
    if (removed) publish(record);
    return removed;
  });
  const iteration = (name, entries) => method(typeSetProto, name, 0, function () {
    const record = setOf(this), value = create(iteratorProto), state = create(null);
    state.record = record; state.entries = entries; state.cursor = 0; state.exhausted = false;
    state.observed = record.types.generation; state.iterator = apply(mapEntries, record.types.values, []);
    apply(weakSet, iterators, [value, state]);
    return value;
  });
  iteration('entries', true);
  method(typeSetProto, 'forEach', 1, function (callback, thisArg) {
    const record = setOf(this), self = this;
    required(arguments.length, 'forEach', 'ViewTransitionTypeSet');
    if (typeof callback !== 'function') throw new ErrorType(typesPrefix('forEach') + "parameter 1 is not of type 'Function'.");
    apply(mapForEach, record.types.values, [(_generation, key) => apply(callback, thisArg, [key, key, self])]);
  });
  method(typeSetProto, 'has', 1, function (value) {
    const record = setOf(this);
    required(arguments.length, 'has', 'ViewTransitionTypeSet');
    return apply(mapHas, record.types.values, [string(value, typesPrefix('has'))]);
  });
  iteration('keys', false);
  iteration('values', false);
  define(typeSetProto, 'constructor', typeSetConstructor);
  define(typeSetProto, Symbol.toStringTag, { value: 'ViewTransitionTypeSet', configurable: true });
  define(typeSetProto, Symbol.iterator, { value: typeSetProto.values, writable: true, configurable: true });

  // https://drafts.csswg.org/css-view-transitions-1/#dom-document-startviewtransition and the
  // element-scoped variant; a transition started on the document element is the document's.
  function start(scope, document, root, value, owner) {
    const parsed = options(value, owner);
    if (root === null) return null;
    const record = create(null);
    record.scope = scope; record.document = document; record.root = root;
    record.callback = parsed.callback; record.phase = PHASE_PENDING;
    record.invoked = false; record.called = false; record.suppressing = false;
    record.callbackState = null; record.callbackReason = undefined;
    record.start = null; record.ended = false; record.waiting = 0;
    record.updateCallbackDone = deferred(); record.ready = deferred(); record.finished = deferred();
    record.types = create(null); record.types.values = new MapValue(); record.types.generation = 0;
    for (const type of parsed.types) if (!apply(mapHas, record.types.values, [type])) apply(mapSet, record.types.values, [type, ++record.types.generation]);
    const transition = create(transitionProto), typeSet = create(typeSetProto);
    record.typeSet = typeSet; record.transition = transition;
    apply(weakSet, transitions, [transition, record]);
    apply(weakSet, typeSets, [typeSet, record]);
    if (Native('inert', document) || scope !== document && !apply(connectedGetter, root, [])) {
      skip(record, invalid('Unsupported layout or style'));
      return transition;
    }
    if (host(record, 'hidden') === true) {
      skip(record, invalid('Document hidden'));
      return transition;
    }
    const previous = read(active, scope);
    if (previous) skip(previous, exception('Transition was skipped. New ViewTransition started', 'AbortError'));
    apply(weakSet, active, [scope, record]);
    publish(record);
    frame(record, () => capture(record));
    return transition;
  }
  const documentProto = g.Document.prototype, elementProto = g.Element.prototype;
  method(documentProto, 'startViewTransition', 0, function (value) {
    apply(documentBrand, this, []);
    return start(this, this, apply(documentElementOf, this, []), value, 'Document');
  });
  getter(documentProto, 'activeViewTransition', function () {
    apply(documentBrand, this, []);
    const record = read(active, this);
    return record ? record.transition : null;
  });
  method(elementProto, 'startViewTransition', 0, function (value) {
    apply(elementBrand, this, []);
    const document = apply(ownerDocumentOf, this, []);
    const scope = apply(documentElementOf, document, []) === this ? document : this;
    return start(scope, document, this, value, 'Element');
  });
  getter(elementProto, 'activeViewTransition', function () {
    apply(elementBrand, this, []);
    const document = apply(ownerDocumentOf, this, []);
    const record = read(active, apply(documentElementOf, document, []) === this ? document : this);
    return record ? record.transition : null;
  });
  define(g, 'ViewTransition', { value: Transition, writable: true, configurable: true });
  define(g, 'ViewTransitionTypeSet', { value: TypeSet, writable: true, configurable: true });
})();
