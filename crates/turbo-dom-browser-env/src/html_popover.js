// Nullable/enumerated HTML reflection and genuine ToggleEvent construction.
// Top-layer rendering and opening/closing policy require separate host support.
(function () {
  'use strict';
  const Native = globalThis.__popoverNative; delete globalThis.__popoverNative;
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction, StringValue = String, ObjectValue = Object;
  const check = Object.getOwnPropertyDescriptor(g.Node.prototype, 'nodeType').get;
  const namespace = Object.getOwnPropertyDescriptor(g.Element.prototype, 'namespaceURI').get;
  const set = g.Element.prototype.setAttributeNS, remove = g.Element.prototype.removeAttributeNS;
  const string = value => { if (typeof value === 'symbol') throw new TypeError('Cannot convert a Symbol to a string'); return StringValue(value); };
  const html = receiver => { if (check.call(receiver) !== 1 || namespace.call(receiver) !== 'http://www.w3.org/1999/xhtml') throw new TypeError('Illegal invocation'); };
  Object.defineProperty(g.HTMLElement.prototype, 'popover', {
    enumerable: true, configurable: true,
    get: make(function () {
      html(this); return Native(this, 'read');
    }, 'get popover', 0),
    set: make(function (value) { html(this); if (value === null) remove.call(this, null, 'popover'); else set.call(this, null, 'popover', string(value)); }, 'set popover', 1),
  });
  // Event `source` getters retarget against the current target, or out of every shadow tree
  // when there is none, so closed shadow internals never leak through the event.
  const rootOf = g.Node.prototype.getRootNode, hostOf = Object.getOwnPropertyDescriptor(g.ShadowRoot.prototype, 'host').get;
  const currentTargetOf = Object.getOwnPropertyDescriptor(g.Event.prototype, 'currentTarget').get;
  const shadowHost = root => { try { return hostOf.call(root); } catch (_) { return null; } };
  const isNode = value => { try { check.call(value); return true; } catch (_) { return false; } };
  const retarget = (source, event) => {
    if (source === null) return null;
    const current = currentTargetOf.call(event), roots = [];
    if (current !== null && isNode(current)) for (let root = rootOf.call(current), host; root; root = host ? rootOf.call(host) : null) { roots[roots.length] = root; host = shadowHost(root); }
    const reached = root => { for (let index = 0; index < roots.length; index++) if (roots[index] === root) return true; return false; };
    for (let root = rootOf.call(source), host; (host = shadowHost(root)) && !reached(root); root = rootOf.call(source)) source = host;
    return source;
  };
  ns.retargetEventSource = retarget;
  if (typeof g.ToggleEvent === 'undefined') {
    const Event = g.Event, construct = Reflect.construct;
    const records = new WeakMap(), read = records.get.bind(records), store = records.set.bind(records);
    const Delegate = function (type, init) {
      if (!new.target) throw new TypeError('ToggleEvent requires new');
      if (!arguments.length) throw new TypeError('ToggleEvent requires 1 argument');
      const convertedType = string(type);
      if (init != null && typeof init !== 'object' && typeof init !== 'function') throw new TypeError('ToggleEventInit must be a dictionary');
      init = init == null ? {} : init;
      const eventInit = { bubbles: !!init.bubbles, cancelable: !!init.cancelable, composed: !!init.composed };
      const newValue = init.newState, newState = newValue === undefined ? '' : string(newValue);
      const oldValue = init.oldState, oldState = oldValue === undefined ? '' : string(oldValue);
      const sourceValue = init.source;
      const source = sourceValue === undefined || sourceValue === null ? null : sourceValue;
      if (source !== null && check.call(source) !== 1) throw new TypeError('ToggleEvent source must be an Element');
      const event = construct(Event, [convertedType, eventInit], new.target);
      store(event, { oldState, newState, source });
      return event;
    };
    Object.setPrototypeOf(Delegate.prototype, Event.prototype);
    Object.defineProperty(Delegate.prototype, Symbol.toStringTag, { value: 'ToggleEvent', configurable: true });
    for (const name of ['oldState', 'newState', 'source']) Object.defineProperty(Delegate.prototype, name, {
      enumerable: true, configurable: true,
      get: make(function () { const record = read(this); if (!record) throw new TypeError('Illegal invocation'); return name === 'source' ? retarget(record.source, this) : record[name]; }, 'get ' + name, 0),
    });
    const constructor = make(Delegate, 'ToggleEvent', 1, true, true);
    constructor.prototype = Delegate.prototype;
    Object.defineProperty(Delegate.prototype, 'constructor', { value: constructor, writable: true, configurable: true });
    Object.defineProperty(constructor, 'prototype', { writable: false });
    Object.setPrototypeOf(constructor, Event);
    Object.defineProperty(g, 'ToggleEvent', { value: constructor, writable: true, configurable: true });
  }
  const methods = {
    showPopover(options) { html(this); if (options != null && typeof options !== 'object' && typeof options !== 'function') throw new TypeError('ShowPopoverOptions must be a dictionary'); options = options == null ? {} : options; const sourceValue = options.source; const source = sourceValue === undefined || sourceValue === null ? null : sourceValue; if (source !== null && check.call(source) !== 1) throw new TypeError('The source must be an Element'); Native(this, 'show', undefined, source); },
    hidePopover() { html(this); Native(this, 'hide'); },
    togglePopover(options) { html(this); let force, source = null; if (options !== undefined) { if (options !== null && typeof options === 'object') { force = options.force; const sourceValue = options.source; source = sourceValue === undefined || sourceValue === null ? null : sourceValue; } else force = !!options; } if (source !== null && check.call(source) !== 1) throw new TypeError('The source must be an Element'); return Native(this, 'toggle', force, source); },
  };
  for (const name of ObjectValue.keys(methods)) ObjectValue.defineProperty(g.HTMLElement.prototype, name, {value: make(methods[name], name, 0), writable: true, configurable: true, enumerable: true});
})();
