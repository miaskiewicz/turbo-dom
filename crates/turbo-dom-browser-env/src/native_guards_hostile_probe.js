(async () => {
  // Hostile and boundary arguments of the native members fixed by turbo-dom#114: argument
  // counts, Node / Attr parameter types, attribute name validation, Element's ChildNode brand,
  // the promise-returning scroll operations and their dictionary conversions, doctype textContent.
  const name = run => { try { const v = run(); return v === undefined ? 'undefined' : v; } catch (e) { return [e.constructor.name, e.name]; } };
  const settle = p => p.then(v => ['fulfilled', v === undefined ? 'undefined' : v], e => ['rejected', e.constructor.name, e.name]);
  const p = document.body.appendChild(document.createElement('p')), other = document.implementation.createHTMLDocument('');
  const proxy = new Proxy(p, {}), revocable = Proxy.revocable({}, {}); revocable.revoke();
  const bad = { toString() { throw new RangeError('name'); } };
  const s = Symbol(), log = [];
  const spy = { get behavior() { log.push('behavior'); return 'auto'; }, get block() { log.push('block'); return 'start'; }, get container() { log.push('container'); return 'all'; }, get inline() { log.push('inline'); return 'nearest'; }, get left() { log.push('left'); return 1; }, get top() { log.push('top'); return 2; } };
  const scrollCases = [
    () => p.scrollIntoView(), () => p.scrollIntoView(false), () => p.scrollIntoView(null), () => p.scrollIntoView(5), () => p.scrollIntoView('x'), () => p.scrollIntoView(s),
    () => p.scrollIntoView({ inline: 'bogus' }), () => p.scrollIntoView({ behavior: 'bogus' }), () => p.scrollIntoView({ container: 'bogus' }), () => p.scrollIntoView({ container: 'nearest', behavior: 'smooth', block: 'center', inline: 'end' }), () => p.scrollIntoView({ block: s }),
    () => p.scrollTo(), () => p.scrollTo(5), () => p.scrollTo('5'), () => p.scrollTo(null), () => p.scrollTo(1, s), () => p.scrollTo(s, 1), () => p.scrollTo({ left: s }), () => p.scrollTo({ behavior: 'bogus' }), () => p.scrollTo({ behavior: 'instant', left: NaN, top: Infinity }), () => p.scrollTo(1, 2, 3),
    () => p.scrollBy(), () => p.scrollBy(5), () => p.scrollBy({ top: { valueOf() { throw new RangeError('v'); } } }), () => p.scrollBy(1, 2),
    () => document.createElement('i').scrollIntoView(),
    () => { log.length = 0; return p.scrollIntoView(spy).then(() => log.slice()); }, () => { log.length = 0; return p.scrollTo(spy).then(() => log.slice()); },
  ];
  const scrolls = [];
  for (const c of scrollCases) scrolls.push(await settle((() => { try { return c(); } catch (e) { return Promise.resolve(['threw', e.name]); } })()));
  return {
    scrolls,
    contains: [name(() => document.contains(other.body)), name(() => p.contains(undefined)), name(() => p.contains(proxy)), name(() => p.contains(revocable.proxy)), name(() => p.contains(5)), name(() => document.contains(p)), name(() => p.contains(document.createAttribute('a')))],
    removeAttributeNode: [name(() => p.removeAttributeNode(proxy)), name(() => p.removeAttributeNode(other.createElement('i'))), name(() => p.removeAttributeNode(document)), name(() => p.removeAttributeNode(other.createAttribute('a')))],
    names: [name(() => p.setAttribute(bad, 'v')), name(() => p.setAttribute('a=b', 'v')), name(() => p.setAttribute('a>', 'v')), name(() => p.setAttribute('a/', 'v')), name(() => p.setAttribute('\u0000', 'v')), name(() => p.setAttribute('é$1', 'v')), name(() => p.toggleAttribute('', true)), name(() => p.toggleAttribute('a\tb', false)), name(() => p.toggleAttribute('x-y', true)), p.getAttribute('é$1'), p.hasAttribute('x-y'), name(() => document.createAttribute('a=b'))],
    arity: [name(() => p.setAttribute()), name(() => p.toggleAttribute(undefined)), p.getAttribute('undefined'), name(() => document.createComment(undefined).data), name(() => p.matches(undefined)), name(() => document.createTextNode('ab').splitText(undefined).data)],
    childNode: [name(() => Element.prototype.before.call(document.createTextNode('t'), 'x')), name(() => CharacterData.prototype.before.call(p, 'x')), Element.prototype.before === CharacterData.prototype.before, Element.prototype.remove.length, Element.prototype.before.length, Element.prototype.before.name, name(() => { const t = document.createElement('i').appendChild(document.createTextNode('t')); t.after('u'); return t.parentNode.textContent; })],
    scroll: [await settle(p.scrollIntoView({ get block() { throw new RangeError('block'); } })), await settle(p.scrollIntoView({ block: 'bogus' })), await settle(p.scrollTo(1, 2)), await settle(Element.prototype.scrollBy.call(proxy)), await settle(Element.prototype.scrollTo.call(document.createTextNode('t'))), p.scrollIntoView() instanceof Promise, Element.prototype.scrollIntoView.length, Element.prototype.scrollTo.name],
    textContent: [document.implementation.createDocumentType('a', '', '').textContent, document.textContent, (() => { const d = document.implementation.createDocumentType('a', '', ''); d.textContent = 'x'; return d.textContent; })()],
  };
})()
