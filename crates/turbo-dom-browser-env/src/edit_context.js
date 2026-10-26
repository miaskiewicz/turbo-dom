// EditContext (text, selection and bounds state, attachment through HTMLElement.editContext, the
// textupdate / textformatupdate / characterboundsupdate / composition event handlers and their event
// interfaces) and Element/Document.ariaNotify.
//
// EditContext events come only from text input methods. Browserless there is no IME or OS keyboard,
// so the engine fires them only for host-driven trusted text input (ns.editContextInput); script
// state updates never fire events, exactly as in Chrome. ariaNotify validates its arguments like
// Chrome 156 and queues nothing observable: announcements go to assistive technology, which a
// browserless engine does not have.
(function () {
  'use strict';
  const native = globalThis.__editContextNative; delete globalThis.__editContextNative;
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction;
  const apply = Reflect.apply, construct = Reflect.construct, define = Object.defineProperty, getOwn = Object.getOwnPropertyDescriptor, getProto = Object.getPrototypeOf, setProto = Object.setPrototypeOf, create = Object.create;
  const ErrorType = TypeError, StringValue = String, NumberValue = Number, WeakMapType = WeakMap, MathMin = Math.min, MathFloor = Math.floor, MathAbs = Math.abs;
  const wmGet = WeakMap.prototype.get, wmSet = WeakMap.prototype.set, wmDelete = WeakMap.prototype.delete;
  const isPrototypeOf = Object.prototype.isPrototypeOf, sliceF = String.prototype.slice, iteratorSymbol = Symbol.iterator;
  const wget = (m, k) => apply(wmGet, m, [k]), wset = (m, k, v) => apply(wmSet, m, [k, v]), wdelete = (m, k) => apply(wmDelete, m, [k]);
  const sub = (s, a, b) => apply(sliceF, s, b === undefined ? [a] : [a, b]);
  const push = (a, v) => define(a, a.length, { __proto__: null, value: v, writable: true, enumerable: true, configurable: true });
  function lookup(object, name) { for (let o = object; o !== null && o !== undefined; o = getProto(o)) { const d = getOwn(o, name); if (d) return d; } return null; }
  const EventType = g.Event, Target = g.EventTarget, Exception = g.DOMException;
  const addListener = lookup(Target.prototype, 'addEventListener').value, removeListener = lookup(Target.prototype, 'removeEventListener').value;
  const typeOf = lookup(g.Node.prototype, 'nodeType').get, localOf = lookup(g.Element.prototype, 'localName').get, namespaceOf = lookup(g.Element.prototype, 'namespaceURI').get;
  const HTML = 'http://www.w3.org/1999/xhtml';
  const fail = (kind, operation, message) => new ErrorType("Failed to " + (kind === 'construct' ? "construct '" + operation + "'" : kind === 'set' ? "set the '" + operation[1] + "' property on '" + operation[0] + "'" : "execute '" + operation[1] + "' on '" + operation[0] + "'") + ': ' + message);
  const string = (value, prefix) => { if (typeof value === 'symbol') throw new ErrorType(prefix + 'Cannot convert a Symbol value to a string'); return StringValue(value); };
  // WebIDL unsigned long (non-enforced): ToNumber, truncate, modulo 2^32.
  function unsignedLong(value) {
    const n = NumberValue(value);
    if (n !== n || n === Infinity || n === -Infinity || n === 0) return 0;
    const t = (n < 0 ? -1 : 1) * MathFloor(MathAbs(n));
    const m = t % 4294967296;
    return m < 0 ? m + 4294967296 : m;
  }
  function dictionary(value, error) {
    if (value === undefined || value === null) return null;
    if (typeof value !== 'object' && typeof value !== 'function') throw error();
    return value;
  }
  function requireArguments(count, args, operation) {
    if (args.length < count) throw fail('execute', operation, count + ' argument' + (count === 1 ? '' : 's') + ' required, but only ' + args.length + ' present.');
  }
  const method = (proto, name, body, length) => define(proto, name, { __proto__: null, value: make(body, name, length), writable: true, enumerable: true, configurable: true });
  const getter = (proto, name, body) => define(proto, name, { __proto__: null, get: make(body, 'get ' + name, 0), enumerable: true, configurable: true });
  function finishInterface(Delegate, name, length, parent, Parent) {
    const constructor = make(Delegate, name, length, true, true, false);
    constructor.prototype = Delegate.prototype;
    define(Delegate.prototype, 'constructor', { __proto__: null, value: constructor, writable: true, configurable: true });
    define(Delegate.prototype, Symbol.toStringTag, { __proto__: null, value: name, configurable: true });
    define(constructor, 'prototype', { __proto__: null, writable: false });
    if (parent) setProto(Delegate.prototype, parent);
    if (Parent) setProto(constructor, Parent);
    define(g, name, { __proto__: null, value: constructor, writable: true, configurable: true });
    return constructor;
  }
  const requireNew = (target, name) => { if (!target) throw new ErrorType("Failed to construct '" + name + "': Please use the 'new' operator, this DOM object constructor cannot be called as a function."); };

  // ---- DOMRect access (resolved on first use: embedders install the geometry interfaces late) ---------
  let rect = null;
  function rects() {
    if (rect !== null) return rect;
    const Rect = g.DOMRect, Read = g.DOMRectReadOnly;
    const x = lookup(Rect.prototype, 'x');
    rect = create(null);
    rect.Rect = Rect;
    rect.prototype = Rect.prototype;
    rect.read = name => { const d = lookup(Read.prototype, name); return d && d.get ? value => apply(d.get, value, []) : value => value[name]; };
    rect.x = rect.read('x'); rect.y = rect.read('y'); rect.width = rect.read('width'); rect.height = rect.read('height');
    // Only DOMRect (not DOMRectReadOnly) has the x setter's mutable brand; writing the current value back is unobservable.
    rect.brand = x && x.set ? value => { try { apply(x.set, value, [apply(x.get, value, [])]); return true; } catch (error) { return false; } } : value => value !== null && typeof value === 'object' && apply(isPrototypeOf, Rect.prototype, [value]);
    return rect;
  }
  const copyRect = value => { const r = rects(); return [r.x(value), r.y(value), r.width(value), r.height(value)]; };

  // ---- EditContext ----------------------------------------------------------------------------------
  const states = new WeakMapType(), attachments = new WeakMapType();
  const stateOf = receiver => { const s = receiver !== null && (typeof receiver === 'object' || typeof receiver === 'function') ? wget(states, receiver) : undefined; if (!s) throw new ErrorType('Illegal invocation'); return s; };
  const handlerNames = ['textupdate', 'textformatupdate', 'characterboundsupdate', 'compositionstart', 'compositionend'];
  class EditContextImplementation extends Target {
    constructor(options) {
      const init = dictionary(options, () => new ErrorType("Failed to construct 'EditContext': The provided value is not of type 'EditContextInit'."));
      let end = 0, start = 0, text = '';
      if (init !== null) {
        const e = init.selectionEnd; if (e !== undefined) end = unsignedLong(e);
        const s = init.selectionStart; if (s !== undefined) start = unsignedLong(s);
        const t = init.text; if (t !== undefined) text = string(t, "Failed to construct 'EditContext': Failed to read the 'text' property from 'EditContextInit': ");
      }
      super();
      const state = create(null);
      state.text = text; state.start = MathMin(start, text.length); state.end = MathMin(end, text.length);
      state.boundsStart = 0; state.bounds = []; state.control = null; state.selection = null; state.element = null; state.handlers = create(null);
      wset(states, this, state);
    }
  }
  const EditContextPrototype = EditContextImplementation.prototype;
  delete EditContextPrototype.constructor;
  for (const [name, read] of [['text', s => s.text], ['selectionStart', s => s.start], ['selectionEnd', s => s.end], ['characterBoundsRangeStart', s => s.boundsStart]]) getter(EditContextPrototype, name, function () { return read(stateOf(this)); });
  for (const type of handlerNames) {
    const name = 'on' + type;
    define(EditContextPrototype, name, {
      __proto__: null, enumerable: true, configurable: true,
      get: make(function () { const h = stateOf(this).handlers[type]; return h ? h.callback : null; }, 'get ' + name, 0),
      set: make(function (value) {
        const state = stateOf(this), callback = typeof value === 'function' || (value !== null && typeof value === 'object') ? value : null, current = state.handlers[type];
        if (callback === null) { if (current) { apply(removeListener, this, [type, current.listener]); delete state.handlers[type]; } return; }
        if (current) { current.callback = callback; return; }
        const handler = create(null);
        handler.callback = callback;
        handler.listener = function (event) { const f = handler.callback; return typeof f === 'function' ? apply(f, this, [event]) : undefined; };
        state.handlers[type] = handler;
        apply(addListener, this, [type, handler.listener]);
      }, 'set ' + name, 1),
    });
  }
  const operation = name => ['EditContext', name];
  method(EditContextPrototype, 'attachedElements', function () { const s = stateOf(this), out = []; if (s.element !== null) push(out, s.element); return out; }, 0);
  method(EditContextPrototype, 'characterBounds', function () {
    const s = stateOf(this), out = [], r = rects();
    for (let i = 0; i < s.bounds.length; i++) { const b = s.bounds[i]; push(out, construct(r.Rect, [b[0], b[1], b[2], b[3]])); }
    return out;
  }, 0);
  method(EditContextPrototype, 'updateCharacterBounds', function (rangeStart, characterBounds) {
    const s = stateOf(this);
    requireArguments(2, arguments, operation('updateCharacterBounds'));
    const start = unsignedLong(rangeStart);
    if (characterBounds === null || (typeof characterBounds !== 'object' && typeof characterBounds !== 'function') || typeof characterBounds[iteratorSymbol] !== 'function') throw fail('execute', operation('updateCharacterBounds'), 'The provided value cannot be converted to a sequence.');
    const r = rects(), copies = [];
    for (const value of characterBounds) { if (!r.brand(value)) throw fail('execute', operation('updateCharacterBounds'), "Failed to convert value to 'DOMRect'."); push(copies, copyRect(value)); }
    s.boundsStart = start; s.bounds = copies;
  }, 2);
  for (const name of ['updateControlBounds']) method(EditContextPrototype, name, function (bounds) {
    const s = stateOf(this);
    requireArguments(1, arguments, operation(name));
    if (!rects().brand(bounds)) throw fail('execute', operation(name), "parameter 1 is not of type 'DOMRect'.");
    if (name === 'updateControlBounds') s.control = copyRect(bounds); else s.selection = copyRect(bounds);
  }, 1);
  method(EditContextPrototype, 'updateSelection', function (start, end) {
    const s = stateOf(this);
    requireArguments(2, arguments, operation('updateSelection'));
    const a = unsignedLong(start), b = unsignedLong(end);
    s.start = MathMin(a, s.text.length); s.end = MathMin(b, s.text.length);
  }, 2);
  for (const name of ['updateSelectionBounds']) method(EditContextPrototype, name, function (bounds) {
    const s = stateOf(this);
    requireArguments(1, arguments, operation(name));
    if (!rects().brand(bounds)) throw fail('execute', operation(name), "parameter 1 is not of type 'DOMRect'.");
    if (name === 'updateControlBounds') s.control = copyRect(bounds); else s.selection = copyRect(bounds);
  }, 1);
  method(EditContextPrototype, 'updateText', function (rangeStart, rangeEnd, text) {
    const s = stateOf(this);
    requireArguments(3, arguments, operation('updateText'));
    let a = unsignedLong(rangeStart), b = unsignedLong(rangeEnd);
    const value = string(text, "Failed to execute 'updateText' on 'EditContext': ");
    if (a > b) { const t = a; a = b; b = t; }
    b = MathMin(b, s.text.length); a = MathMin(a, b);
    s.text = sub(s.text, 0, a) + value + sub(s.text, b);
  }, 3);
  const EditContextDelegate = function EditContext(options) { requireNew(new.target, 'EditContext'); return construct(EditContextImplementation, [options], new.target); };
  EditContextDelegate.prototype = EditContextPrototype;
  const EditContext = finishInterface(EditContextDelegate, 'EditContext', 0, null, null);
  setProto(EditContextImplementation.prototype, Target.prototype);
  setProto(EditContext, Target);
  const isEditContext = value => value !== null && (typeof value === 'object' || typeof value === 'function') && wget(states, value) !== undefined;

  // ---- HTMLElement.editContext -------------------------------------------------------------------------
  // Elements that can host an EditContext: valid shadow hosts, <canvas> and autonomous custom elements.
  const hostNames = { __proto__: null, article: 1, aside: 1, blockquote: 1, body: 1, div: 1, footer: 1, h1: 1, h2: 1, h3: 1, h4: 1, h5: 1, h6: 1, header: 1, main: 1, nav: 1, p: 1, section: 1, span: 1, canvas: 1 };
  const reservedNames = { __proto__: null, 'annotation-xml': 1, 'color-profile': 1, 'font-face': 1, 'font-face-src': 1, 'font-face-uri': 1, 'font-face-format': 1, 'font-face-name': 1, 'missing-glyph': 1 };
  function validCustomName(name) {
    if (reservedNames[name] || name.length < 2) return false;
    const first = name.charCodeAt(0);
    if (first < 97 || first > 122) return false;
    let dash = false;
    for (let i = 1; i < name.length; i++) { const c = name.charCodeAt(i); if (c === 45) dash = true; else if (c >= 65 && c <= 90) return false; }
    return dash;
  }
  const htmlBrand = lookup(g.HTMLElement.prototype, 'hidden');
  const brandElement = receiver => { if (htmlBrand && htmlBrand.get) apply(htmlBrand.get, receiver, []); else if (!(receiver instanceof g.HTMLElement)) throw new ErrorType('Illegal invocation'); };
  const setError = message => new Exception("Failed to set the 'editContext' property on 'HTMLElement': " + message, 'NotSupportedError');
  define(g.HTMLElement.prototype, 'editContext', {
    __proto__: null, enumerable: true, configurable: true,
    get: make(function () { brandElement(this); const c = wget(attachments, this); return c === undefined ? null : c; }, 'get editContext', 0),
    set: make(function (value) {
      brandElement(this);
      if (value !== null && value !== undefined && !isEditContext(value)) throw new ErrorType("Failed to set the 'editContext' property on 'HTMLElement': Failed to convert value to 'EditContext'.");
      const context = value === undefined ? null : value;
      if (context !== null) {
        const name = apply(localOf, this, []);
        if (apply(namespaceOf, this, []) !== HTML || !(hostNames[name] || validCustomName(name))) throw setError('This element does not support EditContext');
        const state = wget(states, context);
        if (state.element !== null && state.element !== this) throw setError('An EditContext can be only be associated with a single element');
      }
      const previous = wget(attachments, this);
      if (previous !== undefined) { wget(states, previous).element = null; wdelete(attachments, this); }
      if (context !== null) { wget(states, context).element = this; wset(attachments, this, context); }
    }, 'set editContext', 1),
  });
  const contextOf = element => { const c = wget(attachments, element); return c === undefined ? null : c; };

  // ---- event interfaces -------------------------------------------------------------------------------
  const records = new WeakMapType();
  const recordOf = (receiver, kind) => { const r = receiver !== null && typeof receiver === 'object' ? wget(records, receiver) : undefined; if (!r || r.kind !== kind) throw new ErrorType('Illegal invocation'); return r; };
  function eventInit(init) { const out = create(null); out.bubbles = !!init.bubbles; out.cancelable = !!init.cancelable; out.composed = !!init.composed; return out; }
  function eventInterface(name, members, read) {
    const Delegate = function (type, initValue) {
      requireNew(new.target, name);
      if (!arguments.length) throw new ErrorType("Failed to construct '" + name + "': 1 argument required, but only 0 present.");
      const t = string(type, "Failed to construct '" + name + "': ");
      const init = dictionary(initValue, () => new ErrorType("Failed to construct '" + name + "': The provided value is not of type '" + name + "Init'.")) || create(null);
      const base = eventInit(init), values = read(init);
      const event = construct(EventType, [t, base], new.target);
      values.kind = name;
      wset(records, event, values);
      return event;
    };
    Delegate.prototype = create(EventType.prototype);
    for (const m of members) {
      if (m[1] === 'method') method(Delegate.prototype, m[0], m[2], 0);
      else getter(Delegate.prototype, m[0], function () { return recordOf(this, name)[m[0]]; });
    }
    return finishInterface(Delegate, name, 1, EventType.prototype, EventType);
  }
  const prefix = (name, dict, member) => "Failed to construct '" + name + "': Failed to read the '" + member + "' property from '" + dict + "': ";
  eventInterface('TextUpdateEvent', [['updateRangeStart'], ['updateRangeEnd'], ['text'], ['selectionStart'], ['selectionEnd']], init => {
    const r = create(null);
    const se = init.selectionEnd, ss = init.selectionStart, text = init.text, ure = init.updateRangeEnd, urs = init.updateRangeStart;
    r.selectionEnd = se === undefined ? 0 : unsignedLong(se);
    r.selectionStart = ss === undefined ? 0 : unsignedLong(ss);
    r.text = text === undefined ? '' : string(text, prefix('TextUpdateEvent', 'TextUpdateEventInit', 'text'));
    r.updateRangeEnd = ure === undefined ? 0 : unsignedLong(ure);
    r.updateRangeStart = urs === undefined ? 0 : unsignedLong(urs);
    return r;
  });
  const formats = new WeakMapType();
  const underlineStyles = { __proto__: null, none: 1, solid: 1, dotted: 1, dashed: 1, wavy: 1 }, underlineThicknesses = { __proto__: null, none: 1, thin: 1, thick: 1 };
  const TextFormatDelegate = function TextFormat(initValue) {
    requireNew(new.target, 'TextFormat');
    const init = dictionary(initValue, () => new ErrorType("Failed to construct 'TextFormat': The provided value is not of type 'TextFormatInit'.")) || create(null);
    const r = create(null);
    const re = init.rangeEnd, rs = init.rangeStart, style = init.underlineStyle, thickness = init.underlineThickness;
    r.rangeEnd = re === undefined ? 0 : unsignedLong(re);
    r.rangeStart = rs === undefined ? 0 : unsignedLong(rs);
    r.underlineStyle = 'none';
    if (style !== undefined) { const v = string(style, prefix('TextFormat', 'TextFormatInit', 'underlineStyle')); if (!underlineStyles[v]) throw new ErrorType(prefix('TextFormat', 'TextFormatInit', 'underlineStyle') + "The provided value '" + v + "' is not a valid enum value of type UnderlineStyle."); r.underlineStyle = v; }
    r.underlineThickness = 'none';
    if (thickness !== undefined) { const v = string(thickness, prefix('TextFormat', 'TextFormatInit', 'underlineThickness')); if (!underlineThicknesses[v]) throw new ErrorType(prefix('TextFormat', 'TextFormatInit', 'underlineThickness') + "The provided value '" + v + "' is not a valid enum value of type UnderlineThickness."); r.underlineThickness = v; }
    const object = create(new.target.prototype);
    wset(formats, object, r);
    return object;
  };
  delete TextFormatDelegate.prototype.constructor;
  for (const name of ['rangeStart', 'rangeEnd', 'underlineStyle', 'underlineThickness']) getter(TextFormatDelegate.prototype, name, function () { const r = this !== null && typeof this === 'object' ? wget(formats, this) : undefined; if (!r) throw new ErrorType('Illegal invocation'); return r[name]; });
  finishInterface(TextFormatDelegate, 'TextFormat', 0, null, null);
  eventInterface('TextFormatUpdateEvent', [['getTextFormats', 'method', function () { const r = recordOf(this, 'TextFormatUpdateEvent'), out = []; for (let i = 0; i < r.formats.length; i++) push(out, r.formats[i]); return out; }]], init => {
    const r = create(null), list = init.textFormats;
    r.formats = [];
    if (list !== undefined) {
      const p = prefix('TextFormatUpdateEvent', 'TextFormatUpdateEventInit', 'textFormats');
      if (list === null || (typeof list !== 'object' && typeof list !== 'function') || typeof list[iteratorSymbol] !== 'function') throw new ErrorType(p + 'The provided value cannot be converted to a sequence.');
      for (const value of list) { if (value === null || typeof value !== 'object' || wget(formats, value) === undefined) throw new ErrorType(p + "Failed to convert value to 'TextFormat'."); push(r.formats, value); }
    }
    return r;
  });
  eventInterface('CharacterBoundsUpdateEvent', [['rangeStart'], ['rangeEnd']], init => {
    const r = create(null), re = init.rangeEnd, rs = init.rangeStart;
    r.rangeEnd = re === undefined ? 0 : unsignedLong(re);
    r.rangeStart = rs === undefined ? 0 : unsignedLong(rs);
    return r;
  });
  const TextUpdateEvent = g.TextUpdateEvent;
  // Trusted text input into an element's EditContext (Chrome's EditContext::InsertText and its
  // deletion handlers): updates the context's text and selection and fires textupdate. Only the
  // embedder's input driver can reach this, through the private handoff to editing.js.
  function input(element, inputType, data, beforeInput) {
    const context = contextOf(element);
    if (context === null) return false;
    const s = wget(states, context);
    let a = MathMin(s.start, s.end), b = s.start > s.end ? s.start : s.end, text = '';
    if (inputType === 'insertText') text = data;
    else if (inputType === 'deleteContentBackward' || inputType === 'deleteContentForward') {
      if (!beforeInput(element, inputType, null)) return true;
      if (a === b) { if (inputType === 'deleteContentBackward') { if (a === 0) return true; a--; } else { if (b >= s.text.length) return true; b++; } }
    } else { beforeInput(element, inputType, null); return true; }
    s.text = sub(s.text, 0, a) + text + sub(s.text, b);
    s.start = s.end = a + text.length;
    const event = new TextUpdateEvent('textupdate', { __proto__: null, cancelable: true, text, updateRangeStart: a, updateRangeEnd: b, selectionStart: s.start, selectionEnd: s.end });
    native('dispatch', context, event);
    return true;
  }
  g.__editContextHandoff = { __proto__: null, contextOf, input };

  // ---- ariaNotify -----------------------------------------------------------------------------------------
  const priorities = { __proto__: null, normal: 1, high: 1 };
  function ariaNotify(kind, allowed) {
    return function ariaNotify(announcement) {
      let type;
      try { type = apply(typeOf, this, []); } catch (error) { throw new ErrorType('Illegal invocation'); }
      if (!allowed(type)) throw new ErrorType('Illegal invocation');
      requireArguments(1, arguments, [kind, 'ariaNotify']);
      string(announcement, "Failed to execute 'ariaNotify' on '" + kind + "': ");
      const options = dictionary(arguments[1], () => new ErrorType("Failed to execute 'ariaNotify' on '" + kind + "': The provided value is not of type 'AriaNotificationOptions'."));
      if (options !== null) {
        const priority = options.priority;
        if (priority !== undefined) {
          const p = "Failed to execute 'ariaNotify' on '" + kind + "': Failed to read the 'priority' property from 'AriaNotificationOptions': ";
          const v = string(priority, p);
          if (!priorities[v]) throw new ErrorType(p + "The provided value '" + v + "' is not a valid enum value of type AriaNotifyPriority.");
        }
      }
    };
  }
  method(g.Element.prototype, 'ariaNotify', ariaNotify('Element', type => type === 1), 1);
  method(g.Document.prototype, 'ariaNotify', ariaNotify('Document', type => type === 9), 1);
})();
