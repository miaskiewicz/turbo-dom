// Chrome 156 control probe: enumerate DedicatedWorkerGlobalScope's prototype chain
// and the own members of `self`, inside a real dedicated Worker. The page runs this
// as a classic Worker script (via a Blob URL) so the globals the probe sees are the
// real worker scope's — not the page's window. Captured by
// scripts/dedicated-worker-global-scope-chrome-control.mjs. See turbo-dom#385.
//
// The probe posts one message back: a frozen JSON record with the names it saw. The
// parity test on the Rust side (interface_parity_tests.rs::worker_global_scope) uses
// it as the Chrome 156 control for `install_worker_global_scope`.
(() => {
  const own = Reflect.ownKeys;
  const getProto = Object.getPrototypeOf;
  const isSym = Symbol.iterator; // reserve for tagging
  const names = (object) => {
    const keys = own(object);
    const result = [];
    for (let i = 0; i < keys.length; i++) {
      const k = keys[i];
      if (typeof k === "string") result.push(k);
      else result.push(`@@${String(k)}`);
    }
    result.sort();
    return result;
  };
  // Walk the prototype chain of `self` and label each layer by constructor.name so the
  // test can assert that DedicatedWorkerGlobalScope extends WorkerGlobalScope extends
  // EventTarget extends Object, and see the members at each level.
  const chain = [];
  for (let proto = getProto(self); proto; proto = getProto(proto)) {
    const ctor = proto.constructor;
    chain.push({
      name: ctor && ctor.name ? ctor.name : "(anonymous)",
      members: names(proto),
    });
  }
  // The own members of `self` are the globals installed on the worker scope. Filter
  // out the probe's own locals (none here — this IIFE's `const`s don't become globals).
  const globals = names(self);
  // Constants that are known to be DedicatedWorkerGlobalScope interface constructors.
  // The parity test checks that the ones it needs are present on `self`.
  const interfaces = {};
  const probeInterfaces = [
    "DedicatedWorkerGlobalScope",
    "WorkerGlobalScope",
    "EventTarget",
    "MessageEvent",
    "ErrorEvent",
    "PromiseRejectionEvent",
    "Event",
    "MessagePort",
    "MessageChannel",
  ];
  for (let i = 0; i < probeInterfaces.length; i++) {
    const name = probeInterfaces[i];
    interfaces[name] = typeof self[name];
  }
  self.postMessage({
    userAgent: navigator && navigator.userAgent,
    constructor: self.constructor && self.constructor.name,
    globals,
    chain,
    interfaces,
  });
})();
