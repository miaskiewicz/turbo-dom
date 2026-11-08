(async () => {
  const name = 'turbo-dom-dispatch-probe-' + Math.random().toString(36).slice(2);
  // Open a database and create the store.
  const db = await new Promise((resolve, reject) => {
    const open = indexedDB.open(name, 1);
    open.onupgradeneeded = (event) => {
      event.target.result.createObjectStore('s', { keyPath: 'k' });
    };
    open.onsuccess = (event) => resolve(event.target.result);
    open.onerror = (event) => reject(event.target.error);
  });
  // Seed a row so the next add with the same key raises a ConstraintError whose event
  // bubbles IDBRequest -> IDBTransaction -> IDBDatabase.
  await new Promise((resolve, reject) => {
    const tx = db.transaction('s', 'readwrite');
    tx.objectStore('s').add({ k: 1, v: 'a' });
    tx.oncomplete = resolve;
    tx.onerror = reject;
    tx.onabort = reject;
  });
  // Capture phase records across the whole chain for the error event.
  const seen = [];
  const microtaskSeen = [];
  const errorTx = db.transaction('s', 'readwrite');
  const errorStore = errorTx.objectStore('s');
  const errorReq = errorStore.add({ k: 1, v: 'b' });
  const levels = [
    ['request', errorReq],
    ['tx', errorTx],
    ['db', db],
  ];
  const push = (level, obj, capture, event) => {
    const path = event.composedPath();
    seen.push({
      level,
      capture,
      phase: event.eventPhase,
      currentIsLevel: event.currentTarget === obj,
      targetIsReq: event.target === errorReq,
      pathLen: path.length,
      pathFirst: path.length > 0 && path[0] === obj,
    });
  };
  for (const [level, obj] of levels) {
    for (const capture of [true, false]) {
      obj.addEventListener('error', (event) => push(level, obj, capture, event), { capture });
    }
  }
  // Microtask observability: listener A queues a microtask; listener B records whether it
  // ran before B was invoked. Expectation in Chrome 156: microtask ran (yes).
  let microFlag = 'pending';
  errorReq.addEventListener('error', () => {
    Promise.resolve().then(() => { microFlag = 'ran'; });
    microtaskSeen.push(['listenerA', microFlag]);
  }, { capture: true });
  errorReq.addEventListener('error', () => {
    microtaskSeen.push(['listenerB', microFlag]);
  }, { capture: true });
  // Prevent the tx from aborting on the constraint error so we see every listener.
  errorReq.addEventListener('error', (e) => e.preventDefault(), { capture: false });
  await new Promise((resolve) => {
    errorTx.oncomplete = resolve;
    errorTx.onabort = resolve;
  });
  db.close();
  await new Promise((resolve, reject) => {
    const del = indexedDB.deleteDatabase(name);
    del.onsuccess = resolve;
    del.onerror = reject;
    del.onblocked = resolve;
  });
  return { phases: seen, microtask: microtaskSeen };
})()
