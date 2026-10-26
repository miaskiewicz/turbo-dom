(async () => {
  'use strict';
  // turbo-surf#42: every DOMException the platform raises is an instance of the realm's
  // DOMException interface — for exceptions the binding's JS modules raise too, also when the
  // embedder installs its own DOMException after the binding (turbo-surf), and whatever page
  // script later assigns to `globalThis.DOMException` (the interface object is the one the
  // realm was created with). Each operation raises (or creates) one exception through a
  // different module; recorded per operation: [name, instanceof DOMException, prototype is
  // DOMException.prototype], first with `DOMException` as installed, then after page script
  // replaced `globalThis.DOMException` (checked against the original interface).
  // A fresh connected element per operation (no operation depends on another's leftovers).
  const box = () => document.documentElement.appendChild(document.createElement('div'));
  const operations = {
    imageDecode: () => document.createElement('img').decode(),
    animationPseudo: () => document.createElement('div').animate([], { duration: 1, pseudoElement: 'nope' }),
    customElementName: () => customElements.define('bad', class extends HTMLElement {}),
    xpathSyntax: () => document.evaluate('(', document, null, 0, null),
    pictureInPicture: () => document.exitPictureInPicture(),
    qualifiedName: () => document.implementation.createDocument(null, 'a:b'),
    attachInternals: () => document.createElement('div').attachInternals(),
    editContext: () => { document.createElement('img').editContext = new EditContext(); },
    fileInputValue: () => { const input = document.createElement('input'); input.type = 'file'; input.value = 'x'; },
    abortReason: () => { const controller = new AbortController(); controller.abort(); throw controller.signal.reason; },
    requestSubmit: () => document.createElement('form').requestSubmit(document.createElement('button')),
    outerText: () => { document.createElement('span').outerText = 'x'; },
    dialogModal: () => document.createElement('dialog').showModal(),
    pointerCapture: () => box().setPointerCapture(12345),
    stepUp: () => document.createElement('input').stepUp(),
    rangeOffset: () => document.createRange().setStart(box(), 999),
    rangeSurround: () => { const range = document.createRange(); range.selectNode(box()); range.surroundContents(document.createComment('c')); },
    selectionIndex: () => getSelection().getRangeAt(5),
    showPicker: () => document.createElement('input').showPicker(),
    typedOm: () => CSSNumericValue.parse('???'),
    adoptSheet: () => { document.adoptedStyleSheets = [document.querySelector('style').sheet]; },
    selectionRange: () => { const input = document.createElement('input'); input.type = 'number'; input.setSelectionRange(0, 1); },
    insertRule: () => new CSSStyleSheet().insertRule('a{}', 99),
    processingInstruction: () => document.createProcessingInstruction('1', ''),
    createElement: () => document.createElement('1'),
    querySelector: () => document.querySelector('['),
    appendChild: () => document.appendChild(document.createElement('div')),
  };
  const run = async (Interface) => {
    const out = {};
    for (const [id, operation] of Object.entries(operations)) {
      let error;
      try {
        await operation();
        out[id] = 'no exception';
        continue;
      } catch (e) {
        error = e;
      }
      out[id] = [
        error && error.name,
        error instanceof Interface,
        error !== null && typeof error === 'object' && Object.getPrototypeOf(error) === Interface.prototype,
      ];
    }
    return out;
  };
  const Original = DOMException;
  const installed = await run(Original);
  globalThis.DOMException = function DOMException() {};
  let replaced;
  try {
    replaced = await run(Original);
  } finally {
    globalThis.DOMException = Original;
  }
  return { installed, replaced };
})()
