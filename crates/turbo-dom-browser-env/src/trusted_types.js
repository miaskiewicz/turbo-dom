(function () {
  'use strict';
  const g = globalThis, nativeRecord = g.__trustedTypesRecord, mappingSource = g.__trustedTypesMapping;
  delete g.__trustedTypesRecord; delete g.__trustedTypesMapping;
  const native = g[Symbol.for('ts')].makeNativeFunction;
  const create = Object.create, define = Object.defineProperty, apply = Reflect.apply;
  const own = Function.prototype.call.bind(Object.prototype.hasOwnProperty);
  const lower = Function.prototype.call.bind(String.prototype.toLowerCase);
  const wellFormed = Function.prototype.call.bind(String.prototype.toWellFormed);
  const parse = JSON.parse, TypeErrorClass = TypeError, toPrimitive = Symbol.toPrimitive;
  const interfaces = create(null);
  const typeNames = ['TrustedHTML', 'TrustedScript', 'TrustedScriptURL'];
  const methods = ['createHTML', 'createScript', 'createScriptURL'];
  const callbackNames = ['CreateHTMLCallback', 'CreateScriptCallback', 'CreateURLCallback'];
  const mapping = parse(mappingSource), HTML = 'http://www.w3.org/1999/xhtml', SVG = 'http://www.w3.org/2000/svg', XLINK = 'http://www.w3.org/1999/xlink';
  function read(object, kind) {
    const state = nativeRecord(true, object);
    if (!state || (kind && state.kind !== kind)) throw new TypeErrorClass('Illegal invocation');
    return state;
  }
  function make(kind, state) {
    const object = create(interfaces[kind].prototype);
    state.kind = kind;
    return nativeRecord(false, object, state);
  }
  function failure(method, owner, text) {
    return new TypeErrorClass("Failed to execute '" + method + "' on '" + owner + "': " + text);
  }
  function required(method, owner, actual, count) {
    if (actual < count) throw failure(method, owner, count + ' argument' + (count === 1 ? '' : 's') + ' required, but only ' + actual + ' present.');
  }
  function string(value, method, owner) {
    if (value !== null && (typeof value === 'object' || typeof value === 'function')) {
      const object = value, exotic = object[toPrimitive];
      if (exotic != null) {
        if (typeof exotic !== 'function') throw failure(method, owner, 'Cannot convert object to primitive value');
        value = apply(exotic, object, ['string']);
      } else {
        let found = false;
        for (let index = 0; index < 2; index++) {
          const name = index === 0 ? 'toString' : 'valueOf';
          const callback = object[name];
          if (typeof callback !== 'function') continue;
          const result = apply(callback, object, []);
          if (result === null || (typeof result !== 'object' && typeof result !== 'function')) { value = result; found = true; break; }
        }
        if (!found) throw failure(method, owner, 'Cannot convert object to primitive value');
      }
      if (value !== null && (typeof value === 'object' || typeof value === 'function')) throw failure(method, owner, 'Cannot convert object to primitive value');
    }
    if (typeof value === 'symbol') throw failure(method, owner, 'Cannot convert a Symbol value to a string');
    return nativeRecord(2, value);
  }
  function member(prototype, name, callback, length) {
    define(prototype, name, { value: native(callback, name, length), writable: true, enumerable: true, configurable: true });
  }
  function accessor(prototype, name, callback) {
    define(prototype, name, { get: native(callback, 'get ' + name, 0), enumerable: true, configurable: true });
  }
  function startInterface(name) {
    const constructor = native(function () { throw new TypeErrorClass("Failed to construct '" + name + "': Illegal constructor"); }, name, 0, true, false, false);
    const prototype = create(Object.prototype);
    define(constructor, 'prototype', { value: prototype, writable: false });
    interfaces[name] = constructor;
    define(g, name, { value: constructor, writable: true, configurable: true });
    return prototype;
  }
  function finishInterface(name) {
    const constructor = interfaces[name], prototype = constructor.prototype;
    define(prototype, 'constructor', { value: constructor, writable: true, configurable: true });
    define(prototype, Symbol.toStringTag, { value: name, configurable: true });
  }
  const factoryPrototype = startInterface('TrustedTypePolicyFactory');
  const policyPrototype = startInterface('TrustedTypePolicy');
  for (const type of typeNames) {
    const prototype = startInterface(type);
    for (const name of ['toJSON', 'toString']) member(prototype, name, function () { return read(this, type).value; }, 0);
    finishInterface(type);
  }
  const emptyHTML = make('TrustedHTML', { value: '' }), emptyScript = make('TrustedScript', { value: '' });
  const factory = make('TrustedTypePolicyFactory', { names: create(null), defaultPolicy: null, emptyHTML, emptyScript });
  for (const name of ['emptyHTML', 'emptyScript', 'defaultPolicy']) accessor(factoryPrototype, name, function () { return read(this, 'TrustedTypePolicyFactory')[name]; });
  member(factoryPrototype, 'createPolicy', function (name, options) {
    const state = read(this, 'TrustedTypePolicyFactory');
    required('createPolicy', 'TrustedTypePolicyFactory', arguments.length, 1);
    name = string(name, 'createPolicy', 'TrustedTypePolicyFactory');
    if (options != null && typeof options !== 'object' && typeof options !== 'function') throw failure('createPolicy', 'TrustedTypePolicyFactory', "The provided value is not of type 'TrustedTypePolicyOptions'.");
    const rules = create(null);
    for (let index = 0; index < methods.length; index++) {
      const method = methods[index];
      const callback = options == null ? undefined : options[method];
      if (callback !== undefined && typeof callback !== 'function') throw failure('createPolicy', 'TrustedTypePolicyFactory', "Failed to read the '" + method + "' property from 'TrustedTypePolicyOptions': The given value is not a function.");
      rules[method] = callback;
    }
    if (state.authorize && !state.authorize('policy', name, own(state.names, name))) throw failure('createPolicy', 'TrustedTypePolicyFactory', own(state.names, name) ? 'Policy with name "' + name + '" already exists.' : 'Policy "' + name + '" disallowed.');
    if (name === 'default' && state.defaultPolicy) throw failure('createPolicy', 'TrustedTypePolicyFactory', 'Policy with name "default" already exists.');
    const policy = make('TrustedTypePolicy', { name, rules });
    state.names[name] = true;
    if (name === 'default') state.defaultPolicy = policy;
    return policy;
  }, 1);
  function namespace(value, method) { return value == null || value === '' ? HTML : string(value, method, 'TrustedTypePolicyFactory'); }
  function attributeType(tag, attribute, ns, attributeNS) {
    if (attributeNS != null && attributeNS !== '' && !(ns === SVG && attributeNS === XLINK)) return null;
    if (ns !== HTML && ns !== SVG && ns !== 'http://www.w3.org/1998/Math/MathML') return null;
    const events = mapping['*'].attributes;
    if (own(events, attribute) && (attributeNS == null || attributeNS === '')) return 'TrustedScript';
    if (ns === HTML && (attributeNS == null || attributeNS === '')) {
      if (tag === 'script' && attribute === 'src' || tag === 'embed' && attribute === 'src' || tag === 'object' && (attribute === 'data' || attribute === 'codebase')) return 'TrustedScriptURL';
      if (tag === 'iframe' && attribute === 'srcdoc') return 'TrustedHTML';
    }
    if (ns === SVG && tag === 'script' && attribute === 'href') return 'TrustedScriptURL';
    return null;
  }
  member(factoryPrototype, 'getAttributeType', function (tag, attribute, ns, attributeNS) {
    read(this, 'TrustedTypePolicyFactory'); required('getAttributeType', 'TrustedTypePolicyFactory', arguments.length, 2);
    tag = lower(string(tag, 'getAttributeType', 'TrustedTypePolicyFactory')); attribute = lower(string(attribute, 'getAttributeType', 'TrustedTypePolicyFactory'));
    ns = namespace(ns, 'getAttributeType'); attributeNS = attributeNS == null ? '' : string(attributeNS, 'getAttributeType', 'TrustedTypePolicyFactory');
    return attributeType(tag, attribute, ns, attributeNS);
  }, 2);
  member(factoryPrototype, 'getPropertyType', function (tag, property, ns) {
    read(this, 'TrustedTypePolicyFactory'); required('getPropertyType', 'TrustedTypePolicyFactory', arguments.length, 2);
    tag = lower(string(tag, 'getPropertyType', 'TrustedTypePolicyFactory')); property = string(property, 'getPropertyType', 'TrustedTypePolicyFactory'); ns = namespace(ns, 'getPropertyType');
    if (property === 'innerHTML' || property === 'outerHTML') return 'TrustedHTML';
    if (ns !== HTML && ns !== SVG && ns !== 'http://www.w3.org/1998/Math/MathML') return null;
    if (ns === HTML && tag === 'script' && (property === 'text' || property === 'textContent' || property === 'innerText')) return 'TrustedScript';
    if (ns === SVG) return tag === 'script' && property === 'href' ? 'TrustedScriptURL' : null;
    if (ns === HTML && tag === 'script' && property === 'src') return 'TrustedScriptURL';
    if (ns !== HTML || tag === 'script') return null;
    if (!own(mapping, tag) || !own(mapping[tag].properties, property)) return null;
    return mapping[tag].properties[property];
  }, 2);
  member(factoryPrototype, 'getTypeMapping', function (ns) {
    read(this, 'TrustedTypePolicyFactory');
    if (ns !== undefined && string(ns, 'getTypeMapping', 'TrustedTypePolicyFactory') !== '') return null;
    return parse(mappingSource);
  }, 0);
  for (let index = 0; index < typeNames.length; index++) member(factoryPrototype, ['isHTML', 'isScript', 'isScriptURL'][index], function (value) {
    read(this, 'TrustedTypePolicyFactory'); const state = nativeRecord(true, value); return !!state && state.kind === typeNames[index];
  }, 1);
  finishInterface('TrustedTypePolicyFactory');
  accessor(policyPrototype, 'name', function () { return read(this, 'TrustedTypePolicy').name; });
  for (let index = 0; index < methods.length; index++) {
    const method = methods[index], type = typeNames[index];
    member(policyPrototype, method, function (input) {
      const state = read(this, 'TrustedTypePolicy'); required(method, 'TrustedTypePolicy', arguments.length, 1);
      input = string(input, method, 'TrustedTypePolicy');
      const callback = state.rules[method];
      if (!callback) throw failure(method, 'TrustedTypePolicy', "Policy " + state.name + "'s TrustedTypePolicyOptions did not specify a '" + method + "' member.");
      const args = create(null); args.length = arguments.length; args[0] = input; for (let i = 1; i < arguments.length; i++) args[i] = arguments[i];
      const output = apply(callback, undefined, args);
      let value;
      try { value = output == null ? '' : string(output, method, 'TrustedTypePolicy'); }
      catch (error) {
        if (error !== null && (typeof error === 'object' || typeof error === 'function')) {
          error.message = "Failed to execute 'invoke' on '" + callbackNames[index] + "': " + error.message;
        }
        throw error;
      }
      if (type === 'TrustedScriptURL') value = wellFormed(value);
      return make(type, { value });
    }, 1);
  }
  finishInterface('TrustedTypePolicy');
  function convert(input, type, sink, prefix, nullToEmpty) {
    const record = nativeRecord(true, input);
    if (record && record.kind === type) return input;
    const text = nullToEmpty && input === null ? '' : string(input, sink, type);
    const state = read(factory, 'TrustedTypePolicyFactory');
    if (!state.authorize || !state.authorize('required')) return text;
    if (state.defaultPolicy) {
      const policy = read(state.defaultPolicy, 'TrustedTypePolicy');
      let index = 0; while (index < typeNames.length && typeNames[index] !== type) index++;
      const callback = policy.rules[methods[index]];
      if (callback) {
        const result = apply(callback, undefined, [text, type, sink]);
        if (result != null) {
          let value = string(result, methods[index], 'TrustedTypePolicy');
          if (type === 'TrustedScriptURL') value = wellFormed(value);
          return make(type, { value });
        }
      }
    }
    if (!state.authorize('sink', type, sink, text)) throw new TypeErrorClass(prefix + "This document requires '" + type + "' assignment" + (state.defaultPolicy ? " and the 'default' policy failed to execute" : '') + '.');
    return make(type, { value: text });
  }
  const setAttribute = Element.prototype.setAttribute;
  const getAttribute = Element.prototype.getAttribute;
  for (const entry of [
    ['HTMLScriptElement', 'src', 'TrustedScriptURL'],
    ['HTMLEmbedElement', 'src', 'TrustedScriptURL'],
    ['HTMLObjectElement', 'data', 'TrustedScriptURL'],
    ['HTMLObjectElement', 'codeBase', 'TrustedScriptURL'],
    ['HTMLIFrameElement', 'srcdoc', 'TrustedHTML'],
  ]) {
    const owner = entry[0], property = entry[1], type = entry[2], prototype = g[owner].prototype;
    const previous = Object.getOwnPropertyDescriptor(prototype, property);
    const check = g[Symbol.for('ts')].domAccessor('nodeType', false, 'HTML:' + ({HTMLScriptElement:'script',HTMLEmbedElement:'embed',HTMLObjectElement:'object',HTMLIFrameElement:'iframe'})[owner]);
    const attribute = lower(property);
    define(prototype, property, { configurable: true, enumerable: true,
      get: previous && previous.get || native(function () { apply(check, this, []); return apply(getAttribute, this, [attribute]) || ''; }, 'get ' + property, 0),
      set: native(function (value) {
        apply(check, this, []);
        value = convert(value, type, owner + ' ' + property, "Failed to set the '" + property + "' property on '" + owner + "': ", false);
        return apply(setAttribute, this, [attribute, value]);
      }, 'set ' + property, 1),
    });
  }
  nativeRecord(false, g, { factory, convert });
  define(g, 'trustedTypes', { get: native(function () { return factory; }, 'get trustedTypes', 0), enumerable: true, configurable: true });
})();
