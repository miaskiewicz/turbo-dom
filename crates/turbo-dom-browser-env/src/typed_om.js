// CSS Typed OM Level 1 as Chrome 156 ships it: the CSSStyleValue hierarchy (numeric values and
// their math, keywords, unparsed values and variable references, transforms, positions and
// images), CSSStyleValue.parse/parseAll and CSSNumericValue.parse over a Blink-shaped calc()
// parser, and StylePropertyMap views over inline style (attributeStyleMap), CSSStyleRule
// declarations (styleMap) and computed style (computedStyleMap()).
(function () {
  'use strict';
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction;
  const Native = g.__typedOmNative; delete g.__typedOmNative;
  const registry = JSON.parse(g.__typedOmRegistry); delete g.__typedOmRegistry;
  const define = Object.defineProperty, create = Object.create, getProto = Object.getPrototypeOf, setProto = Object.setPrototypeOf;
  const apply = Reflect.apply, ownKeys = Reflect.ownKeys, freeze = Object.freeze;
  const ErrorType = TypeError, RangeType = RangeError;
  const isFiniteNumber = Number.isFinite, isNaNValue = Number.isNaN, mathAbs = Math.abs, mathMin = Math.min, mathMax = Math.max, mathRound = Math.round, mathTrunc = Math.trunc, mathTan = Math.tan, mathPI = Math.PI, mathE = Math.E, mathPow = Math.pow, mathSqrt = Math.sqrt, mathSin = Math.sin, mathCos = Math.cos, mathAsin = Math.asin, mathAcos = Math.acos, mathAtan = Math.atan, mathAtan2 = Math.atan2, mathLog = Math.log, mathExp = Math.exp, mathFloor = Math.floor, mathCeil = Math.ceil, mathSign = Math.sign;
  const lowerCase = Function.prototype.call.bind(String.prototype.toLowerCase);
  const charCode = Function.prototype.call.bind(String.prototype.charCodeAt), sliceText = Function.prototype.call.bind(String.prototype.slice), startsWith = Function.prototype.call.bind(String.prototype.startsWith), indexOfText = Function.prototype.call.bind(String.prototype.indexOf), trimText = Function.prototype.call.bind(String.prototype.trim), replaceText = Function.prototype.call.bind(String.prototype.replace), testRegExp = Function.prototype.call.bind(RegExp.prototype.test), execRegExp = Function.prototype.call.bind(RegExp.prototype.exec);
  const weakGet = Function.prototype.call.bind(WeakMap.prototype.get), weakSet = Function.prototype.call.bind(WeakMap.prototype.set);
  const mapGet = Function.prototype.call.bind(Map.prototype.get), mapSet = Function.prototype.call.bind(Map.prototype.set);
  const arrayIterator = Array.prototype[Symbol.iterator], symbolIterator = Symbol.iterator, symbolTag = Symbol.toStringTag;
  const Exception = g.DOMException;
  const elementCheck = ns.domAccessor('nodeType', false, 'Element'), htmlCheck = ns.domAccessor('nodeType', false, 'HTMLElement');
  const namespaceOf = ns.domAccessor('namespaceURI', false, 'Element'), connectedOf = ns.domAccessor('isConnected', false, 'Node'), tagOf = ns.domAccessor('localName', false, 'Element');
  const getAttributeNS = g.Element.prototype.getAttributeNS, attributeOf = (element, name) => apply(getAttributeNS, element, [null, name]);

  // ---- Arrays without consulting (possibly patched) Array.prototype ------------------------
  const list = () => { const result = []; setProto(result, null); return result; };
  const push = (array, value) => { define(array, array.length, { __proto__: null, value, writable: true, enumerable: true, configurable: true }); return array; };
  const toArray = (array) => { const result = []; for (let index = 0; index < array.length; index++) define(result, index, { __proto__: null, value: array[index], writable: true, enumerable: true, configurable: true }); return result; };
  const plain = (array) => { const result = list(); for (let index = 0; index < array.length; index++) push(result, array[index]); return result; };

  // ---- Error helpers in Chrome's binding wording ---------------------------------------------
  const constructMessage = (name, message) => "Failed to construct '" + name + "': " + message;
  const executeMessage = (operation, owner, message) => "Failed to execute '" + operation + "' on '" + owner + "': " + message;
  const setMessage = (property, owner, message) => "Failed to set the '" + property + "' property on '" + owner + "': " + message;
  const arity = (required, present) => required + ' argument' + (required > 1 ? 's' : '') + ' required, but only ' + present + ' present.';
  const domError = (message, name) => new Exception(message, name);
  let context = null;
  // Binding failures carry the operation context of the outermost IDL entry point.
  const fail = (message, Type) => { const text = context ? context(message) : message; return Type === 'SyntaxError' ? domError(text, 'SyntaxError') : new (Type || ErrorType)(text); };
  function enter(prefix, body) { const previous = context; context = prefix; try { return body(); } finally { context = previous; } }

  // ---- WebIDL conversions --------------------------------------------------------------------
  const toStringValue = (value) => { if (typeof value === 'symbol') throw fail('Cannot convert a Symbol value to a string'); return '' + value; };
  const toDouble = (value) => { if (typeof value === 'bigint') throw fail('Cannot convert a BigInt value to a number'); if (typeof value === 'symbol') throw fail('Cannot convert a Symbol value to a number'); const number = +value; if (!isFiniteNumber(number)) throw fail('The provided double value is non-finite.'); return number; };
  function toSequence(value, convert) {
    if (value === null || (typeof value !== 'object' && typeof value !== 'function')) throw fail('The provided value cannot be converted to a sequence.');
    const method = value[symbolIterator];
    if (method === undefined || method === null) throw fail('The provided value cannot be converted to a sequence.');
    if (typeof method !== 'function') throw fail('The provided value cannot be converted to a sequence.');
    const iterator = apply(method, value, []), next = iterator.next, result = list();
    for (;;) { const step = apply(next, iterator, []); if (step === null || typeof step !== 'object') throw new ErrorType('Iterator result ' + step + ' is not an object'); if (step.done) break; push(result, convert(step.value, result.length)); }
    return result;
  }

  // ---- Private records and interface objects -------------------------------------------------
  const records = new WeakMap();
  const recordOf = (value) => (value !== null && (typeof value === 'object' || typeof value === 'function')) ? weakGet(records, value) : undefined;
  function brand(value, families) { const record = recordOf(value); if (!record || !families[record.family]) throw new ErrorType('Illegal invocation'); return record; }
  const interfaces = create(null), prototypes = create(null);
  const brandSets = create(null);
  // `implements` lists every interface whose brand check the instance passes.
  function family(name, parents) { const set = create(null); set[name] = true; brandSets[name] = set; for (let index = 0; index < parents.length; index++) brandSets[parents[index]][name] = true; return set; }
  function attach(object, record) { weakSet(records, object, record); return object; }
  function instance(name, record) { const object = create(prototypes[name]); record.family = name; return attach(object, record); }
  function defineInterface(name, parent, length, constructor) {
    const target = constructor
      ? function () { if (new.target === undefined) throw new ErrorType(constructMessage(name, "Please use the 'new' operator, this DOM object constructor cannot be called as a function.")); const self = this; return enter((message) => constructMessage(name, message), () => apply(constructor, self, arguments)); }
      : function () { if (new.target === undefined) throw new ErrorType('Illegal constructor'); throw new ErrorType(constructMessage(name, 'Illegal constructor')); };
    const ctor = make(target, name, length, true, true, false);
    const proto = ctor.prototype;
    if (parent) { setProto(proto, prototypes[parent]); setProto(ctor, interfaces[parent]); }
    define(proto, 'constructor', { __proto__: null, value: ctor, writable: true, enumerable: false, configurable: true });
    define(proto, symbolTag, { __proto__: null, value: name, writable: false, enumerable: false, configurable: true });
    define(ctor, 'prototype', { __proto__: null, writable: false, enumerable: false, configurable: false });
    define(g, name, { __proto__: null, value: ctor, writable: true, enumerable: false, configurable: true });
    interfaces[name] = ctor; prototypes[name] = proto;
    return proto;
  }
  function method(owner, name, length, body, operationOwner) {
    const label = operationOwner || owner;
    define(prototypes[owner], name, { __proto__: null, value: make(function () { const self = this, args = arguments; return enter((message) => executeMessage(name, label, message), () => apply(body, self, args)); }, name, length), writable: true, enumerable: true, configurable: true });
  }
  function staticMethod(owner, name, length, body) {
    define(interfaces[owner], name, { __proto__: null, value: make(function () { const args = arguments; return enter((message) => executeMessage(name, owner, message), () => apply(body, undefined, args)); }, name, length), writable: true, enumerable: true, configurable: true });
  }
  function attribute(owner, name, read, write) {
    const descriptor = { __proto__: null, enumerable: true, configurable: true, get: make(function () { return read(this); }, 'get ' + name, 0) };
    if (write) descriptor.set = make(function (value) { const self = this; return enter((message) => setMessage(name, owner, message), () => { write(self, value); }); }, 'set ' + name, 1);
    define(prototypes[owner], name, descriptor);
  }
  function requireArguments(args, count) { if (args.length < count) throw fail(arity(count, args.length)); }
  function stringifier(owner, read) {
    define(prototypes[owner], 'toString', { __proto__: null, value: make(function () { return read(this); }, 'toString', 0), writable: true, enumerable: true, configurable: true });
  }
  // iterable<V>: entries/keys/values/forEach over an indexed snapshot, @@iterator === values.
  const arrayEntries = Array.prototype.entries, arrayKeys = Array.prototype.keys, arrayValues = Array.prototype.values;
  // iterable<V> on an interface with an indexed getter uses %Array.prototype% functions.
  function valueIterable(owner) {
    const proto = prototypes[owner];
    define(proto, 'entries', { __proto__: null, value: arrayEntries, writable: true, enumerable: true, configurable: true });
    define(proto, 'keys', { __proto__: null, value: arrayKeys, writable: true, enumerable: true, configurable: true });
    define(proto, 'values', { __proto__: null, value: arrayValues, writable: true, enumerable: true, configurable: true });
    define(proto, 'forEach', { __proto__: null, value: Array.prototype.forEach, writable: true, enumerable: true, configurable: true });
    define(proto, symbolIterator, { __proto__: null, value: arrayValues, writable: true, enumerable: false, configurable: true });
  }

  // ---- Units ---------------------------------------------------------------------------------
  // Blink unit table: canonical serialization, category and scale factor to the category's
  // canonical unit (px, deg, s, hz, dppx). `relative` lengths do not convert.
  const BASE = { length: 0, angle: 1, time: 2, frequency: 3, resolution: 4, flex: 5, percent: 6 };
  const BASE_NAMES = ['length', 'angle', 'time', 'frequency', 'resolution', 'flex', 'percent'];
  const UNITS = create(null);
  function unit(name, base, category, factor) { UNITS[name] = freeze({ __proto__: null, name, base, category, factor }); }
  unit('number', -1, 'number', 1); unit('percent', BASE.percent, 'percent', 1);
  for (const name of ['em', 'ex', 'ch', 'ic', 'rem', 'lh', 'rlh', 'vw', 'vh', 'vi', 'vb', 'vmin', 'vmax', 'svw', 'svh', 'svi', 'svb', 'svmin', 'svmax', 'lvw', 'lvh', 'lvi', 'lvb', 'lvmin', 'lvmax', 'dvw', 'dvh', 'dvi', 'dvb', 'dvmin', 'dvmax', 'cqw', 'cqh', 'cqi', 'cqb', 'cqmin', 'cqmax', 'cap', 'rcap', 'rch', 'rex', 'ric']) unit(name, BASE.length, 'other', 1);
  unit('px', BASE.length, 'length', 1); unit('cm', BASE.length, 'length', 96 / 2.54); unit('mm', BASE.length, 'length', 96 / 2.54 / 10); unit('q', BASE.length, 'length', 96 / 2.54 / 10 / 4); unit('in', BASE.length, 'length', 96); unit('pt', BASE.length, 'length', 96 / 72); unit('pc', BASE.length, 'length', 96 / 6);
  unit('deg', BASE.angle, 'angle', 1); unit('rad', BASE.angle, 'angle', 180 / mathPI); unit('grad', BASE.angle, 'angle', 0.9); unit('turn', BASE.angle, 'angle', 360);
  unit('s', BASE.time, 'time', 1); unit('ms', BASE.time, 'time', 0.001);
  unit('hz', BASE.frequency, 'frequency', 1); unit('khz', BASE.frequency, 'frequency', 1000);
  unit('dppx', BASE.resolution, 'resolution', 1); unit('x', BASE.resolution, 'resolution', 1); unit('dpi', BASE.resolution, 'resolution', 1 / 96); unit('dpcm', BASE.resolution, 'resolution', 1 / (96 / 2.54));
  unit('fr', BASE.flex, 'other', 1);
  const CANONICAL = { __proto__: null, number: 'number', length: 'px', angle: 'deg', time: 's', frequency: 'hz', resolution: 'dppx' };
  const canonicalUnit = (name) => CANONICAL[UNITS[name].category] || null;
  const canonicalIfPossible = (name) => canonicalUnit(name) || name;
  // CSSNumericValue::UnitFromName.
  function unitFromName(text) {
    if (text === '') return null;
    const lower = lowerCase(text);
    if (lower === 'number') return 'number';
    if (lower === 'percent' || text === '%') return 'percent';
    if (lower === 'percent' || lower === 'number') return null;
    return UNITS[lower] && lower !== 'percent' ? lower : null;
  }
  const unitSuffix = (name) => name === 'number' ? '' : name === 'percent' ? '%' : name;

  // %.6g: six significant digits, exponent form outside [1e-4, 1e6), trailing zeros dropped.
  const toExponential = Function.prototype.call.bind(Number.prototype.toExponential);
  const stripZeros = (text) => indexOfText(text, '.') >= 0 ? replaceText(replaceText(text, /0+$/, ''), /\.$/, '') : text;
  // Round to six significant digits half-to-even on the exact binary value, as %.6g does.
  function formatG(number) {
    if (number === 0) return 1 / number < 0 ? '-0' : '0';
    const negative = number < 0, exact = toExponential(negative ? -number : number, 99);
    const at = indexOfText(exact, 'e');
    let digits = replaceText(sliceText(exact, 0, at), '.', ''), exponent = +sliceText(exact, at + 1);
    const kept = sliceText(digits, 0, 6), rest = sliceText(digits, 6);
    let roundUp = false;
    if (charCode(rest, 0) > 53) roundUp = true;
    else if (charCode(rest, 0) === 53) roundUp = testRegExp(/[1-9]/, sliceText(rest, 1)) || (charCode(kept, 5) - 48) % 2 === 1;
    let mantissa = kept;
    if (roundUp) {
      let carry = 1, result = '';
      for (let index = 5; index >= 0; index--) { let digit = charCode(kept, index) - 48 + carry; carry = digit === 10 ? 1 : 0; result = (digit % 10) + result; }
      if (carry) { result = '1' + sliceText(result, 0, 5); exponent++; }
      mantissa = result;
    }
    const sign = negative ? '-' : '';
    if (exponent < -4 || exponent >= 6) { const magnitude = mathAbs(exponent); return sign + stripZeros(mantissa[0] + '.' + sliceText(mantissa, 1)) + 'e' + (exponent < 0 ? '-' : '+') + (magnitude < 10 ? '0' : '') + magnitude; }
    let text;
    if (exponent < 0) { let zeros = ''; for (let index = -1; index > exponent; index--) zeros += '0'; text = '0.' + zeros + mantissa; }
    else text = sliceText(mantissa, 0, exponent + 1) + '.' + sliceText(mantissa, exponent + 1);
    return sign + stripZeros(text);
  }
  // CSSNumericLiteralValue::CustomCSSText.
  function formatNumber(value, unitKey) {
    const suffix = unitSuffix(unitKey);
    if (!isFiniteNumber(value)) { const word = isNaNValue(value) ? 'NaN' : value > 0 ? 'infinity' : '-infinity'; return suffix ? word + ' * 1' + suffix : word; }
    if (value >= -999999 && value <= 999999 && mathTrunc(value) === value) return (value === 0 ? '0' : '' + value) + suffix;
    return formatG(value) + suffix;
  }

  // ---- CSSNumericValueType -------------------------------------------------------------------
  function makeType(unitKey, exponent) {
    const type = { __proto__: null, e: [0, 0, 0, 0, 0, 0, 0], hint: -1 };
    if (unitKey !== 'number') type.e[UNITS[unitKey].base] = exponent === undefined ? 1 : exponent;
    return type;
  }
  const copyType = (type) => ({ __proto__: null, e: [type.e[0], type.e[1], type.e[2], type.e[3], type.e[4], type.e[5], type.e[6]], hint: type.hint });
  function applyHint(type, hint) { type.e[hint] += type.e[BASE.percent]; type.e[BASE.percent] = 0; type.hint = hint; }
  function addTypes(left, right) {
    const a = copyType(left), b = copyType(right);
    if (a.hint >= 0 && b.hint >= 0 && a.hint !== b.hint) return null;
    if (a.hint >= 0) applyHint(b, a.hint); else if (b.hint >= 0) applyHint(a, b.hint);
    for (let index = 0; index < 7; index++) {
      if (a.e[index] !== b.e[index]) {
        if (index !== BASE.percent) { applyHint(a, index); applyHint(b, index); }
        if (a.e[index] !== b.e[index]) return null;
      }
    }
    return a;
  }
  function multiplyTypes(left, right) {
    const a = copyType(left), b = copyType(right);
    if (a.hint >= 0 && b.hint >= 0 && a.hint !== b.hint) return null;
    if (a.hint >= 0) applyHint(b, a.hint); else if (b.hint >= 0) applyHint(a, b.hint);
    for (let index = 0; index < 7; index++) a.e[index] += b.e[index];
    return a;
  }
  function negateType(type) { const result = copyType(type); for (let index = 0; index < 7; index++) result.e[index] = -result.e[index] || 0; return result; }
  const nonZero = (type) => { let count = 0; for (let index = 0; index < 7; index++) if (type.e[index]) count++; return count; };
  const onlyEntry = (type, base, value) => nonZero(type) === 1 && type.e[base] === value;
  const matchesBase = (type, base) => onlyEntry(type, base, 1) && type.hint < 0;
  const matchesPercentage = (type) => onlyEntry(type, BASE.percent, 1);
  const matchesBasePercentage = (type, base) => onlyEntry(type, base, 1) || onlyEntry(type, BASE.percent, 1);
  const matchesNumber = (type) => nonZero(type) === 0 && type.hint < 0;

  // ---- Numeric value records -----------------------------------------------------------------
  // record: {family, kind, type, ...}. kind: unit | sum | product | negate | invert | min | max | clamp.
  // Brand families: an instance passes the brand check of its own interface and every ancestor.
  const HIERARCHY = [['CSSStyleValue', []], ['CSSNumericValue', ['CSSStyleValue']], ['CSSUnitValue', ['CSSNumericValue', 'CSSStyleValue']], ['CSSMathValue', ['CSSNumericValue', 'CSSStyleValue']]];
  for (const name of ['CSSMathSum', 'CSSMathProduct', 'CSSMathNegate', 'CSSMathInvert', 'CSSMathMin', 'CSSMathMax', 'CSSMathClamp']) push(HIERARCHY, [name, ['CSSMathValue', 'CSSNumericValue', 'CSSStyleValue']]);
  for (const name of ['CSSKeywordValue', 'CSSUnparsedValue', 'CSSTransformValue', 'CSSImageValue', 'CSSPositionValue']) push(HIERARCHY, [name, ['CSSStyleValue']]);
  push(HIERARCHY, ['CSSNumericArray', []]); push(HIERARCHY, ['CSSVariableReferenceValue', []]); push(HIERARCHY, ['CSSTransformComponent', []]);
  for (const name of ['CSSTranslate', 'CSSRotate', 'CSSScale', 'CSSSkew', 'CSSSkewX', 'CSSSkewY', 'CSSPerspective', 'CSSMatrixComponent']) push(HIERARCHY, [name, ['CSSTransformComponent']]);
  push(HIERARCHY, ['StylePropertyMapReadOnly', []]); push(HIERARCHY, ['StylePropertyMap', ['StylePropertyMapReadOnly']]);
  for (let index = 0; index < HIERARCHY.length; index++) family(HIERARCHY[index][0], HIERARCHY[index][1]);
  const KIND_INTERFACE = { __proto__: null, unit: 'CSSUnitValue', sum: 'CSSMathSum', product: 'CSSMathProduct', negate: 'CSSMathNegate', invert: 'CSSMathInvert', min: 'CSSMathMin', max: 'CSSMathMax', clamp: 'CSSMathClamp' };
  const OPERATOR = { __proto__: null, sum: 'sum', product: 'product', negate: 'negate', invert: 'invert', min: 'min', max: 'max', clamp: 'clamp' };
  const numericRecord = (value) => { const record = recordOf(value); return record && record.numeric ? record : null; };
  function unitValue(value, unitKey) { return instance('CSSUnitValue', { numeric: true, kind: 'unit', value, unit: unitKey, type: makeType(unitKey), text: null }); }
  function numericArray(values) { return numericArrayObject(values); }
  function variadic(kind, values, type) { return instance(KIND_INTERFACE[kind], { numeric: true, kind, values, array: numericArray(values), type, text: null }); }
  function typeCheck(values, combine) {
    let type = numericRecord(values[0]).type;
    for (let index = 1; index < values.length; index++) { type = combine(type, numericRecord(values[index]).type); if (!type) return null; }
    return type;
  }
  function createVariadic(kind, values) { const type = typeCheck(values, kind === 'product' ? multiplyTypes : addTypes); return type ? variadic(kind, values, type) : null; }
  function createNegate(value) { return instance('CSSMathNegate', { numeric: true, kind: 'negate', value, type: copyType(numericRecord(value).type), text: null }); }
  function createInvert(value) { return instance('CSSMathInvert', { numeric: true, kind: 'invert', value, type: negateType(numericRecord(value).type), text: null }); }
  function createClamp(lower, value, upper) {
    let type = addTypes(numericRecord(lower).type, numericRecord(value).type);
    if (type) type = addTypes(type, numericRecord(upper).type);
    return type ? instance('CSSMathClamp', { numeric: true, kind: 'clamp', lower, value, upper, type, text: null }) : null;
  }
  // CSSNumberish: a CSSNumericValue, else a finite double as a number unit value.
  function fromNumberish(value) { if (numericRecord(value)) return value; return unitValue(toDouble(value), 'number'); }
  function negateValue(value) { const record = numericRecord(value); return record.kind === 'unit' ? unitValue(-record.value, record.unit) : createNegate(value); }
  function invertValue(value) {
    const record = numericRecord(value);
    if (record.kind === 'unit') { if (record.unit === 'number') return record.value === 0 ? null : unitValue(1 / record.value, 'number'); return createInvert(value); }
    return createInvert(value);
  }

  // CSSNumericSumValue: terms of value × unit-exponent map (canonical units).
  function sumValue(value) {
    const record = numericRecord(value);
    switch (record.kind) {
      case 'unit': {
        const units = create(null);
        if (record.unit !== 'number') units[canonicalIfPossible(record.unit)] = 1;
        return [{ value: record.value * UNITS[record.unit].factor, units }];
      }
      case 'sum': {
        const terms = [];
        for (let index = 0; index < record.values.length; index++) {
          const child = sumValue(record.values[index]);
          if (!child) return null;
          for (let at = 0; at < child.length; at++) {
            const term = child[at]; let found = -1;
            for (let existing = 0; existing < terms.length; existing++) if (sameUnits(terms[existing].units, term.units)) { found = existing; break; }
            if (found < 0) push(terms, { value: term.value, units: term.units }); else terms[found].value += term.value;
          }
        }
        // Every term's type must add to the first's.
        const first = typeOfUnits(terms[0].units);
        for (let index = 0; index < terms.length; index++) if (!addTypes(first, typeOfUnits(terms[index].units))) return null;
        return terms;
      }
      case 'product': {
        let terms = [{ value: 1, units: create(null) }];
        for (let index = 0; index < record.values.length; index++) {
          const child = sumValue(record.values[index]);
          if (!child) return null;
          const next = [];
          for (let a = 0; a < terms.length; a++) for (let b = 0; b < child.length; b++) push(next, { value: terms[a].value * child[b].value, units: multiplyUnits(terms[a].units, child[b].units) });
          terms = next;
        }
        return terms;
      }
      case 'negate': { const child = sumValue(record.value); if (!child) return null; for (let index = 0; index < child.length; index++) child[index].value *= -1; return child; }
      case 'invert': {
        const child = sumValue(record.value);
        if (!child || child.length !== 1) return null;
        const units = create(null), keys = ownKeys(child[0].units);
        for (let index = 0; index < keys.length; index++) units[keys[index]] = -child[0].units[keys[index]];
        return [{ value: 1 / child[0].value, units }];
      }
      case 'min': case 'max': {
        let current = sumValue(record.values[0]);
        if (!current || current.length !== 1) return null;
        for (let index = 0; index < record.values.length; index++) {
          const child = sumValue(record.values[index]);
          if (!child || child.length !== 1 || !sameUnits(child[0].units, current[0].units)) return null;
          if (record.kind === 'min' ? child[0].value < current[0].value : child[0].value > current[0].value) current = child;
        }
        return current;
      }
      case 'clamp': {
        const lower = sumValue(record.lower);
        const parts = [record.lower, record.value, record.upper];
        for (let index = 0; index < 3; index++) { const child = sumValue(parts[index]); if (!child || child.length !== 1 || !sameUnits(child[0].units, lower[0].units)) return null; }
        const middle = sumValue(record.value), upper = sumValue(record.upper);
        middle[0].value = mathMax(lower[0].value, mathMin(middle[0].value, upper[0].value));
        return middle;
      }
    }
    return null;
  }
  function sameUnits(a, b) { const left = ownKeys(a), right = ownKeys(b); if (left.length !== right.length) return false; for (let index = 0; index < left.length; index++) if (a[left[index]] !== b[left[index]]) return false; return true; }
  function multiplyUnits(a, b) { const result = create(null), left = ownKeys(a), right = ownKeys(b); for (let index = 0; index < left.length; index++) result[left[index]] = a[left[index]]; for (let index = 0; index < right.length; index++) { const key = right[index], next = (result[key] || 0) + b[key]; if (next === 0) delete result[key]; else result[key] = next; } return result; }
  function typeOfUnits(units) { let type = makeType('number'); const keys = ownKeys(units); for (let index = 0; index < keys.length; index++) type = multiplyTypes(type, makeType(keys[index], units[keys[index]])); return type; }
  function termToUnit(term) { const keys = ownKeys(term.units); if (keys.length === 0) return unitValue(term.value, 'number'); if (keys.length === 1 && term.units[keys[0]] === 1) return unitValue(term.value, keys[0]); return null; }
  function convertUnit(value, target) {
    const record = numericRecord(value);
    if (record.unit === target) return unitValue(record.value, record.unit);
    const canonical = canonicalUnit(record.unit);
    if (!canonical || canonical !== canonicalUnit(target)) return null;
    return unitValue(record.value * (UNITS[record.unit].factor / UNITS[target].factor), target);
  }
  function toUnit(value, target) {
    const sum = sumValue(value);
    if (!sum || sum.length !== 1) return null;
    const single = termToUnit(sum[0]);
    return single ? convertUnit(single, target) : null;
  }
  function equalsValue(a, b) {
    const left = numericRecord(a), right = numericRecord(b);
    if (left.kind !== right.kind) return false;
    switch (left.kind) {
      case 'unit': return left.value === right.value && left.unit === right.unit;
      case 'negate': case 'invert': return equalsValue(left.value, right.value);
      case 'clamp': return equalsValue(left.lower, right.lower) && equalsValue(left.value, right.value) && equalsValue(left.upper, right.upper);
      default: {
        if (left.values.length !== right.values.length) return false;
        for (let index = 0; index < left.values.length; index++) if (!equalsValue(left.values[index], right.values[index])) return false;
        return true;
      }
    }
  }
  // Typed OM's own serialization (CSSNumericValue::BuildCSSText).
  function numericText(value, nested, parenLess) {
    const record = numericRecord(value);
    switch (record.kind) {
      case 'unit': return formatNumber(record.value, record.unit);
      case 'sum': case 'product': {
        let text = parenLess ? '' : nested ? '(' : 'calc(';
        text += numericText(record.values[0], true, false);
        for (let index = 1; index < record.values.length; index++) {
          const argument = numericRecord(record.values[index]);
          if (record.kind === 'sum' && argument.kind === 'negate') text += ' - ' + numericText(argument.value, true, false);
          else if (record.kind === 'product' && argument.kind === 'invert') text += ' / ' + numericText(argument.value, true, false);
          else text += (record.kind === 'sum' ? ' + ' : ' * ') + numericText(record.values[index], true, false);
        }
        return parenLess ? text : text + ')';
      }
      case 'negate': return (parenLess ? '' : nested ? '(' : 'calc(') + '-' + numericText(record.value, true, false) + (parenLess ? '' : ')');
      case 'invert': return (parenLess ? '' : nested ? '(' : 'calc(') + '1 / ' + numericText(record.value, true, false) + (parenLess ? '' : ')');
      case 'min': case 'max': {
        let text = record.kind + '(';
        for (let index = 0; index < record.values.length; index++) text += (index ? ', ' : '') + numericText(record.values[index], true, true);
        return text + ')';
      }
      case 'clamp': return 'clamp(' + numericText(record.lower, true, true) + ', ' + numericText(record.value, true, true) + ', ' + numericText(record.upper, true, true) + ')';
    }
    return '';
  }
  const prependForArithmetic = (kind, values, value) => {
    const record = numericRecord(value), result = list();
    if (record.kind === kind) for (let index = 0; index < record.values.length; index++) push(result, record.values[index]); else push(result, value);
    for (let index = 0; index < values.length; index++) push(result, values[index]);
    return result;
  };
  function simplifyAsUnit(values, operation) {
    const first = numericRecord(values[0]);
    if (first.kind !== 'unit') return null;
    let result = first.value;
    for (let index = 1; index < values.length; index++) { const record = numericRecord(values[index]); if (record.kind !== 'unit' || record.unit !== first.unit) return null; result = operation(result, record.value); }
    return unitValue(result, first.unit);
  }
  function multiplyAsUnit(values) {
    let other = 'number', result = 1;
    for (let index = 0; index < values.length; index++) {
      const record = numericRecord(values[index]);
      if (record.kind !== 'unit') return null;
      if (record.unit !== 'number') { if (other !== 'number') return null; other = record.unit; }
      result *= record.value;
    }
    return unitValue(result, other);
  }
  const numberishes = (args) => { const result = list(); for (let index = 0; index < args.length; index++) push(result, fromNumberish(args[index])); return result; };
  const incompatible = (value) => { if (!value) throw fail('Incompatible types'); return value; };

  // ---- CSS Syntax tokenizer (css-syntax-3) -----------------------------------------------------
  // Tokens: {type, value, unit, number (integer|number), delim, hashType, raw}.
  const isDigit = (c) => c >= 48 && c <= 57, isHex = (c) => isDigit(c) || (c >= 65 && c <= 70) || (c >= 97 && c <= 102);
  const isNameStart = (c) => (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c === 95 || c >= 128;
  const isName = (c) => isNameStart(c) || isDigit(c) || c === 45;
  const isSpace = (c) => c === 32 || c === 9 || c === 10 || c === 12 || c === 13;
  const fromCode = String.fromCodePoint;
  // css-syntax-3 input preprocessing: CRLF/CR/FF to LF and NUL to U+FFFD.
  function preprocess(source) {
    let text = '';
    for (let index = 0; index < source.length; index++) {
      const c = charCode(source, index);
      if (c === 13) { text += '\n'; if (charCode(source, index + 1) === 10) index++; } else if (c === 12) text += '\n'; else if (c === 0) text += '\uFFFD'; else text += source[index];
    }
    return text;
  }
  function tokenize(source) {
    const text = preprocess(toStringValue(source));
    const tokens = list(), length = text.length; let at = 0;
    const code = (offset) => at + offset < length ? charCode(text, at + offset) : -1;
    const validEscape = (a, b) => a === 92 && b !== 10;
    const startsIdent = (a, b, c) => a === 45 ? (isNameStart(b) || b === 45 || validEscape(b, c)) : isNameStart(a) ? true : validEscape(a, b);
    const startsNumber = (a, b, c) => (a === 43 || a === 45) ? (isDigit(b) || (b === 46 && isDigit(c))) : a === 46 ? isDigit(b) : isDigit(a);
    function escape() {
      let c = code(0); at++;
      if (isHex(c)) { let hex = fromCode(c), count = 1; while (count < 6 && isHex(code(0))) { hex += text[at]; at++; count++; } if (isSpace(code(0))) at++; const point = parseInt(hex, 16); return point === 0 || (point >= 0xd800 && point <= 0xdfff) || point > 0x10ffff ? '�' : fromCode(point); }
      if (c === -1) return '�';
      const char = fromCode(c); at += char.length - 1; return char;
    }
    function name() { let result = ''; for (;;) { const c = code(0); if (isName(c)) { result += text[at]; at++; } else if (validEscape(c, code(1))) { at++; result += escape(); } else return result; } }
    function number() {
      const start = at; let integer = true;
      if (code(0) === 43 || code(0) === 45) at++;
      while (isDigit(code(0))) at++;
      if (code(0) === 46 && isDigit(code(1))) { integer = false; at += 2; while (isDigit(code(0))) at++; }
      const e = code(0);
      if ((e === 69 || e === 101) && (isDigit(code(1)) || ((code(1) === 43 || code(1) === 45) && isDigit(code(2))))) { integer = false; at += isDigit(code(1)) ? 2 : 3; while (isDigit(code(0))) at++; }
      return { value: +sliceText(text, start, at), integer };
    }
    function consumeString(quote) {
      let value = ''; at++;
      for (;;) {
        const c = code(0);
        if (c === -1) return { type: 'string', value };
        if (c === quote) { at++; return { type: 'string', value }; }
        if (c === 10) return { type: 'bad-string' };
        if (c === 92) { if (code(1) === -1) { at++; continue; } if (code(1) === 10) { at += 2; continue; } at++; value += escape(); continue; }
        value += text[at]; at++;
      }
    }
    function consumeBadUrl() { for (;;) { const c = code(0); if (c === -1) return; if (c === 41) { at++; return; } if (validEscape(c, code(1))) { at++; escape(); } else at++; } }
    function consumeUrl() {
      let value = ''; while (isSpace(code(0))) at++;
      for (;;) {
        const c = code(0);
        if (c === -1) return { type: 'url', value };
        if (c === 41) { at++; return { type: 'url', value }; }
        if (isSpace(c)) { while (isSpace(code(0))) at++; if (code(0) === 41 || code(0) === -1) { if (code(0) === 41) at++; return { type: 'url', value }; } consumeBadUrl(); return { type: 'bad-url' }; }
        if (c === 34 || c === 39 || c === 40 || (c >= 0 && c <= 8) || c === 11 || (c >= 14 && c <= 31) || c === 127) { consumeBadUrl(); return { type: 'bad-url' }; }
        if (c === 92) { if (validEscape(c, code(1))) { at++; value += escape(); continue; } consumeBadUrl(); return { type: 'bad-url' }; }
        value += text[at]; at++;
      }
    }
    function identLike() {
      const value = name();
      if (lowerCase(value) === 'url' && code(0) === 40) {
        at++; while (isSpace(code(0)) && isSpace(code(1))) at++;
        const next = isSpace(code(0)) ? code(1) : code(0);
        if (next === 34 || next === 39) return { type: 'function', value };
        return consumeUrl();
      }
      if (code(0) === 40) { at++; return { type: 'function', value }; }
      return { type: 'ident', value };
    }
    while (at < length) {
      const start = at, c = code(0); let token;
      if (c === 47 && code(1) === 42) { const end = indexOfText(text, '*/', at + 2); at = end < 0 ? length : end + 2; continue; }
      if (isSpace(c)) { while (isSpace(code(0))) at++; token = { type: 'whitespace' }; }
      else if (c === 34 || c === 39) token = consumeString(c);
      else if (c === 35) { if (isName(code(1)) || validEscape(code(1), code(2))) { at++; const unrestricted = !startsIdent(code(0), code(1), code(2)); token = { type: 'hash', value: name(), hashType: unrestricted ? 'unrestricted' : 'id' }; } else { at++; token = { type: 'delim', delim: '#' }; } }
      else if (c === 40) { at++; token = { type: '(' }; } else if (c === 41) { at++; token = { type: ')' }; }
      else if (c === 91) { at++; token = { type: '[' }; } else if (c === 93) { at++; token = { type: ']' }; }
      else if (c === 123) { at++; token = { type: '{' }; } else if (c === 125) { at++; token = { type: '}' }; }
      else if (c === 44) { at++; token = { type: 'comma' }; } else if (c === 58) { at++; token = { type: 'colon' }; } else if (c === 59) { at++; token = { type: 'semicolon' }; }
      else if (startsNumber(c, code(1), code(2))) {
        const parsed = number();
        if (startsIdent(code(0), code(1), code(2))) token = { type: 'dimension', value: parsed.value, integer: false, numberInteger: parsed.integer, unit: name() };
        else if (code(0) === 37) { at++; token = { type: 'percentage', value: parsed.value, integer: parsed.integer }; }
        else token = { type: 'number', value: parsed.value, integer: parsed.integer };
      }
      else if (c === 45 && code(1) === 45 && code(2) === 62) { at += 3; token = { type: 'cdc' }; }
      else if (c === 60 && code(1) === 33 && code(2) === 45 && code(3) === 45) { at += 4; token = { type: 'cdo' }; }
      else if (startsIdent(c, code(1), code(2))) token = identLike();
      else if (c === 64) { at++; if (startsIdent(code(0), code(1), code(2))) token = { type: 'at', value: name() }; else token = { type: 'delim', delim: '@' }; }
      else if (c === 92) { if (validEscape(c, code(1))) token = identLike(); else { at++; token = { type: 'delim', delim: '\\' }; } }
      else { const char = fromCode(charCodeFull(text, at)); at += char.length; token = { type: 'delim', delim: char }; }
      token.raw = sliceText(text, start, at);
      push(tokens, token);
    }
    return tokens;
  }
  const codePointAt = Function.prototype.call.bind(String.prototype.codePointAt);
  function charCodeFull(text, at) { return codePointAt(text, at); }
  const BLOCK_END = { __proto__: null, '(': ')', '[': ']', '{': '}', function: ')' };
  // Component values: tokens with blocks and functions grouped into {type, value, children}.
  function components(tokens) {
    let index = 0;
    function block(open) {
      const end = BLOCK_END[open.type], children = list();
      while (index < tokens.length) { const token = tokens[index]; if (token.type === end) { index++; return { __proto__: null, block: true, type: open.type, value: open.value, children, closed: true, open }; } push(children, component()); }
      return { __proto__: null, block: true, type: open.type, value: open.value, children, closed: false, open };
    }
    function component() { const token = tokens[index++]; if (BLOCK_END[token.type]) return block(token); return token; }
    const result = list();
    while (index < tokens.length) push(result, component());
    return result;
  }
  // The ASCII-lowercase identifier of a token, as Blink compares keyword ids.
  const ident = (token) => token && token.type === 'ident' ? lowerCase(token.value) : null;
  const trimSpace = (items) => { let start = 0, end = items.length; while (start < end && items[start].type === 'whitespace') start++; while (end > start && items[end - 1].type === 'whitespace') end--; const result = list(); for (let index = start; index < end; index++) push(result, items[index]); return result; };
  // Split component values at top-level commas (or any predicate), trimming whitespace.
  function splitComponents(items, separator) {
    const groups = list(); let current = list();
    for (let index = 0; index < items.length; index++) { const item = items[index]; if (item.type === separator) { push(groups, trimSpace(current)); current = list(); } else push(current, item); }
    push(groups, trimSpace(current));
    return groups;
  }
  const significant = (items) => { const result = list(); for (let index = 0; index < items.length; index++) if (items[index].type !== 'whitespace') push(result, items[index]); return result; };
  // ECMAScript Number::toString, as DoubleToStringConverter formats it.
  const shortNumber = (value) => '' + value;
  // cssom serialize-an-identifier.
  function serializeIdentifier(value) {
    let result = '';
    for (let index = 0; index < value.length; index++) {
      const c = charCode(value, index);
      if (c === 0) result += '�';
      else if ((c >= 1 && c <= 31) || c === 127 || (index === 0 && isDigit(c)) || (index === 1 && isDigit(c) && charCode(value, 0) === 45)) result += '\\' + lowerCase(c.toString(16)) + ' ';
      else if (index === 0 && c === 45 && value.length === 1) result += '\\' + value[index];
      else if (c >= 128 || c === 45 || c === 95 || isDigit(c) || (c >= 65 && c <= 90) || (c >= 97 && c <= 122)) result += value[index];
      else result += '\\' + value[index];
    }
    return result;
  }
  function serializeString(value) {
    let result = '"';
    for (let index = 0; index < value.length; index++) {
      const c = charCode(value, index);
      if (c === 0) result += '�'; else if ((c >= 1 && c <= 31) || c === 127) result += '\\' + lowerCase(c.toString(16)) + ' '; else if (c === 34 || c === 92) result += '\\' + value[index]; else result += value[index];
    }
    return result + '"';
  }
  // CSSParserToken::Serialize.
  function serializeToken(token) {
    switch (token.type) {
      case 'ident': return serializeIdentifier(token.value);
      case 'function': return serializeIdentifier(token.value) + '(';
      case 'at': return '@' + serializeIdentifier(token.value);
      case 'hash': return '#' + (token.hashType === 'unrestricted' ? serializeIdentifierRest(token.value) : serializeIdentifier(token.value));
      case 'url': return 'url(' + serializeIdentifier(token.value) + ')';
      case 'delim': return token.delim === '\\' ? '\\\n' : token.delim;
      case 'number': { if (token.integer) return '' + mathTrunc(token.value); const text = shortNumber(token.value); return indexOfText(text, '.') < 0 && indexOfText(text, 'e') < 0 ? text + '.0' : text; }
      case 'percentage': return shortNumber(token.value) + '%';
      case 'dimension': return shortNumber(token.value) + serializeIdentifier(token.unit);
      case 'string': return serializeString(token.value);
      case 'bad-string': return "'\n";
      case 'bad-url': return 'url(()';
      case 'whitespace': return ' ';
      case 'colon': return ':'; case 'semicolon': return ';'; case 'comma': return ',';
      case 'cdo': return '<!--'; case 'cdc': return '-->';
      default: return token.type;
    }
  }
  function serializeIdentifierRest(value) { const text = serializeIdentifier('a' + value); return sliceText(text, 1); }
  function needsComment(a, b) {
    const at = a.type, bt = b.type;
    if (at === 'ident' || at === 'at' || at === 'hash' || at === 'dimension' || at === 'number' || (at === 'delim' && (a.delim === '#' || a.delim === '-'))) {
      if (at === 'ident' && bt === '(') return true;
      if (at === 'number' && bt === 'delim') { if (b.delim === '-') return false; if (b.delim === '%') return true; }
      return bt === 'ident' || bt === 'function' || bt === 'url' || bt === 'bad-url' || bt === 'number' || bt === 'percentage' || bt === 'dimension' || bt === 'cdc' || (bt === 'delim' && b.delim === '-');
    }
    if (at === 'delim' && a.delim === '@') return bt === 'ident' || bt === 'function' || bt === 'url' || bt === 'bad-url' || bt === 'cdc' || (bt === 'delim' && b.delim === '-');
    if (at === 'delim' && (a.delim === '.' || a.delim === '+')) return bt === 'number' || bt === 'percentage' || bt === 'dimension';
    return at === 'delim' && bt === 'delim' && a.delim === '/' && b.delim === '*';
  }

  // ---- Calc expression trees (Blink CSSMathExpressionNode) -------------------------------------
  // lit: {t:'lit', value, unit}; op: {t:'op', op, operands, type}; kw: {t:'kw', id}. `nested`
  // marks a parenthesized/calc() subtree.
  const ADD = 'add', SUB = 'sub', MUL = 'mul', DIV = 'div';
  const lit = (value, unitKey) => ({ __proto__: null, t: 'lit', value, unit: unitKey, nested: false });
  const nest = (node) => { const copy = { __proto__: null }; const keys = ownKeys(node); for (let index = 0; index < keys.length; index++) copy[keys[index]] = node[keys[index]]; copy.nested = true; return copy; };
  const unnest = (node) => { if (!node.nested) return node; const copy = nest(node); copy.nested = false; return copy; };
  const nodeType = (node) => node.t === 'lit' ? makeType(node.unit) : node.type;
  // CalculationResultCategory from the node's numeric type.
  const CATEGORY_BASES = plain([plain([BASE.length, 'length']), plain([BASE.angle, 'angle']), plain([BASE.time, 'time']), plain([BASE.frequency, 'frequency']), plain([BASE.resolution, 'resolution'])]);
  function category(node) {
    if (node.t === 'kw') return 'ident';
    const type = nodeType(node);
    if (!type) return 'other';
    if (matchesNumber(type)) return 'number';
    if (onlyEntry(type, BASE.percent, 1) && type.hint < 0) return 'percent';
    for (let index = 0; index < CATEGORY_BASES.length; index++) {
      const base = CATEGORY_BASES[index][0], name = CATEGORY_BASES[index][1];
      if (onlyEntry(type, base, 1)) return type.hint >= 0 ? (base === BASE.length ? 'length-percent' : base === BASE.angle ? 'percent-angle' : 'intermediate') : name;
    }
    return 'intermediate';
  }
  const isAddOrSub = (node) => node.t === 'op' && (node.op === ADD || node.op === SUB);
  const isMulOrDiv = (node) => node.t === 'op' && (node.op === MUL || node.op === DIV);
  const isMathFunction = (node) => node.t === 'op' && (node.op === 'min' || node.op === 'max' || node.op === 'clamp' || node.op === 'fn');
  const literalNumber = (node) => node.t === 'lit' && node.unit === 'number';
  // HasNestedIntermediateResult: an operand subtree whose type is an intermediate (e.g. px*px).
  function nestedIntermediate(node) { if (node.t !== 'op') return false; if (category(node) === 'intermediate') return true; for (let index = 0; index < node.operands.length; index++) if (nestedIntermediate(node.operands[index])) return true; return false; }
  const canonicalValue = (node) => node.t === 'lit' && canonicalUnit(node.unit) ? node.value * UNITS[node.unit].factor : null;
  function canEagerlySimplify(node) {
    if (node.t !== 'lit') return false;
    const kind = category(node);
    if (kind === 'number' || kind === 'angle' || kind === 'time' || kind === 'frequency' || kind === 'resolution') return canonicalValue(node) !== null;
    if (kind === 'length') return UNITS[node.unit].category === 'length';
    return false;
  }
  function createOperation(left, right, op) {
    const leftType = nodeType(left), rightType = nodeType(right);
    if (!leftType || !rightType || left.t === 'kw' || right.t === 'kw') return null;
    let type = op === ADD || op === SUB ? addTypes(leftType, rightType) : op === MUL ? multiplyTypes(leftType, rightType) : multiplyTypes(leftType, negateType(rightType));
    if (!type) return null;
    if (op === DIV) {
      const inverted = invertNode(right);
      if (category(left) === 'number' && left.t === 'lit' && left.value === 1) return inverted;
      op = MUL; right = inverted;
    }
    return { __proto__: null, t: 'op', op, operands: [left, right], type, nested: false };
  }
  function invertNode(operand) {
    if (operand.t === 'lit' && category(operand) === 'number') { const value = canonicalValue(operand); if (value !== null && value !== 0) return lit(1 / value, 'number'); }
    if (operand.t === 'op' && operand.op === 'invert') return unnest(operand.operands[0]);
    return { __proto__: null, t: 'op', op: 'invert', operands: [operand], type: negateType(nodeType(operand)), nested: false };
  }
  function evaluate(values, op) {
    switch (op) {
      case ADD: return values[0] + values[1];
      case SUB: return values[0] - values[1];
      case MUL: return values[0] * values[1];
      case DIV: return values[0] / values[1];
      case 'min': { let minimum = values[0]; for (let index = 0; index < values.length; index++) { const operand = values[index]; if (minimum === 0 && operand === 0 && (1 / operand < 0)) minimum = operand; else minimum = isNaNValue(operand) || isNaNValue(minimum) ? NaN : mathMin(minimum, operand); } return minimum; }
      case 'max': { let maximum = values[0]; for (let index = 0; index < values.length; index++) { const operand = values[index]; if (maximum === 0 && operand === 0 && (1 / operand > 0)) maximum = operand; else maximum = isNaNValue(operand) || isNaNValue(maximum) ? NaN : mathMax(maximum, operand); } return maximum; }
      case 'clamp': { const minimum = evaluate([values[1], values[2]], 'min'); return evaluate([values[0], minimum], 'max'); }
    }
    return NaN;
  }
  function canSimplifyArithmetic(left, right, op) {
    if (left.t !== 'lit' || right.t !== 'lit') return false;
    if ((op === MUL || op === DIV) && category(left) !== 'number' && category(right) !== 'number') return false;
    const type = op === ADD || op === SUB ? addTypes(nodeType(left), nodeType(right)) : op === MUL ? multiplyTypes(nodeType(left), nodeType(right)) : multiplyTypes(nodeType(left), negateType(nodeType(right)));
    return !(type && category({ t: 'op', type }) === 'intermediate');
  }
  function distribute(left, right, op) {
    if (op !== MUL && op !== DIV) return null;
    if (category(left) === 'intermediate' || category(right) === 'intermediate') return null;
    const allLiterals = (node) => node.operands[0].t === 'lit' && node.operands[1].t === 'lit';
    if (isAddOrSub(left) && allLiterals(left) && right.t === 'lit' && category(right) === 'number') {
      const a = simplified(left.operands[0], right, op), b = simplified(left.operands[1], right, op);
      if (!a || !b) return null;
      const result = simplified(a, b, left.op);
      return result ? nest(result) : null;
    }
    if (isAddOrSub(right) && allLiterals(right) && left.t === 'lit' && category(left) === 'number' && op !== DIV) {
      const a = simplified(left, right.operands[0], op), b = simplified(left, right.operands[1], op);
      if (!a || !b) return null;
      const result = simplified(a, b, right.op);
      return result ? nest(result) : null;
    }
    return null;
  }
  // CreateArithmeticOperationSimplified.
  function simplified(left, right, op) {
    if (left.t === 'kw' || right.t === 'kw') return null;
    const distributed = distribute(left, right, op);
    if (distributed) return distributed;
    if (!canSimplifyArithmetic(left, right, op)) return createOperation(left, right, op);
    const leftCategory = category(left), rightCategory = category(right);
    if (leftCategory === 'number' && rightCategory === 'number') return lit(evaluate([left.value, right.value], op), 'number');
    if (op === ADD || op === SUB) {
      if (leftCategory === rightCategory) {
        if (left.unit === right.unit) return lit(evaluate([left.value, right.value], op), left.unit);
        const leftUnit = UNITS[left.unit], rightUnit = UNITS[right.unit];
        if (leftUnit.category !== 'other' && leftUnit.category !== 'percent' && leftUnit.category === rightUnit.category) {
          const canonical = CANONICAL[leftUnit.category];
          if (canonical) return lit(evaluate([left.value * leftUnit.factor, right.value * rightUnit.factor], op), canonical);
        }
      }
    } else {
      if (rightCategory !== 'number' && op === DIV) return null;
      const numberSide = literalNumber(left) ? left : literalNumber(right) ? right : null;
      if (!numberSide) return createOperation(left, right, op);
      const other = numberSide === left ? right : left;
      return lit(evaluate([other.value, numberSide.value], op), other.unit);
    }
    return createOperation(left, right, op);
  }
  // Collect the terms of an add/sub (or mul) tree, flipping signs inside nested subtractions.
  function traverse(root, op, visit) {
    const pending = list();
    const should = (node, operator) => node.t === 'op' && !nestedIntermediate(node) && (operator === MUL ? isMulOrDiv(node) : isAddOrSub(node));
    const enqueue = (node, operator, nesting) => {
      nesting = nesting || node.nested;
      let backOp = node.op;
      if (nesting && operator === SUB && node.op === ADD) backOp = SUB; else if (nesting && operator === SUB && node.op === SUB) backOp = ADD;
      push(pending, { node: node.operands[1], op: backOp, nesting });
      push(pending, { node: node.operands[0], op: operator, nesting });
    };
    if (!should(root, op)) { visit(root, op); return; }
    enqueue(root, op, false);
    while (pending.length) {
      const last = pending[pending.length - 1]; pending.length--;
      if (!should(last.node, last.op)) { visit(last.node, last.op); continue; }
      enqueue(last.node, last.op, last.nesting);
    }
  }
  // MaybeSimplifySumOrProductNode.
  function simplifySumOrProduct(root) {
    const multiply = isMulOrDiv(root), combined = create(null), children = list();
    traverse(root, multiply ? MUL : ADD, (node, op) => {
      if (node.t === 'lit') {
        const value = op === SUB ? -node.value : node.value;
        if (node.unit in combined) combined[node.unit] = op === MUL ? combined[node.unit] * value : combined[node.unit] + value; else combined[node.unit] = value;
      }
      push(children, { node, op });
    });
    let factor = 1;
    if (multiply && 'number' in combined) factor = combined.number;
    const used = create(null); let result = null;
    for (let index = 0; index < children.length; index++) {
      let { node, op } = children[index];
      if (multiply && node.t === 'lit' && node.unit === 'number') continue;
      if (node.t === 'lit' && node.unit in combined) {
        let value = combined[node.unit];
        if (multiply) { value *= factor; factor = 1; } else { op = (value < 0 || (value === 0 && 1 / value < 0)) ? SUB : ADD; value = mathAbs(value); }
        node = lit(value, node.unit);
      }
      if (node.t === 'lit') { if (used[node.unit]) continue; used[node.unit] = true; }
      if (!result) { result = op === SUB && node.t === 'lit' ? lit(-node.value, node.unit) : unnest(node); continue; }
      result = simplified(result, node, op);
      if (!result) return null;
    }
    if (multiply) {
      if (!result) result = lit(factor, 'number');
      else if (factor !== 1) { result = simplified(result, lit(factor, 'number'), MUL); if (!result) return null; }
    }
    return nest(result);
  }
  const maybeSimplifySumOrProduct = (node) => node && (isAddOrSub(node) || isMulOrDiv(node)) && !nestedIntermediate(node) ? simplifySumOrProduct(node) : node;
  function createComparison(operands, op) {
    if (op === 'clamp') {
      const low = operands[0], value = operands[1], high = operands[2];
      if (value.t === 'kw') return null;
      if (low.t === 'kw' && high.t === 'kw') return unnest(value);
      if (low.t === 'kw') return createComparison([value, high], 'min');
      if (high.t === 'kw') return createComparison([low, value], 'max');
    }
    let type = null;
    for (let index = 0; index < operands.length; index++) { if (operands[index].t === 'kw') return null; type = index ? addTypes(type, nodeType(operands[index])) : nodeType(operands[index]); if (!type) return null; }
    let eager = true;
    for (let index = 0; index < operands.length; index++) if (!canEagerlySimplify(operands[index])) { eager = false; break; }
    if (eager) {
      const values = list();
      for (let index = 0; index < operands.length; index++) push(values, canonicalValue(operands[index]));
      return lit(evaluate(values, op), canonicalUnit(operands[0].unit));
    }
    if (operands.length === 1) return unnest(operands[0]);
    return { __proto__: null, t: 'op', op, operands, type, nested: false };
  }
  // Remaining css-values-4 math functions: eagerly evaluated over canonical literals, otherwise
  // an opaque function node that Typed OM cannot reify.
  function createFunction(name, operands) {
    for (let index = 0; index < operands.length; index++) if (operands[index].t === 'kw') return null;
    const types = list(); for (let index = 0; index < operands.length; index++) push(types, nodeType(operands[index]));
    const same = () => { let type = types[0]; for (let index = 1; index < types.length; index++) { type = addTypes(type, types[index]); if (!type) return null; } return type; };
    let type;
    switch (name) {
      case 'sin': case 'cos': case 'tan': { const kind = category(operands[0]); if (kind !== 'number' && kind !== 'angle') return null; type = makeType('number'); break; }
      case 'asin': case 'acos': case 'atan': if (category(operands[0]) !== 'number') return null; type = makeType('deg'); break;
      case 'atan2': if (!same()) return null; type = makeType('deg'); break;
      case 'pow': if (category(operands[0]) !== 'number' || category(operands[1]) !== 'number') return null; type = makeType('number'); break;
      case 'sqrt': case 'exp': if (category(operands[0]) !== 'number') return null; type = makeType('number'); break;
      case 'log': for (let index = 0; index < operands.length; index++) if (category(operands[index]) !== 'number') return null; type = makeType('number'); break;
      case 'hypot': case 'mod': case 'rem': case 'round': type = same(); if (!type) return null; break;
      case 'abs': type = copyType(types[0]); break;
      case 'sign': type = makeType('number'); break;
      default: return null;
    }
    let eager = true;
    for (let index = 0; index < operands.length; index++) if (!canEagerlySimplify(operands[index])) { eager = false; break; }
    if (eager) {
      const values = list(); for (let index = 0; index < operands.length; index++) push(values, canonicalValue(operands[index]));
      const unitOf = canonicalUnit(operands[0].unit);
      const radians = (index) => operands[index].unit === 'number' ? values[index] : values[index] * mathPI / 180;
      switch (name) {
        case 'sin': return lit(mathSin(radians(0)), 'number');
        case 'cos': return lit(mathCos(radians(0)), 'number');
        case 'tan': return lit(mathTan(radians(0)), 'number');
        case 'asin': return lit(mathAsin(values[0]) * 180 / mathPI, 'deg');
        case 'acos': return lit(mathAcos(values[0]) * 180 / mathPI, 'deg');
        case 'atan': return lit(mathAtan(values[0]) * 180 / mathPI, 'deg');
        case 'atan2': return lit(mathAtan2(values[0], values[1]) * 180 / mathPI, 'deg');
        case 'pow': return lit(mathPow(values[0], values[1]), 'number');
        case 'sqrt': return lit(mathSqrt(values[0]), 'number');
        case 'exp': return lit(mathExp(values[0]), 'number');
        case 'log': return lit(values.length > 1 ? mathLog(values[0]) / mathLog(values[1]) : mathLog(values[0]), 'number');
        case 'hypot': { let total = 0; for (let index = 0; index < values.length; index++) total += values[index] * values[index]; return lit(mathSqrt(total), unitOf); }
        case 'abs': return lit(mathAbs(values[0]), unitOf);
        case 'sign': return lit(mathSign(values[0]), 'number');
        case 'mod': { const a = values[0], b = values[1]; return lit(b === 0 ? NaN : a - b * mathFloor(a / b), unitOf); }
        case 'rem': { const a = values[0], b = values[1]; return lit(b === 0 ? NaN : a % b, unitOf); }
      }
    }
    return { __proto__: null, t: 'op', op: 'fn', name, operands, type, nested: false };
  }
  // Rounding strategies parse as a leading keyword argument.
  const ROUNDING = { __proto__: null, nearest: 1, up: 1, down: 1, 'to-zero': 1 };
  function createRound(strategy, operands) {
    if (operands.length === 1) { if (category(operands[0]) !== 'number') return null; operands = [operands[0], lit(1, 'number')]; }
    const type = addTypes(nodeType(operands[0]), nodeType(operands[1]));
    if (!type) return null;
    if (canEagerlySimplify(operands[0]) && canEagerlySimplify(operands[1])) {
      const a = canonicalValue(operands[0]), b = canonicalValue(operands[1]), unitOf = canonicalUnit(operands[0].unit);
      if (b === 0) return lit(NaN, unitOf);
      const lower = mathFloor(a / b) * b, upper = mathCeil(a / b) * b;
      let value;
      switch (strategy) { case 'up': value = upper; break; case 'down': value = lower; break; case 'to-zero': value = mathAbs(lower) < mathAbs(upper) ? lower : upper; break; default: value = mathAbs(a - lower) < mathAbs(upper - a) ? lower : upper; }
      return lit(value, unitOf);
    }
    return { __proto__: null, t: 'op', op: 'fn', name: 'round', strategy, operands, type, nested: false };
  }
  const MATH_FUNCTIONS = { __proto__: null, calc: 1, '-webkit-calc': 1, min: 1, max: 1, clamp: 1, sin: 1, cos: 1, tan: 1, asin: 1, acos: 1, atan: 1, atan2: 1, pow: 1, sqrt: 1, hypot: 1, log: 1, exp: 1, round: 1, mod: 1, rem: 1, abs: 1, sign: 1 };
  const CONSTANTS = { __proto__: null, infinity: Infinity, '-infinity': -Infinity, nan: NaN, pi: mathPI, e: mathE };
  // Recursive-descent parse over component values of one math function's arguments.
  function parseMathFunction(name, children, depth) {
    if (!MATH_FUNCTIONS[name] || depth > 100) return null;
    let items = list(); for (let index = 0; index < children.length; index++) push(items, children[index]);
    let at = 0;
    const skip = () => { while (at < items.length && items[at].type === 'whitespace') at++; };
    let strategy = null;
    skip();
    if (name === 'round' && at < items.length && items[at].type === 'ident' && ROUNDING[lowerCase(items[at].value)]) { strategy = lowerCase(items[at].value); at++; skip(); if (at >= items.length || items[at].type !== 'comma') return null; at++; }
    const allowNone = name === 'clamp';
    const maximum = name === 'calc' || name === '-webkit-calc' || name === 'sin' || name === 'cos' || name === 'tan' || name === 'asin' || name === 'acos' || name === 'atan' || name === 'sqrt' || name === 'exp' || name === 'abs' || name === 'sign' ? 1 : name === 'clamp' ? 3 : name === 'pow' || name === 'atan2' || name === 'mod' || name === 'rem' || name === 'log' || name === 'round' ? 2 : 100;
    const minimum = name === 'clamp' ? 3 : name === 'pow' || name === 'atan2' || name === 'mod' || name === 'rem' ? 2 : 1;
    const nodes = list();
    while (at < items.length && nodes.length < maximum) {
      if (nodes.length) { skip(); if (at >= items.length || items[at].type !== 'comma') return null; at++; }
      skip();
      const state = { items, at, depth: depth + 1, allowNone };
      const node = parseSum(state);
      if (!node) return null;
      at = state.at; skip();
      if (at < items.length && items[at].type !== 'comma') return null;
      push(nodes, node);
    }
    skip();
    if (at < items.length || nodes.length < minimum) return null;
    switch (name) {
      case 'calc': case '-webkit-calc': return nodes[0];
      case 'min': case 'max': case 'clamp': return createComparison(nodes, name);
      case 'round': return createRound(strategy || 'nearest', nodes);
      default: return createFunction(name, nodes);
    }
  }
  function parseTerm(state, info) {
    const items = state.items;
    if (state.at >= items.length) return null;
    const item = items[state.at];
    let result = null;
    if ((item.block && item.type === '(') || (item.block && item.type === 'function' && lowerCase(item.value) === 'calc')) {
      const inner = { items: item.children, at: 0, depth: state.depth + 1, allowNone: false };
      while (inner.at < inner.items.length && inner.items[inner.at].type === 'whitespace') inner.at++;
      result = parseSum(inner);
      if (!result) return null;
      while (inner.at < inner.items.length && inner.items[inner.at].type === 'whitespace') inner.at++;
      if (inner.at < inner.items.length || result.t === 'kw') return null;
      result = nest(result); state.at++;
    } else if (item.block && item.type === 'function') {
      result = parseMathFunction(lowerCase(item.value), item.children, state.depth + 1); state.at++;
      if (!result) return null;
    } else if (item.block) return null;
    else {
      state.at++;
      const id = ident(item);
      if (id !== null && id in CONSTANTS) result = lit(CONSTANTS[id], 'number');
      else if (id === 'none' && state.allowNone) result = { __proto__: null, t: 'kw', id: 'none', nested: false };
      else if (item.type === 'number') result = lit(item.value, 'number');
      else if (item.type === 'percentage') result = lit(item.value, 'percent');
      else if (item.type === 'dimension') { const key = lowerCase(item.unit); if (!UNITS[key] || key === 'number' || key === 'percent' || key === 'fr') return null; result = lit(item.value, key); }
      else return null;
    }
    info.space = state.at < items.length && items[state.at].type === 'whitespace';
    while (state.at < items.length && items[state.at].type === 'whitespace') state.at++;
    return result;
  }
  function parseProduct(state, info) {
    let result = parseTerm(state, info);
    if (!result) return null;
    for (;;) {
      const item = state.items[state.at];
      if (!item || item.type !== 'delim' || (item.delim !== '*' && item.delim !== '/')) break;
      state.at++;
      while (state.at < state.items.length && state.items[state.at].type === 'whitespace') state.at++;
      const right = parseTerm(state, info);
      if (!right) return null;
      result = simplified(result, right, item.delim === '*' ? MUL : DIV);
      if (!result) return null;
    }
    if (isMulOrDiv(result) && !nestedIntermediate(result)) result = simplifySumOrProduct(result);
    return result;
  }
  function parseSum(state) {
    const info = { space: false };
    let result = parseProduct(state, info);
    if (!result) return null;
    for (;;) {
      const item = state.items[state.at];
      if (!item || item.type !== 'delim' || (item.delim !== '+' && item.delim !== '-')) break;
      if (!info.space) return null;
      state.at++;
      const after = state.items[state.at];
      if (!after || after.type !== 'whitespace') return null;
      while (state.at < state.items.length && state.items[state.at].type === 'whitespace') state.at++;
      const right = parseProduct(state, info);
      if (!right) return null;
      result = simplified(result, right, item.delim === '+' ? ADD : SUB);
      if (!result) return null;
    }
    if (isAddOrSub(result) && !nestedIntermediate(result)) result = simplifySumOrProduct(result);
    return result;
  }
  // A whole math function component as CSSMathExpressionNode::ParseMathFunction sees it (the
  // property parser's view: `calc(1px + 2px)` becomes the literal 3px).
  function parseMathComponent(component) {
    if (!component || !component.block || component.type !== 'function') return null;
    const node = parseMathFunction(lowerCase(component.value), component.children, 0);
    if (!node || node.t === 'kw') return null;
    const kind = category(node);
    return kind === 'other' || kind === 'ident' ? null : node;
  }

  // CSSMathExpressionNode::CustomCSSText with the sort-a-calculation's-children ordering.
  function sortedTerms(root) {
    const multiply = isMulOrDiv(root), byUnit = create(null), unitOrder = list(), complex = list();
    traverse(root, multiply ? MUL : ADD, (node, op) => {
      if (node.t === 'lit') { if (!(node.unit in byUnit)) { byUnit[node.unit] = list(); push(unitOrder, node.unit); } push(byUnit[node.unit], { node, op }); }
      else push(complex, { node, op });
    });
    const result = list();
    const take = (key) => { if (key in byUnit) { const entries = byUnit[key]; for (let index = 0; index < entries.length; index++) push(result, entries[index]); delete byUnit[key]; } };
    take('number'); take('percent');
    const keys = list(); for (let index = 0; index < unitOrder.length; index++) if (unitOrder[index] in byUnit) push(keys, unitOrder[index]);
    for (let index = 1; index < keys.length; index++) { const key = keys[index]; let at = index - 1; while (at >= 0 && keys[at] > key) { keys[at + 1] = keys[at]; at--; } keys[at + 1] = key; }
    for (let index = 0; index < keys.length; index++) take(keys[index]);
    for (let index = 0; index < complex.length; index++) push(result, complex[index]);
    return result;
  }
  function nodeText(node) {
    if (node.t === 'lit') return formatNumber(node.value, node.unit);
    if (node.t === 'kw') return node.id;
    switch (node.op) {
      case ADD: case SUB: case MUL: {
        let terms = sortedTerms(node);
        if (terms.length === 1) terms = [{ op: MUL, node: node.operands[0] }, { op: node.op, node: node.operands[1] }];
        const first = terms[0];
        let text = '(' + nodeText(first.op === SUB && first.node.t === 'lit' ? lit(-first.node.value, first.node.unit) : first.node);
        for (let index = 1; index < terms.length; index++) {
          let { op, node: term } = terms[index];
          if (op === MUL && term.t === 'op' && term.op === 'invert') { text += ' / ' + nodeText(term.operands[0]); continue; }
          if (term.t === 'lit' && (op === ADD || op === SUB) && term.value < 0) { op = op === ADD ? SUB : ADD; term = lit(-term.value, term.unit); }
          text += ' ' + (op === ADD ? '+' : op === SUB ? '-' : '*') + ' ' + nodeText(term);
        }
        return text + ')';
      }
      case 'invert': return '(1 / ' + nodeText(node.operands[0]) + ')';
      case 'fn': {
        let text = node.name + '(';
        if (node.name === 'round' && node.strategy && node.strategy !== 'nearest') text += node.strategy + ', ';
        const operands = node.name === 'round' && node.operands[1].t === 'lit' && node.operands[1].value === 1 && node.operands[1].unit === 'number' ? [node.operands[0]] : node.operands;
        for (let index = 0; index < operands.length; index++) text += (index ? ', ' : '') + topLevelText(operands[index]);
        return text + ')';
      }
      default: {
        let text = node.op + '(';
        for (let index = 0; index < node.operands.length; index++) text += (index ? ', ' : '') + topLevelText(node.operands[index]);
        return text + ')';
      }
    }
  }
  function topLevelText(node) { const text = nodeText(node); return startsWith(text, '(') ? sliceText(text, 1, text.length - 1) : text; }
  // CSSMathFunctionValue::CustomCSSText.
  function mathFunctionText(node) { if (isMathFunction(node)) return nodeText(node); const text = nodeText(node); return startsWith(text, '(') ? 'calc' + text : 'calc(' + text + ')'; }

  // CSSNumericValue reification of a calc tree (CalcToNumericValue).
  function nodeToNumeric(node) {
    if (node.t === 'lit') { const value = unitValue(node.value, node.unit); return node.nested ? variadic('sum', toArray([value]), copyType(numericRecord(value).type)) : value; }
    if (node.t !== 'op') return null;
    if (node.op === 'invert') { const value = nodeToNumeric(node.operands[0]); return value ? createInvert(value) : null; }
    if (node.op === 'min' || node.op === 'max' || node.op === 'clamp') {
      const values = list();
      for (let index = 0; index < node.operands.length; index++) { const value = nodeToNumeric(node.operands[index]); if (!value) return null; push(values, value); }
      return node.op === 'clamp' ? createClamp(values[0], values[1], values[2]) : createVariadic(node.op, toArray(values));
    }
    if (node.op === 'fn') return null;
    const values = list(); let current = node;
    const combine = (candidate) => candidate.t === 'op' && !candidate.nested && !isMathFunction(candidate) && candidate.op !== 'invert' && ((node.op === ADD || node.op === SUB) ? isAddOrSub(candidate) : isMulOrDiv(candidate));
    do {
      const right = nodeToNumeric(current.operands[1]);
      if (!right) return null;
      push(values, current.op === SUB ? createNegate(right) : current.op === DIV ? createInvert(right) : right);
      current = current.operands[0];
    } while (combine(current));
    const first = nodeToNumeric(current);
    if (!first) return null;
    push(values, first);
    const ordered = list(); for (let index = values.length - 1; index >= 0; index--) push(ordered, values[index]);
    return createVariadic(node.op === ADD || node.op === SUB ? 'sum' : 'product', toArray(ordered));
  }
  // CSSNumericValue::ToCalcExpressionNode.
  function numericToNode(value) {
    const record = numericRecord(value);
    switch (record.kind) {
      case 'unit': return lit(record.value, record.unit);
      case 'sum': case 'product': {
        let node = numericToNode(record.values[0]);
        for (let index = 1; index < record.values.length; index++) { const next = numericToNode(record.values[index]); node = node && next ? createOperation(node, next, record.kind === 'sum' ? ADD : MUL) : null; }
        return node;
      }
      case 'negate': { const inner = numericToNode(record.value); return inner ? simplified(lit(-1, 'number'), inner, MUL) : null; }
      case 'invert': { const inner = numericToNode(record.value); return inner ? createOperation(lit(1, 'number'), inner, DIV) : null; }
      case 'min': case 'max': { const operands = list(); for (let index = 0; index < record.values.length; index++) push(operands, numericToNode(record.values[index])); return createComparison(operands, record.kind); }
      case 'clamp': return createComparison([numericToNode(record.lower), numericToNode(record.value), numericToNode(record.upper)], 'clamp');
    }
    return null;
  }
  // CSSStyleValue::toString for a numeric value placed in a declaration (ToCSSValue()->CssText()).
  function numericCssText(value) { const record = numericRecord(value); if (record.kind === 'unit') return formatNumber(record.value, record.unit); const node = numericToNode(value); return node ? mathFunctionText(node) : ''; }

  // ---- Interfaces: CSSStyleValue and numerics ---------------------------------------------------
  defineInterface('CSSStyleValue', null, 0, null);
  defineInterface('CSSNumericValue', 'CSSStyleValue', 0, null);
  defineInterface('CSSUnitValue', 'CSSNumericValue', 2, function (value, unitText) {
    requireArguments(arguments, 2);
    const number = toDouble(value), text = toStringValue(unitText), key = unitFromName(text);
    if (!key) throw fail('Invalid unit: ' + text);
    return attach(this, { family: 'CSSUnitValue', numeric: true, kind: 'unit', value: number, unit: key, type: makeType(key), text: null });
  });
  defineInterface('CSSMathValue', 'CSSNumericValue', 0, null);
  function variadicConstructor(kind, name) {
    return function () {
      if (!arguments.length) throw fail("Arguments can't be empty", 'SyntaxError');
      const values = numberishes(arguments), type = typeCheck(values, kind === 'product' ? multiplyTypes : addTypes);
      if (!type) throw fail('Incompatible types');
      return attach(this, { family: name, numeric: true, kind, values: toArray(values), array: numericArray(toArray(values)), type, text: null });
    };
  }
  defineInterface('CSSMathSum', 'CSSMathValue', 0, variadicConstructor('sum', 'CSSMathSum'));
  defineInterface('CSSMathProduct', 'CSSMathValue', 0, variadicConstructor('product', 'CSSMathProduct'));
  defineInterface('CSSMathNegate', 'CSSMathValue', 1, function (value) {
    requireArguments(arguments, 1);
    const argument = fromNumberish(value);
    return attach(this, { family: 'CSSMathNegate', numeric: true, kind: 'negate', value: argument, type: copyType(numericRecord(argument).type), text: null });
  });
  defineInterface('CSSMathInvert', 'CSSMathValue', 1, function (value) {
    requireArguments(arguments, 1);
    const argument = fromNumberish(value);
    return attach(this, { family: 'CSSMathInvert', numeric: true, kind: 'invert', value: argument, type: negateType(numericRecord(argument).type), text: null });
  });
  defineInterface('CSSMathMin', 'CSSMathValue', 0, variadicConstructor('min', 'CSSMathMin'));
  defineInterface('CSSMathMax', 'CSSMathValue', 0, variadicConstructor('max', 'CSSMathMax'));
  defineInterface('CSSMathClamp', 'CSSMathValue', 3, function (lower, value, upper) {
    requireArguments(arguments, 3);
    const a = fromNumberish(lower), b = fromNumberish(value), c = fromNumberish(upper);
    const created = createClamp(a, b, c);
    if (!created) throw fail('Incompatible types');
    return attach(this, { family: 'CSSMathClamp', numeric: true, kind: 'clamp', lower: a, value: b, upper: c, type: numericRecord(created).type, text: null });
  });
  defineInterface('CSSNumericArray', null, 0, null);
  const STYLE = brandSets.CSSStyleValue;
  // CSSStyleValue.prototype.toString
  stringifier('CSSStyleValue', function (self) { return styleText(brand(self, STYLE), self); });
  // CSSNumericValue arithmetic: the receiver is prepended, flattening a receiver of the same kind.
  method('CSSNumericValue', 'add', 0, function () { const self = brand(this, brandSets.CSSNumericValue) && this; const values = prependForArithmetic('sum', numberishes(arguments), self); return simplifyAsUnit(values, (a, b) => a + b) || incompatible(createVariadic('sum', toArray(values))); });
  method('CSSNumericValue', 'sub', 0, function () { const self = brand(this, brandSets.CSSNumericValue) && this; const values = numberishes(arguments); for (let index = 0; index < values.length; index++) values[index] = negateValue(values[index]); const all = prependForArithmetic('sum', values, self); return simplifyAsUnit(all, (a, b) => a + b) || incompatible(createVariadic('sum', toArray(all))); });
  method('CSSNumericValue', 'mul', 0, function () { const self = brand(this, brandSets.CSSNumericValue) && this; const values = prependForArithmetic('product', numberishes(arguments), self); return multiplyAsUnit(values) || incompatible(createVariadic('product', toArray(values))); });
  method('CSSNumericValue', 'div', 0, function () {
    const self = brand(this, brandSets.CSSNumericValue) && this; const values = numberishes(arguments);
    for (let index = 0; index < values.length; index++) { const inverted = invertValue(values[index]); if (!inverted) throw fail("Can't divide-by-zero", RangeType); values[index] = inverted; }
    const all = prependForArithmetic('product', values, self);
    return multiplyAsUnit(all) || incompatible(createVariadic('product', toArray(all)));
  });
  method('CSSNumericValue', 'min', 0, function () { const self = brand(this, brandSets.CSSNumericValue) && this; const values = prependForArithmetic('min', numberishes(arguments), self); return simplifyAsUnit(values, (a, b) => mathMin(a, b)) || incompatible(createVariadic('min', toArray(values))); });
  method('CSSNumericValue', 'max', 0, function () { const self = brand(this, brandSets.CSSNumericValue) && this; const values = prependForArithmetic('max', numberishes(arguments), self); return simplifyAsUnit(values, (a, b) => mathMax(a, b)) || incompatible(createVariadic('max', toArray(values))); });
  method('CSSNumericValue', 'equals', 0, function () { brand(this, brandSets.CSSNumericValue); const values = numberishes(arguments); for (let index = 0; index < values.length; index++) if (!equalsValue(this, values[index])) return false; return true; });
  method('CSSNumericValue', 'to', 1, function (unitText) {
    brand(this, brandSets.CSSNumericValue); requireArguments(arguments, 1);
    const text = toStringValue(unitText), key = unitFromName(text);
    if (!key) throw fail('Invalid unit for conversion', 'SyntaxError');
    const result = toUnit(this, key);
    if (!result) throw fail('Cannot convert to ' + text);
    return result;
  });
  method('CSSNumericValue', 'toSum', 0, function () {
    brand(this, brandSets.CSSNumericValue);
    const units = list();
    for (let index = 0; index < arguments.length; index++) { const text = toStringValue(arguments[index]); push(units, text); }
    for (let index = 0; index < units.length; index++) if (!unitFromName(units[index])) throw fail('Invalid unit for conversion', 'SyntaxError');
    const sum = sumValue(this);
    if (!sum) throw fail('Invalid value for conversion');
    const values = list();
    for (let index = 0; index < sum.length; index++) { const value = termToUnit(sum[index]); if (!value) throw fail('Invalid value for conversion'); push(values, value); }
    if (!units.length) {
      const unitString = (value) => { const key = numericRecord(value).unit; return key; };
      for (let index = 1; index < values.length; index++) { const value = values[index]; let at = index - 1; while (at >= 0 && unitString(values[at]) > unitString(value)) { values[at + 1] = values[at]; at--; } values[at + 1] = value; }
      return createVariadic('sum', toArray(values));
    }
    const result = list();
    for (let index = 0; index < units.length; index++) {
      const target = unitFromName(units[index]); let total = 0;
      for (let at = 0; at < values.length; at++) { if (!values[at]) continue; const converted = convertUnit(values[at], target); if (converted) { total += numericRecord(converted).value; values[at] = null; } }
      push(result, unitValue(total, target));
    }
    for (let index = 0; index < values.length; index++) if (values[index]) throw fail('There were leftover terms that were not converted');
    return incompatible(createVariadic('sum', toArray(result)));
  });
  const TYPE_MEMBERS = plain(['angle', 'flex', 'frequency', 'length', 'percent', 'percentHint', 'resolution', 'time']);
  method('CSSNumericValue', 'type', 0, function () {
    // CSSNumericType members in dictionary (lexicographic) order.
    const type = brand(this, brandSets.CSSNumericValue).type, result = {};
    for (let index = 0; index < TYPE_MEMBERS.length; index++) {
      const name = TYPE_MEMBERS[index];
      if (name === 'percentHint') { if (type.hint >= 0) result.percentHint = BASE_NAMES[type.hint]; continue; }
      const exponent = type.e[BASE[name]]; if (exponent) result[name] = exponent;
    }
    return result;
  });
  staticMethod('CSSNumericValue', 'parse', 1, function (cssText) {
    requireArguments(arguments, 1);
    const value = parseNumeric(toStringValue(cssText));
    if (!value) throw fail('Invalid math expression', 'SyntaxError');
    return value;
  });
  // CSSNumericValue::parse: one numeric token, or a calc()/-webkit-calc()/min()/max()/clamp().
  function parseNumeric(text) {
    const items = trimSpace(components(tokenize(text)));
    if (items.length !== 1) return null;
    const item = items[0];
    if (item.type === 'number') return unitValue(item.value, 'number');
    if (item.type === 'percentage') return unitValue(item.value, 'percent');
    if (item.type === 'dimension') { const key = lowerCase(item.unit); return UNITS[key] && key !== 'number' && key !== 'percent' ? unitValue(item.value, key) : null; }
    if (item.block && item.type === 'function') {
      const name = lowerCase(item.value);
      if (name !== 'calc' && name !== '-webkit-calc' && name !== 'min' && name !== 'max' && name !== 'clamp') return null;
      // Blink parses the outer function as a calc() argument, so a top-level calc() is nested.
      const wrapper = list(); push(wrapper, item);
      const node = name === 'calc' ? (() => { const state = { items: wrapper, at: 0, depth: 0, allowNone: false }; const result = parseSum(state); return result && state.at === wrapper.length ? result : null; })() : parseMathFunction(name, item.children, 0);
      if (!node || node.t === 'kw') return null;
      return nodeToNumeric(node);
    }
    return null;
  }
  attribute('CSSUnitValue', 'value', (self) => brand(self, brandSets.CSSUnitValue).value, (self, value) => { const record = brand(self, brandSets.CSSUnitValue); record.value = toDouble(value); });
  attribute('CSSUnitValue', 'unit', (self) => { const key = brand(self, brandSets.CSSUnitValue).unit; return key; });
  attribute('CSSMathValue', 'operator', (self) => OPERATOR[brand(self, brandSets.CSSMathValue).kind]);
  for (const name of ['CSSMathSum', 'CSSMathProduct', 'CSSMathMin', 'CSSMathMax']) attribute(name, 'values', (self) => brand(self, brandSets[name]).array);
  attribute('CSSMathNegate', 'value', (self) => brand(self, brandSets.CSSMathNegate).value);
  attribute('CSSMathInvert', 'value', (self) => brand(self, brandSets.CSSMathInvert).value);
  for (const part of ['lower', 'value', 'upper']) attribute('CSSMathClamp', part, (self) => brand(self, brandSets.CSSMathClamp)[part]);
  attribute('CSSNumericArray', 'length', (self) => brand(self, brandSets.CSSNumericArray).values.length);
  valueIterable('CSSNumericArray', (record) => record.values);
  // Indexed getters: [LegacyUnenumerableNamedProperties]-free platform objects with own indices.
  function indexed(target, record, items, setter) {
    const index = (key) => { if (typeof key !== 'string') return -1; const number = +key; return '' + number === key && number >= 0 && number < 4294967295 && mathTrunc(number) === number ? number : -1; };
    const proxy = new Proxy(target, {
      get(object, key, receiver) { const at = index(key); if (at >= 0) { const values = items(record); return at < values.length ? values[at] : undefined; } return Reflect.get(object, key, receiver); },
      has(object, key) { const at = index(key); return at >= 0 ? at < items(record).length : Reflect.has(object, key); },
      ownKeys(object) { const keys = []; const values = items(record); for (let at = 0; at < values.length; at++) push(keys, '' + at); const own = ownKeys(object); for (let at = 0; at < own.length; at++) push(keys, own[at]); return keys; },
      getOwnPropertyDescriptor(object, key) { const at = index(key); if (at >= 0) { const values = items(record); return at < values.length ? { value: values[at], writable: !!setter, enumerable: true, configurable: true } : undefined; } return Reflect.getOwnPropertyDescriptor(object, key); },
      set(object, key, value, receiver) { const at = index(key); if (at >= 0) { if (!setter) throw new ErrorType("Failed to set an indexed property [" + at + "] on '" + record.family + "': Indexed property setter is not supported."); setter(record, at, value); return true; } return Reflect.set(object, key, value, receiver); },
      defineProperty(object, key, descriptor) { if (index(key) >= 0) return false; return Reflect.defineProperty(object, key, descriptor); },
      deleteProperty(object, key) { const at = index(key); if (at >= 0) return at >= items(record).length; return Reflect.deleteProperty(object, key); },
    });
    attach(proxy, record);
    return proxy;
  }
  function numericArrayObject(values) { const record = { family: 'CSSNumericArray', values }; return indexed(create(prototypes.CSSNumericArray), record, (state) => state.values, null); }

  // ---- Keywords ----------------------------------------------------------------------------------
  const KEYWORD_IDS = create(null);
  for (let index = 0; index < registry.keywords.length; index++) KEYWORD_IDS[registry.keywords[index]] = true;
  const keywordId = (text) => { const lower = lowerCase(text); return KEYWORD_IDS[lower] ? lower : null; };
  const CSS_WIDE = { __proto__: null, inherit: 1, initial: 1, unset: 1, revert: 1, 'revert-layer': 1 };
  function keywordValue(text) { return instance('CSSKeywordValue', { value: text }); }
  // CSSKeywordValue::ToCSSValue()->CssText(): a known CSSValueID serializes by its name, any
  // other string as a <custom-ident>.
  const keywordText = (text) => { const id = keywordId(text); return id !== null ? id : serializeIdentifier(text); };
  defineInterface('CSSKeywordValue', 'CSSStyleValue', 1, function (keyword) {
    requireArguments(arguments, 1);
    const text = toStringValue(keyword);
    if (text === '') throw fail('CSSKeywordValue does not support empty strings');
    return attach(this, { family: 'CSSKeywordValue', value: text });
  });
  attribute('CSSKeywordValue', 'value', (self) => brand(self, brandSets.CSSKeywordValue).value, (self, value) => {
    const record = brand(self, brandSets.CSSKeywordValue), text = toStringValue(value);
    if (text === '') throw fail('CSSKeywordValue does not support empty strings');
    record.value = text;
  });

  // ---- CSSUnparsedValue and CSSVariableReferenceValue ------------------------------------------
  defineInterface('CSSVariableReferenceValue', null, 1, function (variable, fallback) {
    requireArguments(arguments, 1);
    const name = toStringValue(variable);
    if (fallback !== undefined && fallback !== null && !brandedAs(fallback, 'CSSUnparsedValue')) throw fail('parameter 2 is not of type \'CSSUnparsedValue\'.');
    if (!startsWith(name, '--')) throw fail('Invalid custom property name');
    return attach(this, { family: 'CSSVariableReferenceValue', variable: name, fallback: fallback === undefined ? null : fallback });
  });
  const brandedAs = (value, name) => { const record = recordOf(value); return !!record && !!brandSets[name][record.family]; };
  attribute('CSSVariableReferenceValue', 'variable', (self) => brand(self, brandSets.CSSVariableReferenceValue).variable, (self, value) => {
    const record = brand(self, brandSets.CSSVariableReferenceValue), name = toStringValue(value);
    if (!startsWith(name, '--')) throw fail('Invalid custom property name');
    record.variable = name;
  });
  attribute('CSSVariableReferenceValue', 'fallback', (self) => brand(self, brandSets.CSSVariableReferenceValue).fallback);
  const segment = (value) => brandedAs(value, 'CSSVariableReferenceValue') ? value : toStringValue(value);
  function unparsedObject(target, segments) { return indexed(target, { family: 'CSSUnparsedValue', segments }, (state) => state.segments, unparsedSet); }
  function unparsedSet(record, at, value) {
    const converted = segment(value);
    if (at < record.segments.length) { record.segments[at] = converted; return; }
    if (at === record.segments.length) { push(record.segments, converted); return; }
    throw new RangeType("Failed to set an indexed property [" + at + "] on 'CSSUnparsedValue': The index provided (" + at + ") is outside the range [0, " + record.segments.length + "].");
  }
  defineInterface('CSSUnparsedValue', 'CSSStyleValue', 1, function (members) {
    requireArguments(arguments, 1);
    const segments = toSequence(members, segment);
    // The binding returns an indexed exotic wrapper around the freshly allocated instance.
    return unparsedObject(this, plain(segments));
  });
  attribute('CSSUnparsedValue', 'length', (self) => brand(self, brandSets.CSSUnparsedValue).segments.length);
  valueIterable('CSSUnparsedValue', (record) => record.segments);
  function newUnparsed(segments) { return unparsedObject(create(prototypes.CSSUnparsedValue), segments); }
  // CSSUnparsedValue::SerializeSegments (null on a fallback cycle).
  function appendSegments(record, stack) {
    if (stack.indexOf(record) >= 0) return null;
    push(stack, record); let text = '';
    for (let index = 0; index < record.segments.length; index++) {
      if (index) text += '/**/';
      const part = record.segments[index];
      if (typeof part === 'string') text += part;
      else {
        const reference = recordOf(part);
        text += 'var(' + serializeIdentifier(reference.variable);
        if (reference.fallback) { const inner = appendSegments(recordOf(reference.fallback), stack); if (inner === null) return null; text += ',' + inner; }
        text += ')';
      }
    }
    stack.length--;
    return text;
  }
  // CSSUnparsedValue::ToStringInternal: re-serialize tokens, keeping only the /**/ separators
  // that change tokenization.
  function unparsedText(record) {
    const serialized = appendSegments(record, []);
    const tokens = tokenize(serialized === null ? '' : serialized);
    if (!tokens.length) return '';
    let text = serializeToken(tokens[0]);
    for (let index = 1; index < tokens.length; index++) { if (needsComment(tokens[index - 1], tokens[index])) text += '/**/'; text += serializeToken(tokens[index]); }
    return text;
  }
  // CSSUnparsedValue::FromCSSVariableData: var()/env() become references, the rest strings.
  function unparsedFromText(text) {
    const tokens = tokenize(text); let index = 0;
    function read(stopAtClose) {
      const segments = list(); let builder = '', depth = 0;
      while (index < tokens.length) {
        const token = tokens[index];
        if (token.type === 'function' && (lowerCase(token.value) === 'var' || lowerCase(token.value) === 'env')) {
          if (builder) { push(segments, builder); builder = ''; }
          index++;
          while (index < tokens.length && tokens[index].type === 'whitespace') index++;
          let variable = '';
          if (index < tokens.length && tokens[index].type === 'ident') { variable = tokens[index].value; index++; }
          while (index < tokens.length && tokens[index].type === 'whitespace') index++;
          if (index < tokens.length && tokens[index].type === 'comma') index++;
          const inner = read(true);
          if (index < tokens.length && tokens[index].type === ')') index++;
          if (!startsWith(variable, '--')) return segments;
          push(segments, instance('CSSVariableReferenceValue', { variable, fallback: inner.length ? newUnparsed(inner) : null }));
          continue;
        }
        if (BLOCK_END[token.type]) depth++;
        else if (token.type === ')' || token.type === ']' || token.type === '}') { depth--; if (depth < 0 && stopAtClose) break; }
        builder += serializeToken(token); index++;
      }
      if (builder) push(segments, builder);
      return segments;
    }
    return newUnparsed(read(false));
  }

  // ---- Transforms -------------------------------------------------------------------------------
  const COMPONENT = brandSets.CSSTransformComponent;
  defineInterface('CSSTransformComponent', null, 0, null);
  attribute('CSSTransformComponent', 'is2D', (self) => brand(self, COMPONENT).is2D, (self, value) => { brand(self, COMPONENT).is2D = !!value; });
  method('CSSTransformComponent', 'toMatrix', 0, function () { return componentMatrix(brand(this, COMPONENT)); });
  stringifier('CSSTransformComponent', function (self) { const text = componentText(brand(self, COMPONENT)); return text === null ? '' : text; });
  const requireNumeric = (value, position) => { if (!numericRecord(value)) throw fail('parameter ' + position + " is not of type 'CSSNumericValue'."); return value; };
  const setterNumeric = (value) => { if (!numericRecord(value)) throw fail("Failed to convert value to 'CSSNumericValue'."); return value; };
  const typeOf = (value) => numericRecord(value).type;
  const validTranslateXY = (value) => matchesBasePercentage(typeOf(value), BASE.length);
  const validLength = (value) => matchesBase(typeOf(value), BASE.length);
  const validAngle = (value) => matchesBase(typeOf(value), BASE.angle);
  const validNumber = (value) => matchesNumber(typeOf(value));
  const px = (value) => unitValue(value, 'px');
  function component(name, record) { record.family = name; return attach(create(prototypes[name]), record); }
  defineInterface('CSSTranslate', 'CSSTransformComponent', 2, function (x, y, z) {
    requireArguments(arguments, 2);
    requireNumeric(x, 1); requireNumeric(y, 2); if (z !== undefined) requireNumeric(z, 3);
    if (z === undefined) { if (!validTranslateXY(x) || !validTranslateXY(y)) throw fail('Must pass length or percentage to X and Y of CSSTranslate'); }
    else if (!validTranslateXY(x) || !validTranslateXY(y) || !validLength(z)) throw fail('Must pass length or percentage to X, Y and Z of CSSTranslate');
    return attach(this, { family: 'CSSTranslate', kind: 'translate', x, y, z: z === undefined ? px(0) : z, is2D: z === undefined });
  });
  attribute('CSSTranslate', 'x', (self) => brand(self, brandSets.CSSTranslate).x, (self, value) => { const record = brand(self, brandSets.CSSTranslate); setterNumeric(value); if (!validTranslateXY(value)) throw fail('Must pass length or percentage to X of CSSTranslate'); record.x = value; });
  attribute('CSSTranslate', 'y', (self) => brand(self, brandSets.CSSTranslate).y, (self, value) => { const record = brand(self, brandSets.CSSTranslate); setterNumeric(value); if (!validTranslateXY(value)) throw fail('Must pass length or percent to Y of CSSTranslate'); record.y = value; });
  attribute('CSSTranslate', 'z', (self) => brand(self, brandSets.CSSTranslate).z, (self, value) => { const record = brand(self, brandSets.CSSTranslate); setterNumeric(value); if (!validLength(value)) throw fail('Must pass length to Z of CSSTranslate'); record.z = value; });
  defineInterface('CSSRotate', 'CSSTransformComponent', 1, function (a, b, c, d) {
    requireArguments(arguments, 1);
    if (arguments.length === 1) {
      requireNumeric(a, 1);
      if (!validAngle(a)) throw fail('Must pass an angle to CSSRotate');
      return attach(this, { family: 'CSSRotate', kind: 'rotate', x: unitValue(0, 'number'), y: unitValue(0, 'number'), z: unitValue(1, 'number'), angle: a, is2D: true });
    }
    if (arguments.length < 4) throw fail('Overload resolution failed.');
    const x = fromNumberish(a), y = fromNumberish(b), z = fromNumberish(c);
    requireNumeric(d, 4);
    if (!validNumber(x) || !validNumber(y) || !validNumber(z)) throw fail('Must specify an number unit');
    if (!validAngle(d)) throw fail('Must pass an angle to CSSRotate');
    return attach(this, { family: 'CSSRotate', kind: 'rotate', x, y, z, angle: d, is2D: false });
  });
  attribute('CSSRotate', 'angle', (self) => brand(self, brandSets.CSSRotate).angle, (self, value) => { const record = brand(self, brandSets.CSSRotate); setterNumeric(value); if (!validAngle(value)) throw fail('Must pass an angle to CSSRotate'); record.angle = value; });
  for (const axis of ['x', 'y', 'z']) attribute('CSSRotate', axis, (self) => brand(self, brandSets.CSSRotate)[axis], (self, value) => { const record = brand(self, brandSets.CSSRotate), converted = fromNumberish(value); if (!validNumber(converted)) throw fail('Must specify a number unit'); record[axis] = converted; });
  // IsValidScaleCoord: a non-unit value must resolve to a number.
  const validScale = (value) => validNumber(value);
  defineInterface('CSSScale', 'CSSTransformComponent', 2, function (x, y, z) {
    requireArguments(arguments, 2);
    const a = fromNumberish(x), b = fromNumberish(y), c = z === undefined ? undefined : fromNumberish(z);
    if (c === undefined) { if (!validScale(a) || !validScale(b)) throw fail('Must specify an number unit'); }
    else if (!validScale(a) || !validScale(b) || !validScale(c)) throw fail('Must specify a number for X, Y and Z');
    return attach(this, { family: 'CSSScale', kind: 'scale', x: a, y: b, z: c === undefined ? unitValue(1, 'number') : c, is2D: c === undefined });
  });
  for (const axis of ['x', 'y', 'z']) attribute('CSSScale', axis, (self) => brand(self, brandSets.CSSScale)[axis], (self, value) => { const record = brand(self, brandSets.CSSScale), converted = fromNumberish(value); if (!validScale(converted)) throw fail('Must specify a number unit'); record[axis] = converted; });
  defineInterface('CSSSkew', 'CSSTransformComponent', 2, function (ax, ay) {
    requireArguments(arguments, 2); requireNumeric(ax, 1); requireNumeric(ay, 2);
    if (!validAngle(ax) || !validAngle(ay)) throw fail('CSSSkew does not support non-angles');
    return attach(this, { family: 'CSSSkew', kind: 'skew', ax, ay, is2D: true });
  });
  for (const axis of ['ax', 'ay']) attribute('CSSSkew', axis, (self) => brand(self, brandSets.CSSSkew)[axis], (self, value) => { const record = brand(self, brandSets.CSSSkew); setterNumeric(value); if (!validAngle(value)) throw fail('Must specify an angle unit'); record[axis] = value; });
  defineInterface('CSSSkewX', 'CSSTransformComponent', 1, function (ax) {
    requireArguments(arguments, 1); requireNumeric(ax, 1);
    if (!validAngle(ax)) throw fail('CSSSkewX does not support non-angles');
    return attach(this, { family: 'CSSSkewX', kind: 'skewX', ax, is2D: true });
  });
  attribute('CSSSkewX', 'ax', (self) => brand(self, brandSets.CSSSkewX).ax, (self, value) => { const record = brand(self, brandSets.CSSSkewX); setterNumeric(value); if (!validAngle(value)) throw fail('Must specify an angle unit'); record.ax = value; });
  defineInterface('CSSSkewY', 'CSSTransformComponent', 1, function (ay) {
    requireArguments(arguments, 1); requireNumeric(ay, 1);
    if (!validAngle(ay)) throw fail('CSSSkewY does not support non-angles');
    return attach(this, { family: 'CSSSkewY', kind: 'skewY', ay, is2D: true });
  });
  attribute('CSSSkewY', 'ay', (self) => brand(self, brandSets.CSSSkewY).ay, (self, value) => { const record = brand(self, brandSets.CSSSkewY); setterNumeric(value); if (!validAngle(value)) throw fail('Must specify an angle unit'); record.ay = value; });
  // (CSSNumericValue or CSSKeywordish): strings become keywords; only a length or 'none' passes.
  function perspectiveLength(value) {
    if (numericRecord(value)) return matchesBase(typeOf(value), BASE.length) ? value : null;
    const keyword = brandedAs(value, 'CSSKeywordValue') ? value : keywordValue(toStringValue(value));
    return keywordId(recordOf(keyword).value) === 'none' ? keyword : null;
  }
  defineInterface('CSSPerspective', 'CSSTransformComponent', 1, function (length) {
    requireArguments(arguments, 1);
    const value = perspectiveLength(length);
    if (!value) throw fail('Must pass length or none to CSSPerspective');
    return attach(this, { family: 'CSSPerspective', kind: 'perspective', length: value, is2D: false });
  });
  attribute('CSSPerspective', 'length', (self) => brand(self, brandSets.CSSPerspective).length, (self, length) => { const record = brand(self, brandSets.CSSPerspective), value = perspectiveLength(length); if (!value) throw fail('Must pass length or none to CSSPerspective'); record.length = value; });
  let matrixApi = null;
  const MATRIX_FIELDS = plain(['m11', 'm12', 'm13', 'm14', 'm21', 'm22', 'm23', 'm24', 'm31', 'm32', 'm33', 'm34', 'm41', 'm42', 'm43', 'm44', 'a', 'b', 'c', 'd', 'e', 'f']), MATRIX_SETTERS = plain(['m12', 'm21', 'm34', 'a', 'b', 'c', 'd', 'e', 'f']);
  // DOMMatrix lives in the embedding (turbo-surf web_geometry); capture its members on first use.
  function matrices() {
    if (matrixApi) return matrixApi;
    const Matrix = g.DOMMatrix, ReadOnly = g.DOMMatrixReadOnly;
    if (typeof Matrix !== 'function' || typeof ReadOnly !== 'function') return null;
    const getter = (name) => Object.getOwnPropertyDescriptor(ReadOnly.prototype, name).get;
    const fields = list(); for (let index = 0; index < MATRIX_FIELDS.length; index++) fields[MATRIX_FIELDS[index]] = getter(MATRIX_FIELDS[index]);
    matrixApi = { Matrix, is2D: getter('is2D'), fields, fromMatrix: Matrix.fromMatrix, multiplySelf: Matrix.prototype.multiplySelf, translateSelf: Matrix.prototype.translateSelf, scaleSelf: Matrix.prototype.scaleSelf, rotateAxisAngleSelf: Matrix.prototype.rotateAxisAngleSelf, skewXSelf: Matrix.prototype.skewXSelf, skewYSelf: Matrix.prototype.skewYSelf, setters: (() => { const result = create(null); for (let index = 0; index < MATRIX_SETTERS.length; index++) result[MATRIX_SETTERS[index]] = Object.getOwnPropertyDescriptor(Matrix.prototype, MATRIX_SETTERS[index]).set; return result; })() };
    return matrixApi;
  }
  const isMatrix = (value) => { const api = matrices(); if (!api || value === null || typeof value !== 'object') return false; try { apply(api.is2D, value, []); return true; } catch { return false; } };
  const matrixField = (matrix, name) => apply(matrices().fields[name], matrix, []);
  defineInterface('CSSMatrixComponent', 'CSSTransformComponent', 1, function (matrix, options) {
    requireArguments(arguments, 1);
    if (!isMatrix(matrix)) throw fail("parameter 1 is not of type 'DOMMatrixReadOnly'.");
    let is2D;
    if (options !== undefined && options !== null) { if (typeof options !== 'object' && typeof options !== 'function') throw fail("The provided value is not of type 'CSSMatrixComponentOptions'."); const flag = options.is2D; if (flag !== undefined) is2D = !!flag; }
    if (is2D === undefined) is2D = apply(matrices().is2D, matrix, []);
    // The component keeps its own DOMMatrix copy of the argument.
    return attach(this, { family: 'CSSMatrixComponent', kind: 'matrix', matrix: apply(matrices().fromMatrix, matrices().Matrix, [matrix]), is2D });
  });
  attribute('CSSMatrixComponent', 'matrix', (self) => brand(self, brandSets.CSSMatrixComponent).matrix, (self, value) => {
    const record = brand(self, brandSets.CSSMatrixComponent);
    const api = matrices();
    if (!api || !(value instanceof api.Matrix) || !isMatrix(value)) throw fail("Failed to convert value to 'DOMMatrix'.");
    record.matrix = value;
  });
  const numericCss = (value) => numericCssText(value);
  function componentText(record) {
    switch (record.kind) {
      case 'translate': return record.is2D ? 'translate(' + numericCss(record.x) + ', ' + numericCss(record.y) + ')' : 'translate3d(' + numericCss(record.x) + ', ' + numericCss(record.y) + ', ' + numericCss(record.z) + ')';
      case 'rotate': return record.is2D ? 'rotate(' + numericCss(record.angle) + ')' : 'rotate3d(' + numericCss(record.x) + ', ' + numericCss(record.y) + ', ' + numericCss(record.z) + ', ' + numericCss(record.angle) + ')';
      case 'scale': return record.is2D ? 'scale(' + numericCss(record.x) + ', ' + numericCss(record.y) + ')' : 'scale3d(' + numericCss(record.x) + ', ' + numericCss(record.y) + ', ' + numericCss(record.z) + ')';
      case 'skew': { const ay = numericRecord(record.ay); return 'skew(' + numericCss(record.ax) + (ay.kind !== 'unit' || ay.value !== 0 ? ', ' + numericCss(record.ay) : '') + ')'; }
      case 'skewX': return 'skewX(' + numericCss(record.ax) + ')';
      case 'skewY': return 'skewY(' + numericCss(record.ay) + ')';
      case 'perspective': {
        const length = record.length, numeric = numericRecord(length);
        if (!numeric) return 'perspective(' + keywordText(recordOf(length).value) + ')';
        if (numeric.kind === 'unit' && numeric.value < 0) return 'perspective(calc(' + formatNumber(numeric.value, numeric.unit) + '))';
        return 'perspective(' + numericCss(length) + ')';
      }
      case 'matrix': {
        const names = record.is2D ? ['a', 'b', 'c', 'd', 'e', 'f'] : ['m11', 'm12', 'm13', 'm14', 'm21', 'm22', 'm23', 'm24', 'm31', 'm32', 'm33', 'm34', 'm41', 'm42', 'm43', 'm44'];
        let text = record.is2D ? 'matrix(' : 'matrix3d(';
        for (let index = 0; index < names.length; index++) text += (index ? ', ' : '') + formatNumber(matrixField(record.matrix, names[index]), 'number');
        return text + ')';
      }
    }
    return null;
  }
  function newMatrix() { const api = matrices(); if (!api) throw fail('DOMMatrix is not available'); return new api.Matrix(); }
  function componentMatrix(record) {
    const api = matrices();
    switch (record.kind) {
      case 'translate': {
        const x = toUnit(record.x, 'px'), y = toUnit(record.y, 'px'), z = toUnit(record.z, 'px');
        if (!x || !y || !z) throw fail('Cannot create matrix if units are not compatible with px');
        const matrix = newMatrix();
        if (record.is2D) apply(api.translateSelf, matrix, [numericRecord(x).value, numericRecord(y).value]); else apply(api.translateSelf, matrix, [numericRecord(x).value, numericRecord(y).value, numericRecord(z).value]);
        return matrix;
      }
      case 'rotate': {
        const x = toUnit(record.x, 'number'), y = toUnit(record.y, 'number'), z = toUnit(record.z, 'number');
        if (!x || !y || !z) throw fail('Cannot create matrix if units cannot be converted to CSSUnitValue');
        const matrix = newMatrix(), angle = numericRecord(toUnit(record.angle, 'deg')).value;
        if (record.is2D) apply(api.rotateAxisAngleSelf, matrix, [0, 0, 1, angle]); else apply(api.rotateAxisAngleSelf, matrix, [numericRecord(x).value, numericRecord(y).value, numericRecord(z).value, angle]);
        return matrix;
      }
      case 'scale': {
        const x = toUnit(record.x, 'number'), y = toUnit(record.y, 'number'), z = toUnit(record.z, 'number');
        if (!x || !y || !z) throw fail('Cannot create matrix if values are not numbers');
        const matrix = newMatrix();
        if (record.is2D) apply(api.scaleSelf, matrix, [numericRecord(x).value, numericRecord(y).value]); else apply(api.scaleSelf, matrix, [numericRecord(x).value, numericRecord(y).value, numericRecord(z).value]);
        return matrix;
      }
      case 'skew': { const matrix = newMatrix(); apply(api.setters.m12, matrix, [mathTan(numericRecord(toUnit(record.ay, 'rad')).value)]); apply(api.setters.m21, matrix, [mathTan(numericRecord(toUnit(record.ax, 'rad')).value)]); return matrix; }
      case 'skewX': { const matrix = newMatrix(); apply(api.skewXSelf, matrix, [numericRecord(toUnit(record.ax, 'deg')).value]); return matrix; }
      case 'skewY': { const matrix = newMatrix(); apply(api.skewYSelf, matrix, [numericRecord(toUnit(record.ay, 'deg')).value]); return matrix; }
      case 'perspective': {
        if (!numericRecord(record.length)) return newMatrix();
        const length = toUnit(record.length, 'px');
        if (!length) throw fail('Cannot create matrix if units are not compatible with px');
        // gfx::Transform::ApplyPerspectiveDepth: m34 = -1 / depth.
        const matrix = newMatrix(); apply(api.setters.m34, matrix, [-1 / mathMax(1, numericRecord(length).value)]); return matrix;
      }
      case 'matrix': {
        if (record.is2D && !apply(api.is2D, record.matrix, [])) {
          const matrix = newMatrix(), map = [['a', 'm11'], ['b', 'm12'], ['c', 'm21'], ['d', 'm22'], ['e', 'm41'], ['f', 'm42']];
          for (let index = 0; index < map.length; index++) apply(api.setters[map[index][0]], matrix, [matrixField(record.matrix, map[index][1])]);
          return matrix;
        }
        return apply(api.fromMatrix, api.Matrix, [record.matrix]);
      }
    }
    return null;
  }
  const componentOf = (value) => { const record = recordOf(value); if (!record || !COMPONENT[record.family]) throw fail("Failed to convert value to 'CSSTransformComponent'."); return value; };
  function transformObject(target, components) { return indexed(target, { family: 'CSSTransformValue', components }, (state) => state.components, transformSet); }
  function transformSet(record, at, value) {
    const record0 = recordOf(value);
    if (!record0 || !COMPONENT[record0.family]) throw new ErrorType("Failed to set an indexed property [" + at + "] on 'CSSTransformValue': parameter 2 is not of type 'CSSTransformComponent'.");
    const converted = value;
    if (at < record.components.length) { record.components[at] = converted; return; }
    if (at === record.components.length) { push(record.components, converted); return; }
    throw new RangeType("Failed to set an indexed property [" + at + "] on 'CSSTransformValue': The index provided (" + at + ") is outside the range [0, " + record.components.length + "].");
  }
  defineInterface('CSSTransformValue', 'CSSStyleValue', 1, function (transforms) {
    requireArguments(arguments, 1);
    const components = toSequence(transforms, componentOf);
    if (!components.length) throw fail('CSSTransformValue must have at least one component');
    return transformObject(this, plain(components));
  });
  const newTransform = (components) => transformObject(create(prototypes.CSSTransformValue), components);
  attribute('CSSTransformValue', 'length', (self) => brand(self, brandSets.CSSTransformValue).components.length);
  attribute('CSSTransformValue', 'is2D', (self) => { const components = brand(self, brandSets.CSSTransformValue).components; for (let index = 0; index < components.length; index++) if (!recordOf(components[index]).is2D) return false; return true; });
  method('CSSTransformValue', 'toMatrix', 0, function () {
    const record = brand(this, brandSets.CSSTransformValue), api = matrices(), matrix = newMatrix();
    for (let index = 0; index < record.components.length; index++) { const part = componentMatrix(recordOf(record.components[index])); if (part) apply(api.multiplySelf, matrix, [part]); }
    return matrix;
  });
  valueIterable('CSSTransformValue', (record) => record.components);
  function transformText(record) { let text = ''; for (let index = 0; index < record.components.length; index++) { const part = componentText(recordOf(record.components[index])); if (part === null) return ''; text += (index ? ' ' : '') + part; } return text; }

  // ---- CSSPositionValue, CSSImageValue -----------------------------------------------------------
  defineInterface('CSSPositionValue', 'CSSStyleValue', 2, function (x, y) {
    requireArguments(arguments, 2); requireNumeric(x, 1); requireNumeric(y, 2);
    if (!validTranslateXY(x)) throw fail('Must pass length or percentage to x in CSSPositionValue');
    if (!validTranslateXY(y)) throw fail('Must pass length or percentage to y in CSSPositionValue');
    return attach(this, { family: 'CSSPositionValue', x, y });
  });
  for (const axis of ['x', 'y']) attribute('CSSPositionValue', axis, (self) => brand(self, brandSets.CSSPositionValue)[axis], (self, value) => { const record = brand(self, brandSets.CSSPositionValue); setterNumeric(value); if (!validTranslateXY(value)) throw fail('Must pass length or percentage to ' + axis + ' in CSSPositionValue'); record[axis] = value; });
  defineInterface('CSSImageValue', 'CSSStyleValue', 0, null);
  const positionValue = (x, y) => instance('CSSPositionValue', { x, y });
  const imageValue = (text) => instance('CSSImageValue', { text });
  const unsupported = (text, property) => instance('CSSStyleValue', { unsupported: true, text, property });

  // CSSStyleValue::toString per kind.
  function styleText(record, object) {
    switch (record.family) {
      case 'CSSKeywordValue': return keywordText(record.value);
      case 'CSSUnparsedValue': return unparsedText(record);
      case 'CSSTransformValue': return transformText(record);
      case 'CSSPositionValue': return numericCss(record.x) + ' ' + numericCss(record.y);
      case 'CSSImageValue': return record.text;
      case 'CSSStyleValue': return record.text;
    }
    return record.numeric ? numericText(object, false, false) : '';
  }

  // ---- Property registry ------------------------------------------------------------------------
  const PROPERTIES = create(null);
  {
    const names = ownKeys(registry.properties);
    for (let index = 0; index < names.length; index++) {
      const meta = registry.properties[names[index]];
      meta.keywordSet = create(null); meta.typeSet = create(null);
      if (meta.keywords) for (let at = 0; at < meta.keywords.length; at++) meta.keywordSet[meta.keywords[at]] = true;
      if (meta.types) for (let at = 0; at < meta.types.length; at++) meta.typeSet[meta.types[at]] = true;
      PROPERTIES[names[index]] = meta;
    }
  }
  const hasKeyword = (meta, lower) => !!meta.keywordSet && !!meta.keywordSet[lower];
  const hasType = (meta, name) => !!meta.typeSet && !!meta.typeSet[name];
  // CSSPropertyName::From: a custom property, else a (case-insensitive) enabled property or alias.
  // Descriptor-only names are known property ids that no style declaration accepts.
  const DESCRIPTORS = plainSet(['src', 'font-display', 'unicode-range', 'size-adjust', 'ascent-override', 'descent-override', 'line-gap-override', 'font-named-instance', 'base-palette', 'override-colors', 'syntax', 'inherits', 'initial-value', 'navigation', 'types', 'additive-symbols', 'negative', 'pad', 'prefix', 'range', 'speak-as', 'suffix', 'symbols', 'system', 'fallback']);
  function propertyName(text) {
    if (startsWith(text, '--')) return text.length > 2 ? { __proto__: null, custom: true, name: text, meta: null } : null;
    const lower = lowerCase(text), meta = PROPERTIES[lower];
    if (!meta) return DESCRIPTORS[lower] ? { __proto__: null, custom: false, name: lower, meta: { descriptor: 1 }, descriptor: true } : null;
    const name = meta.alias || lower;
    return { __proto__: null, custom: false, name, meta: PROPERTIES[name] || meta };
  }
  const isShorthand = (property) => !property.custom && !!property.meta.longhands;
  const isRepeated = (property) => !property.custom && !!property.meta.separator;
  const LAYERED_SPACE = { __proto__: null, 'grid-auto-columns': 1, 'grid-auto-rows': 1, 'overflow-clip-margin': 1 };
  const NON_NEGATIVE_LENGTH = plainSet(['border-bottom-width', 'border-left-width', 'border-right-width', 'border-top-width', 'flex-basis', 'height', 'line-height', 'max-height', 'max-width', 'min-height', 'min-width', 'outline-width', 'padding-bottom', 'padding-left', 'padding-right', 'padding-top', 'perspective', 'r', 'rx', 'ry', 'shape-margin', 'stroke-width', '-webkit-border-horizontal-spacing', '-webkit-border-vertical-spacing', 'column-gap', 'row-gap', 'column-width', 'column-height', 'width', 'tab-size']);
  const NON_NEGATIVE = plainSet(['animation-duration', 'animation-iteration-count', 'background-size', 'block-size', 'border-block-end-width', 'border-block-start-width', 'border-image-outset', 'border-image-slice', 'border-image-width', 'border-inline-end-width', 'border-inline-start-width', 'column-rule-width', 'flex-grow', 'flex-shrink', 'font-size', 'font-size-adjust', 'font-stretch', 'grid-auto-columns', 'grid-auto-rows', 'inline-size', 'max-block-size', 'max-inline-size', 'min-block-size', 'min-inline-size', 'padding-block-end', 'padding-block-start', 'padding-inline-end', 'padding-inline-start', 'r', 'rx', 'ry', 'scroll-padding-block-end', 'scroll-padding-block-start', 'scroll-padding-bottom', 'scroll-padding-inline-end', 'scroll-padding-inline-start', 'scroll-padding-left', 'scroll-padding-right', 'scroll-padding-top', 'stroke-miterlimit', 'transition-duration']);
  function plainSet(names) { const set = create(null); for (let index = 0; index < names.length; index++) set[names[index]] = true; return set; }
  // CSSUnitValue::IsValueOutOfRangeForProperty.
  function outOfRange(name, value, unitKey) {
    if (unitKey === 'fr') return false;
    if (NON_NEGATIVE_LENGTH[name] && value < 0) return true;
    switch (name) {
      case 'order': case 'reading-order': case 'z-index': case 'math-depth': return mathRound(value) !== value;
      case 'tab-size': return value < 0 || (unitKey === 'number' && mathRound(value) !== value);
      case 'orphans': case 'widows': case 'column-count': return mathRound(value) !== value || value < 1;
      case 'font-weight': return value < 1 || value > 1000;
    }
    return !!NON_NEGATIVE[name] && value < 0;
  }

  // ---- Value grammar (validation) --------------------------------------------------------------
  // Unbalanced blocks, top-level `!`/`;`/`{}` and bad tokens are never a declaration value.
  function declarationValueOk(tokens, allowEmpty) {
    let depth = 0; const stack = list(); let content = false;
    for (let index = 0; index < tokens.length; index++) {
      const token = tokens[index];
      if (token.type === 'bad-string' || token.type === 'bad-url') return false;
      if (token.type !== 'whitespace') content = true;
      if (BLOCK_END[token.type]) { push(stack, BLOCK_END[token.type]); depth++; continue; }
      if (token.type === ')' || token.type === ']' || token.type === '}') { if (!stack.length || stack[stack.length - 1] !== token.type) return false; stack.length--; depth--; continue; }
      if (depth === 0 && (token.type === 'semicolon' || (token.type === 'delim' && token.delim === '!'))) return false;
    }
    return allowEmpty || content;
  }
  // var()/env() argument grammar: a custom property name first, then an optional fallback.
  function substitutionsOk(items) {
    for (let index = 0; index < items.length; index++) {
      const item = items[index];
      if (!item.block) continue;
      if (item.type === 'function' && (lowerCase(item.value) === 'var' || lowerCase(item.value) === 'env')) {
        const inner = significant(item.children);
        if (!inner.length || inner[0].type !== 'ident' || (lowerCase(item.value) === 'var' && !startsWith(inner[0].value, '--'))) return false;
        if (inner.length > 1 && inner[1].type !== 'comma') return false;
      }
      if (!substitutionsOk(item.children)) return false;
    }
    return true;
  }
  function hasSubstitution(items) {
    for (let index = 0; index < items.length; index++) {
      const item = items[index];
      if (item.block && item.type === 'function') { const name = lowerCase(item.value); if (name === 'var' || name === 'env' || name === 'attr' || name === 'if' || name === 'inherit') return true; }
      if (item.block && hasSubstitution(item.children)) return true;
    }
    return false;
  }
  // Blink rejects negative values of non-negative properties at parse time.
  const NON_NEGATIVE_TIME = plainSet(['transition-duration', 'animation-duration']);
  function rangeOk(property, items) {
    const name = property.name;
    if (!NON_NEGATIVE_LENGTH[name] && !NON_NEGATIVE[name] && !NON_NEGATIVE_TIME[name]) return true;
    for (let index = 0; index < items.length; index++) { const item = items[index]; if ((item.type === 'number' || item.type === 'percentage' || item.type === 'dimension') && item.value < 0) return false; }
    return true;
  }
  const POSITION_PROPERTIES = plainSet(['object-position', 'perspective-origin', 'offset-anchor', 'offset-position']);
  const positionCountOk = (items) => items.length !== 3;
  // Grammar Lightning CSS lacks for properties Blink parses.
  const EXTRA_GRAMMAR = {
    __proto__: null,
    transform: (items) => { if (!items.length) return false; for (let index = 0; index < items.length; index++) if (!transformComponent(items[index])) return false; return true; },
    'will-change': (items) => { const layers = splitComponents(items, 'comma'); for (let index = 0; index < layers.length; index++) { const layer = significant(layers[index]); if (layer.length !== 1 || layer[0].type !== 'ident') return false; } return true; },
    'grid-auto-flow': (items) => { if (!items.length || items.length > 2) return false; for (let index = 0; index < items.length; index++) { const id = ident(items[index]); if (id !== 'row' && id !== 'column' && id !== 'dense') return false; } return true; },
  };
  // Whether `text` is a valid value of a (non-custom) property: Lightning CSS's grammar when it
  // models the property, else the property's Blink keyword table for single identifiers.
  function validValue(property, text) {
    const tokens = tokenize(text);
    if (!declarationValueOk(tokens, false)) return false;
    const items = significant(components(tokens));
    if (items.length === 1 && items[0].type === 'ident' && CSS_WIDE[lowerCase(items[0].value)]) return true;
    if (hasSubstitution(items)) return false;
    const meta = property.meta;
    if (property.descriptor) return false;
    // -webkit-box-reflect serializes `none` but its grammar has no such keyword.
    if (property.name === '-webkit-box-reflect' && items.length === 1 && ident(items[0]) === 'none') return false;
    if (!rangeOk(property, items)) return false;
    if (POSITION_PROPERTIES[property.name] && !positionCountOk(items)) return false;
    const native = Native('valid', property.name, text);
    if (native === true) return true;
    // Keywords Blink lists for the property (flex-basis: content, box-shadow: none, …).
    if (items.length === 1 && items[0].type === 'ident' && hasKeyword(meta, lowerCase(items[0].value))) return true;
    if (EXTRA_GRAMMAR[property.name] && EXTRA_GRAMMAR[property.name](items)) return true;
    if (native === false) return false;
    if (items.length === 1 && items[0].type === 'ident' && meta.keywords && !meta.customIdent && meta.types && meta.types.length === 1 && meta.types[0] === 'Keyword') return hasKeyword(meta, lowerCase(items[0].value));
    return true;
  }

  // ---- Reification (StyleValueFactory) -----------------------------------------------------------
  const RADIUS = plainSet(['border-bottom-left-radius', 'border-bottom-right-radius', 'border-top-left-radius', 'border-top-right-radius', 'border-end-end-radius', 'border-end-start-radius', 'border-start-end-radius', 'border-start-start-radius']);
  const COLOR_KEYWORD_ONLY = plainSet(['background-color', 'border-bottom-color', 'border-left-color', 'border-right-color', 'border-top-color', 'color', 'column-rule-color', 'flood-color', 'lighting-color', 'outline-color', 'stop-color', 'text-decoration-color', 'text-emphasis-color']);
  const COLOR_FUNCTIONS = plainSet(['rgb', 'rgba', 'hsl', 'hsla', 'hwb', 'lab', 'lch', 'oklab', 'oklch', 'color', 'color-mix', 'light-dark', 'contrast-color', 'device-cmyk', '-webkit-gradient-color']);
  const SINGLE_ONLY = plainSet(['clip-path', 'contain', 'container-type', 'container-name', 'font-variant-east-asian', 'font-variant-ligatures', 'font-variant-numeric', 'grid-auto-flow', 'offset-rotate', 'text-decoration-line', 'text-transform', 'text-indent', 'transition-property', 'touch-action']);
  const textOf = (items) => { let text = ''; for (let index = 0; index < items.length; index++) text += rawText(items[index]); return text; };
  function rawText(item) { if (!item.block) return item.raw; let text = item.open.raw; for (let index = 0; index < item.children.length; index++) text += rawText(item.children[index]); return text + (item.closed ? BLOCK_END[item.type] : ''); }
  const isUrl = (item) => item.type === 'url' || (item.block && item.type === 'function' && lowerCase(item.value) === 'url');
  const urlString = (item) => item.type === 'url' ? item.value : (() => { const inner = significant(item.children); return inner.length && inner[0].type === 'string' ? inner[0].value : ''; })();
  const isColor = (item) => item.type === 'hash' || (item.block && item.type === 'function' && COLOR_FUNCTIONS[lowerCase(item.value)]) || false;
  // Lowercased identifier value for Blink's CSSIdentifierValue, case-preserved for custom idents.
  function identifierKeyword(property, item) {
    const lower = lowerCase(item.value), meta = property.meta;
    if (CSS_WIDE[lower]) return lower;
    const ownKeyword = !!meta && hasKeyword(meta, lower);
    if (meta && meta.customIdent && !ownKeyword) return item.value;
    return KEYWORD_IDS[lower] ? lower : item.value;
  }
  const USER_UNITS = plainSet(['stroke-width', 'stroke-dashoffset', 'cx', 'cy', 'r', 'rx', 'ry', 'x', 'y', 'baseline-shift', 'shape-margin']);
  const ALPHA = plainSet(['opacity', 'fill-opacity', 'flood-opacity', 'stop-opacity', 'stroke-opacity']);
  // Two-value display maps to its legacy single keyword.
  const DISPLAY_PAIRS = { __proto__: null, 'block flow': 'block', 'inline flow': 'inline', 'block flow-root': 'flow-root', 'inline flow-root': 'inline-block', 'block flex': 'flex', 'inline flex': 'inline-flex', 'block grid': 'grid', 'inline grid': 'inline-grid', 'block table': 'table', 'inline table': 'inline-table', 'block list-item': 'list-item', 'block flow list-item': 'list-item', 'block ruby': 'block ruby', 'inline ruby': 'ruby', 'run-in flow': 'run-in', 'block math': 'block math', 'inline math': 'math' };
  // CreateStyleValue: identifiers, numerics, colors and images reify without a property.
  function genericValue(property, item, mode) {
    if (item.type === 'ident') return keywordValue(identifierKeyword(property, item));
    const meta = property.meta || { __proto__: null };
    const numericTyped = hasType(meta, 'Length') || hasType(meta, 'Number') || hasType(meta, 'Percentage') || hasType(meta, 'Time') || hasType(meta, 'Angle') || hasType(meta, 'Flex') || hasType(meta, 'Resolution');
    if ((item.type === 'number' || item.type === 'percentage' || item.type === 'dimension') && !numericTyped && meta.types) return null;
    if (item.type === 'number') {
      // Unitless lengths: 0 is 0px in CSS length grammar; SVG user units have no Typed OM unit.
      if (hasType(meta, 'Length') && !hasType(meta, 'Number')) return item.value === 0 && !USER_UNITS[property.name] ? unitValue(0, 'px') : null;
      return unitValue(item.value, 'number');
    }
    if (item.type === 'percentage') return ALPHA[property.name] ? unitValue(item.value / 100, 'number') : unitValue(item.value, 'percent');
    if (item.type === 'dimension') { const key = lowerCase(item.unit); return UNITS[key] && key !== 'number' && key !== 'percent' ? unitValue(item.value, key) : null; }
    if (item.block && item.type === 'function' && MATH_FUNCTIONS[lowerCase(item.value)]) { const node = parseMathComponent(item); return node ? nodeToNumeric(node) : null; }
    if (isColor(item)) return unsupported(colorText(item, mode), property.name);
    if (isUrl(item) && property.meta && hasType(property.meta, 'Image')) return imageValue('url(' + serializeString(urlString(item)) + ')');
    return null;
  }
  const colorText = (item, mode) => mode.normalize(plain([item]));
  // Position syntax (css-values-4 <position>) as a CSSValuePair of x and y offsets.
  function positionValueOf(items) {
    const parts = significant(items);
    if (!parts.length || parts.length > 4) return null;
    const isKeyword = (item, names) => item.type === 'ident' && indexOfText(names, ' ' + lowerCase(item.value) + ' ') >= 0;
    const offset = (item) => item.type === 'number' && item.value === 0 ? unitValue(0, 'px') : item.type === 'percentage' ? unitValue(item.value, 'percent') : item.type === 'dimension' && UNITS[lowerCase(item.unit)] && UNITS[lowerCase(item.unit)].base === BASE.length ? unitValue(item.value, lowerCase(item.unit)) : item.block && item.type === 'function' && MATH_FUNCTIONS[lowerCase(item.value)] ? (() => { const node = parseMathComponent(item); return node ? nodeToNumeric(node) : null; })() : null;
    const keywordOffset = (name) => name === 'left' || name === 'top' ? unitValue(0, 'percent') : name === 'center' ? unitValue(50, 'percent') : unitValue(100, 'percent');
    const fromEdge = (name, value) => name === 'left' || name === 'top' ? value : createVariadic('sum', toArray([unitValue(100, 'percent'), negateValue(value)]));
    if (parts.length === 1) {
      const item = parts[0];
      if (isKeyword(item, ' left right center ')) return [keywordOffset(lowerCase(item.value)), unitValue(50, 'percent')];
      if (isKeyword(item, ' top bottom ')) return [unitValue(50, 'percent'), keywordOffset(lowerCase(item.value))];
      const value = offset(item); return value ? [value, unitValue(50, 'percent')] : null;
    }
    if (parts.length === 2) {
      const a = parts[0], b = parts[1];
      const aValue = offset(a), bValue = offset(b);
      const aX = isKeyword(a, ' left right '), aY = isKeyword(a, ' top bottom '), aC = isKeyword(a, ' center ');
      const bX = isKeyword(b, ' left right '), bY = isKeyword(b, ' top bottom '), bC = isKeyword(b, ' center ');
      const valueOf = (item, number) => number || keywordOffset(lowerCase(item.value));
      if ((aX || aC || aValue) && (bY || bC || bValue) && !(aC && bX)) return [valueOf(a, aValue), valueOf(b, bValue)];
      if ((aY || aC) && (bX || bC)) return [valueOf(b, bValue), valueOf(a, aValue)];
      return null;
    }
    // Three or four values: keyword-offset pairs.
    const groups = list(); let index = 0;
    while (index < parts.length) {
      const item = parts[index];
      if (item.type !== 'ident') return null;
      const name = lowerCase(item.value);
      const next = parts[index + 1], value = next ? offset(next) : null;
      if (value && name !== 'center') { push(groups, { name, value }); index += 2; } else { push(groups, { name, value: null }); index++; }
    }
    if (groups.length !== 2) return null;
    let first = groups[0], second = groups[1];
    const horizontal = (group) => group.name === 'left' || group.name === 'right';
    const vertical = (group) => group.name === 'top' || group.name === 'bottom';
    if (vertical(first) || (first.name === 'center' && horizontal(second))) { const swap = first; first = second; second = swap; }
    if (!(horizontal(first) || first.name === 'center') || !(vertical(second) || second.name === 'center')) return null;
    const resolve = (group) => group.value ? fromEdge(group.name, group.value) : keywordOffset(group.name);
    return [resolve(first), resolve(second)];
  }
  // CSSTransformComponent::FromCSSValue over one transform function.
  function transformComponent(item) {
    if (!item.block || item.type !== 'function') return null;
    const name = lowerCase(item.value), args = splitComponents(item.children, 'comma');
    const numeric = (group) => { const parts = significant(group); if (parts.length !== 1) return null; const part = parts[0]; if (part.type === 'number') return unitValue(part.value, 'number'); if (part.type === 'percentage') return unitValue(part.value, 'percent'); if (part.type === 'dimension') { const key = lowerCase(part.unit); return UNITS[key] ? unitValue(part.value, key) : null; } if (part.block && part.type === 'function' && MATH_FUNCTIONS[lowerCase(part.value)]) { const node = parseMathComponent(part); return node ? nodeToNumeric(node) : null; } return null; };
    const values = list();
    if (!(args.length === 1 && !args[0].length)) for (let index = 0; index < args.length; index++) { const value = numeric(args[index]); if (!value && name !== 'perspective') return null; push(values, value); }
    // Unitless zero lengths/angles are stored as px/deg.
    const zeroAs = (value, unitKey) => { const record = value && numericRecord(value); return record && record.kind === 'unit' && record.unit === 'number' && record.value === 0 ? unitValue(0, unitKey) : value; };
    const number = (value) => value;
    switch (name) {
      case 'translate': return values.length === 1 ? component('CSSTranslate', { kind: 'translate', x: zeroAs(values[0], 'px'), y: px(0), z: px(0), is2D: true }) : values.length === 2 ? component('CSSTranslate', { kind: 'translate', x: zeroAs(values[0], 'px'), y: zeroAs(values[1], 'px'), z: px(0), is2D: true }) : null;
      case 'translatex': return values.length === 1 ? component('CSSTranslate', { kind: 'translate', x: zeroAs(values[0], 'px'), y: px(0), z: px(0), is2D: true }) : null;
      case 'translatey': return values.length === 1 ? component('CSSTranslate', { kind: 'translate', x: px(0), y: zeroAs(values[0], 'px'), z: px(0), is2D: true }) : null;
      case 'translatez': return values.length === 1 ? component('CSSTranslate', { kind: 'translate', x: px(0), y: px(0), z: zeroAs(values[0], 'px'), is2D: false }) : null;
      case 'translate3d': return values.length === 3 ? component('CSSTranslate', { kind: 'translate', x: zeroAs(values[0], 'px'), y: zeroAs(values[1], 'px'), z: zeroAs(values[2], 'px'), is2D: false }) : null;
      case 'rotate': case 'rotatez': return values.length === 1 ? (name === 'rotate' ? component('CSSRotate', { kind: 'rotate', x: unitValue(0, 'number'), y: unitValue(0, 'number'), z: unitValue(1, 'number'), angle: zeroAs(values[0], 'deg'), is2D: true }) : component('CSSRotate', { kind: 'rotate', x: unitValue(0, 'number'), y: unitValue(0, 'number'), z: unitValue(1, 'number'), angle: zeroAs(values[0], 'deg'), is2D: false })) : null;
      case 'rotatex': return values.length === 1 ? component('CSSRotate', { kind: 'rotate', x: unitValue(1, 'number'), y: unitValue(0, 'number'), z: unitValue(0, 'number'), angle: zeroAs(values[0], 'deg'), is2D: false }) : null;
      case 'rotatey': return values.length === 1 ? component('CSSRotate', { kind: 'rotate', x: unitValue(0, 'number'), y: unitValue(1, 'number'), z: unitValue(0, 'number'), angle: zeroAs(values[0], 'deg'), is2D: false }) : null;
      case 'rotate3d': return values.length === 4 ? component('CSSRotate', { kind: 'rotate', x: number(values[0]), y: number(values[1]), z: number(values[2]), angle: zeroAs(values[3], 'deg'), is2D: false }) : null;
      case 'scale': return values.length === 1 ? component('CSSScale', { kind: 'scale', x: values[0], y: values[0], z: unitValue(1, 'number'), is2D: true }) : values.length === 2 ? component('CSSScale', { kind: 'scale', x: values[0], y: values[1], z: unitValue(1, 'number'), is2D: true }) : null;
      case 'scalex': return values.length === 1 ? component('CSSScale', { kind: 'scale', x: values[0], y: unitValue(1, 'number'), z: unitValue(1, 'number'), is2D: true }) : null;
      case 'scaley': return values.length === 1 ? component('CSSScale', { kind: 'scale', x: unitValue(1, 'number'), y: values[0], z: unitValue(1, 'number'), is2D: true }) : null;
      case 'scalez': return values.length === 1 ? component('CSSScale', { kind: 'scale', x: unitValue(1, 'number'), y: unitValue(1, 'number'), z: values[0], is2D: false }) : null;
      case 'scale3d': return values.length === 3 ? component('CSSScale', { kind: 'scale', x: values[0], y: values[1], z: values[2], is2D: false }) : null;
      case 'skew': return values.length === 1 ? component('CSSSkew', { kind: 'skew', ax: zeroAs(values[0], 'deg'), ay: unitValue(0, 'deg'), is2D: true }) : values.length === 2 ? component('CSSSkew', { kind: 'skew', ax: zeroAs(values[0], 'deg'), ay: zeroAs(values[1], 'deg'), is2D: true }) : null;
      case 'skewx': return values.length === 1 ? component('CSSSkewX', { kind: 'skewX', ax: zeroAs(values[0], 'deg'), is2D: true }) : null;
      case 'skewy': return values.length === 1 ? component('CSSSkewY', { kind: 'skewY', ay: zeroAs(values[0], 'deg'), is2D: true }) : null;
      case 'perspective': {
        const parts = significant(args[0] || list());
        if (args.length !== 1 || parts.length !== 1) return null;
        if (parts[0].type === 'ident' && lowerCase(parts[0].value) === 'none') return component('CSSPerspective', { kind: 'perspective', length: keywordValue('none'), is2D: false });
        const value = values[0]; return value ? component('CSSPerspective', { kind: 'perspective', length: value, is2D: false }) : null;
      }
      case 'matrix': case 'matrix3d': {
        const api = matrices();
        if (!api || values.length !== (name === 'matrix' ? 6 : 16)) return null;
        const entries = list();
        for (let index = 0; index < values.length; index++) { const record = numericRecord(values[index]); if (record.kind !== 'unit' || record.unit !== 'number') return null; push(entries, record.value); }
        const matrix = new api.Matrix(toArray(entries));
        return component('CSSMatrixComponent', { kind: 'matrix', matrix, is2D: name === 'matrix' });
      }
    }
    return null;
  }
  // CreateStyleValueWithPropertyInternal + CreateStyleValue for one value (or list layer).
  function reifyOne(property, layer, mode) {
    const items = significant(layer), name = property.name, single = items.length === 1 ? items[0] : null;
    if (RADIUS[name]) {
      if (single) return genericValue(property, single, mode);
      if (items.length === 2) { const a = genericValue(property, items[0], mode), b = genericValue(property, items[1], mode); if (a && b && numericRecord(a) && numericRecord(b) && equalsValue(a, b)) return a; }
      return null;
    }
    if (name === 'background-size' || name === 'mask-size') {
      if (single) return genericValue(property, single, mode);
      if (items.length === 2 && ident(items[1]) === 'auto') return genericValue(property, items[0], mode);
      return null;
    }
    if (name === 'accent-color' || name === 'caret-color' || COLOR_KEYWORD_ONLY[name]) {
      const id = single && ident(single);
      if (id === 'auto' && (name === 'accent-color' || name === 'caret-color')) return keywordValue('auto');
      if (id === 'currentcolor') return keywordValue('currentcolor');
      return unsupported(mode.normalize(layer), name);
    }
    if (name === 'transform') {
      if (single && single.type === 'ident') return genericValue(property, single, mode);
      const parts = list();
      for (let index = 0; index < items.length; index++) { const part = transformComponent(items[index]); if (!part) return null; push(parts, part); }
      return parts.length ? newTransform(parts) : null;
    }
    if (name === 'offset-anchor' || name === 'offset-position') { if (single && single.type === 'ident' && (ident(single) === 'auto' || ident(single) === 'normal')) return genericValue(property, single, mode); }
    if (name === 'object-position' || name === 'perspective-origin' || name === 'offset-anchor' || name === 'offset-position') { const pair = positionValueOf(items); return pair ? positionValue(pair[0], pair[1]) : null; }
    if (name === 'transform-origin') return null;
    if (name === 'will-change') return single && ident(single) === 'auto' ? keywordValue('auto') : null;
    if (name === 'display' && !single) { let key = ''; for (let index = 0; index < items.length; index++) { if (items[index].type !== 'ident') return null; key += (index ? ' ' : '') + lowerCase(items[index].value); } const mapped = DISPLAY_PAIRS[key]; return mapped && indexOfText(mapped, ' ') < 0 ? keywordValue(mapped) : null; }
    if (name === 'grid-auto-flow') { const kept = list(); for (let index = 0; index < items.length; index++) if (!(items.length > 1 && ident(items[index]) === 'row')) push(kept, items[index]); return kept.length === 1 ? genericValue(property, kept[0], mode) : null; }
    // Property names in transition-property are custom idents naming a known property.
    if (name === 'transition-property') { if (!single || single.type !== 'ident') return null; const id = lowerCase(single.value); return id === 'none' ? keywordValue('none') : PROPERTIES[id] || id === 'all' ? null : genericValue(property, single, mode); }
    if (SINGLE_ONLY[name]) return single ? genericValue(property, single, mode) : null;
    return single ? genericValue(property, single, mode) : null;
  }
  // CssValueToStyleValueVector over a value's component list (CSS-wide keywords, var() and
  // shorthands are handled by the caller).
  // Longhands Blink stores as quads or per-gap value lists, which Typed OM level 1 does not reify.
  const STRUCTURED = plainSet(['border-image-outset', 'border-image-repeat', 'border-image-slice', 'border-image-width', 'column-rule-style', 'column-rule-width', 'row-rule-style', 'row-rule-width', 'position-visibility']);
  function reifyValue(property, items, mode) {
    if (!property.meta.types || STRUCTURED[property.name]) return [unsupported(mode.normalize(items), property.name)];
    if (isRepeated(property)) {
      const layers = LAYERED_SPACE[property.name] ? (() => { const result = list(), parts = significant(items); for (let index = 0; index < parts.length; index++) push(result, plain([parts[index]])); return result; })() : splitComponents(items, 'comma');
      const whole = layers.length === 1 ? reifyOne(property, items, mode) : null;
      if (whole) return [whole];
      const values = list();
      for (let index = 0; index < layers.length; index++) { const value = reifyOne(property, layers[index], mode); if (!value) return [unsupported(mode.normalize(items), property.name)]; push(values, value); }
      return toArray(values);
    }
    const value = reifyOne(property, items, mode);
    return [value || unsupported(mode.normalize(items), property.name)];
  }
  // Serialization of a stored/computed value for CSSUnsupportedStyleValue (CSSValue::CssText):
  // tokens re-serialized with collapsed whitespace, ", " separators and canonical numbers.
  function normalizedText(items) {
    let text = '', space = false;
    for (let index = 0; index < items.length; index++) {
      const item = items[index];
      if (item.type === 'whitespace') { space = text !== ''; continue; }
      if (item.type === 'comma') { text += ', '; space = false; continue; }
      if (space && !endsWithText(text, ' ')) text += ' ';
      space = false;
      text += componentCssText(item);
    }
    return text;
  }
  const endsWithText = (text, suffix) => text.length >= suffix.length && sliceText(text, text.length - suffix.length) === suffix;
  function componentCssText(item) {
    if (item.block) {
      if (item.type === 'function' && MATH_FUNCTIONS[lowerCase(item.value)]) { const node = parseMathComponent(item); if (node) return node.t === 'lit' ? 'calc(' + formatNumber(node.value, node.unit) + ')' : mathFunctionText(node); }
      const open = item.type === 'function' ? lowerCase(item.value) + '(' : item.type;
      return open + normalizedText(item.children) + BLOCK_END[item.type];
    }
    switch (item.type) {
      case 'ident': { const lower = lowerCase(item.value); return KEYWORD_IDS[lower] ? lower : serializeIdentifier(item.value); }
      case 'number': return formatNumber(item.value, 'number');
      case 'percentage': return formatNumber(item.value, 'percent');
      case 'dimension': { const key = lowerCase(item.unit); return UNITS[key] ? formatNumber(item.value, key) : formatNumber(item.value, 'number') + serializeIdentifier(item.unit); }
      case 'string': return serializeString(item.value);
      case 'url': return 'url(' + serializeString(item.value) + ')';
      case 'hash': return '#' + item.value;
      default: return serializeToken(item);
    }
  }
  // Text modes: parse() keeps the author's text for a single unsupported result; stored and
  // computed values serialize their parsed form.
  const STORED = { normalize: (items) => normalizedText(items) };
  // StyleValueFactory::FromString.
  function fromString(property, text) {
    const tokens = tokenize(text), items = components(tokens), meaningful = significant(items), whole = trimSpace(items);
    if (!property.custom && validValue(property, text)) {
      if (meaningful.length === 1 && meaningful[0].type === 'ident' && CSS_WIDE[lowerCase(meaningful[0].value)]) return [keywordValue(lowerCase(meaningful[0].value))];
      if (isShorthand(property)) return [unsupported(text, property.name)];
      // A single unsupported result keeps the author's text (StyleValueFactory::FromString).
      return reifyValue(property, whole, { normalize: (parts) => parts === whole ? text : normalizedText(parts) });
    }
    if (!declarationValueOk(tokens, true) || !substitutionsOk(items)) return [];
    const original = trimText(text);
    if ((property.custom && original.length > 0) || hasSubstitution(items)) return [unparsedFromText(original)];
    return [];
  }
  // Reify a declaration value read from a CSSStyleDeclaration (inline style or a style rule).
  function reifyStored(property, text) {
    const tokens = tokenize(text), items = components(tokens), meaningful = significant(items);
    if (property.custom || hasSubstitution(meaningful)) return [unparsedFromText(trimText(text))];
    if (meaningful.length === 1 && meaningful[0].type === 'ident' && CSS_WIDE[lowerCase(meaningful[0].value)]) return [keywordValue(lowerCase(meaningful[0].value))];
    if (isShorthand(property)) return [unsupported(normalizedText(trimSpace(items)), property.name)];
    return reifyValue(property, trimSpace(items), STORED);
  }

  staticMethod('CSSStyleValue', 'parse', 2, function (name, cssText) { requireArguments(arguments, 2); return parseValues(name, cssText)[0]; });
  staticMethod('CSSStyleValue', 'parseAll', 2, function (name, cssText) { requireArguments(arguments, 2); return toArray(parseValues(name, cssText)); });
  function parseValues(name, cssText) {
    const propertyText = toStringValue(name), text = toStringValue(cssText);
    const property = propertyName(propertyText);
    if (!property) throw fail('Invalid property name');
    const values = fromString(property, text);
    if (!values.length) throw fail("The value provided ('" + text + "') could not be parsed as a '" + propertyText + "'.");
    return values;
  }

  // ---- Serializing typed values into declarations (StyleValueToCSSValue) -------------------------
  const typeMatches = (value, typeName) => {
    const record = recordOf(value);
    switch (typeName) {
      case 'Length': return !!record.numeric && matchesBase(record.type, BASE.length);
      case 'Number': return !!record.numeric && matchesNumber(record.type);
      case 'Time': return !!record.numeric && matchesBase(record.type, BASE.time);
      case 'Angle': return !!record.numeric && matchesBase(record.type, BASE.angle);
      case 'Percentage': return !!record.numeric && matchesPercentage(record.type);
      case 'Resolution': return !!record.numeric && matchesBase(record.type, BASE.resolution);
      case 'Flex': return !!record.numeric && matchesBase(record.type, BASE.flex);
      case 'Image': return record.family === 'CSSImageValue';
      case 'Transform': return record.family === 'CSSTransformValue';
      case 'Position': return record.family === 'CSSPositionValue';
    }
    return false;
  };
  // CSSOMTypes::PropertyCanTake.
  function propertyCanTake(property, value) {
    const record = recordOf(value);
    if (record.family === 'CSSKeywordValue') {
      const id = keywordId(record.value);
      if (id !== null && CSS_WIDE[id] && id !== 'revert-layer') return !property.custom;
      if (id === 'revert-layer') return !property.custom;
      if (property.custom) return false;
      if (property.meta.customIdent) return id !== 'default';
      if (id === null) return false;
      return hasType(property.meta, 'Keyword') && hasKeyword(property.meta, id);
    }
    if (record.family === 'CSSStyleValue') return !!record.property && record.property === property.name;
    if (record.family === 'CSSUnparsedValue') return unparsedDeclarationOk(record);
    if (property.custom) return false;
    const types = property.meta.types;
    if (!types) return false;
    for (let index = 0; index < types.length; index++) if (types[index] !== 'Keyword' && typeMatches(value, types[index])) return true;
    return false;
  }
  function unparsedDeclarationOk(record) { const text = unparsedText(record); const tokens = tokenize(text); return declarationValueOk(tokens, true) && substitutionsOk(components(tokens)); }
  // The declaration text of one typed value for `property`, or null when it cannot take it.
  function valueDeclarationText(property, value) {
    if (!propertyCanTake(property, value)) return null;
    const record = recordOf(value);
    if (record.family === 'CSSStyleValue') { const text = record.text; return validValue(property, text) ? normalizedText(trimSpace(components(tokenize(text)))) : null; }
    if (record.family === 'CSSKeywordValue') {
      const id = keywordId(record.value);
      if (!property.custom && property.meta.customIdent && !(id !== null && CSS_WIDE[id])) { const text = keywordText(record.value); return validValue(property, text) ? text : null; }
      return keywordText(record.value);
    }
    if (record.family === 'CSSUnparsedValue') return unparsedText(record);
    if (record.numeric) {
      if (record.kind === 'unit') {
        const wrapped = !property.custom && outOfRange(property.name, record.value, record.unit);
        writtenNode = wrapped ? nest(lit(record.value, record.unit)) : lit(record.value, record.unit);
        return wrapped ? 'calc(' + formatNumber(record.value, record.unit) + ')' : formatNumber(record.value, record.unit);
      }
      const node = numericToNode(value);
      writtenNode = node;
      return node ? mathFunctionText(node) : null;
    }
    return styleText(record, value) || null;
  }
  const isCssWideText = (text) => { const items = significant(components(tokenize(text))); return items.length === 1 && items[0].type === 'ident' && !!CSS_WIDE[lowerCase(items[0].value)]; };
  // (CSSStyleValue or DOMString)... conversion.
  function styleValuesOrStrings(args, from) { const result = list(); for (let index = from; index < args.length; index++) { const value = args[index]; push(result, brandedAs(value, 'CSSStyleValue') ? value : toStringValue(value)); } return result; }
  // CoerceStyleValueOrString / CoerceStyleValuesOrStrings: the declaration text, or null.
  // The calc tree a typed numeric value stores, which reads back as that tree (Blink keeps the
  // CSSValue it set: a wrapped out-of-range literal stays a CSSMathSum, 50% stays a percent).
  let writtenNode = null;
  function coerceSingle(property, value) {
    if (typeof value !== 'string') return valueDeclarationText(property, value);
    const values = fromString(property, value);
    return values.length === 1 ? valueDeclarationText(property, values[0]) : null;
  }
  function coerceList(property, values) {
    if (!values.length) return null;
    const styleValues = list();
    for (let index = 0; index < values.length; index++) {
      const value = values[index];
      if (typeof value !== 'string') { push(styleValues, value); continue; }
      const parsed = fromString(property, value);
      if (!parsed.length) return null;
      for (let at = 0; at < parsed.length; at++) push(styleValues, parsed[at]);
    }
    const parts = list();
    for (let index = 0; index < styleValues.length; index++) {
      const text = valueDeclarationText(property, styleValues[index]);
      if (text === null) return null;
      if (isCssWideText(text) || recordOf(styleValues[index]).family === 'CSSUnparsedValue') return styleValues.length === 1 ? { single: text } : null;
      push(parts, text);
    }
    return { parts };
  }
  const separatorOf = (property) => property.meta.separator === ',' ? ', ' : ' ';
  const joinList = (items, separator) => { let text = ''; for (let index = 0; index < items.length; index++) text += (index ? separator : '') + items[index]; return text; };

  // ---- StylePropertyMap ----------------------------------------------------------------------
  const declarationPrototype = g.CSSStyleDeclaration.prototype;
  const declarationMethod = (name) => declarationPrototype[name];
  const declarationGet = declarationMethod('getPropertyValue'), declarationSet = declarationMethod('setProperty'), declarationRemove = declarationMethod('removeProperty'), declarationItem = declarationMethod('item');
  const declarationLength = Object.getOwnPropertyDescriptor(declarationPrototype, 'length').get;
  const ruleStyle = Object.getOwnPropertyDescriptor(g.CSSStyleRule.prototype, 'style').get;
  // A declaration-backed map reads the live CSSStyleDeclaration of its owner.
  function declarationOf(record) { return record.kind === 'inline' ? ns.makeInlineStyleDeclaration(record.owner) : apply(ruleStyle, record.owner, []); }
  function declarationNames(record) { const declaration = declarationOf(record), length = apply(declarationLength, declaration, []), names = list(); for (let index = 0; index < length; index++) push(names, apply(declarationItem, declaration, [index])); return names; }
  function declarationValue(record, name) { const declaration = declarationOf(record), text = apply(declarationGet, declaration, [name]); return text === '' ? null : text; }
  // Blink stores box shorthands as their longhands; the binding keeps the authored shorthand,
  // so the map's view expands them in place (a later longhand replaces its slot).
  const SPLIT_SHORTHANDS = plainSet(['margin', 'padding', 'inset', 'scroll-margin', 'scroll-padding', 'border-width', 'border-style', 'border-color', 'gap', 'overflow']);
  function splitShorthand(name, text) {
    const meta = PROPERTIES[name], items = components(tokenize(text));
    if (!meta || !meta.longhands || hasSubstitution(items)) return null;
    const parts = significant(items), longhands = meta.longhands, result = list();
    const textOfPart = (item) => normalizedText(plain([item]));
    if (parts.length === 1 && parts[0].type === 'ident' && CSS_WIDE[lowerCase(parts[0].value)]) { for (let index = 0; index < longhands.length; index++) push(result, [longhands[index], lowerCase(parts[0].value)]); return result; }
    if (longhands.length === 4) {
      if (!parts.length || parts.length > 4) return null;
      const box = parts.length === 1 ? [parts[0], parts[0], parts[0], parts[0]] : parts.length === 2 ? [parts[0], parts[1], parts[0], parts[1]] : parts.length === 3 ? [parts[0], parts[1], parts[2], parts[1]] : [parts[0], parts[1], parts[2], parts[3]];
      for (let index = 0; index < 4; index++) push(result, [longhands[index], textOfPart(box[index])]);
      return result;
    }
    if (longhands.length === 2 && parts.length >= 1 && parts.length <= 2) { push(result, [longhands[0], textOfPart(parts[0])]); push(result, [longhands[1], textOfPart(parts[parts.length - 1])]); return result; }
    return null;
  }
  function storedView(record) {
    const names = declarationNames(record), entries = list(), slots = create(null);
    for (let index = 0; index < names.length; index++) {
      const name = names[index], text = declarationValue(record, name);
      if (text === null) continue;
      const pairs = SPLIT_SHORTHANDS[name] ? splitShorthand(name, text) : null;
      const list0 = pairs || [[name, text]];
      for (let at = 0; at < list0.length; at++) { const entry = list0[at]; if (entry[0] in slots) entries[slots[entry[0]]][1] = entry[1]; else { slots[entry[0]] = entries.length; push(entries, [entry[0], entry[1]]); } }
    }
    return entries;
  }
  function storedNames(record) { const entries = storedView(record), names = list(); for (let index = 0; index < entries.length; index++) push(names, entries[index][0]); return names; }
  function storedText(record, name) { const entries = storedView(record); for (let index = 0; index < entries.length; index++) if (entries[index][0] === name) return entries[index][1]; return null; }
  const writtenValues = new WeakMap();
  function writeDeclaration(record, name, text, node) {
    apply(declarationSet, declarationOf(record), [name, text]);
    let memo = weakGet(writtenValues, record.owner);
    if (!memo) { memo = create(null); weakSet(writtenValues, record.owner, memo); }
    const stored = storedText(record, name);
    if (node && stored !== null) memo[name] = { text: stored, node }; else delete memo[name];
  }
  function storedValues(record, property, text) {
    const memo = weakGet(writtenValues, record.owner), entry = memo && memo[property.name];
    if (entry && entry.text === text) { const value = nodeToNumeric(entry.node); if (value) return [value]; }
    return reifyStored(property, text);
  }
  function removeDeclaration(record, name) {
    const declaration = declarationOf(record);
    if (declarationValue(record, name) === null) {
      // The longhand lives inside a stored shorthand: re-store the shorthand's other longhands.
      const names = declarationNames(record);
      for (let index = 0; index < names.length; index++) {
        const owner = names[index], pairs = SPLIT_SHORTHANDS[owner] ? splitShorthand(owner, declarationValue(record, owner)) : null;
        if (!pairs) continue;
        let found = false; for (let at = 0; at < pairs.length; at++) if (pairs[at][0] === name) found = true;
        if (!found) continue;
        apply(declarationRemove, declaration, [owner]);
        for (let at = 0; at < pairs.length; at++) if (pairs[at][0] !== name && declarationValue(record, pairs[at][0]) === null) apply(declarationSet, declaration, [pairs[at][0], pairs[at][1]]);
      }
      return;
    }
    apply(declarationRemove, declaration, [name]);
  }
  const MAP_READ = brandSets.StylePropertyMapReadOnly, MAP_WRITE = brandSets.StylePropertyMap;
  defineInterface('StylePropertyMapReadOnly', null, 0, null);
  defineInterface('StylePropertyMap', 'StylePropertyMapReadOnly', 0, null);
  const mapRecord = (self) => brand(self, MAP_READ);
  // getAll over a property name, through the map's backing store.
  function mapValues(record, property) {
    if (record.kind === 'computed') return computedValues(record.owner, property);
    if (isShorthand(property) && !storedText(record, property.name)) return shorthandFromLonghands(record, property);
    const text = storedText(record, property.name);
    return text === null ? [] : storedValues(record, property, text);
  }
  // A shorthand reads as its serialization over the stored longhands (all equal, or 4-side boxes).
  function shorthandFromLonghands(record, property) {
    const longhands = property.meta.longhands, texts = list();
    for (let index = 0; index < longhands.length; index++) { const text = storedText(record, longhands[index]); if (text === null) return []; push(texts, normalizedText(trimSpace(components(tokenize(text))))); }
    let text;
    if (longhands.length === 4 && endsWithText(longhands[0], 'top')) { const top = texts[0], right = texts[1], bottom = texts[2], left = texts[3]; text = left !== right ? top + ' ' + right + ' ' + bottom + ' ' + left : bottom !== top ? top + ' ' + right + ' ' + bottom : right !== top ? top + ' ' + right : top; }
    else { let same = true; for (let index = 1; index < texts.length; index++) if (texts[index] !== texts[0]) same = false; if (!same) return []; text = texts[0]; }
    return [unsupported(text, property.name)];
  }
  const readName = (args) => { requireArguments(args, 1); return toStringValue(args[0]); };
  method('StylePropertyMapReadOnly', 'get', 1, function () {
    const record = mapRecord(this), text = readName(arguments), property = propertyName(text);
    if (!property) throw fail('Invalid propertyName: ' + text);
    const values = mapValues(record, property);
    return values.length ? values[0] : undefined;
  });
  method('StylePropertyMapReadOnly', 'getAll', 1, function () {
    const record = mapRecord(this), text = readName(arguments), property = propertyName(text);
    if (!property) throw fail('Invalid propertyName: ' + text);
    return toArray(mapValues(record, property));
  });
  method('StylePropertyMapReadOnly', 'has', 1, function () {
    const record = mapRecord(this), text = readName(arguments), property = propertyName(text);
    if (!property) throw fail('Invalid propertyName: ' + text);
    return mapValues(record, property).length > 0;
  });
  attribute('StylePropertyMapReadOnly', 'size', (self) => { const record = mapRecord(self); return record.kind === 'computed' ? computedSize(record.owner) : storedNames(record).length; });
  // Entries in the store's order (declaration order, or Chrome's computed ordering).
  function mapEntries(record) {
    if (record.kind === 'computed') return computedEntries(record.owner);
    const names = storedNames(record), entries = list();
    for (let index = 0; index < names.length; index++) {
      const name = names[index], property = propertyName(name);
      if (!property) continue;
      const text = storedText(record, name);
      push(entries, [property.custom ? name : property.name, toArray(text === null ? [] : storedValues(record, property, text))]);
    }
    return entries;
  }
  // Pair iterator objects share one %StylePropertyMapReadOnly Iterator% prototype.
  const IteratorPrototype = getProto(getProto(apply(arrayIterator, [], [])));
  const mapIteratorPrototype = create(IteratorPrototype), iteratorStates = new WeakMap();
  define(mapIteratorPrototype, 'next', { __proto__: null, value: make(function () {
    const state = weakGet(iteratorStates, this);
    if (!state) throw new ErrorType("Failed to execute 'next' on 'StylePropertyMapReadOnly Iterator': Illegal invocation");
    if (state.index >= state.entries.length) return { value: undefined, done: true };
    const entry = state.entries[state.index++];
    return { value: state.kind === 'keys' ? entry[0] : state.kind === 'values' ? entry[1] : [entry[0], entry[1]], done: false };
  }, 'next', 0), writable: true, enumerable: true, configurable: true });
  define(mapIteratorPrototype, symbolTag, { __proto__: null, value: 'StylePropertyMapReadOnly Iterator', writable: false, enumerable: false, configurable: true });
  function mapIterator(self, kind) { const record = mapRecord(self), iterator = create(mapIteratorPrototype); weakSet(iteratorStates, iterator, { entries: mapEntries(record), index: 0, kind }); return iterator; }
  {
    const proto = prototypes.StylePropertyMapReadOnly;
    const entries = make(function () { return mapIterator(this, 'entries'); }, 'entries', 0);
    define(proto, 'entries', { __proto__: null, value: entries, writable: true, enumerable: true, configurable: true });
    define(proto, 'forEach', { __proto__: null, value: make(function (callback) {
      const self = this, args = arguments;
      return enter((message) => executeMessage('forEach', 'StylePropertyMapReadOnly', message), () => {
        const record = mapRecord(self);
        if (typeof callback !== 'function') throw fail('The callback provided as parameter 1 is not a function.');
        const items = mapEntries(record);
        const receiver = args[1] === undefined ? g : args[1];
        for (let index = 0; index < items.length; index++) apply(callback, receiver, [items[index][1], items[index][0], self]);
      });
    }, 'forEach', 1), writable: true, enumerable: true, configurable: true });
    define(proto, 'keys', { __proto__: null, value: make(function () { return mapIterator(this, 'keys'); }, 'keys', 0), writable: true, enumerable: true, configurable: true });
    define(proto, 'values', { __proto__: null, value: make(function () { return mapIterator(this, 'values'); }, 'values', 0), writable: true, enumerable: true, configurable: true });
    define(proto, symbolIterator, { __proto__: null, value: entries, writable: true, enumerable: false, configurable: true });
  }
  const writableRecord = (self) => brand(self, MAP_WRITE);
  method('StylePropertyMap', 'set', 1, function () {
    const record = writableRecord(this), text = readName(arguments), property = propertyName(text);
    const values = styleValuesOrStrings(arguments, 1);
    if (!property || (!property.custom && property.meta.descriptor)) throw fail('Invalid propertyName: ' + text);
    if (isShorthand(property)) {
      if (values.length !== 1) throw fail('Invalid type for property');
      const value = values[0];
      const cssText = typeof value === 'string' ? value : propertyCanTake(property, value) ? styleText(recordOf(value), value) : '';
      if (cssText === '' || !validValue(property, cssText)) throw fail('Invalid type for property');
      writeDeclaration(record, property.name, cssText);
      return undefined;
    }
    let declaration = null;
    writtenNode = null;
    if (isRepeated(property)) { const result = coerceList(property, values); declaration = result ? (result.single !== undefined ? result.single : joinList(result.parts, separatorOf(property))) : null; }
    else if (values.length === 1) declaration = coerceSingle(property, values[0]);
    if (declaration === null) throw fail('Invalid type for property');
    writeDeclaration(record, property.custom ? text : property.name, declaration, !isRepeated(property) && !property.custom ? writtenNode : null);
    return undefined;
  }, 'StylePropertyMap');
  method('StylePropertyMap', 'append', 1, function () {
    const record = writableRecord(this), text = readName(arguments), values = styleValuesOrStrings(arguments, 1);
    if (!values.length) return undefined;
    const property = propertyName(text);
    if (!property) throw fail('Invalid propertyName: ' + text);
    if (property.custom) throw fail('Appending to custom properties is not supported');
    if (!isRepeated(property)) throw fail('Property does not support multiple values');
    const current = storedText(record, property.name);
    if (current !== null) {
      if (hasSubstitution(components(tokenize(current)))) throw fail('Cannot append to a list containing a variable reference');
      if (isCssWideText(current)) throw fail('Cannot append to something that is not a list');
    }
    const result = coerceList(property, values);
    if (!result || result.single !== undefined) throw fail('Invalid type for property');
    const joined = joinList(result.parts, separatorOf(property));
    writeDeclaration(record, property.name, current === null ? joined : normalizedText(trimSpace(components(tokenize(current)))) + separatorOf(property) + joined);
    return undefined;
  }, 'StylePropertyMap');
  method('StylePropertyMap', 'delete', 1, function () {
    const record = writableRecord(this), text = readName(arguments), property = propertyName(text);
    if (!property) throw fail('Invalid property name: ' + text);
    removeDeclaration(record, property.custom ? text : property.name);
    if (isShorthand(property)) { const longhands = property.meta.longhands; for (let index = 0; index < longhands.length; index++) removeDeclaration(record, longhands[index]); }
    return undefined;
  }, 'StylePropertyMap');
  const declarationText = Object.getOwnPropertyDescriptor(declarationPrototype, 'cssText').set;
  method('StylePropertyMap', 'clear', 0, function () {
    const record = writableRecord(this);
    if (declarationNames(record).length) apply(declarationText, declarationOf(record), ['']);
    return undefined;
  }, 'StylePropertyMap');
  const newMap = (name, kind, owner) => instance(name, { kind, owner });

  // ---- Computed values (computedStyleMap) ------------------------------------------------------
  // Specified values come from the binding's cascade (author sheets, inline style, shadow
  // scopes); unspecified properties inherit or take the registry's initial value. Values are
  // then computed without layout, as ComputedStylePropertyMap reads them (layout_object null):
  // absolute lengths, font-relative and viewport units resolve to px, percentages stay, colors
  // and currentcolor resolve, and keywords lowercase.
  const cascadeHook = ns.typedOmCascade; delete ns.typedOmCascade;
  const computedReader = g.getComputedStyle;
  const computedGet = (style, name) => apply(declarationGet, style, [name]);
  const GROUPS = create(null);
  { const names = ownKeys(PROPERTIES); for (let index = 0; index < names.length; index++) { const meta = PROPERTIES[names[index]]; if (!meta.group || meta.alias) continue; const [group, resolver] = meta.group; if (!GROUPS[group]) GROUPS[group] = create(null); GROUPS[group][resolver] = names[index]; } }
  const LOGICAL_RESOLVERS = plainSet(['inline', 'block', 'inline-start', 'inline-end', 'block-start', 'block-end', 'start-start', 'start-end', 'end-start', 'end-end']);
  function physicalResolver(resolver, mode, rtl) {
    const vertical = mode === 'vertical-rl' || mode === 'vertical-lr' || mode === 'sideways-rl' || mode === 'sideways-lr';
    const inlineStart = vertical ? (rtl ? 'bottom' : 'top') : (rtl ? 'right' : 'left'), inlineEnd = vertical ? (rtl ? 'top' : 'bottom') : (rtl ? 'left' : 'right');
    const blockStart = vertical ? (mode === 'vertical-rl' || mode === 'sideways-rl' ? 'right' : 'left') : 'top', blockEnd = vertical ? (blockStart === 'right' ? 'left' : 'right') : 'bottom';
    const corner = (block, inline) => { const sides = [block, inline]; const vertical0 = sides[0] === 'top' || sides[0] === 'bottom' ? sides[0] : sides[1]; const horizontal0 = vertical0 === sides[0] ? sides[1] : sides[0]; return vertical0 + '-' + horizontal0; };
    switch (resolver) {
      case 'inline': return vertical ? 'vertical' : 'horizontal';
      case 'block': return vertical ? 'horizontal' : 'vertical';
      case 'inline-start': return inlineStart; case 'inline-end': return inlineEnd;
      case 'block-start': return blockStart; case 'block-end': return blockEnd;
      case 'start-start': return corner(blockStart, inlineStart); case 'start-end': return corner(blockStart, inlineEnd);
      case 'end-start': return corner(blockEnd, inlineStart); case 'end-end': return corner(blockEnd, inlineEnd);
    }
    return resolver;
  }
  const CURRENTCOLOR_INITIAL = plainSet(['border-top-color', 'border-right-color', 'border-bottom-color', 'border-left-color', 'border-block-start-color', 'border-block-end-color', 'border-inline-start-color', 'border-inline-end-color', 'outline-color', 'column-rule-color', 'row-rule-color', 'text-decoration-color', 'text-emphasis-color', 'caret-color', '-webkit-text-fill-color', '-webkit-text-stroke-color']);
  const BORDER_WIDTH = plainSet(['border-top-width', 'border-right-width', 'border-bottom-width', 'border-left-width', 'outline-width', 'column-rule-width', 'row-rule-width']);
  const FONT_FROM_BINDING = plainSet(['font-size', 'font-family', 'font-weight', 'font-style', 'font-stretch', 'font-variant-caps']);
  const SVG_NAMESPACE = 'http://www.w3.org/2000/svg', HTML_NAMESPACE = 'http://www.w3.org/1999/xhtml';
  function computedContext(element) {
    const result = cascadeHook.cascade(element), cascaded = result ? result.values : create(null), order = result ? result.order : create(null);
    expandTimingShorthands(cascaded, order);
    return { __proto__: null, element, cascaded, order, cache: create(null), style: null, parent: undefined };
  }
  // transition / animation shorthands the binding's cascade leaves unexpanded.
  const EASING = plainSet(['ease', 'linear', 'ease-in', 'ease-out', 'ease-in-out', 'step-start', 'step-end']);
  const EASING_FUNCTIONS = plainSet(['cubic-bezier', 'steps', 'linear']);
  const TRANSITION = ['transition-property', 'transition-duration', 'transition-timing-function', 'transition-delay', 'transition-behavior'];
  const ANIMATION = ['animation-name', 'animation-duration', 'animation-timing-function', 'animation-delay', 'animation-iteration-count', 'animation-direction', 'animation-fill-mode', 'animation-play-state'];
  function expandTimingShorthands(cascaded, order) {
    const expand = (shorthand, longhands, assign) => {
      if (!(shorthand in cascaded)) return;
      const layers = splitComponents(components(tokenize(cascaded[shorthand])), 'comma'), columns = create(null);
      for (let index = 0; index < longhands.length; index++) columns[longhands[index]] = list();
      for (let index = 0; index < layers.length; index++) { const values = assign(significant(layers[index])); for (let at = 0; at < longhands.length; at++) push(columns[longhands[at]], values[longhands[at]]); }
      for (let index = 0; index < longhands.length; index++) { const name = longhands[index]; if (name in cascaded && order[name] > order[shorthand]) continue; cascaded[name] = joinList(columns[name], ', '); order[name] = order[shorthand]; }
    };
    const timing = (item) => (item.type === 'ident' && EASING[lowerCase(item.value)]) || (item.block && item.type === 'function' && EASING_FUNCTIONS[lowerCase(item.value)]);
    const isTime = (item) => item.type === 'dimension' && UNITS[lowerCase(item.unit)] && UNITS[lowerCase(item.unit)].base === BASE.time;
    expand('transition', TRANSITION, (items) => {
      const values = { __proto__: null, 'transition-property': 'all', 'transition-duration': '0s', 'transition-timing-function': 'ease', 'transition-delay': '0s', 'transition-behavior': 'normal' };
      let times = 0;
      for (let index = 0; index < items.length; index++) {
        const item = items[index], text = componentCssText(item), id = ident(item);
        if (isTime(item)) values[times++ ? 'transition-delay' : 'transition-duration'] = text;
        else if (timing(item)) values['transition-timing-function'] = text;
        else if (id === 'normal' || id === 'allow-discrete') values['transition-behavior'] = id;
        else values['transition-property'] = text;
      }
      return values;
    });
    expand('animation', ANIMATION, (items) => {
      const values = { __proto__: null, 'animation-name': 'none', 'animation-duration': 'auto', 'animation-timing-function': 'ease', 'animation-delay': '0s', 'animation-iteration-count': '1', 'animation-direction': 'normal', 'animation-fill-mode': 'none', 'animation-play-state': 'running' };
      let times = 0;
      for (let index = 0; index < items.length; index++) {
        const item = items[index], text = componentCssText(item), id = ident(item);
        if (isTime(item)) values[times++ ? 'animation-delay' : 'animation-duration'] = text;
        else if (timing(item)) values['animation-timing-function'] = text;
        else if (item.type === 'number' || id === 'infinite') values['animation-iteration-count'] = text;
        else if (id === 'reverse' || id === 'alternate' || id === 'alternate-reverse') values['animation-direction'] = id;
        else if (id === 'forwards' || id === 'backwards' || id === 'both') values['animation-fill-mode'] = id;
        else if (id === 'running' || id === 'paused') values['animation-play-state'] = id;
        else values['animation-name'] = text;
      }
      return values;
    });
  }
  function parentContext(context, contexts) {
    if (context.parent !== undefined) return context.parent;
    const parent = cascadeHook.parent(context.element);
    context.parent = parent ? contextFor(parent, contexts) : null;
    return context.parent;
  }
  function contextFor(element, contexts) { let context = mapGet(contexts, element); if (!context) { context = computedContext(element); context.contexts = contexts; mapSet(contexts, element, context); } return context; }
  const bindingStyle = (context) => context.style || (context.style = apply(computedReader, g, [context.element]));
  function initialText(name) {
    const meta = PROPERTIES[name];
    if (CURRENTCOLOR_INITIAL[name]) return 'currentcolor';
    const values = meta.initial;
    return values ? joinList(values, meta.separator === ',' ? ', ' : ' ') : '';
  }
  // UA stylesheet values the cascade does not carry: HTML display, [hidden], body margins.
  function uaText(context, name) {
    const element = context.element;
    if (name === 'display') {
      if (apply(namespaceOf, element, []) !== HTML_NAMESPACE) return 'inline';
      if (attributeOf(element, 'hidden') !== null) return 'none';
      const tag = apply(tagOf, element, []);
      if (tag === 'html') return 'block';
      if (tag === 'dialog' && attributeOf(element, 'open') !== null) return 'block';
      return registry.display[tag] || 'inline';
    }
    if (startsWith(name, 'margin-') && apply(namespaceOf, element, []) === HTML_NAMESPACE && apply(tagOf, element, []) === 'body') return '8px';
    if (name === 'unicode-bidi' && apply(namespaceOf, element, []) === HTML_NAMESPACE) return registry.unicodeBidi[apply(tagOf, element, [])] || null;
    return null;
  }
  // The physical property's cascaded value, or a later-declared logical counterpart's.
  function specifiedText(context, name) {
    const cascaded = context.cascaded, order = context.order;
    let text = name in cascaded ? cascaded[name] : undefined, rank = name in cascaded ? order[name] || 0 : -1;
    const surrogate = SURROGATE_OF[name];
    if (surrogate && surrogate in cascaded && (order[surrogate] || 0) > rank) { text = mapKeyword(SURROGATES[surrogate][1], lowerCase(trimText(cascaded[surrogate]))); rank = order[surrogate] || 0; }
    const meta = PROPERTIES[name];
    if (meta.group && !LOGICAL_RESOLVERS[meta.group[1]]) {
      const group = GROUPS[meta.group[0]], resolvers = ownKeys(group);
      for (let index = 0; index < resolvers.length; index++) {
        const resolver = resolvers[index];
        if (!LOGICAL_RESOLVERS[resolver]) continue;
        const logical = group[resolver];
        if (logical in cascaded && (order[logical] || 0) > rank && physicalResolver(resolver, computeText(context, 'writing-mode'), computeText(context, 'direction') === 'rtl') === meta.group[1]) { text = cascaded[logical]; rank = order[logical] || 0; }
      }
    }
    return text;
  }
  // Blink surrogate longhands (css_properties.json5 surrogate_for) compute from their target;
  // the legacy keywords map onto the standard ones.
  const SURROGATES = { __proto__: null, '-webkit-writing-mode': ['writing-mode', null], '-webkit-text-orientation': ['text-orientation', { __proto__: null, mixed: 'vertical-right', 'vertical-right': 'mixed' }], 'app-region': ['window-drag', null], '-webkit-box-decoration-break': ['box-decoration-break', null], '-webkit-line-break': ['line-break', null], '-webkit-ruby-position': ['ruby-position', { __proto__: null, over: 'before', under: 'after', before: 'over', after: 'under' }], '-webkit-text-combine': ['text-combine-upright', { __proto__: null, all: 'horizontal', horizontal: 'all' }] };
  const SURROGATE_OF = create(null);
  { const names = ownKeys(SURROGATES); for (let index = 0; index < names.length; index++) SURROGATE_OF[SURROGATES[names[index]][0]] = names[index]; }
  const mapKeyword = (map, text) => map && text in map ? map[text] : text;
  // The computed value text of a longhand.
  function computeText(context, name) {
    if (name in context.cache) return context.cache[name];
    if (SURROGATES[name]) { const target = SURROGATES[name][0], map = SURROGATES[name][1]; const value = mapKeyword(map, computeText(context, target)); context.cache[name] = value; return value; }
    context.cache[name] = initialText(name);
    const meta = PROPERTIES[name];
    let result;
    if (meta.group && LOGICAL_RESOLVERS[meta.group[1]] && name !== 'writing-mode' && name !== 'direction') {
      const physical = GROUPS[meta.group[0]][physicalResolver(meta.group[1], computeText(context, 'writing-mode'), computeText(context, 'direction') === 'rtl')];
      result = physical ? computeText(context, physical) : initialText(name);
    } else result = computeFresh(context, name, meta);
    if (BACKGROUND_LAYERS[name]) result = repeatLayers(context, name, result);
    if (name === 'display') result = blockified(context, result);
    context.cache[name] = result;
    return result;
  }
  function inheritedText(context, name) { const parent = parentContext(context, context.contexts); return parent ? computeText(parent, name) : rootText(context, name); }
  const rootText = (context, name) => absolutize(context, name, uaText(context, name) || initialText(name));
  // currentcolor computes to itself and inherits as a keyword; it resolves per element.
  function rawInherited(context, name) {
    let text = specifiedText(context, name);
    if (text !== undefined) text = substitute(context, trimText(text));
    const keyword = text === undefined || text === null ? 'inherit' : lowerCase(trimText(text));
    if (keyword === 'initial') return 'currentcolor';
    if (keyword === 'inherit' || keyword === 'unset' || keyword === 'revert' || keyword === 'revert-layer') { const parent = parentContext(context, context.contexts); return parent ? rawInherited(parent, name) : 'currentcolor'; }
    const items = significant(components(tokenize(text)));
    if (items.length === 1 && (ident(items[0]) === 'currentcolor' || (name === 'caret-color' && ident(items[0]) === 'auto'))) return 'currentcolor';
    return absolutize(context, name, text);
  }
  function computeFresh(context, name, meta) {
    if (CURRENTCOLOR_INITIAL[name] && meta.inherited) { const raw = rawInherited(context, name); return raw === 'currentcolor' ? computeText(context, 'color') : raw; }
    let text = specifiedText(context, name);
    if (text !== undefined) text = substitute(context, trimText(text));
    const keyword = text === undefined ? null : lowerCase(trimText(text));
    if (text === undefined || text === null || keyword === 'unset' || keyword === 'revert' || keyword === 'revert-layer') {
      if (meta.inherited) return inheritedText(context, name);
      const ua = keyword === 'unset' ? null : uaText(context, name);
      return absolutize(context, name, ua || initialText(name));
    }
    if (keyword === 'inherit') return inheritedText(context, name);
    if (keyword === 'initial') return absolutize(context, name, initialText(name));
    return absolutize(context, name, text);
  }
  // var() substitution over the element's computed custom properties.
  function customText(context, name) {
    const key = '--custom:' + name;
    if (key in context.cache) return context.cache[key];
    context.cache[key] = null;
    let value = name in context.cascaded ? trimText(context.cascaded[name]) : undefined;
    if (value !== undefined) value = substitute(context, value);
    if (value === undefined) { const parent = parentContext(context, context.contexts); value = parent ? customText(parent, name) : null; }
    context.cache[key] = value;
    return value;
  }
  function substitute(context, text) {
    if (indexOfText(lowerCase(text), 'var(') < 0) return text;
    const items = components(tokenize(text));
    let failed = false;
    function expand(items) {
      let out = '';
      for (let index = 0; index < items.length; index++) {
        const item = items[index];
        if (item.block && item.type === 'function' && lowerCase(item.value) === 'var') {
          const parts = splitArguments(item.children), name = trimText(textOf(parts[0]));
          let value = startsWith(name, '--') ? customText(context, name) : null;
          if (value === null || value === undefined) { if (parts.length > 1) value = expand(parts[1]); else { failed = true; return ''; } }
          out += value; continue;
        }
        out += item.block ? (item.type === 'function' ? item.open.raw : item.open.raw) + expand(item.children) + (item.closed ? BLOCK_END[item.type] : '') : item.raw;
      }
      return out;
    }
    const result = expand(items);
    return failed ? null : result;
  }
  function splitArguments(children) { const first = list(), rest = list(); let seen = false; for (let index = 0; index < children.length; index++) { const item = children[index]; if (!seen && item.type === 'comma') { seen = true; continue; } push(seen ? rest : first, item); } return seen ? [first, rest] : [first]; }
  const pxNumber = (text) => { const match = execRegExp(/^(-?[\d.]+(?:e[+-]?\d+)?)px$/i, text); return match ? +match[1] : null; };
  function fontSizePx(context) { const value = pxNumber(computeText(context, 'font-size')); return value === null ? 16 : value; }
  function rootContext(context) { let current = context; for (;;) { const parent = parentContext(current, current.contexts); if (!parent) return current; current = parent; } }
  const FONT_SIZE_KEYWORDS = { __proto__: null, 'xx-small': 9, 'x-small': 10, small: 13, medium: 16, large: 18, 'x-large': 24, 'xx-large': 32, 'xxx-large': 48 };
  const WIDTH_KEYWORDS = { __proto__: null, thin: 1, medium: 3, thick: 5 };
  // Resolve one numeric literal to its computed form.
  function computedLiteral(context, name, node) {
    const unitKey = node.unit, info = UNITS[unitKey];
    if (!info || info.base !== BASE.length) {
      if (info && info.base === BASE.time) return lit(node.value * info.factor, 's');
      if (info && info.base === BASE.angle) return lit(node.value * info.factor, 'deg');
      return node;
    }
    if (info.category === 'length') return lit(node.value * info.factor, 'px');
    const viewportWidth = +g.innerWidth || 0, viewportHeight = +g.innerHeight || 0;
    const fontBase = () => name === 'font-size' ? (parentContext(context, context.contexts) ? fontSizePx(parentContext(context, context.contexts)) : 16) : fontSizePx(context);
    switch (unitKey) {
      case 'em': return lit(node.value * fontBase(), 'px');
      case 'rem': { const root = rootContext(context); return lit(node.value * (root === context && name === 'font-size' ? 16 : fontSizePx(root)), 'px'); }
      case 'vw': case 'svw': case 'lvw': case 'dvw': case 'vi': case 'svi': case 'lvi': case 'dvi': return lit(node.value * viewportWidth / 100, 'px');
      case 'vh': case 'svh': case 'lvh': case 'dvh': case 'vb': case 'svb': case 'lvb': case 'dvb': return lit(node.value * viewportHeight / 100, 'px');
      case 'vmin': case 'svmin': case 'lvmin': case 'dvmin': return lit(node.value * mathMin(viewportWidth, viewportHeight) / 100, 'px');
      case 'vmax': case 'svmax': case 'lvmax': case 'dvmax': return lit(node.value * mathMax(viewportWidth, viewportHeight) / 100, 'px');
    }
    return node;
  }
  function computedNode(context, name, node) {
    if (node.t === 'lit') return computedLiteral(context, name, node);
    if (node.t !== 'op') return node;
    const operands = list();
    for (let index = 0; index < node.operands.length; index++) push(operands, computedNode(context, name, node.operands[index]));
    if (node.op === ADD || node.op === SUB || node.op === MUL) return simplified(operands[0], operands[1], node.op) || node;
    if (node.op === 'invert') return invertNode(operands[0]);
    if (node.op === 'min' || node.op === 'max' || node.op === 'clamp') return createComparison(toArray(operands), node.op) || node;
    return node;
  }
  function computedComponent(context, name, item) {
    if (item.type === 'number' || item.type === 'percentage' || item.type === 'dimension') {
      const node = item.type === 'number' ? lit(item.value, 'number') : item.type === 'percentage' ? lit(item.value, 'percent') : UNITS[lowerCase(item.unit)] ? lit(item.value, lowerCase(item.unit)) : null;
      if (!node) return componentCssText(item);
      if (node.unit === 'number' && node.value === 0 && LENGTH_ZERO(name)) return '0px';
      return nodeText(computedLiteral(context, name, node));
    }
    if (item.block && item.type === 'function' && MATH_FUNCTIONS[lowerCase(item.value)]) {
      const node = parseMathComponent(item);
      if (!node) return componentCssText(item);
      const resolved = maybeSimplifySumOrProduct(computedNode(context, name, node));
      return resolved.t === 'lit' ? nodeText(resolved) : mathFunctionText(resolved);
    }
    if (item.block) { const open = item.type === 'function' ? lowerCase(item.value) + '(' : item.type; return open + computedList(context, name, item.children) + BLOCK_END[item.type]; }
    if (item.type === 'ident') {
      const lower = lowerCase(item.value);
      if (lower === 'currentcolor') return computeText(context, 'color');
      return KEYWORD_IDS[lower] ? lower : serializeIdentifier(item.value);
    }
    return componentCssText(item);
  }
  const LENGTH_ZERO = (name) => { const meta = PROPERTIES[name]; return !!meta && hasType(meta, 'Length') && !hasType(meta, 'Number') && !USER_UNITS[name]; };
  function computedList(context, name, items) {
    let text = '', space = false;
    for (let index = 0; index < items.length; index++) {
      const item = items[index];
      if (item.type === 'whitespace') { space = text !== ''; continue; }
      if (item.type === 'comma') { text += ', '; space = false; continue; }
      if (space && !endsWithText(text, ' ')) text += ' ';
      space = false;
      text += computedComponent(context, name, item);
    }
    return text;
  }
  // Background/mask layer longhands repeat to the image layer count; url() resolves.
  const IMAGE_LAYERS = plainSet(['background-image', 'mask-image', 'list-style-image', 'border-image-source', 'shape-outside']);
  const BACKGROUND_LAYERS = plainSet(['background-attachment', 'background-blend-mode', 'background-clip', 'background-origin', 'background-position-x', 'background-position-y', 'background-repeat', 'background-size']);
  // The embedding refines Node.baseURI after the binding installs, so capture it on first use.
  let urlApi, baseOf;
  // The embedding's WHATWG URL parser when present, else RFC 3986 reference resolution.
  function absoluteUrl(context, relative) {
    if (!baseOf) { const descriptor = Object.getOwnPropertyDescriptor(g.Node.prototype, 'baseURI'); baseOf = descriptor && descriptor.get ? descriptor.get : ns.domAccessor('baseURI', false, 'Node'); }
    const base = apply(baseOf, context.element, []);
    if (urlApi === undefined) { const URLConstructor = g.URL; urlApi = typeof URLConstructor === 'function' ? { URLConstructor, href: Object.getOwnPropertyDescriptor(URLConstructor.prototype, 'href').get } : null; }
    if (urlApi) { try { return apply(urlApi.href, new urlApi.URLConstructor(relative, base), []); } catch { return relative; } }
    return resolveReference(relative, base);
  }
  function resolveReference(reference, base) {
    if (execRegExp(/^[A-Za-z][A-Za-z0-9+.-]*:/, reference)) return reference;
    const parts = execRegExp(/^([A-Za-z][A-Za-z0-9+.-]*:)(\/\/[^/?#]*)?([^?#]*)(\?[^#]*)?(#.*)?$/, base);
    if (!parts) return reference;
    const scheme = parts[1], authority = parts[2] || '', basePath = parts[3] || '/';
    // An opaque-path base (about:blank) cannot resolve a relative reference.
    if (!authority && !startsWith(basePath, '/')) return reference;
    if (startsWith(reference, '//')) return scheme + reference;
    if (reference === '') return scheme + authority + basePath + (parts[4] || '');
    if (startsWith(reference, '#')) return scheme + authority + basePath + (parts[4] || '') + reference;
    if (startsWith(reference, '?')) return scheme + authority + basePath + reference;
    const split = execRegExp(/^([^?#]*)(.*)$/, reference), path = split[1], suffix = split[2];
    const merged = startsWith(path, '/') ? path : sliceText(basePath, 0, basePath.lastIndexOf('/') + 1) + path;
    const segments = list(), input = merged.split('/');
    for (let index = 0; index < input.length; index++) {
      const segment = input[index];
      if (segment === '.') { if (index === input.length - 1) push(segments, ''); continue; }
      if (segment === '..') { if (segments.length > 1) segments.length--; if (index === input.length - 1) push(segments, ''); continue; }
      push(segments, segment);
    }
    return scheme + authority + joinList(segments, '/') + suffix;
  }
  function computedImages(context, name, items) {
    const layers = splitComponents(items, 'comma'), parts = list();
    for (let index = 0; index < layers.length; index++) {
      const layer = significant(layers[index]);
      push(parts, layer.length === 1 && isUrl(layer[0]) ? 'url(' + serializeString(absoluteUrl(context, urlString(layer[0]))) + ')' : computedList(context, name, layers[index]));
    }
    return joinList(parts, ', ');
  }
  function repeatLayers(context, name, text) {
    const count = splitComponents(components(tokenize(computeText(context, name === 'background-attachment' || startsWith(name, 'background-') ? 'background-image' : 'mask-image'))), 'comma').length;
    const layers = splitComponents(components(tokenize(text)), 'comma');
    if (layers.length >= count) return text;
    const parts = list(); for (let index = 0; index < count; index++) push(parts, normalizedText(layers[index % layers.length]));
    return joinList(parts, ', ');
  }
  // CSS Display: blockification under flex/grid containers and for floats/out-of-flow boxes.
  const BLOCKIFY = { __proto__: null, inline: 'block', 'inline-block': 'block', 'inline-flex': 'flex', 'inline-grid': 'grid', 'inline-table': 'table', 'inline-list-item': 'list-item', 'table-row-group': 'block', 'table-column': 'block', 'table-column-group': 'block', 'table-header-group': 'block', 'table-footer-group': 'block', 'table-row': 'block', 'table-cell': 'block', 'table-caption': 'block', ruby: 'block', 'ruby-text': 'block' };
  function blockified(context, display) {
    if (!(display in BLOCKIFY)) return display;
    const parent = parentContext(context, context.contexts);
    const parentDisplay = parent ? computeText(parent, 'display') : null;
    const container = parentDisplay === 'flex' || parentDisplay === 'inline-flex' || parentDisplay === 'grid' || parentDisplay === 'inline-grid';
    const position = computeText(context, 'position'), float = computeText(context, 'float');
    return container || !parent || position === 'absolute' || position === 'fixed' || (float !== 'none' && float !== '') ? BLOCKIFY[display] : display;
  }
  const COLOR_PROPERTY = (name) => endsWithText(name, '-color') || name === 'color' || name === 'fill' || name === 'stroke';
  function absolutize(context, name, text) {
    if (FONT_FROM_BINDING[name]) {
      const value = computedGet(bindingStyle(context), name);
      if (value) return value;
    }
    const all = trimSpace(components(tokenize(text))), items = significant(all);
    if (name === 'color' || (COLOR_PROPERTY(name) && items.length === 1 && (isColor(items[0]) || (items[0].type === 'ident' && !KEYWORD_ONLY_COLOR(name, items[0]))))) {
      if (items.length === 1 && ident(items[0]) === 'currentcolor') return name === 'color' ? inheritedText(context, 'color') : computeText(context, 'color');
      if (name === 'caret-color' && items.length === 1 && ident(items[0]) === 'auto') return computeText(context, 'color');
      const color = cascadeHook.color(textOf(items));
      if (color) return color;
    }
    if (BORDER_WIDTH[name] && items.length === 1 && items[0].type === 'ident' && lowerCase(items[0].value) in WIDTH_KEYWORDS) return WIDTH_KEYWORDS[lowerCase(items[0].value)] + 'px';
    if (name === 'font-size' && items.length === 1 && items[0].type === 'ident') {
      const lower = lowerCase(items[0].value), parent = parentContext(context, context.contexts), base = parent ? fontSizePx(parent) : 16;
      if (lower in FONT_SIZE_KEYWORDS) return FONT_SIZE_KEYWORDS[lower] + 'px';
      if (lower === 'smaller') return formatNumber(base / 1.2, 'px');
      if (lower === 'larger') return formatNumber(base * 1.2, 'px');
    }
    if (name === 'font-size' && items.length === 1 && items[0].type === 'percentage') { const parent = parentContext(context, context.contexts); return formatNumber(items[0].value / 100 * (parent ? fontSizePx(parent) : 16), 'px'); }
    if (name === 'line-height' && items.length === 1 && items[0].type === 'percentage') return formatNumber(items[0].value / 100 * fontSizePx(context), 'px');
    if (IMAGE_LAYERS[name]) return computedImages(context, name, all);
    return computedList(context, name, all);
  }
  const KEYWORD_ONLY_COLOR = (name, item) => { const lower = lowerCase(item.value); return lower === 'auto' && name === 'accent-color' || lower === 'none' || lower === 'transparent' && false; };
  const COMPUTED = { normalize: (items) => normalizedText(items) };
  const connected = (element) => apply(connectedOf, element, []);
  function computedValues(element, property) {
    if (!connected(element)) return [];
    const context = contextFor(element, new Map());
    if (property.custom) { const value = customText(context, property.name); return value === null || value === undefined ? [] : [unparsedFromText(value)]; }
    if (isShorthand(property)) { const text = computedShorthand(context, property); return text === null ? [] : [unsupported(text, property.name)]; }
    if (!PROPERTIES[property.name].initial && !(property.name in context.cascaded)) return [];
    return reifyComputed(property, computeText(context, property.name));
  }
  function reifyComputed(property, text) {
    const items = trimSpace(components(tokenize(text)));
    if (items.length === 1 && items[0].type === 'ident' && CSS_WIDE[lowerCase(items[0].value)]) return [keywordValue(lowerCase(items[0].value))];
    // Computed scroll-snap-align is an alignment pair.
    if (property.name === 'scroll-snap-align') return [unsupported(normalizedText(items), property.name)];
    // Computed transition-property idents are plain identifiers.
    if (property.name === 'transition-property') { const layers = splitComponents(items, 'comma'), values = list(); for (let index = 0; index < layers.length; index++) { const layer = significant(layers[index]); if (layer.length !== 1 || layer[0].type !== 'ident') return [unsupported(normalizedText(items), property.name)]; push(values, keywordValue(identifierKeyword(property, layer[0]))); } return toArray(values); }
    return reifyValue(property, items, COMPUTED);
  }
  function computedShorthand(context, property) {
    const longhands = property.meta.longhands, texts = list();
    for (let index = 0; index < longhands.length; index++) { if (!PROPERTIES[longhands[index]] || !PROPERTIES[longhands[index]].initial) return null; push(texts, computeText(context, longhands[index])); }
    if (longhands.length === 4 && endsWithText(longhands[0], 'top')) { const top = texts[0], right = texts[1], bottom = texts[2], left = texts[3]; return left !== right ? top + ' ' + right + ' ' + bottom + ' ' + left : bottom !== top ? top + ' ' + right + ' ' + bottom : right !== top ? top + ' ' + right : top; }
    let same = true; for (let index = 1; index < texts.length; index++) if (texts[index] !== texts[0]) same = false;
    return same ? texts[0] : joinList(texts, ' ');
  }
  function customNames(context) {
    const names = create(null);
    for (let current = context; current; current = parentContext(current, current.contexts)) { const keys = ownKeys(current.cascaded); for (let index = 0; index < keys.length; index++) if (startsWith(keys[index], '--')) names[keys[index]] = true; }
    const result = list(), keys = ownKeys(names);
    for (let index = 0; index < keys.length; index++) if (customText(context, keys[index]) !== null) push(result, keys[index]);
    for (let index = 1; index < result.length; index++) { const key = result[index]; let at = index - 1; while (at >= 0 && result[at] > key) { result[at + 1] = result[at]; at--; } result[at + 1] = key; }
    return result;
  }
  function computedSize(element) { if (!connected(element)) return 0; const context = contextFor(element, new Map()); return registry.computable.length + customNames(context).length; }
  function computedEntries(element) {
    if (!connected(element)) return list();
    const context = contextFor(element, new Map()), entries = list();
    for (let index = 0; index < registry.computable.length; index++) { const name = registry.computable[index]; push(entries, [name, toArray(reifyComputed(propertyName(name), computeText(context, name)))]); }
    const customs = customNames(context);
    for (let index = 0; index < customs.length; index++) push(entries, [customs[index], [unparsedFromText(customText(context, customs[index]))]]);
    return entries;
  }

  // ---- Element, rule and CSS namespace hooks ---------------------------------------------------
  const computedMaps = new WeakMap(), inlineMaps = new WeakMap(), ruleMaps = new WeakMap();
  define(g.Element.prototype, 'computedStyleMap', { __proto__: null, value: make(function () {
    apply(elementCheck, this, []);
    let map = weakGet(computedMaps, this);
    if (!map) { map = newMap('StylePropertyMapReadOnly', 'computed', this); weakSet(computedMaps, this, map); }
    return map;
  }, 'computedStyleMap', 0), writable: true, enumerable: true, configurable: true });
  function inlineMap(element) { let map = weakGet(inlineMaps, element); if (!map) { map = newMap('StylePropertyMap', 'inline', element); weakSet(inlineMaps, element, map); } return map; }
  define(g.HTMLElement.prototype, 'attributeStyleMap', { __proto__: null, get: make(function () { apply(htmlCheck, this, []); return inlineMap(this); }, 'get attributeStyleMap', 0), enumerable: true, configurable: true });
  if (g.SVGElement && g.SVGElement.prototype) define(g.SVGElement.prototype, 'attributeStyleMap', { __proto__: null, get: make(function () { apply(elementCheck, this, []); if (apply(namespaceOf, this, []) !== SVG_NAMESPACE) throw new ErrorType('Illegal invocation'); return inlineMap(this); }, 'get attributeStyleMap', 0), enumerable: true, configurable: true });
  // MathMLElement is installed only where the embedding gives MathML elements their own interface.
  const MathML = g.MathMLElement;
  if (typeof MathML === 'function' && MathML.prototype && getProto(MathML.prototype) === g.Element.prototype) define(MathML.prototype, 'attributeStyleMap', { __proto__: null, get: make(function () { apply(elementCheck, this, []); if (apply(namespaceOf, this, []) !== 'http://www.w3.org/1998/Math/MathML') throw new ErrorType('Illegal invocation'); return inlineMap(this); }, 'get attributeStyleMap', 0), enumerable: true, configurable: true });
  {
    const proto = g.CSSStyleRule.prototype;
    const descriptor = { __proto__: null, get: make(function () { apply(ruleStyle, this, []); let map = weakGet(ruleMaps, this); if (!map) { map = newMap('StylePropertyMap', 'rule', this); weakSet(ruleMaps, this, map); } return map; }, 'get styleMap', 0), enumerable: true, configurable: true };
    // Keep Chrome's member order: selectorText, style, styleMap.
    const later = list(), names = ownKeys(proto);
    let after = false;
    for (let index = 0; index < names.length; index++) { if (after) { const key = names[index]; push(later, [key, Object.getOwnPropertyDescriptor(proto, key)]); } if (names[index] === 'style') after = true; }
    for (let index = 0; index < later.length; index++) delete proto[later[index][0]];
    define(proto, 'styleMap', descriptor);
    for (let index = 0; index < later.length; index++) define(proto, later[index][0], later[index][1]);
  }
  // Blink's member order: value-iterable functions, attributes, operations, pair-iterable
  // functions, then the constructor and the symbol-keyed members.
  {
    const ITERABLE = plainSet(['entries', 'keys', 'values', 'forEach']);
    const names = ownKeys(prototypes);
    for (let index = 0; index < names.length; index++) {
      const proto = prototypes[names[index]], keys = ownKeys(proto), pair = names[index] === 'StylePropertyMapReadOnly';
      const groups = [list(), list(), list(), list(), list()];
      for (let at = 0; at < keys.length; at++) {
        const key = keys[at], descriptor = Object.getOwnPropertyDescriptor(proto, key);
        const slot = typeof key === 'symbol' ? 4 : key === 'constructor' ? 3 : ITERABLE[key] && typeof descriptor.value === 'function' ? (pair ? 2 : 0) : descriptor.get ? 1 : 2;
        push(groups[slot], [key, descriptor]);
      }
      const operations = groups[2], sorted = list(), trailing = list();
      for (let at = 0; at < operations.length; at++) push(pair && ITERABLE[operations[at][0]] ? trailing : sorted, operations[at]);
      for (let at = 1; at < sorted.length; at++) { const entry = sorted[at]; let back = at - 1; while (back >= 0 && sorted[back][0] > entry[0]) { sorted[back + 1] = sorted[back]; back--; } sorted[back + 1] = entry; }
      for (let at = 0; at < trailing.length; at++) push(sorted, trailing[at]);
      groups[2] = sorted;
      for (let group = 0; group < 5; group++) for (let at = 0; at < groups[group].length; at++) { delete proto[groups[group][at][0]]; define(proto, groups[group][at][0], groups[group][at][1]); }
    }
  }
  // CSSStyleDeclaration admits the Chrome-enabled properties Lightning CSS does not model,
  // validated by the same grammar as Typed OM.
  const nativeKnown = create(null);
  ns.typedOmProperty = (name) => { const property = propertyName(name); return !!property && !property.custom && !property.descriptor; };
  ns.typedOmValid = (name, value) => {
    if (!(name in nativeKnown)) nativeKnown[name] = Native('known', name);
    if (nativeKnown[name]) return true;
    const property = propertyName(name);
    return !!property && !property.custom && !property.descriptor && validValue(property, value);
  };
  // CSS.number()/CSS.px()/…: the embedding's CSS namespace installs these factories.
  const FACTORIES = plain(['number', 'percent', 'em', 'ex', 'ch', 'ic', 'rem', 'lh', 'rlh', 'vw', 'vh', 'vi', 'vb', 'vmin', 'vmax', 'svw', 'svh', 'svi', 'svb', 'svmin', 'svmax', 'lvw', 'lvh', 'lvi', 'lvb', 'lvmin', 'lvmax', 'dvw', 'dvh', 'dvi', 'dvb', 'dvmin', 'dvmax', 'cqw', 'cqh', 'cqi', 'cqb', 'cqmin', 'cqmax', 'cm', 'mm', 'Q', 'in', 'pt', 'pc', 'px', 'deg', 'grad', 'rad', 'turn', 's', 'ms', 'Hz', 'kHz', 'dpi', 'dpcm', 'dppx', 'x', 'fr', 'cap', 'rcap', 'rch', 'rex', 'ric']);
  ns.typedOmFactories = function () {
    const result = list();
    for (let index = 0; index < FACTORIES.length; index++) {
      const name = FACTORIES[index], key = lowerCase(name);
      push(result, [name, make(function (value) { const args = arguments; return enter((message) => executeMessage(name, 'CSS', message), () => { requireArguments(args, 1); return unitValue(toDouble(value), key); }); }, name, 1)]);
    }
    return toArray(result);
  };
})();
