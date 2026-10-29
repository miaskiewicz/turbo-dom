// ShadowRoot.referenceTarget, the <template shadowrootreferencetarget> reflection, and the
// private resolver ID-reference consumers (invokers, interest invokers) call.
(function () {
  'use strict';
  const g = globalThis, ns = __tsNs, make = ns.makeNativeFunction;
  const Native = g.__referenceTargetNative; delete g.__referenceTargetNative;
  const StringValue = String, TypeErrorValue = TypeError, apply = Reflect.apply;
  const nodeType = Object.getOwnPropertyDescriptor(g.Node.prototype, 'nodeType').get;
  const namespace = Object.getOwnPropertyDescriptor(g.Element.prototype, 'namespaceURI').get;
  const localName = Object.getOwnPropertyDescriptor(g.Element.prototype, 'localName').get;
  const getAttribute = g.Element.prototype.getAttribute, setAttribute = g.Element.prototype.setAttribute, removeAttribute = g.Element.prototype.removeAttribute;
  // `DOMString?`: undefined and null are null; a Symbol throws Chrome's setter message.
  const nullable = (value, owner, member) => {
    if (value === undefined || value === null) return null;
    if (typeof value === 'symbol') throw new TypeErrorValue(`Failed to set the '${member}' property on '${owner}': Cannot convert a Symbol value to a string`);
    return StringValue(value);
  };
  Object.defineProperty(g.ShadowRoot.prototype, 'referenceTarget', {
    enumerable: true, configurable: true,
    get: make(function () { return Native('get', this); }, 'get referenceTarget', 0),
    set: make(function (value) { const converted = nullable(value, 'ShadowRoot', 'referenceTarget'); Native('set', this, converted); }, 'set referenceTarget', 1),
  });
  const template = receiver => {
    let element = false;
    try { element = apply(nodeType, receiver, []) === 1 && apply(namespace, receiver, []) === 'http://www.w3.org/1999/xhtml' && apply(localName, receiver, []) === 'template'; } catch {}
    if (!element) throw new TypeErrorValue('Illegal invocation');
  };
  Object.defineProperty(g.HTMLTemplateElement.prototype, 'shadowRootReferenceTarget', {
    enumerable: true, configurable: true,
    get: make(function () { template(this); return apply(getAttribute, this, ['shadowrootreferencetarget']); }, 'get shadowRootReferenceTarget', 0),
    set: make(function (value) {
      template(this);
      const converted = nullable(value, 'HTMLTemplateElement', 'shadowRootReferenceTarget');
      if (converted === null) apply(removeAttribute, this, ['shadowrootreferencetarget']);
      else apply(setAttribute, this, ['shadowrootreferencetarget', converted]);
    }, 'set shadowRootReferenceTarget', 1),
  });
  ns.resolveReferenceTarget = element => element === null ? null : Native('resolve', element);
})();
