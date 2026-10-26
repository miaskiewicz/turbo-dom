// Element pointer capture and Element.requestPointerLock over native per-document state
// (pointer_events.rs). The only pointer is the mouse (id 1): it is always active, and its
// active buttons state, capture processing and event retargeting belong to the embedder's
// trusted input driver. Lock requests follow Chrome's PointerLockController: a request
// without transient activation (every request when the embedder supplies no activation)
// is refused with NotAllowedError and a pointerlockerror, a granted one resolves and
// queues pointerlockchange. Document.exitPointerLock and DocumentOrShadowRoot's
// pointerLockElement read the same state natively (document_legacy, shadow_html).
(function () {
  'use strict';
  const Native = globalThis.__pointerNative; delete globalThis.__pointerNative;
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction;
  const apply = Reflect.apply, define = Object.defineProperty;
  const accessor = (proto, name) => Object.getOwnPropertyDescriptor(proto, name).get;
  const nodeType = accessor(g.Node.prototype, 'nodeType'), connectedOf = accessor(g.Node.prototype, 'isConnected');
  const ownerDocument = accessor(g.Node.prototype, 'ownerDocument');
  const Exception = g.DOMException, ErrorType = TypeError, PromiseType = Promise, EventType = g.Event;
  const MOUSE = 1;
  const isElement = value => { try { return apply(nodeType, value, []) === 1; } catch (_) { return false; } };
  const brand = receiver => { if (!isElement(receiver)) throw new ErrorType('Illegal invocation'); };
  const execute = (operation, message) => "Failed to execute '" + operation + "' on 'Element': " + message;
  const connected = node => apply(connectedOf, node, []);
  // Web IDL `long`: ToNumber then ToInt32. Chrome prefixes V8's own conversion TypeErrors
  // with the operation context; an exception thrown by guest valueOf/toString propagates.
  const conversionFailures = ['Cannot convert a Symbol value to a number', 'Cannot convert a BigInt value to a number', 'Cannot convert object to primitive value'];
  const includes = Array.prototype.includes;
  function pointerId(operation, args) {
    if (args.length < 1) throw new ErrorType(execute(operation, '1 argument required, but only 0 present.'));
    const value = args[0];
    if (typeof value === 'symbol') throw new ErrorType(execute(operation, conversionFailures[0]));
    if (typeof value === 'bigint') throw new ErrorType(execute(operation, conversionFailures[1]));
    try {
      return +value | 0;
    } catch (error) {
      if (error instanceof ErrorType && apply(includes, conversionFailures, [error.message])) throw new ErrorType(execute(operation, error.message));
      throw error;
    }
  }
  const inactive = operation => new Exception(execute(operation, 'No active pointer with the given id is found.'), 'NotFoundError');
  const method = (name, body, arity) => define(g.Element.prototype, name, { value: make(body, name, arity), writable: true, enumerable: true, configurable: true });

  // https://w3c.github.io/pointerevents/#dom-element-setpointercapture: a document without
  // a browsing context ignores the call; the override waits for the next trusted event.
  method('setPointerCapture', function (pointer) {
    brand(this);
    const id = pointerId('setPointerCapture', arguments);
    if (!Native('frame', this)) return;
    if (id !== MOUSE) throw inactive('setPointerCapture');
    if (!connected(this) || Native('lock', this) !== null) throw new Exception(execute('setPointerCapture', 'InvalidStateError'), 'InvalidStateError');
    if (Native('buttons', this)) Native('set-pending', this, this);
  }, 1);
  method('releasePointerCapture', function (pointer) {
    brand(this);
    const id = pointerId('releasePointerCapture', arguments);
    if (!Native('frame', this)) return;
    if (id !== MOUSE) throw inactive('releasePointerCapture');
    if (Native('pending', this) === this) Native('set-pending', this, null);
  }, 1);
  method('hasPointerCapture', function (pointer) {
    brand(this);
    const id = pointerId('hasPointerCapture', arguments);
    return id === MOUSE && Native('frame', this) && Native('pending', this) === this;
  }, 1);

  // Lock events are trusted, non-bubbling plain Events queued at the lock's document.
  const queue = (type, target) => { if (target !== null) Native('schedule', target, () => Native('dispatch', target, new EventType(type))); };
  globalThis.__pointerLockEvents = queue;
  // dictionary PointerLockOptions { boolean unadjustedMovement = false; }
  function lockOptions(options) {
    if (options === undefined || options === null) return false;
    if (typeof options !== 'object' && typeof options !== 'function') throw new ErrorType(execute('requestPointerLock', "The provided value is not of type 'PointerLockOptions'."));
    return !!options.unadjustedMovement;
  }
  const refusal = message => new Exception(message, 'NotAllowedError');
  function request(target, options, settle) {
    if (!isElement(target)) throw new ErrorType(execute('requestPointerLock', 'Illegal invocation'));
    const unadjusted = lockOptions(options);
    if (!Native('frame', target)) return settle.reject(new Exception('PointerLock cannot be requested when there is no frame or that frame has no page.', 'WrongDocumentError'));
    const document = apply(ownerDocument, target, []);
    if (!connected(target)) {
      queue('pointerlockerror', document);
      throw new Exception(execute('requestPointerLock', 'Target Element removed from DOM'), 'WrongDocumentError');
    }
    if (Native('sandboxed', target)) {
      queue('pointerlockerror', document);
      throw new Exception(execute('requestPointerLock', "Blocked pointer lock on an element because the element's frame is sandboxed and the 'allow-pointer-lock' permission is not set."), 'SecurityError');
    }
    if (Native('lock', target) !== null) {
      // Changing only the movement option renegotiates the held lock without a change event.
      if (unadjusted !== Native('unadjusted', target)) {
        if (Native('lock-pending', target)) {
          queue('pointerlockerror', document);
          throw new Exception(execute('requestPointerLock', 'Pointer lock pending.'), 'InUseAttributeError');
        }
        return Native('schedule', target, () => {
          if (Native('lock', target) !== null) Native('set-lock', target, target, false, unadjusted);
          settle.resolve();
        });
      }
      queue('pointerlockchange', document);
      Native('set-lock', target, target, Native('lock-pending', target), unadjusted);
      return settle.resolve();
    }
    // The grant is decided from the activation at request time and arrives as a later task.
    const activated = Native('activation', target);
    Native('set-lock', target, target, true, unadjusted);
    Native('schedule', target, () => {
      const holder = Native('lock', target);
      if (!activated) {
        queue('pointerlockerror', holder === null ? null : apply(ownerDocument, holder, []));
        Native('set-lock', target, null, false, false);
        return settle.reject(refusal('A user gesture is required to request Pointer Lock.'));
      }
      Native('grant', target, unadjusted);
      if (holder !== null) queue('pointerlockchange', apply(ownerDocument, holder, []));
      settle.resolve();
    });
  }
  method('requestPointerLock', function () {
    let settle;
    const promise = new PromiseType((resolve, reject) => { settle = { resolve, reject }; });
    try { request(this, arguments[0], settle); } catch (error) { settle.reject(error); }
    return promise;
  }, 0);
})();
