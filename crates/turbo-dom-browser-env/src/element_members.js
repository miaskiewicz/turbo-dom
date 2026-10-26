// Element-owned legacy members: the webkit-prefixed matcher and Element IDL handlers.
// Content-attribute compilation stays native (event_handlers.rs); this only owns IDL state.
(function () {
  'use strict';
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction;
  const matches = g.__elementMembersMatches, prepare = g.__prepareInlineHandler, note = g.__noteInlineHandler;
  delete g.__elementMembersMatches; delete g.__prepareInlineHandler; delete g.__noteInlineHandler;
  const nodeType = Object.getOwnPropertyDescriptor(g.Node.prototype, 'nodeType').get;
  const listen = g.EventTarget.prototype.addEventListener, unlisten = g.EventTarget.prototype.removeEventListener;
  const apply = Reflect.apply, StringValue = String, LocalTypeError = TypeError;
  const records = new WeakMap(), read = records.get.bind(records), store = records.set.bind(records);
  const element = receiver => { if (apply(nodeType, receiver, []) !== 1) throw new LocalTypeError('Illegal invocation'); };
  const failure = (operation, detail) => new LocalTypeError(`Failed to execute '${operation}' on 'Element': ${detail}`);
  Object.defineProperty(g.Element.prototype, 'webkitMatchesSelector', {
    value: make(function (selectors) {
      element(this);
      if (!arguments.length) throw failure('webkitMatchesSelector', '1 argument required, but only 0 present.');
      if (typeof selectors === 'symbol') throw failure('webkitMatchesSelector', 'Cannot convert a Symbol value to a string');
      return apply(matches, this, [StringValue(selectors)]);
    }, 'webkitMatchesSelector', 1),
    writable: true, enumerable: true, configurable: true,
  });
  // Chrome keeps these on Element, not GlobalEventHandlers: SVG and MathML share them.
  const types = ['beforecopy', 'beforecut', 'beforepaste', 'search', 'fullscreenchange', 'fullscreenerror', 'webkitfullscreenchange', 'webkitfullscreenerror'];
  for (const type of types) {
    const name = 'on' + type;
    // A non-callable object is retained ([LegacyTreatNonObjectAsNull]) but invokes nothing.
    const callback = make(function (event) {
      const current = read(this)[name].value;
      if (typeof current === 'function' && apply(current, this, [event]) === false) event.preventDefault();
    }, name, 1);
    Object.defineProperty(g.Element.prototype, name, {
      enumerable: true, configurable: true,
      get: make(function () {
        element(this); apply(prepare, this, [name]);
        const record = read(this);
        return record && record[name] ? record[name].value : null;
      }, 'get ' + name, 0),
      set: make(function (value) {
        element(this); apply(note, this, [name]);
        value = value !== null && (typeof value === 'object' || typeof value === 'function') ? value : null;
        let record = read(this);
        if (!record) { record = { __proto__: null }; store(this, record); }
        const entry = record[name] || (record[name] = { value: null });
        if (value && !entry.value) apply(listen, this, [type, callback]);
        if (!value && entry.value) apply(unlisten, this, [type, callback]);
        entry.value = value;
      }, 'set ' + name, 1),
    });
  }
})();
