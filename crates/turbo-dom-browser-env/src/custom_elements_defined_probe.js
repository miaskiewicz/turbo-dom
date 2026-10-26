(() => {
  'use strict';
  // turbo-dom#24 item 3: `:defined` follows the element's custom element state (HTML
  // "defined": uncustomized or custom). Elements parsed or created before
  // customElements.define() are undefined; upgrade (connected elements on define, on
  // insertion, customElements.upgrade(), clones) makes them custom, a constructor that throws
  // leaves them failed (never defined). Seen by matches(), querySelectorAll() and
  // getComputedStyle() (`:defined` -> order 1, `:not(:defined)` -> order 2 from the case sheet).
  const out = {};
  const view = (el) => {
    const row = [el.matches(':defined'), el.matches(':not(:defined)')];
    if (el.isConnected) row.push(getComputedStyle(el).order);
    return row;
  };
  const ids = (selector) => Array.from(document.querySelectorAll(selector), (el) => el.id || el.localName);
  const byId = (id) => document.getElementById(id);
  const parsed = ['a', 'b', 'btn', 'btn2', 'f', 'plain', 'sx', 'up', 'later'];
  const snapshot = (extra) => {
    const row = {};
    for (const id of parsed) row[id] = view(byId(id));
    for (const [label, el] of Object.entries(extra)) row[label] = view(el);
    row.qsaDefined = ids('body :defined');
    row.qsaUndefined = ids('body :not(:defined)');
    return row;
  };
  const created = {
    autonomous: document.createElement('x-a'),
    customized: document.createElement('button', { is: 'x-btn' }),
    isAttribute: (() => { const b = document.createElement('button'); b.setAttribute('is', 'x-btn'); return b; })(),
    unknown: document.createElement('x-unknown'),
    upgradeLater: document.createElement('x-a'),
  };
  out.createdIsAttribute = created.customized.getAttribute('is');
  out.before = snapshot(created);
  const during = [];
  customElements.define('x-a', class extends HTMLElement {
    constructor() { super(); during.push([this.id || 'new', this.matches(':defined')]); }
  });
  customElements.define('x-btn', class extends HTMLButtonElement {}, { extends: 'button' });
  customElements.define('x-fail', class extends HTMLElement {
    constructor() { super(); throw new Error('nope'); }
  });
  out.during = during;
  out.afterDefine = snapshot(created);
  // insertion upgrades; customElements.upgrade() upgrades a detached tree
  document.body.append(created.autonomous, created.customized);
  customElements.upgrade(created.upgradeLater);
  out.afterInsert = snapshot(created);
  const fresh = {
    createdAfter: document.createElement('x-a'),
    constructed: new (customElements.get('x-a'))(),
    customizedAfter: document.createElement('button', { is: 'x-btn' }),
    failedAfter: (() => { try { return document.createElement('x-fail'); } catch (e) { return e.name; } })(),
    clone: byId('a').cloneNode(),
    templateContent: byId('t').content.firstElementChild,
    innerHTML: (() => { const d = document.createElement('div'); d.innerHTML = '<x-a></x-a><button is="x-btn"></button>'; return d.firstChild; })(),
    innerHTMLButton: (() => { const d = document.createElement('div'); d.innerHTML = '<x-a></x-a><button is="x-btn"></button>'; return d.lastChild; })(),
  };
  out.failedAfterKind = typeof fresh.failedAfter === 'string' ? fresh.failedAfter : Object.prototype.toString.call(fresh.failedAfter);
  if (typeof fresh.failedAfter === 'string') delete fresh.failedAfter;
  out.fresh = Object.fromEntries(Object.entries(fresh).map(([label, el]) => [label, view(el)]));
  // a definition made after parsing upgrades the element in place (and getComputedStyle sees it)
  customElements.define('x-later', class extends HTMLElement {});
  out.later = view(byId('later'));
  out.during = during;
  return out;
})();
