(() => {
  'use strict';
  // turbo-surf#36 items 1-2. getComputedStyle reports Chrome's computed value for properties no
  // author rule sets: display is the element's UA display (block / inline / none / table-* /
  // list-item / inline-block / contents / ruby…, [hidden] -> none, blockified for floats,
  // absolutely positioned elements and flex / grid items), position is static, visibility is
  // visible (inherited); the CSS-wide keywords resolve. Disconnected elements report "" for all.
  // And the document gets no <title> unless the page makes one (document.title's setter does).
  const props = ['display', 'position', 'visibility', 'opacity', 'transform', 'zoom'];
  const out = {};
  const view = (el) => { const s = getComputedStyle(el); return props.map((p) => s.getPropertyValue(p)); };
  const head = document.head;
  out.head = { children: head ? Array.from(head.children, (el) => el.localName) : null, title: document.title, titles: document.querySelectorAll('title').length };
  if (document.getElementById('body')) {
    out.elements = Object.fromEntries(Array.from(document.querySelectorAll('[id]'), (el) => [el.id, view(el)]));
    out.camel = (() => { const s = getComputedStyle(document.getElementById('div')); return [s.display, s.position, s.visibility]; })();
    out.html = view(document.documentElement);
    out.head.view = view(document.head);
    const host = document.getElementById('host');
    const root = host.attachShadow({ mode: 'open' });
    root.innerHTML = '<style>:host{display:block;position:relative}</style><span id="inShadow"></span>';
    out.shadowHost = view(host);
    out.inShadow = view(root.getElementById('inShadow'));
    out.detached = view(document.createElement('div'));
    const later = document.createElement('section');
    document.body.append(later);
    out.appended = view(later);
    later.remove();
    out.removed = view(later);
  } else {
    // the title setter creates a <title> on demand
    document.title = 'set';
    out.afterSet = { children: Array.from(document.head.children, (el) => el.localName), title: document.title, text: document.querySelector('title').textContent };
  }
  return out;
})();
