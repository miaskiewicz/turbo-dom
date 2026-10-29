// Chrome 156's Window interface chain (miaskiewicz/turbo-dom#223):
// window -> Window.prototype -> WindowProperties -> EventTarget.prototype -> Object.prototype.
// The window's own add/remove/dispatchEvent move behind EventTarget.prototype's methods,
// which route a window receiver to the window's listener list.
(function () {
  'use strict';
  const g = globalThis, ns = __tsNs, make = ns.makeNativeFunction;
  const define = Object.defineProperty, apply = Reflect.apply, setPrototype = Object.setPrototypeOf;
  const ErrorType = TypeError, target = g.EventTarget.prototype, windowCallbacks = ns.windowEventCallbacks;
  for (const [name, length] of [['addEventListener', 2], ['removeEventListener', 2], ['dispatchEvent', 1]]) {
    const generic = target[name], forWindow = windowCallbacks[name];
    define(target, name, { value: make({ [name](...args) { return apply(this === g ? forWindow : generic, this, args); } }[name], name, length), writable: true, enumerable: true, configurable: true });
    delete g[name];
  }
  // An embedder whose global template already makes the global a Window instance (turbo-surf's
  // WindowProxy template, with an immutable prototype) keeps its interface object; the chain
  // above Window.prototype is completed here either way.
  const getPrototype = Object.getPrototypeOf, hasOwn = Object.hasOwn;
  let Window = g.Window;
  const embedded = typeof Window === 'function' && getPrototype(g) === Window.prototype;
  if (!embedded) {
    Window = make(function Window() { throw new ErrorType(new.target ? "Failed to construct 'Window': Illegal constructor" : 'Illegal constructor'); }, 'Window', 0, true, true);
    define(Window, 'prototype', { value: {}, writable: false, enumerable: false, configurable: false });
    define(Window.prototype, 'constructor', { value: Window, writable: true, configurable: true });
    define(g, 'Window', { value: Window, writable: true, configurable: true });
  }
  const proto = Window.prototype;
  const properties = setPrototype({}, target);
  define(properties, Symbol.toStringTag, { value: 'WindowProperties', configurable: true });
  setPrototype(proto, properties);
  setPrototype(Window, g.EventTarget);
  for (const [name, value] of [['TEMPORARY', 0], ['PERSISTENT', 1]]) {
    for (const object of [Window, proto]) if (!hasOwn(object, name)) define(object, name, { value, enumerable: true });
  }
  define(proto, Symbol.toStringTag, { value: 'Window', configurable: true });
  if (!embedded) setPrototype(g, proto);
})();
