(async () => {
  // Custom element edges: an upgrade constructor that constructs another custom element
  // after super(), and contextual fragments (empty, and with a defined element).
  const out = {};
  const log = [];
  class XB extends HTMLElement { constructor() { super(); log.push(['b', this.isConnected]); } }
  customElements.define('x-b', XB);
  document.body.innerHTML = '<x-a id=a></x-a>';
  const a = document.getElementById('a');
  const before = [a instanceof HTMLElement, a.matches(':defined')];
  class XA extends HTMLElement { constructor() { super(); log.push(['a', this === a]); this.inner = new XB(); } }
  customElements.define('x-a', XA);
  out.nested = [before, a instanceof XA, a.inner instanceof XB, a.inner.localName, a.inner === a, a.matches(':defined'), log.slice()];
  const range = document.createRange();
  range.selectNodeContents(document.body);
  const empty = range.createContextualFragment('');
  const filled = range.createContextualFragment('<x-b id=f></x-b>text');
  out.fragments = [empty.childNodes.length, empty.nodeType, filled.childNodes.length, filled.firstChild instanceof XB, filled.firstChild.matches(':defined'), log.slice(2)];
  document.body.innerHTML = '';
  return out;
})()
