// XML documents (turbo-dom#186): parsed XML documents take the realm's XMLDocument
// prototype, and XMLSerializer serializes any node with Chrome's XML serialization.
(function () {
  'use strict';
  const g = globalThis, native = g.__xmlDocumentsNative; delete g.__xmlDocumentsNative;
  const make = __tsNs.makeNativeFunction, define = Object.defineProperty, apply = Reflect.apply;
  const NativeTypeError = TypeError;
  const nodeType = Object.getOwnPropertyDescriptor(g.Node.prototype, 'nodeType').get;
  const instances = new WeakSet(), add = WeakSet.prototype.add, has = WeakSet.prototype.has;
  native('prototype', g.XMLDocument.prototype);
  const XMLSerializer = make(function () { apply(add, instances, [this]); }, 'XMLSerializer', 0, true, false);
  const prototype = XMLSerializer.prototype;
  define(prototype, Symbol.toStringTag, { __proto__: null, value: 'XMLSerializer', configurable: true });
  const prefix = "Failed to execute 'serializeToString' on 'XMLSerializer': ";
  define(prototype, 'serializeToString', { __proto__: null, value: make(function serializeToString(node) {
    if (!apply(has, instances, [this])) throw new NativeTypeError('Illegal invocation');
    if (!arguments.length) throw new NativeTypeError(prefix + '1 argument required, but only 0 present.');
    try { apply(nodeType, node, []); } catch (error) { throw new NativeTypeError(prefix + "parameter 1 is not of type 'Node'."); }
    return native('serialize', node);
  }, 'serializeToString', 1), writable: true, enumerable: true, configurable: true });
  define(XMLSerializer, 'prototype', { __proto__: null, writable: false });
  define(g, 'XMLSerializer', { __proto__: null, value: XMLSerializer, writable: true, configurable: true });
})();
