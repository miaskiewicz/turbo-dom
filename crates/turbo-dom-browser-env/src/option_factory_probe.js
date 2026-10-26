(() => {
  'use strict';
  // turbo-dom#20. `factory`: the HTMLOptionElement legacy factory function
  // Option(text, value, defaultSelected, selected) (HTML 4.10.10, Web IDL
  // [LegacyFactoryFunction]). `collection`: HTMLOptionsCollection's indexed definer
  // under Object.defineProperty / Reflect.defineProperty / Object.defineProperties.
  const out = {};
  const error = (f) => { try { return ['ok', f()]; } catch (e) { return ['throw', e.name, e.message]; } };
  const shape = (object, key) => {
    const d = Object.getOwnPropertyDescriptor(object, key);
    if (!d) return null;
    const row = { enumerable: d.enumerable, configurable: d.configurable };
    if ('value' in d) row.writable = d.writable;
    if ('get' in d) row.get = typeof d.get;
    if ('set' in d) row.set = typeof d.set;
    return row;
  };
  const option = (o) => ({
    proto: Object.getPrototypeOf(o) === HTMLOptionElement.prototype,
    tag: o.localName, ns: o.namespaceURI, owner: o.ownerDocument === document, parent: o.parentNode,
    text: o.text, value: o.value, valueAttr: o.getAttribute('value'), hasValue: o.hasAttribute('value'),
    selectedAttr: o.getAttribute('selected'), defaultSelected: o.defaultSelected, selected: o.selected,
    children: Array.from(o.childNodes, (n) => [n.nodeType, n.nodeValue]), own: Object.getOwnPropertyNames(o),
    index: o.index, form: o.form, label: o.label, html: o.outerHTML,
  });
  const f = {};
  f.type = typeof Option;
  f.name = Option.name;
  f.length = Option.length;
  f.ownKeys = Reflect.ownKeys(Option).map(String);
  f.nameShape = shape(Option, 'name');
  f.lengthShape = shape(Option, 'length');
  f.prototypeShape = shape(Option, 'prototype');
  f.globalShape = shape(globalThis, 'Option');
  f.sameProto = Option.prototype === HTMLOptionElement.prototype;
  f.protoConstructor = Option.prototype.constructor === HTMLOptionElement;
  f.notInterface = Option !== HTMLOptionElement;
  f.functionProto = Object.getPrototypeOf(Option) === Function.prototype;
  f.source = Function.prototype.toString.call(Option);
  f.call = error(() => Option());
  f.callArgs = error(() => Option('a', 'b'));
  f.instance = new Option() instanceof HTMLOptionElement;
  f.distinct = new Option() !== new Option();
  f.empty = option(new Option());
  f.text = option(new Option('  a  b  '));
  f.emptyText = option(new Option(''));
  f.textValue = option(new Option('t', 'v'));
  f.emptyValue = option(new Option('t', ''));
  f.undefinedValue = option(new Option('t', undefined));
  f.nullArgs = option(new Option(null, null));
  f.numberArgs = option(new Option(1, 2));
  f.defaultSelected = option(new Option('t', 'v', true));
  f.selectedOnly = option(new Option('t', 'v', false, true));
  f.both = option(new Option('t', 'v', true, true));
  f.truthy = option(new Option('t', 'v', 'yes', 1));
  f.falsy = option(new Option('t', 'v', 0, ''));
  f.symbolText = error(() => new Option(Symbol('s')));
  f.symbolValue = error(() => new Option('t', Symbol('s')));
  f.objectText = option(new Option({ toString() { return 'obj'; } }));
  f.markup = option(new Option('<b>x</b>', '"&'));
  {
    const select = document.createElement('select');
    select.add(new Option('a', '1'));
    select.add(new Option('b', '2', false, true));
    select.add(new Option('c', '3', true));
    f.inSelect = [select.selectedIndex, select.value, Array.from(select.options, (o) => [o.text, o.selected, o.defaultSelected])];
    const multi = document.createElement('select');
    multi.multiple = true;
    multi.append(new Option('a', '1', false, true), new Option('b', '2', false, true), new Option('c', '3'));
    f.inMultiple = Array.from(multi.selectedOptions, (o) => o.value);
  }
  {
    class Sub extends Option {}
    const made = error(() => {
      const o = new Sub('s');
      return [Object.getPrototypeOf(o) === Sub.prototype, Object.getPrototypeOf(o) === HTMLOptionElement.prototype, o instanceof Sub, o.text];
    });
    f.subclass = made;
    const other = function () {};
    other.prototype = Object.create(HTMLOptionElement.prototype);
    f.reflectNewTarget = error(() => {
      const o = Reflect.construct(Option, ['r'], other);
      return [Object.getPrototypeOf(o) === other.prototype, Object.getPrototypeOf(o) === HTMLOptionElement.prototype, o.text];
    });
  }
  out.factory = f;

  const c = {};
  const fresh = () => {
    const select = document.createElement('select');
    select.append(new Option('a'), new Option('b'));
    return select;
  };
  const state = (select) => [select.length, Array.from(select.options, (o) => o.text), Object.getOwnPropertyNames(select.options)];
  const define = (name, run) => {
    const select = fresh();
    const result = error(() => run(select.options));
    c[name] = [result, state(select)];
  };
  const getter = { get() { return 'g'; } };
  const setter = { set(v) {} };
  const both = { get() { return 'g'; }, set(v) {}, configurable: true, enumerable: true };
  define('reflectAccessorSupported', (o) => Reflect.defineProperty(o, '0', getter));
  define('reflectAccessorUnsupported', (o) => Reflect.defineProperty(o, '5', getter));
  define('reflectSetterOnly', (o) => Reflect.defineProperty(o, '1', setter));
  define('reflectAccessorBoth', (o) => Reflect.defineProperty(o, '0', both));
  define('objectAccessorSupported', (o) => { Object.defineProperty(o, '0', getter); return 'returned'; });
  define('objectAccessorUnsupported', (o) => { Object.defineProperty(o, '5', getter); return 'returned'; });
  define('objectDefineProperties', (o) => { Object.defineProperties(o, { 0: getter }); return 'returned'; });
  define('reflectDataOption', (o) => Reflect.defineProperty(o, '0', { value: new Option('z') }));
  define('reflectDataNull', (o) => Reflect.defineProperty(o, '0', { value: null }));
  define('reflectDataWrongType', (o) => Reflect.defineProperty(o, '0', { value: 'x' }));
  define('reflectNamedAccessor', (o) => Reflect.defineProperty(o, 'named', getter));
  define('afterQuietAccessorRead', (o) => { Reflect.defineProperty(o, '0', getter); return [o[0] && o[0].text, o.item(0).text, Object.getOwnPropertyDescriptor(o, '0').get === undefined]; });
  out.collection = c;
  return out;
})();
