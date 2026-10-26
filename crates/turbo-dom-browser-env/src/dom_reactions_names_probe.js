(() => {
  'use strict';
  // turbo-dom#43: the HTML fragment parser queues its elements' upgrades when it creates them,
  // so innerHTML / outerHTML / setHTMLUnsafe run the new elements' constructors and
  // connectedCallbacks before the removed children's disconnectedCallbacks; insertAdjacentHTML
  // and createContextualFragment (which upgrades inside the fragment) complete the set.
  // turbo-dom#44: validate-and-extract strictly splits a qualified name on ':' — the local name
  // is the second part — for elements, attributes and attribute nodes.
  const out = {};
  const log = [], host = document.getElementById('host');
  class Ordered extends HTMLElement {
    constructor() { super(); log.push(['ctor', this.id, this.isConnected, host.childNodes.length]); }
    connectedCallback() { log.push(['conn', this.id, host.childNodes.length]); }
    disconnectedCallback() { log.push(['disc', this.id, host.childNodes.length]); }
  }
  customElements.define('x-order', Ordered);
  const step = (name, run) => { log.push(['--', name]); run(); };
  step('innerHTML', () => { host.innerHTML = '<x-order id="a1"></x-order><x-order id="a2"></x-order>'; });
  step('outerHTML', () => { document.getElementById('a1').outerHTML = '<x-order id="b1"></x-order><x-order id="b2"></x-order>'; });
  step('setHTMLUnsafe', () => { host.setHTMLUnsafe('<x-order id="c1"></x-order>'); });
  step('insertAdjacentHTML', () => { host.insertAdjacentHTML('afterbegin', '<x-order id="d1"></x-order>'); });
  step('contextualFragment', () => {
    const range = document.createRange();
    range.selectNodeContents(host);
    const fragment = range.createContextualFragment('<x-order id="e1"></x-order>');
    log.push(['fragment', fragment.firstChild.matches(':defined')]);
    host.replaceChildren(fragment);
  });
  step('end', () => { host.textContent = ''; });
  out.order = log;
  const attempt = (run) => { try { return run(); } catch (e) { return e.name; } };
  const attr = (a) => [a.prefix, a.localName, a.name, a.namespaceURI];
  out.names = {};
  for (const name of ['a:b:c', 'a:b:c:d', 'a::b', 'a:b:', ':a:b', 'a:1b:c', 'a:b c:d', 'xmlns:a:b', 'xml:a:b']) {
    out.names[name] = {
      element: attempt(() => { const e = document.createElementNS('urn:x', name); return [e.prefix, e.localName, e.tagName]; }),
      attributeNode: attempt(() => attr(document.createAttributeNS('urn:x', name))),
      setAttributeNS: attempt(() => { const e = document.createElement('div'); e.setAttributeNS('urn:x', name, '1'); return [attr(e.attributes[0]), e.getAttribute(name), e.getAttributeNS('urn:x', 'b')]; }),
      xmlElement: attempt(() => { const e = document.createElementNS('http://www.w3.org/XML/1998/namespace', name); return [e.prefix, e.localName]; }),
    };
  }
  return out;
})();
