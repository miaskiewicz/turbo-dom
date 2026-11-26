// Location and History are exposed separately but share one session history per window.
const histories = new WeakMap();

function canRewrite(current, next) {
  if (current.protocol !== next.protocol || current.host !== next.host ||
      current.username !== next.username || current.password !== next.password) return false;
  if (current.protocol === 'http:' || current.protocol === 'https:') return true;
  return current.pathname === next.pathname && (current.protocol === 'file:' || current.search === next.search);
}

function historyUrl(current, value) {
  if (value == null || value === '') return current;
  const text = String(value);
  let next;
  try { next = new URL(text, current); } catch { /* Invalid syntax also raises SecurityError below. */ }
  if (next && canRewrite(current, next)) return next;
  const message = 'Cannot rewrite the current document URL';
  if (typeof DOMException === 'function') throw new DOMException(message, 'SecurityError');
  throw Object.assign(new Error(message), { name: 'SecurityError' });
}

function sessionHistory(initial, hashchange, popstate) {
  const entries = [{ state: null, url: initial }];
  let index = 0;
  const current = () => entries[index].url;
  function write(state, url, replace) {
    if (!replace) entries.length = ++index;
    entries[index] = { state, url };
  }
  function notify(old, next) {
    // URL.hash is empty for both a missing fragment and a trailing '#'.
    if (old.hash !== next.hash || old.href.includes('#') !== next.href.includes('#')) hashchange(old.href, next.href);
  }
  function traverse(delta) {
    const target = index + Math.trunc(Number(delta));
    if (!(target >= 0 && target < entries.length) || target === index) return;
    const old = current();
    index = target;
    popstate(entries[index].state);
    notify(old, current());
  }
  const history = {
    get length() { return entries.length; },
    get state() { return entries[index].state; },
    pushState(state, _title, url) { write(state, historyUrl(current(), url), false); },
    replaceState(state, _title, url) { write(state, historyUrl(current(), url), true); },
    back() { traverse(-1); },
    forward() { traverse(1); },
    go(delta = 0) { traverse(delta); },
  };
  function navigate(next, replace = false) {
    const old = current();
    if (old.href === next.href) return;
    write(null, next, replace);
    notify(old, next);
  }
  return { current, history, navigate };
}

export function makeLocation(href = 'http://localhost/', hashchange = () => {}, popstate = () => {}) {
  const { current, history, navigate } = sessionHistory(new URL(href), hashchange, popstate);
  function component(name, value) {
    const next = new URL(current().href);
    next[name] = value;
    navigate(next);
  }
  function fragmentNavigation(value, replace) {
    const next = new URL(value, current());
    // A Location assignment can navigate a fragment within the document. A full
    // document load is handled by the embedding runtime.
    if (next.href.split('#')[0] === current().href.split('#')[0]) navigate(next, replace);
  }
  const location = {
    get href() { return current().href; }, set href(value) { fragmentNavigation(value, false); },
    get protocol() { return current().protocol; },
    get host() { return current().host; },
    get hostname() { return current().hostname; },
    get port() { return current().port; },
    get pathname() { return current().pathname; }, set pathname(value) { component('pathname', value); },
    get search() { return current().search; }, set search(value) { component('search', value); },
    get hash() { return current().hash; }, set hash(value) { component('hash', value); },
    get origin() { return current().origin; },
    assign(value) { fragmentNavigation(value, false); },
    replace(value) { fragmentNavigation(value, true); },
    reload() {},
    toString() { return current().href; },
  };
  histories.set(location, history);
  return location;
}

export function makeHistory(location) {
  return histories.get(location);
}
