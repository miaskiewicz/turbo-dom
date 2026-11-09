// <meter>/<progress> numeric reflection and <output> default-value state, matching Chrome:
// attributes parse with the HTML floating-point rules, getters clamp, setters reject non-finite
// doubles. Output keeps Chrome's value-mode flag and default value, which form reset restores.
(function () {
  'use strict';
  const g = globalThis, ns = __tsNs, make = ns.makeNativeFunction;
  const apply = Reflect.apply, define = Object.defineProperty, StringValue = String, NumberValue = Number, finite = Number.isFinite;
  const ErrorType = TypeError, max = Math.max, min = Math.min, sliceText = String.prototype.slice;
  const getAttribute = g.Element.prototype.getAttribute, setAttribute = g.Element.prototype.setAttribute, hasAttribute = g.Element.prototype.hasAttribute;
  const textGet = ns.domAccessor('textContent', false, 'Node'), textSet = ns.domAccessor('textContent', true, 'Node');
  const localName = ns.domAccessor('localName', false, 'Element'), namespace = ns.domAccessor('namespaceURI', false, 'Element'), nodeType = ns.domAccessor('nodeType', false, 'Node');
  const states = new WeakMap(), stateGet = WeakMap.prototype.get, stateSet = WeakMap.prototype.set;
  const charCode = Function.prototype.call.bind(String.prototype.charCodeAt);
  const attr = (element, name) => apply(getAttribute, element, [name]);
  const write = (element, name, value) => apply(setAttribute, element, [name, value]);
  const string = value => { if (typeof value === 'symbol') throw new ErrorType('Cannot convert a Symbol to a string'); return StringValue(value); };
  // WebIDL `double`: ToNumber (Symbol/BigInt throw) after the receiver check, then reject NaN/±Infinity.
  const double = (value, name, owner) => { const number = +value; if (!finite(number)) throw new ErrorType("Failed to set the '" + name + "' property on '" + owner + "': The provided double value is non-finite."); return number; };
  const digit = code => code >= 48 && code <= 57;
  // HTML "rules for parsing floating-point number values": leading ASCII whitespace, one optional
  // sign, trailing junk ignored; the collected prefix is rounded by Number(). Error -> fallback.
  // `at` reads a code unit (-1 past the end); each helper returns the index after its run.
  const skipWhitespace = (at, index) => { while (at(index) === 9 || at(index) === 10 || at(index) === 12 || at(index) === 13 || at(index) === 32) index++; return index; };
  const digitsEnd = (at, index) => { while (digit(at(index))) index++; return index; };
  // The `e`/`E` exponent at index ('' when absent or without digits).
  function exponentPart(value, at, index) {
    let exponent = '';
    if (at(index) === 101 || at(index) === 69) {
      let cursor = index + 1, exponentSign = '';
      if (at(cursor) === 45 || at(cursor) === 43) { exponentSign = at(cursor) === 45 ? '-' : ''; cursor++; }
      if (digit(at(cursor))) { const start = cursor; cursor = digitsEnd(at, cursor); exponent = 'e' + exponentSign + apply(sliceText, value, [start, cursor]); }
    }
    return exponent;
  }
  // Parses the integer digits (or a leading-dot float with implicit 0 integer) at `index`;
  // returns `{index, integer}` or `null` if there are no digits to start a number.
  function parseInteger(value, at, index) {
    if (at(index) === 46 && digit(at(index + 1))) return { index, integer: '0' };
    if (!digit(at(index))) return null;
    const start = index;
    index = digitsEnd(at, index);
    return { index, integer: apply(sliceText, value, [start, index]) };
  }
  function parseFraction(value, at, index) {
    if (at(index) !== 46 || (!digit(at(index + 1)) && at(index + 1) !== 101 && at(index + 1) !== 69)) return { index, fraction: '' };
    index++;
    const start = index;
    index = digitsEnd(at, index);
    return { index, fraction: apply(sliceText, value, [start, index]) };
  }
  function parse(value, fallback) {
    if (value === null) return fallback;
    let index = 0, sign = '';
    const length = value.length, at = position => position < length ? charCode(value, position) : -1;
    index = skipWhitespace(at, index);
    if (at(index) === 45) { sign = '-'; index++; } else if (at(index) === 43) index++;
    const integerResult = parseInteger(value, at, index);
    if (integerResult === null) return fallback;
    const { index: afterInteger, integer } = integerResult;
    const { index: afterFraction, fraction } = parseFraction(value, at, afterInteger);
    const exponent = exponentPart(value, at, afterFraction);
    const number = NumberValue(sign + integer + (fraction ? '.' + fraction : '') + exponent);
    if (!finite(number)) return fallback;
    return number === 0 ? 0 : number;
  }
  const brand = name => ns.domAccessor('nodeType', false, 'HTML:' + name);
  function accessors(proto, owner, check, table) {
    for (const name of Object.keys(table)) {
      const [read, assign] = table[name];
      define(proto, name, {enumerable: true, configurable: true,
        get: make(function () { apply(check, this, []); return read(this); }, 'get ' + name, 0),
        set: assign ? make(function (value) { apply(check, this, []); assign(this, value, name, owner); }, 'set ' + name, 1) : undefined});
    }
  }
  const reflect = (element, value, name, owner) => write(element, name, StringValue(double(value, name, owner)));
  const meterMin = element => parse(attr(element, 'min'), 0);
  const meterMax = element => { const low = meterMin(element); return max(parse(attr(element, 'max'), max(1, low)), low); };
  const clamp = (value, low, high) => min(max(value, low), high);
  const meterLow = element => clamp(parse(attr(element, 'low'), meterMin(element)), meterMin(element), meterMax(element));
  if (typeof g.HTMLMeterElement === 'function') accessors(g.HTMLMeterElement.prototype, 'HTMLMeterElement', brand('meter'), {
    value: [element => clamp(parse(attr(element, 'value'), 0), meterMin(element), meterMax(element)), reflect],
    min: [meterMin, reflect],
    max: [meterMax, reflect],
    low: [meterLow, reflect],
    high: [element => clamp(parse(attr(element, 'high'), meterMax(element)), meterLow(element), meterMax(element)), reflect],
    optimum: [element => clamp(parse(attr(element, 'optimum'), (meterMax(element) + meterMin(element)) / 2), meterMin(element), meterMax(element)), reflect],
  });
  const progressMax = element => { const value = parse(attr(element, 'max'), 1); return value > 0 ? value : 1; };
  const progressValue = element => { const value = parse(attr(element, 'value'), 0); return value < 0 ? 0 : min(value, progressMax(element)); };
  if (typeof g.HTMLProgressElement === 'function') accessors(g.HTMLProgressElement.prototype, 'HTMLProgressElement', brand('progress'), {
    // Chrome clamps a negative value to 0 and ignores a non-positive max on setting.
    value: [progressValue, (element, value, name, owner) => write(element, name, StringValue(max(double(value, name, owner), 0)))],
    max: [progressMax, (element, value, name, owner) => { value = double(value, name, owner); if (value > 0) write(element, name, StringValue(value)); }],
    position: [element => apply(hasAttribute, element, ['value']) ? progressValue(element) / progressMax(element) : -1],
  });
  // Chrome's output model: a value-mode flag (default mode until `value` is set) plus a stored
  // default value that `defaultValue` writes only when it changes. Clones start fresh.
  const state = element => { let record = apply(stateGet, states, [element]); if (!record) { record = {defaultMode: true, defaultValue: ''}; apply(stateSet, states, [element, record]); } return record; };
  const content = element => apply(textGet, element, []);
  const replace = (element, value) => apply(textSet, element, [value]);
  const outputDefault = element => { const record = state(element); return record.defaultMode ? content(element) : record.defaultValue; };
  function resetOutput(element) {
    const fallback = outputDefault(element);
    if (fallback === content(element)) return;
    replace(element, fallback);
    state(element).defaultMode = true;
  }
  if (typeof g.HTMLOutputElement === 'function') {
    accessors(g.HTMLOutputElement.prototype, 'HTMLOutputElement', brand('output'), {
      name: [element => { const value = attr(element, 'name'); return value === null ? '' : value; }, (element, value) => write(element, 'name', string(value))],
      type: [() => 'output'],
      defaultValue: [outputDefault, (element, value) => {
        value = string(value); const record = state(element);
        if (record.defaultValue === value) return;
        record.defaultValue = value;
        if (record.defaultMode) replace(element, value);
      }],
      value: [content, (element, value) => {
        value = string(value); const record = state(element), current = content(element);
        if (record.defaultMode) record.defaultValue = current;
        record.defaultMode = false;
        if (value !== current) replace(element, value);
      }],
    });
    // Form reset reaches output through the shared per-control reset hook.
    const resetControl = ns.formResetControl;
    ns.formResetControl = function (element) {
      apply(resetControl, undefined, [element]);
      if (apply(nodeType, element, []) === 1 && apply(localName, element, []) === 'output' && apply(namespace, element, []) === 'http://www.w3.org/1999/xhtml') resetOutput(element);
    };
  }
})();
