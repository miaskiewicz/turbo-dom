(async () => {
  // Brand checks and edge arguments of EventTarget / AbortSignal / AbortController / Event /
  // ErrorEvent members: primitive, plain-object, cross-interface and proper receivers, plus
  // listener options with signals (already aborted, aborted later, abort reasons).
  const describe = value => value === null ? 'null' : value === undefined ? 'undefined' : typeof value === 'object' ? (value.constructor && value.constructor.name || 'object') : typeof value === 'function' ? 'function' : String(value);
  const call = run => { try { return describe(run()); } catch (e) { return e.name; } };
  const receivers = { number: 1, nullish: null, plain: {}, target: new EventTarget(), node: document.createElement('div'), signal: new AbortController().signal, controller: new AbortController(), event: new Event('x'), error: new ErrorEvent('e', { message: 'm', filename: 'f', lineno: 2, colno: 3, error: 7 }), window };
  const members = [
    [EventTarget.prototype, [], ['addEventListener', 'removeEventListener', 'dispatchEvent']],
    [AbortSignal.prototype, ['aborted', 'reason', 'onabort'], ['throwIfAborted']],
    [AbortController.prototype, ['signal'], ['abort']],
    [Event.prototype, ['type', 'target', 'currentTarget', 'eventPhase', 'defaultPrevented'], ['preventDefault', 'composedPath']],
    [ErrorEvent.prototype, ['message', 'filename', 'lineno', 'colno', 'error'], []],
  ];
  const out = {};
  for (const [prototype, getters, methods] of members) {
    const name = prototype.constructor.name;
    for (const getter of getters) {
      const get = Object.getOwnPropertyDescriptor(prototype, getter).get;
      const row = {};
      for (const [kind, receiver] of Object.entries(receivers)) row[kind] = call(() => get.call(receiver));
      out[name + '.' + getter] = row;
    }
    for (const method of methods) {
      const row = {};
      for (const [kind, receiver] of Object.entries(receivers)) row[kind] = call(() => prototype[method].call(receiver, method === 'dispatchEvent' ? new Event('y') : 'y', () => {}));
      out[name + '.' + method + '()'] = row;
    }
  }
  // Listener signals.
  const log = [];
  const target = new EventTarget();
  const aborted = AbortSignal.abort('why');
  target.addEventListener('a', () => log.push('pre-aborted'), { signal: aborted });
  const later = new AbortController();
  target.addEventListener('a', () => log.push('later'), { signal: later.signal });
  const node = document.body.appendChild(document.createElement('p'));
  const nodeController = new AbortController();
  node.addEventListener('a', () => log.push('node'), { signal: nodeController.signal });
  target.dispatchEvent(new Event('a'));
  node.dispatchEvent(new Event('a'));
  later.abort(new Error('stop'));
  nodeController.abort();
  target.dispatchEvent(new Event('a'));
  node.dispatchEvent(new Event('a'));
  out.signals = [log, aborted.reason, later.signal.reason instanceof Error, nodeController.signal.reason.name, call(() => aborted.throwIfAborted())];
  out.badSignal = [call(() => target.addEventListener('a', () => {}, { signal: {} })), call(() => target.addEventListener('a', () => {}, { signal: null }))];
  return out;
})()
