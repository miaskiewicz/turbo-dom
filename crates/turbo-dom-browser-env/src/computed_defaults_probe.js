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
  } else if (document.getElementById('pseudoHost')) {
    // turbo-dom#47: getComputedStyle(el, pseudoElt). ::before / ::after (also :before / :after
    // and a bare before / after) compute the pseudo-element: its own rules, inheritance from
    // the element, display inline (blockified in a flex / grid container), content none. An
    // invalid pseudo-element yields an empty declaration; a string without a leading colon
    // reads the element.
    const props = ['display', 'position', 'visibility', 'color', 'content', 'font-size', 'opacity', 'transform', 'zoom'];
    const read = (el, pseudo) => { const s = getComputedStyle(el, pseudo); return props.map((p) => s.getPropertyValue(p)).join('|'); };
    const pseudos = ['::before', ':before', '::after', ':after', '::BEFORE', ':AFTER', 'before', 'AFTER', '::\\62 efore', 'befor\\65', '::before/**/', '::backdrop', ':backdrop', '::foo', '::part(x)', '::slotted(x)', '::before::marker', '::before:hover', 'x:before', '::before ', '::', ':', ':::before', '::before()', ' ::before', '\t::before', '', 'backdrop', 'beforex', ':marker', ':placeholder', null, undefined, 5];
    const pseudo = {};
    for (const id of ['pseudoHost', 'item', 'grid', 'flex', 'hiddenHost', 'quoted']) {
      const el = document.getElementById(id);
      pseudo[id] = Object.fromEntries(pseudos.map((p) => [String(p), read(el, p)]));
    }
    pseudo.detached = read(document.createElement('div'), '::before');
    pseudo.symbol = (() => { try { getComputedStyle(document.body, Symbol('x')); return 'no throw'; } catch (e) { return e.name; } })();
    pseudo.live = (() => {
      const host = document.getElementById('pseudoHost'), style = getComputedStyle(host, '::before'), before = style.color;
      document.getElementById('pseudoSheet').textContent += '#pseudoHost::before{color:rgb(0, 255, 0)}';
      return [before, style.color, style === getComputedStyle(host, '::before')];
    })();
    out.pseudo = pseudo;
  } else if (document.getElementById('pseudoUa')) {
    // turbo-dom#47: the UA-styled pseudo-elements. Each computes Chrome's html.css values for
    // it, the author rules that select it (only the properties it accepts; ::placeholder,
    // ::file-selector-button, ::details-content and ::picker(select) only on the elements that
    // have them), and inheritance from the element (highlight pseudo-elements: from the
    // parent's highlight once a highlight rule applies). Some forms read as blank declarations
    // (::scroll-marker, invalid arguments); unknown -webkit- / -internal- ones and ::cue read
    // the element. Part `forms` reads every form, part `elements` the UA-styled ones.
    const props = ['display', 'position', 'visibility', 'color', 'content', 'font-size', 'opacity', 'transform', 'zoom'];
    const read = (el, pseudo, list) => { const s = getComputedStyle(el, pseudo); return (list || props).map((p) => s.getPropertyValue(p)).join('|'); };
    const pseudos = ['::marker', '::placeholder', '::first-line', '::first-letter', ':first-line', ':first-letter', '::selection', '::file-selector-button', '::details-content', '::checkmark', '::picker-icon', '::picker(select)', '::picker(foo)', '::picker()', '::scroll-marker', '::scroll-marker-group', '::column', '::view-transition', '::view-transition-group(x)', '::view-transition-group(*)', '::view-transition-image-pair(x)', '::view-transition-old(x)', '::view-transition-new(x)', '::view-transition-old(*)', '::view-transition-group(x y)', '::highlight(x)', '::highlight(*)', '::highlight()', '::spelling-error', '::grammar-error', '::target-text', '::search-text', '::search-text:current', '::cue', '::cue(b)', '::-webkit-scrollbar', '::-webkit-scrollbar-thumb', '::-webkit-scrollbar-track', '::-webkit-scrollbar-button', '::-webkit-scrollbar-corner', '::-webkit-scrollbar-track-piece', '::-webkit-resizer', '::-webkit-input-placeholder', '::-webkit-file-upload-button', '::-webkit-details-marker', '::-webkit-foo', '::scroll-button(up)', '::scroll-button(*)', '::scroll-button(foo)', '::MARKER', '::Placeholder', '::marker()', '::selection:window-inactive', '::column::scroll-marker', '::interest-hint', '::-internal-loading-auto-fill-button', '::-internal-foo'];
    const ua = {};
    const $ = (id) => id === 'root' ? document.documentElement : document.getElementById(id);
    const part = document.getElementById('pseudoUa').getAttribute('data-part');
    if (part === 'forms') {
      // Every form on elements of each kind: valid (computed), blank, or the element's style.
      for (const id of ['div', 'li', 'input']) ua[id] = Object.fromEntries(pseudos.map((p) => [p, read($(id), p)]));
      out.ua = ua;
      return out;
    }
    // The pseudo-elements with their own UA rules, on the elements those rules (and the
    // author rules of class `au`, which set every property) select, plus generic ones.
    const core = ['::marker', '::placeholder', '::first-line', '::first-letter', '::selection', '::file-selector-button', '::details-content', '::checkmark', '::picker-icon', '::picker(select)', '::scroll-marker-group', '::column', '::view-transition', '::view-transition-group(x)', '::view-transition-old(x)', '::highlight(x)', '::spelling-error', '::target-text', '::-webkit-scrollbar', '::-webkit-scrollbar-thumb', '::scroll-button(up)', '::scroll-button(*)'];
    const generic = ['::first-line', '::selection', '::checkmark', '::view-transition', '::scroll-marker-group'];
    const matrix = [
      [['auDiv', 'flexHost', 'hiddenHost', 'svg', 'root'], core],
      [['li', 'styledLi', 'auLi', 'insideLi', 'oli', 'summary', 'span'], ['::marker'].concat(generic)],
      [['input', 'styledInput', 'auInput', 'ta'], ['::placeholder', '::file-selector-button', '::marker', '::first-letter'].concat(generic)],
      [['file', 'auFile'], ['::file-selector-button', '::placeholder'].concat(generic)],
      [['det', 'detOpen', 'auDetails'], ['::details-content', '::marker'].concat(generic)],
      [['select', 'auSelect', 'button'], ['::picker(select)', '::picker-icon', '::checkmark', '::marker'].concat(generic)],
      [['p1', 'div'], ['::first-line', '::first-letter', '::marker'].concat(generic)],
      [['sel', 'selParent', 'selChild', 'selGrand'], ['::selection', '::highlight(x)', '::spelling-error', '::target-text']],
    ];
    for (const [list, names] of matrix) for (const id of list) ua[id] = Object.fromEntries(names.map((p) => [p, read($(id), p)]));
    // Properties the UA rules of particular pseudo-elements set, read where those rules apply.
    const extras = [
      ['::marker', ['li', 'styledLi', 'oli', 'div', 'summary', 'auLi', 'insideLi'], ['white-space', 'font-variant-numeric', 'unicode-bidi', 'text-transform']],
      ['::placeholder', ['input', 'styledInput', 'ta', 'auInput'], ['pointer-events', 'unicode-bidi']],
      ['::placeholder', ['input', 'styledInput', 'auInput'], ['white-space', 'overflow']],
      ['::file-selector-button', ['file', 'auFile'], ['background-color', 'white-space', 'padding-top', 'padding-left', 'margin-right', 'margin-left', 'border-top-width', 'border-top-style', 'text-align', 'box-sizing', 'user-select']],
      ['::details-content', ['det', 'detOpen', 'auDetails'], ['content-visibility']],
      ['::view-transition', ['div', 'auDiv'], ['top', 'right', 'bottom', 'left']],
      ['::view-transition-group(x)', ['div', 'auDiv'], ['top', 'right', 'bottom', 'left']],
      ['::view-transition-image-pair(x)', ['div', 'auDiv'], ['top', 'right', 'bottom', 'left']],
      ['::view-transition-old(x)', ['div', 'auDiv'], ['top', 'right', 'bottom', 'left']],
      ['::view-transition-new(x)', ['div', 'auDiv'], ['top', 'right', 'bottom', 'left']],
      ['::spelling-error', ['div', 'auDiv'], ['text-decoration-line']],
      ['::grammar-error', ['div', 'auDiv'], ['text-decoration-line']],
      ['::picker(select)', ['select', 'auSelect'], ['overflow', 'white-space', 'unicode-bidi']],
      ['::selection', ['sel', 'selChild', 'selGrand', 'auDiv', 'div'], ['background-color']],
      ['::backdrop', ['div'], ['background-color']],
    ];
    const extra = {};
    for (const [pseudo, list, names] of extras) for (const id of list) extra[pseudo + ' ' + id + ' ' + names.join(',')] = read(document.getElementById(id), pseudo, names);
    ua.extra = extra;
    ua.detached = Object.fromEntries(['::marker', '::placeholder', '::selection', '::scroll-marker', '::-webkit-foo'].map((p) => [p, read(document.createElement('li'), p)]));
    ua.live = (() => {
      const li = document.getElementById('li'), style = getComputedStyle(li, '::marker'), before = style.color;
      document.getElementById('uaSheet').textContent += '#li::marker{color:rgb(0, 255, 0)}';
      return [before, style.color, style === getComputedStyle(li, '::marker')];
    })();
    out.ua = ua;
  } else {
    // the title setter creates a <title> on demand
    document.title = 'set';
    out.afterSet = { children: Array.from(document.head.children, (el) => el.localName), title: document.title, text: document.querySelector('title').textContent };
  }
  return out;
})();
