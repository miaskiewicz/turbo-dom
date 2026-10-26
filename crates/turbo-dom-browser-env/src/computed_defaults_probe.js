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
  } else if (document.getElementById('modal')) {
    // turbo-dom#30: show() vs showModal() vs close(). A modal dialog matches :modal and enters
    // the top layer: html.css gives it position fixed, inset-block 0, max sizes of
    // calc(100% - 6px - 2em), overflow auto and visibility visible; a top-layer element whose
    // position is static / relative / sticky computes to absolute; its ::backdrop is fixed at
    // inset 0 (dialogs tint it). Outside the top layer ::backdrop is block / absolute / auto.
    const box = ['display', 'position', 'visibility'];
    const modalProps = box.concat(['overflow', 'overflow-x', 'overflow-y', 'top', 'bottom', 'max-width', 'max-height']);
    const backdropProps = box.concat(['top', 'right', 'bottom', 'left', 'background-color', 'color', 'opacity', 'transform']);
    const read = (el, list, pseudo) => { const s = getComputedStyle(el, pseudo); return list.map((p) => s.getPropertyValue(p)); };
    const backdrop = (el) => read(el, backdropProps, '::backdrop');
    const flags = (el) => [el.open, el.matches(':modal'), el.matches(':open'), el.matches('dialog:not(:modal)'), document.querySelectorAll(':modal').length];
    const $ = (id) => document.getElementById(id);
    const d = $('modal'), dialog = {};
    dialog.closed = [read(d, box), flags(d), backdrop(d)];
    d.show();
    dialog.shown = [read(d, box), flags(d), backdrop(d)];
    d.close();
    dialog.shownClosed = [read(d, box), flags(d), backdrop(d)];
    d.showModal();
    dialog.modal = [read(d, modalProps), flags(d), backdrop(d), read(d, box, '::BACKDROP'), document.querySelector(':modal') === d];
    d.removeAttribute('open');
    dialog.modalOpenRemoved = [read(d, modalProps), flags(d), backdrop(d)];
    d.setAttribute('open', '');
    d.close();
    dialog.modalClosed = [read(d, box), flags(d), backdrop(d)];
    d.showModal();
    d.remove();
    dialog.modalRemoved = [read(d, box), flags(d), backdrop(d)];
    document.body.append(d);
    dialog.modalReinserted = [read(d, box), flags(d), backdrop(d)];
    d.close();
    const h = $('inHidden');
    h.showModal();
    dialog.hiddenModal = [read(h, modalProps), backdrop(h)];
    h.close();
    h.show();
    dialog.hiddenShown = [read(h, box), backdrop(h)];
    h.close();
    for (const id of ['authored', 'big', 'relative', 'sticky', 'absolute', 'overflowX']) {
      const el = $(id);
      el.showModal();
      dialog[id] = [read(el, modalProps), backdrop(el)];
      el.close();
      dialog[id].push(read(el, box));
    }
    const first = $('first'), second = $('second');
    first.showModal();
    second.showModal();
    dialog.stacked = [flags(first), flags(second), read(first, box), read(second, box), backdrop(first), backdrop(second)];
    second.close();
    first.close();
    out.dialog = dialog;
    const popover = {};
    for (const id of ['pop', 'popStatic']) {
      const el = $(id);
      el.showPopover();
      popover[id] = [read(el, box), backdrop(el)];
      el.hidePopover();
      popover[id].push(read(el, box), backdrop(el));
    }
    out.popover = popover;
    out.otherBackdrop = { div: backdrop($('plain')), detached: backdrop(document.createElement('dialog')) };
  } else {
    // the title setter creates a <title> on demand
    document.title = 'set';
    out.afterSet = { children: Array.from(document.head.children, (el) => el.localName), title: document.title, text: document.querySelector('title').textContent };
  }
  return out;
})();
