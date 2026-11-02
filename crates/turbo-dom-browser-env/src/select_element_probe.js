(() => {
  'use strict';
  const out = {}, P = HTMLSelectElement.prototype, svg = 'http://www.w3.org/2000/svg';
  const run = f => { try { return ['ok', f()]; } catch (error) { return ['throw', error.name]; } };
  const make = (tag, label, attrs) => {
    const element = document.createElement(tag);
    if (label) element.append(label);
    for (const [name, value] of Object.entries(attrs || {})) element.setAttribute(name, value);
    return element;
  };
  const opt = (label, attrs) => make('option', label, attrs);
  const own = node => { const first = node.firstChild; return first && first.nodeType === 3 ? first.data : ''; };
  const label = node => node ? (node.localName === 'option' ? own(node) : node.localName) : node;
  const tree = node => Array.from(node.children, child => child.localName === 'option' && !child.children.length ? own(child) : [child.localName, own(child), ...tree(child)]);
  const listed = select => Array.from({ length: select.length }, (_, index) => label(select.item(index)));
  const consistent = select => {
    const options = select.options;
    if (select.length !== options.length) return 'length ' + select.length + '/' + options.length;
    for (let index = 0; index < select.length; index++) if (select.item(index) !== options[index] || select[index] !== options[index]) return 'item ' + index;
    return true;
  };
  const state = select => [listed(select), tree(select), select.selectedIndex, select.value, consistent(select)];
  const shape = key => {
    const d = Object.getOwnPropertyDescriptor(P, key);
    if (!d) return null;
    const row = { enumerable: d.enumerable, configurable: d.configurable };
    if ('value' in d) Object.assign(row, { writable: d.writable, name: d.value.name, length: d.value.length });
    if (d.get) Object.assign(row, { get: d.get.name, getLength: d.get.length });
    if ('set' in d) row.set = d.set ? [d.set.name, d.set.length] : null;
    return row;
  };
  const keys = ['add', 'item', 'length', 'namedItem', 'remove', 'size', 'type', Symbol.iterator];
  out.shape = Object.fromEntries(keys.map(key => [String(key), shape(key)]));
  out.iterator = { arrayValues: P[Symbol.iterator] === Array.prototype.values, ownValues: Object.hasOwn(P, 'values') };

  // Receiver brands: every member rejects non-select receivers, including look-alike tags.
  const receivers = [{}, Object.create(P), make('div'), make('option'), document.createElementNS(svg, 'select'), document];
  const calls = {
    item: r => P.item.call(r, 0), namedItem: r => P.namedItem.call(r, 'a'), add: r => P.add.call(r, opt('x')),
    remove: r => P.remove.call(r, 0), removeSelf: r => P.remove.call(r),
    getLength: r => Object.getOwnPropertyDescriptor(P, 'length').get.call(r), setLength: r => Object.getOwnPropertyDescriptor(P, 'length').set.call(r, 1),
    getSize: r => Object.getOwnPropertyDescriptor(P, 'size').get.call(r), setSize: r => Object.getOwnPropertyDescriptor(P, 'size').set.call(r, 1),
    getType: r => Object.getOwnPropertyDescriptor(P, 'type').get.call(r),
  };
  out.brands = Object.fromEntries(Object.entries(calls).map(([name, call]) => [name, receivers.map(receiver => { const result = run(() => call(receiver)); return result[0] === 'throw' ? result[1] : 'accepted'; })]));
  out.iteratorGeneric = Array.from(P[Symbol.iterator].call({ length: 2, 0: 'x', 1: 'y' }));

  // The list of options: nearest-ancestor-select rules for nested containers.
  const s = make('select'); document.body.append(s);
  const g1 = make('optgroup', 'g1'), g2 = make('optgroup', 'g2'), div = make('div', 'div'), hr = make('hr'), outer = opt('outer'), inner = make('select'), list = make('datalist');
  g2.append(opt('nested')); g1.append(opt('B', { name: 'b' }), g2); div.append(opt('indiv')); hr.append(opt('inhr'));
  outer.append(opt('inopt')); inner.append(opt('insel')); list.append(opt('indl'));
  s.append(opt('A', { id: 'a' }), g1, div, hr, outer, inner, list, opt('D', { id: 'b' }));
  out.list = { state: state(s), iterated: Array.from(s, label), spread: [...s].length, values: Array.from(Array.prototype.values.call(s), label) };
  out.item = [0, -1, 4294967296, 4294967297, '1', NaN, 1.9, 99, null, undefined, '-0'].map(index => run(() => label(s.item(index))));
  out.itemErrors = [() => s.item(), () => s.item(Symbol()), () => s.item(1n)].map(call => run(call));
  out.namedItem = ['a', 'b', '', 'zzz', 'nested', 'inopt', 'insel'].map(name => run(() => label(s.namedItem(name))));
  out.namedItemCoerce = [run(() => label(s.namedItem(null))), run(() => label(s.namedItem(undefined)))];
  out.namedItemErrors = [() => s.namedItem(), () => s.namedItem(Symbol())].map(call => run(call));

  // Indexed property exotic behaviour on the select element itself.
  const descriptor = Object.getOwnPropertyDescriptor(s, '0');
  out.indexed = {
    has: ['0' in s, 3 in s, 4 in s, 5 in s, 99 in s, s.hasOwnProperty('1'), s.hasOwnProperty('4'), s.hasOwnProperty('5')],
    read: [label(s[0]), label(s[3]), label(s[4]), s[5], s['00'], s[-1]],
    descriptor: { writable: descriptor.writable, enumerable: descriptor.enumerable, configurable: descriptor.configurable, value: label(descriptor.value) },
    missing: Object.getOwnPropertyDescriptor(s, '5') === undefined,
    keys: Object.keys(s).filter(key => /^\d+$/.test(key)),
    ownKeys: Reflect.ownKeys(s).filter(key => typeof key === 'string' && /^\d+$/.test(key)),
    propertyIsEnumerable: s.propertyIsEnumerable('0'),
  };
  const t = make('select'); document.body.append(t); t.append(opt('one'), opt('two'));
  const sets = [];
  const record = (name, call) => { const result = run(call); sets.push([name, result[0] === 'throw' ? result[1] : 'ok', state(t)]); };
  record('replace', () => { t[1] = opt('replaced'); });
  record('append-at-length', () => { t[2] = opt('appended'); });
  record('pad', () => { t[5] = opt('padded'); });
  record('null-removes', () => { t[0] = null; });
  record('undefined-removes', () => { t[0] = undefined; });
  record('null-out-of-range', () => { t[40] = null; });
  record('string', () => { t[0] = 'x'; });
  record('div', () => { t[0] = make('div'); });
  record('optgroup', () => { t[0] = make('optgroup'); });
  record('delete-supported', () => delete t[0]);
  record('delete-unsupported', () => { if (!delete t[99]) throw new Error('kept'); });
  record('reflect-delete', () => Reflect.deleteProperty(t, '0') ? 'deleted' : 'kept');
  record('define-value', () => { Object.defineProperty(t, '0', { value: opt('defined') }); });
  record('define-accessor', () => { Object.defineProperty(t, '0', { get() { return 1; } }); });
  record('reflect-define-accessor', () => { if (Reflect.defineProperty(t, '1', { get() { return 1; } })) throw new Error('accepted'); });
  record('reflect-set', () => { if (!Reflect.set(t, '1', opt('reflected'))) throw new Error('rejected'); });
  record('move-from-self', () => { t[0] = t[1]; });
  record('existing-other-parent', () => { const holder = make('div'), moved = opt('moved'); holder.append(moved); t[0] = moved; if (holder.children.length) throw new Error('not moved'); });
  out.indexedSet = sets;
  const sloppy = Function('s', 'o', 's[0] = "bad"; return "no-throw";');
  out.sloppySetString = run(() => sloppy(t, null));
  const nested = make('select'), group = make('optgroup'); group.append(opt('g-a'), opt('g-b')); nested.append(opt('top'), group);
  nested[2] = opt('g-replaced'); out.replaceInGroup = tree(nested);
  const grouped = make('select'), group2 = make('optgroup'); group2.append(opt('h-a'), opt('h-b')); grouped.append(opt('top'), group2);
  out.replaceBeforeGroup = [run(() => { grouped[1] = opt('h-replaced'); }), tree(grouped)];

  // length setter appends blank options to the select and truncates from the end.
  const l = make('select'); document.body.append(l);
  const lg = make('optgroup'); lg.append(opt('l-g1'), opt('l-g2')); l.append(opt('l1'), lg);
  const lengths = [];
  for (const value of [5, 2, 3.9, '4', 0, 'abc', 1, -1, 100001, null]) { const result = run(() => { l.length = value; }); lengths.push([String(value), result[0] === 'throw' ? result[1] : 'ok', l.length, tree(l)]); }
  out.lengthSetter = lengths;
  out.lengthSetterErrors = [run(() => { l.length = Symbol(); }), run(() => { l.length = 1n; })];
  // Growth past 100000 options is refused without an exception (the boundary itself is not built
  // here: materialising 100000 options is too slow for the offline debug binding).
  const big = make('select'); big.append(opt('big'));
  out.lengthLimit = [run(() => { big.length = 100001; return big.length; }), run(() => { big[100001] = opt('far'); return big.length; }), run(() => { big[4294967294] = opt('farther'); return big.length; }), run(() => { big.length = 4294967295; return big.length; }), listed(big)];

  // add(): union before argument, HierarchyRequestError/NotFoundError order, optgroups.
  const a = make('select'); document.body.append(a);
  const ag = make('optgroup'), agOption = opt('in-group'); ag.append(agOption);
  const aDiv = make('div'), aDivOption = opt('in-div'); aDiv.append(aDivOption);
  a.append(opt('first'), ag, aDiv);
  const adds = [];
  const add = (name, call) => { const result = run(call); adds.push([name, result[0] === 'throw' ? result[1] : result[1] === undefined ? 'ok' : 'returned', listed(a), tree(a)]); };
  add('append', () => a.add(opt('appended')));
  add('null', () => a.add(opt('null-before'), null));
  add('undefined', () => a.add(opt('undefined-before'), undefined));
  add('index-0', () => a.add(opt('index-0'), 0));
  add('index-group', () => a.add(opt('index-group'), 2));
  add('index-negative', () => a.add(opt('negative'), -1));
  add('index-large', () => a.add(opt('large'), 99));
  add('index-string', () => a.add(opt('string-before'), 'x'));
  add('index-numeric-string', () => a.add(opt('numeric-string'), '1'));
  add('index-object', () => a.add(opt('object-before'), {}));
  add('before-option-in-group', () => a.add(opt('before-group-option'), agOption));
  add('before-div', () => a.add(opt('before-div'), aDiv));
  add('before-option-in-div', () => a.add(opt('before-div-option'), aDivOption));
  add('optgroup', () => a.add(make('optgroup', 'added-group')));
  add('same-element', () => a.add(agOption, agOption));
  add('move-existing', () => a.add(a.item(0)));
  add('foreign-before', () => a.add(opt('foreign'), opt('detached')));
  add('foreign-before-same', () => { const o = opt('same-detached'); a.add(o, o); });
  add('before-select-itself', () => a.add(opt('self-before'), a));
  add('div', () => a.add(make('div')));
  add('string', () => a.add('x'));
  add('null-element', () => a.add(null));
  add('missing', () => a.add());
  add('svg-option', () => a.add(document.createElementNS(svg, 'option')));
  add('bigint-before', () => a.add(opt('bigint'), 1n));
  const host = make('optgroup'), child = make('select'); host.append(child);
  add('ancestor', () => child.add(host));
  const optionHost = opt('host-option'), optionChild = make('select'); optionHost.append(optionChild);
  add('ancestor-option', () => optionChild.add(optionHost));
  add('ancestor-before-foreign', () => child.add(host, opt('detached')));
  out.add = adds;
  out.addFromOther = (() => { const from = make('select'), to = make('select'), moved = opt('moved'); from.append(moved, opt('left')); to.add(moved); return [listed(from), listed(to)]; })();

  // remove(): no arguments is ChildNode.remove, any argument selects remove(long).
  const r = make('select'), parent = make('div'); parent.append(r); document.body.append(parent);
  const rg = make('optgroup'); rg.append(opt('r-g')); r.append(opt('r0'), rg, opt('r2'), opt('r3'), opt('r4'));
  const removes = [];
  for (const [name, call] of [
    ['negative', () => r.remove(-1)], ['large', () => r.remove(99)], ['wrapped', () => r.remove(4294967297)],
    ['string', () => r.remove('3')], ['group', () => r.remove(1)], ['nan', () => r.remove(NaN)],
    ['undefined', () => r.remove(undefined)], ['null', () => r.remove(null)], ['float', () => r.remove(0.9)],
    ['symbol', () => r.remove(Symbol())], ['bigint', () => r.remove(1n)],
  ]) { const result = run(call); removes.push([name, result[0] === 'throw' ? result[1] : result[1] === undefined ? 'ok' : 'returned', listed(r)]); }
  out.remove = removes;
  out.removeSelf = [run(() => r.remove()), r.parentNode === null, listed(r), parent.children.length, run(() => r.remove()), r.parentNode];
  out.removeBorrow = run(() => { const holder = make('div'), victim = make('select'); holder.append(victim); Element.prototype.remove.call(victim); return holder.children.length; });

  // size reflects an unsigned long with default 0; type reports the multiple state.
  const z = make('select');
  out.sizeRead = [null, '4', 'abc', '-3', ' 7', '7px', '2147483647', '2147483648', '0', ''].map(value => { if (value === null) z.removeAttribute('size'); else z.setAttribute('size', value); return z.size; });
  out.sizeWrite = [5, 0, -1, 2147483647, 2147483648, 4294967295, 4294967296, 'x', 3.7, null, undefined, true].map(value => { const result = run(() => { z.size = value; }); return [String(value), result[0] === 'throw' ? result[1] : z.getAttribute('size'), z.size]; });
  out.sizeErrors = [run(() => { z.size = Symbol(); }), run(() => { z.size = 1n; })];
  const m = make('select'); m.setAttribute('multiple', ''); out.sizeMultiple = m.size;
  const ty = make('select'); out.type = [ty.type];
  ty.multiple = true; out.type.push(ty.type); ty.removeAttribute('multiple'); out.type.push(ty.type);
  ty.setAttribute('multiple', 'false'); out.type.push(ty.type);
  out.typeAssign = [run(() => { ty.type = 'x'; }), ty.type, Function('s', 's.type = "x"; return s.type;')(ty)];

  // Selectedness after collection mutations.
  const sel = make('select'); document.body.append(sel);
  const steps = [];
  const step = (name, call) => { call(); steps.push([name, sel.selectedIndex, sel.value, listed(sel), consistent(sel)]); };
  step('empty', () => {});
  step('add-first', () => sel.add(opt('s1')));
  step('add-second', () => sel.add(opt('s2')));
  step('add-selected-attribute', () => sel.add(opt('s3', { selected: '' }), 0));
  step('remove-selected', () => sel.remove(0));
  step('select-last', () => { sel.selectedIndex = 1; });
  step('add-before-selected', () => sel.add(opt('s4'), 0));
  step('index-replace-selected', () => { sel[2] = opt('s5'); });
  step('length-grow', () => { sel.length = 5; });
  step('select-blank', () => { sel.selectedIndex = 4; });
  step('length-truncate-selected', () => { sel.length = 2; });
  step('add-disabled-first', () => sel.add(opt('s6', { disabled: '' }), 0));
  step('length-zero', () => { sel.length = 0; });
  step('add-after-empty', () => sel.add(opt('s7')));
  step('add-second-after-empty', () => sel.add(opt('s8')));
  step('move-selected-to-end', () => sel.add(sel.item(0)));
  step('replace-selected-by-next', () => { sel[0] = sel.item(1); });
  step('add-two', () => { sel.add(opt('s9')); sel.add(opt('s10')); });
  step('replace-first-by-selected-later', () => { sel.selectedIndex = 2; sel[0] = sel.item(2); });
  step('deselect', () => { sel.selectedIndex = -1; });
  step('indexed-after-deselect', () => { sel[0] = opt('s11'); });
  step('deselect-again', () => { sel.selectedIndex = -1; });
  step('remove-after-deselect', () => sel.remove(0));
  step('deselect-third', () => { sel.selectedIndex = -1; });
  step('add-after-deselect', () => sel.add(opt('s12')));
  step('deselect-fourth', () => { sel.selectedIndex = -1; });
  step('length-after-deselect', () => { sel.length = 4; });
  step('add-selected-group', () => { const group = make('optgroup'); group.append(opt('s13'), opt('s14', { selected: '' })); sel.add(group, 1); });
  step('remove-group-selected', () => sel.remove(2));
  out.selectedness = steps;
  const multi = make('select'); multi.multiple = true; multi.add(opt('m1')); multi.add(opt('m2', { selected: '' })); multi.add(opt('m3', { selected: '' }));
  out.multiple = [multi.selectedIndex, multi.selectedOptions.length, multi.type, multi.length];
  multi.remove(1); out.multiple.push(multi.selectedIndex, multi.selectedOptions.length);
  return out;
})()
