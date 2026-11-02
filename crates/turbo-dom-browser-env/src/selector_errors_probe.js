(() => {
  // turbo-dom#189: the SyntaxError of every selector-taking method names its operation and
  // interface (Document, DocumentFragment and ShadowRoot report the ParentNode mixin's host).
  const host = document.createElement('div'), shadow = host.attachShadow({ mode: 'open' });
  document.body.append(host);
  const element = document.createElement('p'), fragment = document.createDocumentFragment();
  const run = f => { try { return ['ok', f()]; } catch (e) { return [e.name, e.message, e instanceof DOMException]; } };
  const out = {};
  for (const selector of ['!!', '', 'p,', ':modal()']) {
    out[JSON.stringify(selector)] = {
      matches: run(() => element.matches(selector)),
      closest: run(() => element.closest(selector)),
      webkitMatchesSelector: run(() => element.webkitMatchesSelector(selector)),
      elementQuery: run(() => element.querySelector(selector)),
      elementQueryAll: run(() => element.querySelectorAll(selector).length),
      documentQuery: run(() => document.querySelector(selector)),
      documentQueryAll: run(() => document.querySelectorAll(selector).length),
      fragmentQuery: run(() => fragment.querySelector(selector)),
      fragmentQueryAll: run(() => fragment.querySelectorAll(selector).length),
      shadowQuery: run(() => shadow.querySelector(selector)),
      shadowQueryAll: run(() => shadow.querySelectorAll(selector).length),
    };
  }
  host.remove();
  return out;
})()
