(async () => {
  // Pointer capture and pointer lock members observable without trusted input: a fresh page
  // has no pressed mouse button and no transient user activation.
  const out = { shape: {}, capture: {}, conversion: {}, lock: {} };
  const tick = () => new Promise(resolve => setTimeout(resolve, 0));
  const settle = async () => { for (let index = 0; index < 20; index++) await tick(); };
  const run = body => { try { const value = body(); return ['ok', value === undefined ? 'undefined' : value]; } catch (error) { return [error.name, error.message, error.constructor.name]; } };
  const outcome = async promise => { try { return ['resolved', String(await promise)]; } catch (error) { return ['rejected', error.name, error.message, error.constructor.name]; } };
  const describe = (owner, name) => {
    const descriptor = Object.getOwnPropertyDescriptor(owner, name);
    if (!descriptor) return null;
    const fn = descriptor.value || descriptor.get;
    return { kind: 'value' in descriptor ? 'method' : 'accessor', name: fn.name, length: fn.length, writable: descriptor.writable ?? null, enumerable: descriptor.enumerable, configurable: descriptor.configurable, setter: descriptor.set === undefined ? null : typeof descriptor.set };
  };
  for (const name of ['setPointerCapture', 'releasePointerCapture', 'hasPointerCapture', 'requestPointerLock']) out.shape[name] = describe(Element.prototype, name);
  out.shape.exitPointerLock = describe(Document.prototype, 'exitPointerLock');
  out.shape.documentPointerLockElement = describe(Document.prototype, 'pointerLockElement');
  out.shape.shadowPointerLockElement = describe(ShadowRoot.prototype, 'pointerLockElement');
  out.shape.handlers = ['ongotpointercapture', 'onlostpointercapture', 'onpointerlockchange', 'onpointerlockerror', 'onpointerrawupdate'].map(name => [name, name in HTMLElement.prototype, name in Document.prototype]);
  out.shape.brands = {
    set: run(() => Element.prototype.setPointerCapture.call({}, 1)),
    release: run(() => Element.prototype.releasePointerCapture.call(document, 1)),
    has: run(() => Element.prototype.hasPointerCapture.call(document.createTextNode(''), 1)),
    exit: run(() => Document.prototype.exitPointerLock.call(document.body)),
    documentElement: run(() => Object.getOwnPropertyDescriptor(Document.prototype, 'pointerLockElement').get.call(document.body)),
    shadowElement: run(() => Object.getOwnPropertyDescriptor(ShadowRoot.prototype, 'pointerLockElement').get.call(document)),
  };

  const a = document.createElement('div');
  a.id = 'a';
  document.body.append(a);
  out.capture.has1 = run(() => a.hasPointerCapture(1));
  out.capture.has5 = run(() => a.hasPointerCapture(5));
  out.capture.set5 = run(() => a.setPointerCapture(5));
  out.capture.release5 = run(() => a.releasePointerCapture(5));
  out.capture.set1 = run(() => a.setPointerCapture(1));
  out.capture.has1AfterSet = run(() => a.hasPointerCapture(1));
  out.capture.release1 = run(() => a.releasePointerCapture(1));
  out.capture.set0 = run(() => a.setPointerCapture(0));
  out.capture.setNegative = run(() => a.setPointerCapture(-1));
  out.capture.set2 = run(() => a.setPointerCapture(2));
  const detached = document.createElement('div');
  out.capture.detachedSet1 = run(() => detached.setPointerCapture(1));
  out.capture.detachedSet5 = run(() => detached.setPointerCapture(5));
  out.capture.detachedRelease1 = run(() => detached.releasePointerCapture(1));
  out.capture.detachedRelease5 = run(() => detached.releasePointerCapture(5));
  out.capture.detachedHas1 = run(() => detached.hasPointerCapture(1));
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  const foreign = document.createElementNS('urn:x', 'x');
  const host = document.createElement('div');
  document.body.append(svg, foreign, host);
  const root = host.attachShadow({ mode: 'open' });
  root.innerHTML = '<p>x</p>';
  out.capture.svgSet1 = run(() => svg.setPointerCapture(1));
  out.capture.foreignSet1 = run(() => foreign.setPointerCapture(1));
  out.capture.shadowSet1 = run(() => root.firstChild.setPointerCapture(1));
  const inert = document.implementation.createHTMLDocument('');
  const inertElement = inert.createElement('div');
  inert.body.append(inertElement);
  out.capture.inactiveSet1 = run(() => inertElement.setPointerCapture(1));
  out.capture.inactiveSet5 = run(() => inertElement.setPointerCapture(5));
  out.capture.inactiveRelease5 = run(() => inertElement.releasePointerCapture(5));
  out.capture.inactiveHas1 = run(() => inertElement.hasPointerCapture(1));
  const xml = document.implementation.createDocument(null, 'root').documentElement;
  out.capture.xmlSet5 = run(() => xml.setPointerCapture(5));
  // Synthetic pointer events never press the mouse pointer or activate another pointer.
  a.dispatchEvent(new PointerEvent('pointerdown', { pointerId: 1, buttons: 1, bubbles: true }));
  out.capture.afterSyntheticDown = [run(() => a.setPointerCapture(1)), run(() => a.hasPointerCapture(1))];
  a.dispatchEvent(new PointerEvent('pointerdown', { pointerId: 9, buttons: 1, bubbles: true }));
  out.capture.syntheticPointer = run(() => a.setPointerCapture(9));

  const reads = [];
  out.conversion.none = ['setPointerCapture', 'releasePointerCapture', 'hasPointerCapture'].map(name => run(() => a[name]()));
  out.conversion.string = run(() => a.setPointerCapture('1'));
  out.conversion.wrapped = run(() => a.setPointerCapture(2 ** 32 + 1));
  out.conversion.wrappedFive = run(() => a.setPointerCapture(2 ** 32 + 5));
  out.conversion.fraction = run(() => a.setPointerCapture(1.9));
  out.conversion.nan = run(() => a.setPointerCapture(NaN));
  out.conversion.infinity = run(() => a.setPointerCapture(Infinity));
  out.conversion.object = run(() => a.setPointerCapture({ valueOf() { reads.push('valueOf'); return 1; } }));
  out.conversion.symbol = run(() => a.setPointerCapture(Symbol('id')));
  out.conversion.bigint = run(() => a.setPointerCapture(1n));
  out.conversion.valueOfSymbol = run(() => a.setPointerCapture({ valueOf() { return Symbol('id'); } }));
  out.conversion.valueOfThrows = run(() => a.setPointerCapture({ valueOf() { throw new RangeError('own'); } }));
  out.conversion.valueOfThrowsType = run(() => a.setPointerCapture({ valueOf() { throw new TypeError('own'); } }));
  out.conversion.noPrimitive = run(() => a.setPointerCapture({ valueOf() { return {}; }, toString() { return {}; } }));
  out.conversion.hasSymbol = run(() => a.hasPointerCapture(Symbol('id')));
  out.conversion.detachedSymbol = run(() => detached.releasePointerCapture(Symbol('id')));
  out.conversion.reads = reads;

  const log = [];
  for (const type of ['pointerlockchange', 'pointerlockerror']) document.addEventListener(type, event => log.push([event.type, event.target === document, event.isTrusted, event.bubbles, event.cancelable, event.composed, event.constructor.name, String(document.pointerLockElement)].join(' ')));
  a.addEventListener('pointerlockerror', () => log.push('element listener'));
  const track = (label, promise) => promise.then(value => log.push(label + ' resolved ' + value), error => log.push(label + ' rejected ' + error.name + ': ' + error.message));
  out.lock.initial = [document.pointerLockElement, root.pointerLockElement];
  out.lock.exitUnlocked = run(() => document.exitPointerLock());
  out.lock.inertExit = run(() => inert.exitPointerLock());
  const first = a.requestPointerLock();
  out.lock.returns = [Object.prototype.toString.call(first), first instanceof Promise];
  out.lock.pendingElement = [document.pointerLockElement, root.pointerLockElement];
  // The pending request already holds the lock controller's element.
  out.lock.captureWhilePending = run(() => a.setPointerCapture(1));
  track('first', first);
  log.push('requested');
  await settle();
  out.lock.denied = log.splice(0);
  out.lock.afterDenial = [document.pointerLockElement, run(() => a.setPointerCapture(1))];
  out.lock.brand = await outcome(Element.prototype.requestPointerLock.call({}));
  out.lock.brandDocument = await outcome(Element.prototype.requestPointerLock.call(document));
  out.lock.badOptions = await outcome(detached.requestPointerLock(5));
  out.lock.stringOptions = await outcome(detached.requestPointerLock('x'));
  const optionReads = [];
  out.lock.detached = await outcome(detached.requestPointerLock({ get unadjustedMovement() { optionReads.push('unadjustedMovement'); return true; } }));
  out.lock.optionReads = optionReads;
  out.lock.inactive = await outcome(inertElement.requestPointerLock());
  out.lock.nullOptions = await outcome(svg.requestPointerLock(null));
  out.lock.functionOptions = await outcome(root.firstChild.requestPointerLock(() => {}));
  await settle();
  out.lock.events = log.splice(0);
  out.lock.final = [document.pointerLockElement, root.pointerLockElement, inert.pointerLockElement];
  return out;
})()
