(async () => {
  // turbo-dom#301: an exception thrown by an event listener is reported (an ErrorEvent at the
  // listener's window, synchronously, before the next listener) and dispatch continues.
  const out = {}, log = [];
  let reporting = true;
  const describe = (e) => [
    Object.prototype.toString.call(e), e.type, e.isTrusted, e.bubbles, e.cancelable, e.composed,
    e.eventPhase, e.target === window, e.currentTarget === window, e.message, e.filename, e.lineno,
    e.colno, typeof e.error === 'object' && e.error !== null ? e.error.name + ':' + e.error.message : e.error,
  ];
  addEventListener('error', (e) => { if (reporting) log.push(['error', ...describe(e)]); });
  const run = (name, body) => { log.length = 0; let value; try { value = body(); } catch (error) { value = 'threw ' + error; } out[name] = { value, log: log.slice() }; };

  run('generic', () => {
    const target = new EventTarget();
    target.addEventListener('x', () => { log.push('first'); throw new Error('generic'); });
    target.addEventListener('x', (e) => log.push(['second', e.currentTarget === target, e.eventPhase]));
    return target.dispatchEvent(new Event('x', { cancelable: true }));
  });
  run('node', () => {
    const parent = document.createElement('div'), child = document.createElement('span');
    parent.append(child); document.body.append(parent);
    parent.addEventListener('y', () => { log.push('parent-capture'); throw new TypeError('capture'); }, true);
    child.addEventListener('y', () => { log.push('child'); throw new RangeError('child'); });
    child.addEventListener('y', (e) => log.push(['child-2', e.currentTarget === child, e.eventPhase]));
    parent.addEventListener('y', (e) => log.push(['parent-bubble', e.currentTarget === parent, e.eventPhase]));
    const result = child.dispatchEvent(new Event('y', { bubbles: true }));
    parent.remove();
    return result;
  });
  run('detached', () => {
    const node = document.createElement('p');
    node.addEventListener('z', () => { throw new Error('detached'); });
    node.addEventListener('z', () => log.push('after'));
    return node.dispatchEvent(new Event('z'));
  });
  run('window', () => {
    addEventListener('w', () => { log.push('w1'); throw new Error('window'); }, { once: true });
    addEventListener('w', () => log.push('w2'), { once: true });
    return dispatchEvent(new Event('w'));
  });
  run('document', () => {
    const listener = () => { throw new Error('document'); };
    document.addEventListener('d', listener);
    const result = document.dispatchEvent(new Event('d'));
    document.removeEventListener('d', listener);
    return result;
  });
  run('handleEvent', () => {
    const target = new EventTarget(), node = document.createElement('i');
    target.addEventListener('h', { handleEvent() { throw new Error('object'); } });
    node.addEventListener('h', { handleEvent() { throw new Error('node object'); } });
    target.dispatchEvent(new Event('h')); node.dispatchEvent(new Event('h'));
    return true;
  });
  run('handleEventNotCallable', () => {
    const target = new EventTarget(), node = document.createElement('i');
    target.addEventListener('h', { handleEvent: 5 });
    target.addEventListener('h', () => log.push('after-target'));
    node.addEventListener('h', {});
    node.addEventListener('h', () => log.push('after-node'));
    target.dispatchEvent(new Event('h')); node.dispatchEvent(new Event('h'));
    return log.filter((row) => Array.isArray(row)).map((row) => row[0] + ':' + row[14]);
  });
  run('handleEventGetter', () => {
    const target = new EventTarget();
    target.addEventListener('g', { get handleEvent() { throw new Error('getter'); } });
    target.addEventListener('g', () => log.push('after'));
    return target.dispatchEvent(new Event('g'));
  });
  run('primitives', () => {
    const target = new EventTarget();
    for (const value of ['text', 42, null, undefined, false, { custom: 1 }, Symbol('s')])
      target.addEventListener('p', () => { throw value; });
    target.dispatchEvent(new Event('p'));
    return log.map((row) => [row[10], typeof row[14] === 'symbol' ? 'symbol' : row[14] && typeof row[14] === 'object' ? 'object' : row[14]]);
  });
  run('stopImmediate', () => {
    const target = new EventTarget();
    target.addEventListener('s', (e) => { e.stopImmediatePropagation(); throw new Error('stopped'); });
    target.addEventListener('s', () => log.push('not reached'));
    return target.dispatchEvent(new Event('s'));
  });
  run('onerrorCancels', () => {
    const rows = [];
    let canceled;
    const cancel = (e) => { canceled = e.defaultPrevented; };
    onerror = function (message, source, line, column, error) { rows.push([message, source, line, column, error && error.message, arguments.length, this === window]); return true; };
    addEventListener('error', cancel);
    const target = new EventTarget();
    target.addEventListener('c', () => { throw new Error('canceled'); });
    const result = target.dispatchEvent(new Event('c', { cancelable: true }));
    onerror = null; removeEventListener('error', cancel);
    return [result, canceled, rows];
  });
  run('preventedByListener', () => {
    const prevent = (e) => e.preventDefault();
    addEventListener('error', prevent);
    const target = new EventTarget();
    target.addEventListener('v', () => { throw new Error('prevented'); });
    const result = target.dispatchEvent(new Event('v', { cancelable: true }));
    removeEventListener('error', prevent);
    return result;
  });
  run('handlerAttribute', () => {
    const node = document.createElement('b');
    document.body.append(node);
    node.onclick = () => { log.push('handler'); throw new Error('handler'); };
    node.addEventListener('click', () => log.push('listener'));
    const result = node.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    node.remove();
    return result;
  });
  run('abortSignal', () => {
    const controller = new AbortController();
    controller.signal.addEventListener('abort', () => { throw new Error('abort'); });
    controller.signal.addEventListener('abort', () => log.push('second-abort'));
    controller.abort();
    return controller.signal.aborted;
  });
  run('throwingErrorListener', () => {
    // An exception thrown while the window is reporting an exception is not reported again.
    let calls = 0;
    const thrower = () => { calls++; throw new Error('from error listener'); };
    addEventListener('error', thrower);
    const target = new EventTarget();
    target.addEventListener('t', () => { throw new Error('outer'); });
    target.addEventListener('t', () => log.push('next'));
    target.dispatchEvent(new Event('t'));
    removeEventListener('error', thrower);
    return calls;
  });
  run('nestedDispatch', () => {
    const outer = new EventTarget(), inner = new EventTarget();
    inner.addEventListener('n', () => { throw new Error('inner'); });
    outer.addEventListener('n', () => { log.push('outer-start'); inner.dispatchEvent(new Event('n')); log.push('outer-end'); });
    return outer.dispatchEvent(new Event('n'));
  });
  run('reportErrorLocation', () => {
    const early = new Error('made early');
    const target = new EventTarget();
    target.addEventListener('l', () => {
      throw early;
    });
    return target.dispatchEvent(new Event('l'));
  });
  run('reportError', () => {
    const descriptor = Object.getOwnPropertyDescriptor(window, 'reportError');
    const shape = descriptor ? [typeof descriptor.value, descriptor.writable, descriptor.enumerable, descriptor.configurable, descriptor.value.length, descriptor.value.name] : null;
    let missing;
    try { reportError(); } catch (error) { missing = error.name + ': ' + error.message; }
    const values = [];
    for (const value of [new Error('reported'), 'reported text', null]) values.push(reportError(value));
    return [shape, missing, values];
  });
  reporting = false;
  return out;
})()
