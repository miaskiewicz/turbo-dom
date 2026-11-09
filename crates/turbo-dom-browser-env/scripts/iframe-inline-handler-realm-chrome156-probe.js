(async () => {
  // turbo-dom#478 / turbo-surf#287: Chrome 156 compiles an inline on* content-attribute
  // handler in the realm of the element's own document, NOT the realm that happens to
  // trigger the dispatch. We verify this by borrowing the PARENT realm's
  // `EventTarget.prototype.dispatchEvent` and invoking it on an element inside a
  // same-origin iframe: the handler body must still resolve `globalThis`, free
  // identifiers, and `this` (the element) through the iframe's own window.
  const results = {};

  const iframe = document.createElement('iframe');
  iframe.srcdoc =
    '<!doctype html><html><body><div id="target" onload="' +
    'globalThis.realmMarker = \'child\';' +
    'globalThis.hits = (globalThis.hits || 0) + 1;' +
    'globalThis.thisIsElement = (this === document.getElementById(\'target\'));' +
    'globalThis.freeDocument = document;' +
    '"></div></body></html>';
  document.body.appendChild(iframe);
  await new Promise((resolve) => iframe.addEventListener('load', resolve, { once: true }));

  const childWindow = iframe.contentWindow;
  const childDocument = iframe.contentDocument;
  const target = childDocument.getElementById('target');

  // Parent-realm sentinel values, which the handler must NOT touch.
  window.realmMarker = 'parent';
  window.hits = 0;
  window.thisIsElement = false;
  window.freeDocument = null;

  // Borrow the PARENT realm's dispatchEvent and call it on a CHILD-owned element.
  const borrowedDispatch = EventTarget.prototype.dispatchEvent;
  borrowedDispatch.call(target, new childWindow.Event('load'));

  results.realmSelection = {
    childRealmMarker: childWindow.realmMarker,
    parentRealmMarker: window.realmMarker,
    childHits: childWindow.hits,
    parentHits: window.hits,
    // The handler's `this` default binds to the element (step 10 of the event-handler
    // algorithm), checked through the iframe's document — this proves the free
    // identifier `document` resolved against the iframe window.
    childThisIsElement: childWindow.thisIsElement,
    parentThisIsElement: window.thisIsElement,
    // The free `document` identifier resolves against the iframe realm.
    childFreeDocumentIsChild: childWindow.freeDocument === childDocument,
    childFreeDocumentIsParent: childWindow.freeDocument === document,
  };

  // Parent-registered dispatchEvent must still deliver EXACTLY one invocation
  // (turbo-dom#478 / surf#287: double-fire regression check).
  results.oneFirePerDispatch = { childHitsAfterSingleDispatch: childWindow.hits };

  iframe.remove();
  return results;
})()
