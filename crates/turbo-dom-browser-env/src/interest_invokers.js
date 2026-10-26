// Interest invokers: `interestfor` reflection, a genuine InterestEvent, and the private
// interest state machine the native trusted focus/blur/hover/Escape hooks drive. Delays come
// from the invoker's computed `interest-delay-start`/`-end` and run on the host scheduler.
(function () {
  'use strict';
  const Native = globalThis.__interestNative; delete globalThis.__interestNative;
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction, StringValue = String;
  const reference = ns.elementReference, retarget = ns.retargetEventSource, resolve = ns.resolveReferenceTarget;
  const accessor = (proto, name) => Object.getOwnPropertyDescriptor(proto, name).get;
  const nodeType = accessor(g.Node.prototype, 'nodeType'), namespace = accessor(g.Element.prototype, 'namespaceURI');
  const localName = accessor(g.Element.prototype, 'localName'), parentNode = accessor(g.Node.prototype, 'parentNode');
  const connected = accessor(g.Node.prototype, 'isConnected'), hostOf = accessor(g.ShadowRoot.prototype, 'host');
  const popoverOf = accessor(g.HTMLElement.prototype, 'popover'), relatedOf = accessor(g.MouseEvent.prototype, 'relatedTarget');
  const keyOf = accessor(g.KeyboardEvent.prototype, 'key'), preventedOf = accessor(g.Event.prototype, 'defaultPrevented');
  const hasAttribute = g.Element.prototype.hasAttribute, hasAttributeNS = g.Element.prototype.hasAttributeNS;
  const computed = g.getComputedStyle, getPropertyValue = g.CSSStyleDeclaration ? g.CSSStyleDeclaration.prototype.getPropertyValue : null;
  const construct = Reflect.construct, apply = Reflect.apply, Event = g.Event;
  const WeakMapValue = WeakMap, SetValue = Set, ArrayValue = Array;
  const mapGet = WeakMapValue.prototype.get, mapSet = WeakMapValue.prototype.set, mapDelete = WeakMapValue.prototype.delete;
  const setAdd = SetValue.prototype.add, setDelete = SetValue.prototype.delete, setValues = SetValue.prototype.values;
  const sizeOf = Object.getOwnPropertyDescriptor(SetValue.prototype, 'size').get;
  const MapValue = Map, ownerGet = MapValue.prototype.get, ownerSet = MapValue.prototype.set, ownerDelete = MapValue.prototype.delete;
  const ownerKeys = MapValue.prototype.keys, ownerEntries = MapValue.prototype.entries, ownerSize = Object.getOwnPropertyDescriptor(MapValue.prototype, 'size').get;
  const HTML = 'http://www.w3.org/1999/xhtml', SVG = 'http://www.w3.org/2000/svg', XLINK = 'http://www.w3.org/1999/xlink';
  const string = value => { if (typeof value === 'symbol') throw new TypeError('Cannot convert a Symbol to a string'); return StringValue(value); };
  const element = value => { try { return nodeType.call(value) === 1; } catch { return false; } };
  const is = (value, ns, name) => element(value) && namespace.call(value) === ns && localName.call(value) === name;

  // HTMLButtonElement, HTMLAnchorElement and HTMLAreaElement reflect `interestfor` as an
  // element reference sharing the scoped explicit-reference state of popovertarget/commandfor.
  for (const [constructor, tag] of [[g.HTMLButtonElement, 'button'], [g.HTMLAnchorElement, 'a'], [g.HTMLAreaElement, 'area']]) {
    if (typeof constructor !== 'function') continue;
    const check = receiver => { if (!is(receiver, HTML, tag)) throw new TypeError('Illegal invocation'); };
    const conversion = `Failed to set the 'interestForElement' property on '${constructor.name}': Failed to convert value to 'Element'.`;
    Object.defineProperty(constructor.prototype, 'interestForElement', {
      enumerable: true, configurable: true,
      get: make(function () { check(this); return reference.read(this, 'interestfor'); }, 'get interestForElement', 0),
      set: make(function (value) {
        check(this);
        if (value !== null && value !== undefined && !element(value)) throw new TypeError(conversion);
        reference.assign(this, 'interestfor', value);
      }, 'set interestForElement', 1),
    });
  }

  const records = new WeakMapValue(), read = records.get.bind(records), store = records.set.bind(records);
  const Delegate = function (type, init) {
    if (!new.target) throw new TypeError("Failed to construct 'InterestEvent': Please use the 'new' operator, this DOM object constructor cannot be called as a function.");
    if (!arguments.length) throw new TypeError("Failed to construct 'InterestEvent': 1 argument required, but only 0 present.");
    const convertedType = string(type);
    if (init != null && typeof init !== 'object' && typeof init !== 'function') throw new TypeError("Failed to construct 'InterestEvent': The provided value is not of type 'InterestEventInit'.");
    init = init == null ? {} : init;
    const eventInit = { bubbles: !!init.bubbles, cancelable: !!init.cancelable, composed: !!init.composed };
    const sourceValue = init.source, source = sourceValue === undefined || sourceValue === null ? null : sourceValue;
    if (source !== null && !element(source)) throw new TypeError("Failed to construct 'InterestEvent': Failed to read the 'source' property from 'InterestEventInit': Failed to convert value to 'Element'.");
    const event = construct(Event, [convertedType, eventInit], new.target);
    store(event, { source });
    return event;
  };
  Object.setPrototypeOf(Delegate.prototype, Event.prototype);
  delete Delegate.prototype.constructor;
  Object.defineProperty(Delegate.prototype, Symbol.toStringTag, { value: 'InterestEvent', configurable: true });
  Object.defineProperty(Delegate.prototype, 'source', {
    enumerable: true, configurable: true,
    get: make(function () { const record = read(this); if (!record) throw new TypeError('Illegal invocation'); return retarget(record.source, this); }, 'get source', 0),
  });
  // The delegate itself rejects calls without `new`, with Chrome's message.
  const InterestEvent = make(Delegate, 'InterestEvent', 1, true, true, false);
  InterestEvent.prototype = Delegate.prototype;
  Object.defineProperty(Delegate.prototype, 'constructor', { value: InterestEvent, writable: true, configurable: true });
  Object.defineProperty(InterestEvent, 'prototype', { writable: false });
  Object.setPrototypeOf(InterestEvent, Event);
  Object.defineProperty(g, 'InterestEvent', { value: InterestEvent, writable: true, configurable: true });

  // ---- Interest state -------------------------------------------------------------------
  // invoker -> { target, phase: 'pending' | 'shown' | 'losing', token, opened }. `shown` and
  // `losing` invokers are interest sources. `owners` maps each interest target to its invoker;
  // like Chrome, retargeting `interestfor` ends the source state but leaves the target's.
  const state = new WeakMapValue(), active = new SetValue(), owners = new MapValue();
  const recordOf = invoker => apply(mapGet, state, [invoker]);
  const ownerOf = target => apply(ownerGet, owners, [target]);
  let tokens = 0;
  const shadowHost = root => { try { return hostOf.call(root); } catch { return null; } };
  const up = node => parentNode.call(node) || shadowHost(node);
  const within = (node, ancestor) => { for (let current = node; current; current = up(current)) if (current === ancestor) return true; return false; };
  const chain = node => { const list = []; for (let current = node; current; current = up(current)) if (element(current)) list[list.length] = current; return list; };
  const includes = (list, value) => { for (let index = 0; index < list.length; index++) if (list[index] === value) return true; return false; };
  // Interest-invoker elements: an enabled HTML button, or an HTML a/area or SVG a with href.
  const linked = node => hasAttribute.call(node, 'href') || namespace.call(node) === SVG && hasAttributeNS.call(node, XLINK, 'href');
  const invokerElement = node => (is(node, HTML, 'button') && !Native('disabled', node)) || ((is(node, HTML, 'a') || is(node, HTML, 'area') || is(node, SVG, 'a')) && linked(node));
  const targetOf = node => {
    if (!invokerElement(node)) return null;
    const target = resolve(reference.read(node, 'interestfor'));
    return target !== null && connected.call(node) && connected.call(target) ? target : null;
  };
  // `normal` start/end delays are 0.5s and 0.25s; other values are CSS <time>s.
  function delay(invoker, edge) {
    let text = '';
    try { const style = computed(invoker); text = getPropertyValue ? apply(getPropertyValue, style, ['interest-delay-' + edge]) : style['interest-delay-' + edge]; } catch {}
    text = StringValue(text || '').trim().toLowerCase();
    const match = /^(-?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)(ms|s)$/.exec(text);
    if (!match) return edge === 'start' ? 500 : 250;
    const value = +match[1] * (match[2] === 's' ? 1000 : 1);
    return value > 0 ? value : 0;
  }
  const fire = (target, type, source, cancelable) => Native('dispatch', target, new InterestEvent(type, { cancelable, composed: true, source }));
  const popover = node => namespace.call(node) === HTML && popoverOf.call(node) !== null;
  const engaged = () => Native('engaged', apply(sizeOf, active, []) > 0 || apply(ownerSize, owners, []) > 0);
  function clear(invoker, keepTarget) {
    const record = recordOf(invoker);
    if (!record) return;
    apply(mapDelete, state, [invoker]);
    apply(setDelete, active, [invoker]);
    if (!keepTarget && ownerOf(record.target) === invoker) apply(ownerDelete, owners, [record.target]);
    engaged();
  }
  function gain(invoker) {
    const target = targetOf(invoker);
    if (target === null) return;
    const record = recordOf(invoker);
    if (record && record.target === target) {
      // Re-engaging cancels a pending loss; a pending gain keeps its original timer.
      if (record.phase === 'losing') { record.phase = 'shown'; record.token = ++tokens; }
      return;
    }
    if (record) clear(invoker);
    const next = { target, phase: 'pending', token: ++tokens, opened: false }, token = next.token;
    apply(mapSet, state, [invoker, next]);
    Native('schedule', () => { if (recordOf(invoker) === next && next.token === token) show(invoker, next); }, delay(invoker, 'start'));
  }
  function show(invoker, record) {
    if (targetOf(invoker) !== record.target) { clear(invoker); return; }
    const allowed = fire(record.target, 'interest', invoker, true);
    if (recordOf(invoker) !== record) return;
    if (!allowed) { clear(invoker); return; }
    record.phase = 'shown';
    apply(setAdd, active, [invoker]);
    apply(ownerSet, owners, [record.target, invoker]);
    engaged();
    // Only a popover this interest opened is hidden again when the interest ends.
    if (popover(record.target) && !Native('open', record.target)) {
      Native('popover', record.target, 'show', invoker);
      record.opened = Native('open', record.target);
    }
  }
  function lose(invoker) {
    const record = recordOf(invoker);
    if (!record) return;
    if (record.phase === 'pending') { clear(invoker); return; }
    if (record.phase !== 'shown') return;
    record.phase = 'losing';
    const token = record.token = ++tokens;
    Native('schedule', () => { if (recordOf(invoker) === record && record.token === token) end(invoker, record, true); }, delay(invoker, 'end'));
  }
  function end(invoker, record, cancelable) {
    const allowed = fire(record.target, 'loseinterest', invoker, cancelable);
    if (recordOf(invoker) !== record) return;
    if (cancelable && !allowed) { record.phase = 'shown'; record.token = ++tokens; return; }
    clear(invoker);
    if (record.opened && popover(record.target)) Native('popover', record.target, 'hide', invoker);
  }
  const each = callback => { const list = []; for (const invoker of apply(setValues, active, [])) list[list.length] = invoker; for (let index = 0; index < list.length; index++) callback(list[index]); };
  function enter(node, leaving) {
    const left = leaving ? chain(leaving) : [];
    for (const current of chain(node)) {
      if (includes(left, current)) continue;
      const owner = ownerOf(current);
      if (owner && recordOf(owner)) gain(owner);
      // Explicit element references also set the (empty) attribute.
      if (hasAttribute.call(current, 'interestfor')) gain(current);
    }
  }
  function leave(node, entering) {
    const kept = entering ? chain(entering) : [];
    for (const current of chain(node)) {
      if (includes(kept, current)) continue;
      const owner = ownerOf(current);
      if (owner) lose(owner);
      if (recordOf(current)) lose(current);
    }
  }
  const related = event => { try { return relatedOf.call(event); } catch { return null; } };
  // Native hooks: trusted focus/blur/mouseover/mouseout/keydown, removal and attribute changes.
  const hook = (kind, node, event, extra) => {
    switch (kind) {
      case 'focus': enter(node, null); return;
      case 'blur': leave(node, Native('focused')); return;
      case 'mouseover': enter(node, related(event)); return;
      case 'mouseout': leave(node, related(event)); return;
      case 'keydown': {
        let key = '', prevented = true;
        try { key = keyOf.call(event); prevented = preventedOf.call(event); } catch {}
        // An unprevented Escape ends every shown interest at once, without the end delay;
        // the loseinterest it fires cannot be canceled.
        if (key === 'Escape' && !prevented) each(invoker => { const record = recordOf(invoker); if (record) end(invoker, record, false); });
        return;
      }
      case 'disconnect': {
        const inside = target => within(target, node) && (extra || target !== node);
        each(invoker => { const record = recordOf(invoker); if (record && (inside(invoker) || inside(record.target))) clear(invoker); });
        for (const [target, invoker] of apply(ownerEntries, owners, [])) if (!recordOf(invoker) && (inside(target) || inside(invoker))) apply(ownerDelete, owners, [target]);
        engaged();
        return;
      }
      // Removing `interestfor` ends the interest; pointing it elsewhere ends only the source.
      case 'attribute': if (recordOf(node)) clear(node, hasAttribute.call(node, 'interestfor')); return;
      case 'sources': { const list = new ArrayValue(); each(invoker => { list[list.length] = invoker; }); return list; }
      case 'targets': { const list = new ArrayValue(); for (const target of apply(ownerKeys, owners, [])) list[list.length] = target; return list; }
    }
  };
  g.__captureInterestHook = hook;
})();
