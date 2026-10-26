(function () {
  'use strict';
  // Small Chrome 156 interface members measured in interface_members_chrome156.json:
  // HTMLMediaElement constants and `loading`, HTMLScriptElement.supports().
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction;
  const apply = Reflect.apply, define = Object.defineProperty;
  const getAttribute = g.Element.prototype.getAttribute, setAttribute = g.Element.prototype.setAttribute;
  const lower = String.prototype.toLowerCase;
  const string = value => { if (typeof value === 'symbol') throw new TypeError('Cannot convert a Symbol to a string'); return String(value); };
  const media = g.HTMLMediaElement;
  if (media) {
    const constants = [['NETWORK_EMPTY', 0], ['NETWORK_IDLE', 1], ['NETWORK_LOADING', 2], ['NETWORK_NO_SOURCE', 3], ['HAVE_NOTHING', 0], ['HAVE_METADATA', 1], ['HAVE_CURRENT_DATA', 2], ['HAVE_FUTURE_DATA', 3], ['HAVE_ENOUGH_DATA', 4]];
    for (const [name, value] of constants) for (const object of [media, media.prototype]) define(object, name, { value, writable: false, enumerable: true, configurable: false });
    // loading: an enumerated attribute limited to lazy/eager, missing and invalid default eager.
    const brand = ns.domAccessor('nodeType', false, 'HTML:audio|video');
    define(media.prototype, 'loading', {
      enumerable: true, configurable: true,
      get: make(function () { apply(brand, this, []); const value = apply(getAttribute, this, ['loading']); return value !== null && apply(lower, value, []) === 'lazy' ? 'lazy' : 'eager'; }, 'get loading', 0),
      set: make(function (value) { apply(brand, this, []); apply(setAttribute, this, ['loading', string(value)]); }, 'set loading', 1),
    });
  }
  const script = g.HTMLScriptElement;
  if (script) {
    // Script types this engine prepares. Chrome also reports importmap and webbundle; the engine
    // recognises import maps without applying them and has no web bundle loader, so it says so
    // (a feature-detecting page then takes its polyfill path). Speculation rules are advisory:
    // the engine accepts them and, like any UA may, never acts on a speculation.
    const types = ['classic', 'module', 'speculationrules'];
    define(script, 'supports', { value: make(function supports(type) {
      if (!arguments.length) throw new TypeError("Failed to execute 'supports' on 'HTMLScriptElement': 1 argument required, but only 0 present.");
      return types.indexOf(string(type)) >= 0;
    }, 'supports', 1), writable: true, enumerable: true, configurable: true });
  }
  const interfaceObject = (name) => {
    const ctor = make(function () { throw new TypeError('Illegal constructor'); }, name, 0, true);
    define(ctor.prototype, Symbol.toStringTag, { value: name, configurable: true });
    define(ctor, 'prototype', { writable: false });
    define(g, name, { value: ctor, writable: true, configurable: true, enumerable: false });
    return ctor.prototype;
  };
  const weak = () => { const map = new WeakMap(); return { get: (key) => apply(WeakMap.prototype.get, map, [key]), set: (key, value) => apply(WeakMap.prototype.set, map, [key, value]) }; };
  // Document.prototype.fragmentDirective: one FragmentDirective per document. Chrome 156 exposes
  // no members on it; text-fragment scrolling has no viewport to act on here.
  {
    const FragmentPrototype = interfaceObject('FragmentDirective');
    const directives = weak(), checkDocument = ns.domAccessor('nodeType', false, 'Document');
    define(g.Document.prototype, 'fragmentDirective', { enumerable: true, configurable: true, get: make(function () {
      apply(checkDocument, this, []);
      let directive = directives.get(this);
      if (!directive) { directive = Object.create(FragmentPrototype); directives.set(this, directive); }
      return directive;
    }, 'get fragmentDirective', 0) });
  }
  // Element.prototype.pseudo(type) / CSSPseudoElement: the pseudo-elements Chrome 156 exposes,
  // one object per (originating element, parent, type and name). View-transition pseudos hang
  // off their tree parent (::view-transition > group > image-pair > old/new).
  {
    const PseudoPrototype = interfaceObject('CSSPseudoElement');
    const records = weak(), children = weak();
    const checkElement = ns.domAccessor('nodeType', false, 'Element');
    const plain = ['before', 'after', 'marker', 'backdrop', 'view-transition', 'scroll-marker'];
    const named = ['view-transition-group', 'view-transition-image-pair', 'view-transition-old', 'view-transition-new'];
    const treeParent = { 'view-transition-group': null, 'view-transition-image-pair': 'view-transition-group', 'view-transition-old': 'view-transition-image-pair', 'view-transition-new': 'view-transition-image-pair' };
    const exec = RegExp.prototype.exec, lower = String.prototype.toLowerCase, trim = String.prototype.trim, replace = String.prototype.replace;
    const fromCodePoint = String.fromCodePoint, parseInteger = parseInt, minimum = Math.min;
    const ident = /^(?:--|-?(?:[_a-zA-Z\u0080-￿]|\\(?:[0-9a-fA-F]{1,6}[ \t\n]?|[^\n\r\f0-9a-fA-F])))(?:[-_a-zA-Z0-9\u0080-￿]|\\(?:[0-9a-fA-F]{1,6}[ \t\n]?|[^\n\r\f0-9a-fA-F]))*$/;
    // CSS <custom-ident> escapes: a hex escape (with one optional trailing space) or an escaped character.
    const unescape = (name) => apply(replace, name, [/\\([0-9a-fA-F]{1,6} ?|[^\n\r\f])/g, (_, escape) => {
      const hex = apply(trim, escape, []);
      return apply(exec, /^[0-9a-fA-F]+$/, [hex]) ? fromCodePoint(minimum(parseInteger(hex, 16), 0x10ffff) || 0xfffd) : escape;
    }]);
    const parse = (type) => {
      type = apply(trim, string(type), []);
      const match = apply(exec, /^(::?)([a-zA-Z-]+)(?:\((.*)\))?$/, [type]);
      if (!match) return null;
      const kind = apply(lower, match[2], []);
      if (match[1] === ':' && (match[3] !== undefined || (kind !== 'before' && kind !== 'after'))) return null;
      if (match[3] === undefined) return plain.indexOf(kind) >= 0 ? { kind, name: null } : null;
      if (named.indexOf(kind) < 0 || !apply(exec, ident, [match[3]])) return null;
      return { kind, name: unescape(match[3]) };
    };
    const pseudoOf = (element, parent, kind, name) => {
      const owner = parent || element, key = kind + (name === null ? '' : '(' + name + ')');
      let table = children.get(owner);
      if (!table) children.set(owner, (table = { __proto__: null }));
      if (!table[key]) {
        const pseudo = Object.create(PseudoPrototype);
        records.set(pseudo, { element, parent: parent || element, kind });
        table[key] = pseudo;
      }
      return table[key];
    };
    const record = (receiver) => {
      const value = receiver !== null && typeof receiver === 'object' ? records.get(receiver) : undefined;
      if (!value) throw new TypeError('Illegal invocation');
      return value;
    };
    const required = (count) => { if (!count) throw new TypeError("Failed to execute 'pseudo' on 'Element': 1 argument required, but only 0 present."); };
    // A top-level view-transition pseudo sits under its tree parent; the rest under the element.
    const topLevel = (element, kind, name) => {
      if (!(kind in treeParent)) return pseudoOf(element, null, kind, name);
      const parentKind = treeParent[kind];
      const parent = parentKind === null ? pseudoOf(element, null, 'view-transition', null) : topLevel(element, parentKind, name);
      return pseudoOf(element, parent, kind, name);
    };
    define(g.Element.prototype, 'pseudo', { value: make(function pseudo(type) {
      apply(checkElement, this, []);
      required(arguments.length);
      const parsed = parse(type);
      return parsed ? topLevel(this, parsed.kind, parsed.name) : null;
    }, 'pseudo', 1), writable: true, enumerable: true, configurable: true });
    for (const name of ['type', 'element', 'parent']) define(PseudoPrototype, name, { enumerable: true, configurable: true, get: make(function () {
      const value = record(this);
      if (name === 'type') return '::' + value.kind;
      return value[name];
    }, 'get ' + name, 0) });
    // A pseudo-element's own pseudo-elements; ::view-transition only roots the element's tree.
    define(PseudoPrototype, 'pseudo', { value: make(function pseudo(type) {
      const value = record(this);
      if (!arguments.length) throw new TypeError("Failed to execute 'pseudo' on 'CSSPseudoElement': 1 argument required, but only 0 present.");
      const parsed = parse(type);
      return parsed && parsed.kind !== 'view-transition' ? pseudoOf(value.element, this, parsed.kind, parsed.name) : null;
    }, 'pseudo', 1), writable: true, enumerable: true, configurable: true });
  }
})();
