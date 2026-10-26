(() => {
  'use strict';
  // CSS Typed OM Level 1 control: interface shape and brands, constructors and argument
  // conversion, numeric math and serialization, CSSNumericValue.parse/CSSStyleValue.parse
  // reification per property, keyword/unparsed/variable/transform/position values, and
  // StylePropertyMap behavior for attributeStyleMap, CSSStyleRule.styleMap and computedStyleMap.
  // The CSS.* factories and toMatrix()/CSSMatrixComponent need DOMMatrix and the CSS namespace;
  // the embedding's control covers those.
  const out = {};
  const N = (x) => (Object.is(x, -0) ? '-0' : Number.isFinite(x) ? x : String(x));
  const describe = (v, depth = 0) => {
    if (v === undefined) return '<undefined>';
    if (v === null || typeof v !== 'object') return typeof v === 'number' ? N(v) : v;
    if (depth > 6) return '<deep>';
    const d = (x) => describe(x, depth + 1);
    const o = { t: Object.getPrototypeOf(v) ? Object.getPrototypeOf(v).constructor.name : null };
    try { o.s = String(v).split(location.origin).join('<origin>'); } catch (e) { o.s = '!' + e.name; }
    if (v instanceof CSSUnitValue) { o.v = N(v.value); o.u = v.unit; }
    else if (v instanceof CSSMathClamp) { o.l = d(v.lower); o.v = d(v.value); o.up = d(v.upper); }
    else if (v instanceof CSSMathValue) { o.op = v.operator; if ('values' in v) o.vs = [...v.values].map(d); else o.v = d(v.value); }
    else if (v instanceof CSSKeywordValue) o.v = v.value;
    else if (v instanceof CSSUnparsedValue) o.vs = [...v].map(d);
    else if (v instanceof CSSVariableReferenceValue) { o.var = v.variable; o.fb = d(v.fallback); }
    else if (v instanceof CSSTransformValue) { o.is2D = v.is2D; o.vs = [...v].map(d); }
    else if (v instanceof CSSPositionValue) { o.x = d(v.x); o.y = d(v.y); }
    else if (v instanceof CSSTransformComponent) { o.is2D = v.is2D; for (const k of ['x', 'y', 'z', 'angle', 'ax', 'ay', 'length']) if (k in v) o[k] = d(v[k]); }
    else if (Array.isArray(v)) return v.map(d);
    return o;
  };
  const run = (f) => { try { return describe(f()); } catch (e) { return { e: e.name, m: e.message }; } };
  const rows = (pairs) => pairs.map(([label, f]) => [label, run(f)]);

  // ---- Shape: globals, constructors, prototype members, brands -----------------------------
  const NAMES = ['CSSStyleValue', 'CSSNumericValue', 'CSSUnitValue', 'CSSMathValue', 'CSSMathSum', 'CSSMathProduct', 'CSSMathNegate', 'CSSMathInvert', 'CSSMathMin', 'CSSMathMax', 'CSSMathClamp', 'CSSNumericArray', 'CSSKeywordValue', 'CSSUnparsedValue', 'CSSVariableReferenceValue', 'CSSTransformValue', 'CSSTransformComponent', 'CSSTranslate', 'CSSRotate', 'CSSScale', 'CSSSkew', 'CSSSkewX', 'CSSSkewY', 'CSSPerspective', 'CSSMatrixComponent', 'CSSImageValue', 'CSSPositionValue', 'StylePropertyMapReadOnly', 'StylePropertyMap'];
  const describeDescriptor = (d) => {
    const r = {};
    for (const k of ['enumerable', 'configurable', 'writable']) if (k in d) r[k] = d[k];
    for (const k of ['value', 'get', 'set']) if (typeof d[k] === 'function') r[k] = [d[k].name, d[k].length, Object.prototype.hasOwnProperty.call(d[k], 'prototype')];
    if ('value' in d && typeof d.value !== 'function') r.value = String(d.value);
    return r;
  };
  out.shape = NAMES.map((name) => {
    const C = window[name];
    if (typeof C !== 'function') return [name, 'missing'];
    const global = describeDescriptor(Object.getOwnPropertyDescriptor(window, name));
    const P = C.prototype;
    const own = (o, skip) => Reflect.ownKeys(o).filter((k) => !skip.includes(k)).map((k) => [String(k), describeDescriptor(Object.getOwnPropertyDescriptor(o, k))]);
    const prototype = Object.getOwnPropertyDescriptor(C, 'prototype');
    return [name, { global, name: C.name, length: C.length, parent: Object.getPrototypeOf(C).name, protoParent: Object.getPrototypeOf(P) && Object.getPrototypeOf(P).constructor.name, statics: own(C, ['length', 'name', 'prototype']), prototype: [prototype.writable, prototype.enumerable, prototype.configurable], members: own(P, []), tag: Object.prototype.toString.call(Object.create(P)) }];
  });
  out.hostShape = [
    ['Element.computedStyleMap', describeDescriptor(Object.getOwnPropertyDescriptor(Element.prototype, 'computedStyleMap'))],
    ['HTMLElement.attributeStyleMap', describeDescriptor(Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'attributeStyleMap'))],
    ['SVGElement.attributeStyleMap', describeDescriptor(Object.getOwnPropertyDescriptor(SVGElement.prototype, 'attributeStyleMap'))],
    ['CSSStyleRule.styleMap', describeDescriptor(Object.getOwnPropertyDescriptor(CSSStyleRule.prototype, 'styleMap'))],
    ['CSSStyleRule keys', Object.getOwnPropertyNames(CSSStyleRule.prototype).filter((k) => ['selectorText', 'style', 'styleMap'].includes(k))],
    ['iterators', [CSSNumericArray, CSSUnparsedValue, CSSTransformValue].map((C) => [C.prototype[Symbol.iterator] === C.prototype.values, C.prototype.values === Array.prototype.values, C.prototype.entries === Array.prototype.entries, C.prototype.keys === Array.prototype.keys])],
    ['map iterator', StylePropertyMapReadOnly.prototype[Symbol.iterator] === StylePropertyMapReadOnly.prototype.entries],
  ];
  const E = (f) => { try { f(); return 'ok'; } catch (e) { return e.name + ': ' + e.message; } };
  const px = (v) => new CSSUnitValue(v, 'px');
  const pct = (v) => new CSSUnitValue(v, 'percent');
  const num = (v) => new CSSUnitValue(v, 'number');
  const deg = (v) => new CSSUnitValue(v, 'deg');
  const sec = (v) => new CSSUnitValue(v, 's');
  out.construct = [
    ['call CSSUnitValue', E(() => CSSUnitValue(1, 'px'))], ['new CSSStyleValue', E(() => new CSSStyleValue())], ['call CSSStyleValue', E(() => CSSStyleValue())],
    ['new CSSNumericValue', E(() => new CSSNumericValue())], ['new CSSMathValue', E(() => new CSSMathValue())], ['new CSSNumericArray', E(() => new CSSNumericArray())],
    ['new CSSTransformComponent', E(() => new CSSTransformComponent())], ['new CSSImageValue', E(() => new CSSImageValue())], ['new StylePropertyMap', E(() => new StylePropertyMap())], ['new StylePropertyMapReadOnly', E(() => new StylePropertyMapReadOnly())],
    ['unit bad', E(() => new CSSUnitValue(1, 'foo'))], ['unit NaN first', E(() => new CSSUnitValue('x', 'foo'))], ['unit inf', E(() => new CSSUnitValue(Infinity, 'px'))], ['unit 1 arg', E(() => new CSSUnitValue(1))], ['unit 0 args', E(() => new CSSUnitValue())],
    ['unit empty', E(() => new CSSUnitValue(1, ''))], ['unit symbol', E(() => new CSSUnitValue(1, Symbol('x')))], ['unit bigint', E(() => new CSSUnitValue(1n, 'px'))],
    ['sum empty', E(() => new CSSMathSum())], ['sum incompatible', E(() => new CSSMathSum(px(1), sec(1)))], ['sum string', E(() => new CSSMathSum('x'))], ['product empty', E(() => new CSSMathProduct())],
    ['min incompatible', E(() => new CSSMathMin(px(1), sec(1)))], ['clamp 2', E(() => new CSSMathClamp(1, 2))], ['clamp incompatible', E(() => new CSSMathClamp(px(1), 2, 3))], ['negate 0', E(() => new CSSMathNegate())], ['invert object', E(() => new CSSMathInvert({}))],
    ['keyword empty', E(() => new CSSKeywordValue(''))], ['keyword 0', E(() => new CSSKeywordValue())], ['unparsed 0', E(() => new CSSUnparsedValue())], ['unparsed number', E(() => new CSSUnparsedValue(1))], ['unparsed iterable', E(() => new CSSUnparsedValue(new Set(['a'])))],
    ['varref bad', E(() => new CSSVariableReferenceValue('x'))], ['varref 0', E(() => new CSSVariableReferenceValue())], ['varref bad fallback', E(() => new CSSVariableReferenceValue('--x', 1))], ['varref null fallback', E(() => new CSSVariableReferenceValue('--x', null))],
    ['transform empty', E(() => new CSSTransformValue([]))], ['transform 0', E(() => new CSSTransformValue())], ['transform bad item', E(() => new CSSTransformValue([1]))],
    ['translate 1', E(() => new CSSTranslate(px(1)))], ['translate angle', E(() => new CSSTranslate(px(1), deg(1)))], ['translate numbers', E(() => new CSSTranslate(1, 2))], ['translate z percent', E(() => new CSSTranslate(px(1), px(1), pct(1)))],
    ['rotate 0', E(() => new CSSRotate())], ['rotate px', E(() => new CSSRotate(px(1)))], ['rotate 3', E(() => new CSSRotate(1, 2, 3))], ['rotate 2', E(() => new CSSRotate(1, 2))], ['rotate bad coord', E(() => new CSSRotate(1, 2, px(3), deg(1)))], ['rotate bad angle', E(() => new CSSRotate(1, 2, 3, px(1)))], ['rotate number 1', E(() => new CSSRotate(5))],
    ['scale 1', E(() => new CSSScale(1))], ['scale px', E(() => new CSSScale(px(1), 1))], ['scale3d px', E(() => new CSSScale(1, 1, px(1)))], ['skew 1', E(() => new CSSSkew(deg(1)))], ['skew px', E(() => new CSSSkew(px(1), deg(1)))], ['skewX px', E(() => new CSSSkewX(px(1)))], ['skewY px', E(() => new CSSSkewY(px(1)))],
    ['perspective auto', E(() => new CSSPerspective('auto'))], ['perspective deg', E(() => new CSSPerspective(deg(1)))], ['perspective 0', E(() => new CSSPerspective())], ['perspective percent', E(() => new CSSPerspective(pct(1)))],
    ['matrix object', E(() => new CSSMatrixComponent({}))], ['position deg', E(() => new CSSPositionValue(px(1), deg(1)))], ['position x deg', E(() => new CSSPositionValue(deg(1), px(1)))], ['position 1', E(() => new CSSPositionValue(px(1)))], ['position number', E(() => new CSSPositionValue(1, 2))],
  ];
  const u = px(1), sum = new CSSMathSum(px(1), pct(2));
  const getter = (C, k) => Object.getOwnPropertyDescriptor(C.prototype, k).get;
  const setter = (C, k) => Object.getOwnPropertyDescriptor(C.prototype, k).set;
  out.brands = [
    ['unit.value on proto', E(() => CSSUnitValue.prototype.value)], ['unit.value on {}', E(() => getter(CSSUnitValue, 'value').call({}))], ['unit.value on sum', E(() => getter(CSSUnitValue, 'value').call(sum))],
    ['add on {}', E(() => CSSNumericValue.prototype.add.call({}, 1))], ['add on keyword', E(() => CSSNumericValue.prototype.add.call(new CSSKeywordValue('a'), 1))], ['toString on {}', E(() => CSSStyleValue.prototype.toString.call({}))],
    ['toString on unit', CSSStyleValue.prototype.toString.call(px(2))], ['operator on unit', E(() => getter(CSSMathValue, 'operator').call(u))], ['sum.values on product', E(() => getter(CSSMathSum, 'values').call(new CSSMathProduct(1)))],
    ['map.get on {}', E(() => StylePropertyMapReadOnly.prototype.get.call({}, 'x'))], ['map.set on computed', E(() => StylePropertyMap.prototype.set.call(document.body.computedStyleMap(), 'width', '1px'))], ['map size on {}', E(() => getter(StylePropertyMapReadOnly, 'size').call({}))],
    ['attributeStyleMap on svg', E(() => getter(HTMLElement, 'attributeStyleMap').call(document.createElementNS('http://www.w3.org/2000/svg', 'svg')))], ['svg attributeStyleMap on div', E(() => getter(SVGElement, 'attributeStyleMap').call(document.createElement('div')))],
    ['computedStyleMap on document', E(() => Element.prototype.computedStyleMap.call(document))], ['styleMap on {}', E(() => getter(CSSStyleRule, 'styleMap').call({}))], ['parse unbound', E(() => CSSStyleValue.parse.call(null, 'width', '1px'))],
    ['translate.x on rotate', E(() => getter(CSSTranslate, 'x').call(new CSSRotate(deg(1))))], ['component is2D on {}', E(() => getter(CSSTransformComponent, 'is2D').call({}))], ['keyword setter on unit', E(() => setter(CSSKeywordValue, 'value').call(u, 'x'))],
    ['varref.variable on {}', E(() => getter(CSSVariableReferenceValue, 'variable').call({}))], ['numeric array length on {}', E(() => getter(CSSNumericArray, 'length').call({}))], ['unparsed length on {}', E(() => getter(CSSUnparsedValue, 'length').call({}))],
    ['setter no args', E(() => setter(CSSUnitValue, 'value').call(px(1)))], ['setter non-finite', E(() => { px(1).value = NaN; })], ['instanceof', [u instanceof CSSNumericValue, u instanceof CSSStyleValue, sum instanceof CSSMathValue, new CSSKeywordValue('a') instanceof CSSStyleValue, new CSSVariableReferenceValue('--a') instanceof CSSStyleValue]],
    ['tags', [Object.prototype.toString.call(u), Object.prototype.toString.call(sum.values), Object.prototype.toString.call(document.body.attributeStyleMap), Object.prototype.toString.call(document.body.computedStyleMap()), String(new CSSVariableReferenceValue('--a'))]],
  ];

  // ---- Numeric values ------------------------------------------------------------------------
  out.units = rows(['number', 'percent', '%', 'PX', 'Px', 'q', 'Q', 'hz', 'Hz', 'kHz', 'KHZ', 'dppx', 'x', 'X', 'fr', 'em', 'rem', 'vw', 'cqmin', 'deg', 'turn', 'ms', 'S', 'dpi', 'dpcm', 'cap', 'rcap', 'ric', 'integer', 'Number', 'PERCENT', 'user', '__proto__'].map((unit) => [unit, () => new CSSUnitValue(1.5, unit)]));
  out.numbers = rows([0, -0, 1, -1, 0.5, 1 / 3, 123456, 999999, 1000000, -999999, -1000000, 1e21, 1.5e-7, 0.0001, 0.00001, 123456.5, 1234567, 2.5, 0.125, 1e100, Number.MAX_VALUE, Number.MIN_VALUE, 1.0000005, 3.14159265].map((v) => [String(v), () => [String(px(v)), String(num(v)), String(new CSSMathSum(px(v))), String(new CSSMathNegate(pct(v)))]]));
  out.math = rows([
    ['add same', () => px(1).add(px(2))], ['add numbers', () => num(1).add(2, 3)], ['add mixed', () => px(1).add(pct(2))], ['add sum receiver', () => sum.add(px(3))], ['add em', () => px(1).add(new CSSUnitValue(1, 'em'))],
    ['sub', () => px(5).sub(px(2))], ['sub mixed', () => px(5).sub(pct(2), new CSSUnitValue(1, 'em'))], ['sub numbers', () => num(5).sub(1)], ['sub sum receiver', () => sum.sub(px(1))],
    ['mul', () => px(2).mul(3)], ['mul two units', () => px(2).mul(px(3))], ['mul three', () => px(2).mul(3, pct(4))], ['mul product receiver', () => new CSSMathProduct(px(1), 2).mul(3)], ['mul numbers', () => num(2).mul(num(3))],
    ['div', () => px(6).div(3)], ['div unit', () => px(6).div(px(3))], ['div zero', () => px(6).div(0)], ['div zero unit', () => px(6).div(px(0))], ['div sum', () => px(6).div(sum)],
    ['min', () => px(1).min(px(2))], ['min mixed', () => px(1).min(pct(2))], ['min receiver', () => new CSSMathMin(px(1), pct(2)).min(px(0))], ['max', () => px(1).max(px(2), px(3))], ['max mixed', () => px(1).max(new CSSUnitValue(1, 'em'))], ['max incompatible', () => px(1).max(sec(1))],
    ['equals same', () => px(1).equals(px(1))], ['equals other unit', () => px(1).equals(new CSSUnitValue(1, 'em'))], ['equals many', () => px(1).equals(px(1), px(1))], ['equals none', () => px(1).equals()], ['equals sum', () => new CSSMathSum(px(1), pct(2)).equals(new CSSMathSum(px(1), pct(2)))], ['equals number', () => num(1).equals(1)],
    ['to px from in', () => new CSSUnitValue(1, 'in').to('px')], ['to cm', () => px(96).to('cm')], ['to deg from turn', () => new CSSUnitValue(0.5, 'turn').to('deg')], ['to rad', () => deg(180).to('rad')], ['to ms', () => sec(1.5).to('ms')], ['to number', () => num(2).to('number')], ['to percent', () => pct(2).to('percent')], ['to %', () => pct(2).to('%')],
    ['to sum px', () => new CSSMathSum(px(1), new CSSUnitValue(1, 'in')).to('px')], ['to mixed', () => sum.to('px')], ['to em', () => px(1).to('em')], ['to invalid', () => px(1).to('foo')], ['to none', () => px(1).to()], ['to dppx', () => new CSSUnitValue(96, 'dpi').to('dppx')], ['to khz', () => new CSSUnitValue(1, 'Hz').to('kHz')],
    ['to product', () => new CSSMathProduct(px(2), 3).to('px')], ['to negate', () => new CSSMathNegate(px(2)).to('px')], ['to invert', () => new CSSMathInvert(num(4)).to('number')], ['to min', () => new CSSMathMin(px(1), new CSSUnitValue(1, 'in')).to('px')], ['to clamp', () => new CSSMathClamp(px(1), px(5), px(3)).to('px')],
    ['toSum none', () => new CSSMathSum(px(1), pct(2), new CSSUnitValue(3, 'em'), px(4)).toSum()], ['toSum px percent', () => sum.toSum('px', 'percent')], ['toSum leftover', () => sum.toSum('px')], ['toSum invalid', () => px(1).toSum('foo')], ['toSum incompatible', () => px(1).toSum('s')], ['toSum product', () => new CSSMathProduct(px(1), px(2)).toSum()], ['toSum in px', () => new CSSUnitValue(1, 'in').toSum('px', 'cm')],
    ['type px', () => JSON.stringify(px(1).type())], ['type number', () => JSON.stringify(num(1).type())], ['type percent', () => JSON.stringify(pct(1).type())], ['type sum', () => JSON.stringify(sum.type())], ['type product', () => JSON.stringify(new CSSMathProduct(px(1), px(2), sec(1)).type())], ['type invert', () => JSON.stringify(new CSSMathInvert(px(1)).type())], ['type percent product', () => JSON.stringify(new CSSMathProduct(pct(1), px(1)).type())], ['type sum percent number', () => E(() => new CSSMathSum(pct(1), num(1)))], ['type angle percent', () => JSON.stringify(new CSSMathSum(deg(1), pct(1)).type())],
    ['nested text', () => new CSSMathSum(px(1), new CSSMathProduct(px(2), 3), new CSSMathNegate(new CSSMathSum(px(1), pct(2))))], ['invert text', () => new CSSMathProduct(px(1), new CSSMathInvert(num(2)))], ['negate first', () => new CSSMathSum(new CSSMathNegate(px(1)), px(2))], ['clamp text', () => new CSSMathClamp(px(1), new CSSMathSum(px(1), pct(2)), pct(100))],
    ['min nested', () => new CSSMathMin(new CSSMathSum(px(1), pct(2)), new CSSMathMax(px(3), pct(4)))], ['values array', () => { const v = sum.values; return [v.length, v[0] === v[0], v[0] instanceof CSSUnitValue, v[2], Object.keys(v), v === sum.values, [...v.entries()].length, 0 in v, 5 in v]; }],
    ['value setter', () => { const v = px(1); v.value = 3; return v; }], ['value setter string', () => { const v = px(1); v.value = '4'; return v; }], ['unit readonly', () => { const v = px(1); try { v.unit = 'em'; } catch (e) { return e.name; } return v.unit; }],
    ['array write', () => { const v = sum.values; v[0] = 5; return [String(v[0]), Reflect.set(v, 0, 1), Reflect.defineProperty(v, 0, { value: 1 }), delete v[0], delete v[9]]; }],
  ]);
  out.numericParse = rows(['10px', '  10PX ', '-.5e2em', '1e3px', 'calc(1px + 2px)', 'calc(1px + 2%)', 'calc(10px - 5px + 2%)', 'calc(2 * 3px)', 'calc(3px * 2)', 'calc(1px / 2)', 'calc(1)', 'calc(2 * 3)', 'min(1px, 2%)', 'max(1px)', 'clamp(1px, 2px, 3px)', 'clamp(1px, 2%, 3px)', 'clamp(none, 2%, 3px)', 'clamp(1px, 2%, none)', 'calc(1px + (2% - 3em))', 'calc(-1px)', 'calc(1px - -2px)', 'calc(1px + 2px * 3)', 'calc((1px + 2px) * 3)', 'calc(1px*2*3)', 'calc(1px/2/4)', 'calc(1px + 2% + 3em - 4px)', 'calc(100% - 10px)', 'calc(1px + 2em + 3%)', 'calc(sign(1px))', 'calc(pi * 1px)', 'abs(1px)', 'calc(abs(-2px))', '10foo', 'auto', '10px 20px', 'calc(1px + 1s)', 'calc(1deg + 1rad)', 'calc(1s + 1ms)', 'calc(0px)', 'calc(1in + 1cm)', '-webkit-calc(1px + 2px)', 'calc(1px + calc(2px + 3%))', 'min(1px, min(2px, 3%))', 'calc(1px * 0)', 'calc(1px / 0)', 'calc(infinity * 1px)', 'calc(-infinity)', 'calc(NaN * 1px)', '10%', '0', 'calc(1% * 2)', 'calc(1px + 2px + 3%)', 'calc(3% + 1px + 2px)', 'calc(5px + 1em - 2px)', 'calc(2px*3 + 4px)', 'calc(1px + min(2px, 3%))', 'calc(10px * 2 / 4)', 'calc(1px+2px)', 'calc(1px +2px)', 'calc(1px * 1px)', 'calc(1px / 1px)', 'calc(2 / 1px)', 'calc(1px * 1px / 1px)', 'calc(2px * (3% + 1px))', 'calc((3% + 1px) * 2)', 'calc(1px - (2% + 3em))', 'calc(1px - (2% - 3em))', 'calc(1em * (2 + 3))', 'calc(1fr)', '1fr', 'calc(e)', 'calc(1px', 'max(1px, 2px', 'calc()', 'min()', 'calc(1px, 2px)', 'CALC(1PX + 2PX)', 'calc( 1px + 2% )', 'round(1.5px, 1px)', 'calc(round(up, 1.5px, 1px))', 'calc(mod(7px, 3px))', 'calc(sin(90deg))', 'calc(1px + sibling-index())', 'var(--x)', '', ' ', 'calc(1px + 2% + 3px - 4%)', 'calc(1vw + 2vh + 1vw)', 'calc(1px + 1Q)', 'calc(1dpi + 1dppx)', 'calc(1x + 1dppx)'].map((text) => [text, () => CSSNumericValue.parse(text)]));

  // ---- CSSStyleValue.parse per property --------------------------------------------------
  const PARSE = [
    ['width', '10px'], ['width', '10PX'], ['width', '50%'], ['width', 'auto'], ['width', 'AUTO'], ['width', 'calc(1px + 2px)'], ['width', 'calc(10px + 5%)'], ['width', 'min(1px, 2%)'], ['width', 'fit-content'], ['width', 'red'], ['width', '-1px'], ['width', 'inherit'], ['width', 'INITIAL'], ['width', 'unset'], ['width', 'revert'], ['width', 'revert-layer'], ['width', 'var(--x)'], ['width', 'calc(var(--x) + 1px)'], ['width', 'var(x)'], ['width', ' 10px '], ['width', '0'], ['width', 'abs(1px)'],
    ['height', '1em'], ['margin-top', 'auto'], ['margin-left', '-5px'], ['padding-left', '1rem'], ['top', '10%'], ['left', 'calc(100% - 10px)'], ['inset-inline-start', '1px'], ['inline-size', '20px'], ['min-width', 'auto'], ['max-width', 'none'],
    ['opacity', '0.5'], ['opacity', '50%'], ['z-index', '3'], ['z-index', 'auto'], ['order', '-1'], ['flex-grow', '2'], ['flex-basis', 'content'], ['font-weight', 'bold'], ['font-weight', '700'], ['font-size', 'large'], ['font-size', '12px'], ['line-height', '1.5'], ['line-height', 'normal'], ['font-stretch', '50%'], ['font-stretch', 'condensed'],
    ['display', 'block'], ['display', 'BLOCK'], ['display', 'inline flex'], ['display', 'flex'], ['position', 'sticky'], ['visibility', 'hidden'], ['float', 'left'], ['cursor', 'pointer'], ['cursor', 'url(a.png), auto'], ['overflow-x', 'clip'], ['text-align', 'center'], ['white-space-collapse', 'preserve'], ['box-sizing', 'border-box'], ['pointer-events', 'none'], ['user-select', 'none'], ['writing-mode', 'vertical-rl'], ['object-fit', 'cover'], ['isolation', 'isolate'], ['contain', 'size'], ['contain', 'size layout'], ['contain', 'strict'], ['container-type', 'inline-size'], ['container-name', 'foo'], ['container-name', 'foo bar'],
    ['color', 'red'], ['color', 'RED'], ['color', '#ff0000'], ['color', 'rgb(1, 2, 3)'], ['color', 'currentcolor'], ['color', 'CurrentColor'], ['background-color', 'transparent'], ['border-top-color', 'currentColor'], ['caret-color', 'auto'], ['accent-color', 'auto'], ['accent-color', 'blue'], ['fill', 'red'], ['outline-color', 'invert'],
    ['background-image', 'none'], ['background-image', 'url(a.png)'], ['background-image', 'url("b.png"), none'], ['background-image', 'linear-gradient(red, blue)'], ['list-style-image', 'url(x.svg)'], ['border-image-source', 'none'], ['background-size', 'cover'], ['background-size', '10px'], ['background-size', '10px auto'], ['background-size', '10px 20px'], ['background-size', '10px, cover'], ['background-position', 'left top'], ['background-clip', 'text'], ['background-attachment', 'fixed, scroll'],
    ['transition-duration', '1s'], ['transition-duration', '1s, 200ms'], ['transition-duration', '-1s'], ['transition-delay', '0.5s'], ['transition-timing-function', 'ease-in'], ['transition-timing-function', 'cubic-bezier(0, 0, 1, 1)'], ['transition-property', 'opacity'], ['transition-property', 'opacity, width'], ['transition-property', 'all'], ['animation-name', 'spin'], ['animation-name', 'Spin, none'], ['animation-iteration-count', 'infinite'], ['animation-iteration-count', '2'], ['animation-duration', 'auto'], ['animation-direction', 'alternate, reverse'],
    ['transform', 'none'], ['transform', 'translate(10px, 20%)'], ['transform', 'translateX(1px) rotate(45deg)'], ['transform', 'translate3d(1px, 2px, 3px)'], ['transform', 'translateZ(5px)'], ['transform', 'scale(2)'], ['transform', 'scale(2, 3)'], ['transform', 'scale3d(1, 2, 3)'], ['transform', 'scaleX(2)'], ['transform', 'rotate(0)'], ['transform', 'rotate3d(1, 0, 0, 1turn)'], ['transform', 'rotateY(10deg)'], ['transform', 'skew(10deg)'], ['transform', 'skew(10deg, 0deg)'], ['transform', 'skewY(5deg)'], ['transform', 'perspective(100px)'], ['transform', 'perspective(none)'], ['transform', 'translate(calc(1px + 2px), 0)'], ['transform', 'translate(0)'],
    ['transform-origin', '10px 20px'], ['transform-origin', 'center'], ['perspective-origin', 'left top'], ['perspective-origin', 'center'], ['object-position', 'right 10px top'], ['object-position', '10px'], ['object-position', 'top'], ['object-position', 'bottom 10px right 20%'], ['object-position', 'center center'], ['offset-anchor', 'auto'], ['offset-position', 'normal'], ['offset-position', 'left 10px top 5px'], ['offset-rotate', 'auto'], ['offset-rotate', '10deg'], ['offset-rotate', 'auto 10deg'], ['perspective', 'none'], ['perspective', '10px'],
    ['border-top-left-radius', '10px'], ['border-top-left-radius', '10px 10px'], ['border-top-left-radius', '10px 20px'], ['border-top-width', 'thin'], ['border-top-width', '2px'], ['border-top-style', 'dashed'], ['outline-width', 'medium'], ['outline-offset', '2px'],
    ['text-decoration-line', 'underline'], ['text-decoration-line', 'underline overline'], ['text-transform', 'uppercase'], ['text-transform', 'none'], ['text-indent', '5px'], ['text-indent', '5px hanging'], ['touch-action', 'pan-x'], ['touch-action', 'pan-x pan-y'], ['will-change', 'auto'], ['will-change', 'transform'], ['grid-auto-flow', 'row'], ['grid-auto-flow', 'row dense'], ['grid-auto-flow', 'dense'], ['font-variant-ligatures', 'none'], ['font-variant-ligatures', 'common-ligatures'], ['font-variant-numeric', 'oldstyle-nums tabular-nums'], ['clip-path', 'none'], ['clip-path', 'circle(10px)'], ['clip-path', 'border-box'], ['grid-auto-columns', '1fr'], ['grid-auto-columns', '100px 1fr'], ['grid-template-columns', 'none'], ['grid-row-start', 'auto'], ['grid-row-start', '2'],
    ['align-items', 'center'], ['align-items', '  CENTER  '], ['justify-content', 'space-between'], ['font-family', 'Arial, sans-serif'], ['font-family', 'serif'], ['content', '"x"'], ['aspect-ratio', '16 / 9'], ['rotate', '45deg'], ['translate', '10px'], ['scale', '2'], ['box-shadow', 'none'], ['box-shadow', '1px 1px red'], ['text-shadow', 'none'], ['filter', 'blur(2px)'], ['filter', 'none'], ['counter-reset', 'none'], ['counter-reset', 'foo 1'], ['quotes', 'auto'], ['zoom', '2'], ['tab-size', '4'], ['tab-size', '4px'], ['vertical-align', 'middle'], ['vertical-align', '10px'], ['letter-spacing', 'normal'], ['word-spacing', '2px'], ['stroke-width', '2'], ['stroke-dasharray', 'none'], ['x', '5px'], ['r', '3px'],
    ['margin', '1px 2px'], ['margin', '1px'], ['margin', 'red'], ['border', '1px solid red'], ['background', 'red'], ['font', '12px serif'], ['flex', '1'], ['inset', 'auto'], ['transition', 'opacity 1s'],
    ['--x', '10px'], ['--x', '  a  b  '], ['--x', ''], ['--x', 'var(--y)'], ['--x', 'var(--y, 1px)'], ['--x', 'foo(var(--y), bar)'], ['--x', '1px var(--y) 2px'], ['--x', 'env(safe-area-inset-top)'], ['--x', '!important'], ['--x', ')'], ['--x', '{a}'], ['--x', '1;2'], ['--X', 'Y'], ['--', 'a'],
    ['WIDTH', '1px'], ['-webkit-transform', 'none'], ['-webkit-transition-duration', '1s'], ['word-wrap', 'break-word'], ['foo', '1px'], ['src', 'url(a)'], ['', 'x'], ['-x', '1px'], ['width', 'calc(1px + 2s)'], ['width', '1px !important'], ['width', '1px;'], ['color', 'var(--c, red)'], ['width', 'calc(1px +)'], ['width', 'max(1px, 2em)'],
  ];
  out.parse = PARSE.map(([property, value]) => [property + ': ' + value, run(() => CSSStyleValue.parse(property, value))]);
  out.parseAll = rows([['background-image', 'url(a), url(b)'], ['background-image', 'none, url(b)'], ['background-image', 'url(a), linear-gradient(red, blue)'], ['transition-duration', '1s, 2s, 3ms'], ['transition-property', 'opacity, width'], ['animation-name', 'a, b'], ['width', '1px'], ['--x', '1px, 2px'], ['grid-auto-rows', '10px 20%'], ['background-size', 'cover, 10px 20px'], ['margin', '1px'], ['transform', 'scale(1) scale(2)']].map(([p, v]) => [p + ': ' + v, () => CSSStyleValue.parseAll(p, v)]));

  // ---- Keyword, unparsed, variable references, transforms, positions ------------------------
  out.values = rows([
    ['keyword value case', () => { const k = new CSSKeywordValue('AUTO'); return [k.value, String(k)]; }], ['keyword custom', () => { const k = new CSSKeywordValue('Foo Bar'); return [k.value, String(k)]; }], ['keyword digits', () => String(new CSSKeywordValue('1x'))], ['keyword set empty', () => { const k = new CSSKeywordValue('a'); try { k.value = ''; } catch (e) { return [e.name, e.message, k.value]; } }], ['keyword set', () => { const k = new CSSKeywordValue('a'); k.value = 'INHERIT'; return [k.value, String(k)]; }], ['keyword css-wide', () => ['inherit', 'Initial', 'unset', 'revert', 'revert-layer', 'default'].map((w) => String(new CSSKeywordValue(w)))],
    ['unparsed basic', () => new CSSUnparsedValue(['a', new CSSVariableReferenceValue('--b'), 'c'])], ['unparsed join comments', () => String(new CSSUnparsedValue(['foo', 'bar']))], ['unparsed numbers', () => String(new CSSUnparsedValue(['1', '2', 'px']))], ['unparsed spaces', () => String(new CSSUnparsedValue(['  a  ', ' b ']))], ['unparsed empty', () => [String(new CSSUnparsedValue([])), new CSSUnparsedValue([]).length]], ['unparsed fallback', () => String(new CSSUnparsedValue([new CSSVariableReferenceValue('--a', new CSSUnparsedValue(['1px', new CSSVariableReferenceValue('--b')]))]))],
    ['unparsed indexed', () => { const v = new CSSUnparsedValue(['a', 'b']); v[0] = 'z'; v[2] = new CSSVariableReferenceValue('--q'); return [v.length, v[0], String(v[2] && v[2].variable), Object.keys(v), String(v)]; }], ['unparsed out of range', () => { const v = new CSSUnparsedValue(['a']); try { v[5] = 'x'; } catch (e) { return [e.name, e.message]; } return v.length; }], ['unparsed coerce', () => { const v = new CSSUnparsedValue(['a']); v[0] = 5; return [typeof v[0], v[0]]; }], ['unparsed cycle', () => { const inner = new CSSUnparsedValue([]); const v = new CSSUnparsedValue([new CSSVariableReferenceValue('--a', inner)]); inner[0] = new CSSVariableReferenceValue('--b', v); return String(v); }],
    ['unparsed tokens', () => String(new CSSUnparsedValue(['1.50px  , 2.0', '  ', '"q"', 'url(a)']))], ['unparsed escape', () => String(new CSSUnparsedValue([new CSSVariableReferenceValue('--a b')]))], ['unparsed iterate', () => { const v = new CSSUnparsedValue(['a', 'b']); return [[...v], [...v.keys()], [...v.entries()].map(String)]; }],
    ['varref', () => { const r = new CSSVariableReferenceValue('--a'); r.variable = '--b'; let error; try { r.variable = 'c'; } catch (e) { error = e.message; } return [r.variable, r.fallback, error]; }],
    ['translate text', () => [String(new CSSTranslate(px(1), pct(2))), String(new CSSTranslate(px(1), px(2), px(3))), String(new CSSTranslate(new CSSMathSum(px(1), pct(2)), px(0)))]], ['translate setters', () => { const t = new CSSTranslate(px(1), px(2)); const r = []; for (const [k, v] of [['x', deg(1)], ['y', deg(1)], ['z', pct(1)], ['x', 5]]) { try { t[k] = v; r.push('ok'); } catch (e) { r.push(e.message); } } t.z = px(4); return [r, String(t), t.is2D]; }],
    ['rotate text', () => [String(new CSSRotate(deg(45))), String(new CSSRotate(1, 0, 0, new CSSUnitValue(1, 'turn'))), String(new CSSRotate(new CSSUnitValue(1, 'rad')))]], ['rotate setters', () => { const t = new CSSRotate(deg(1)); const r = []; for (const [k, v] of [['angle', px(1)], ['x', px(1)], ['x', 2], ['y', NaN]]) { try { t[k] = v; r.push('ok'); } catch (e) { r.push(e.message); } } return [r, String(t), describe(t.x)]; }],
    ['scale text', () => [String(new CSSScale(2, 3)), String(new CSSScale(1, 2, 3)), String(new CSSScale(num(2), new CSSMathSum(num(1), num(2))))]], ['skew text', () => [String(new CSSSkew(deg(10), deg(0))), String(new CSSSkew(deg(10), deg(5))), String(new CSSSkew(deg(1), new CSSUnitValue(0, 'rad'))), String(new CSSSkewX(deg(3))), String(new CSSSkewY(new CSSUnitValue(1, 'turn')))]],
    ['perspective', () => [String(new CSSPerspective(px(10))), String(new CSSPerspective('none')), String(new CSSPerspective('NONE')), String(new CSSPerspective(new CSSKeywordValue('none'))), String(new CSSPerspective(px(-5))), describe(new CSSPerspective('none').length), new CSSPerspective(px(1)).is2D]],
    ['is2D toggle', () => { const t = new CSSTranslate(px(1), px(2)); t.is2D = false; const r = new CSSRotate(deg(1)); r.is2D = false; const s = new CSSScale(1, 2, 3); s.is2D = true; return [String(t), String(r), String(s)]; }],
    ['transform value', () => { const t = new CSSTransformValue([new CSSTranslate(px(1), px(2)), new CSSRotate(deg(3))]); return [t.length, t.is2D, String(t), t[1] instanceof CSSRotate, Object.keys(t)]; }], ['transform 3d', () => new CSSTransformValue([new CSSTranslate(px(1), px(2), px(3))]).is2D], ['transform indexed', () => { const t = new CSSTransformValue([new CSSSkewX(deg(1))]); t[1] = new CSSSkewY(deg(2)); let error; try { t[5] = new CSSSkewY(deg(2)); } catch (e) { error = [e.name, e.message]; } let error2; try { t[0] = 1; } catch (e) { error2 = [e.name, e.message]; } return [t.length, String(t), error, error2]; }],
    ['position', () => { const p = new CSSPositionValue(px(1), pct(2)); let error; try { p.x = deg(1); } catch (e) { error = e.message; } p.y = new CSSMathSum(px(1), pct(2)); return [String(p), error]; }],
  ]);

  // ---- attributeStyleMap -------------------------------------------------------------------
  const host = document.getElementById('host');
  const el = document.createElement('div');
  host.append(el);
  const map = el.attributeStyleMap;
  const snapshot = () => ({ attr: el.getAttribute('style'), size: map.size, entries: [...map].map(([k, v]) => [k, v.map(String)]) });
  // Every step starts from its own style attribute, so one divergence cannot cascade.
  const step = (label, style, f) => { if (style === null) el.removeAttribute('style'); else el.setAttribute('style', style); let result; try { result = describe(f()); } catch (e) { result = { e: e.name, m: e.message }; } return [label, result, snapshot()]; };
  const BASE = 'width: 10px; color: red; --brand: blue';
  out.inline = [
    step('identity', null, () => [map === el.attributeStyleMap, map instanceof StylePropertyMap, map.size]),
    step('set width', null, () => map.set('width', px(10))),
    step('set height string', BASE, () => map.set('height', '20%')),
    step('get', BASE, () => [map.get('width'), map.get('color'), map.get('--brand'), map.get('height'), map.get('WIDTH')]), step('has', BASE, () => [map.has('width'), map.has('height'), map.has('--brand'), map.has('--nope')]), step('getAll width', BASE, () => map.getAll('width')),
    step('setAttribute live', BASE, () => { el.setAttribute('style', 'opacity: 0.5; margin-left: auto; --brand: red; z-index: 3'); return [map.size, map.get('opacity'), map.get('margin-left'), map.get('--brand'), map.get('z-index')]; }),
    step('style property live', BASE, () => { el.style.paddingTop = '2px'; el.style.width = ''; return [map.get('padding-top'), map.has('width'), map.size]; }),
    step('stored forms', 'width: CALC(1PX + 2PX); height: calc(10px + 5%); opacity: 50%; margin-top: 0; z-index: 2; transition-duration: 1s, 200MS; display: BLOCK; align-items: CENTER', () => [...map].map(([k, v]) => [k, v.map(describe)])),
    step('set opacity big', BASE, () => map.set('opacity', num(2))), step('set opacity negative', BASE, () => map.set('opacity', num(-1))), step('set width negative', BASE, () => map.set('width', px(-5))), step('get wrapped', BASE, () => { map.set('width', px(-5)); return map.get('width'); }),
    step('set order fraction', BASE, () => map.set('order', num(1.5))), step('set z-index fraction', BASE, () => map.set('z-index', num(1.5))),
    step('set keyword', BASE, () => map.set('display', new CSSKeywordValue('flex'))), step('set bad keyword', BASE, () => map.set('display', new CSSKeywordValue('wide'))), step('set keyword string', BASE, () => map.set('display', 'grid')), step('set css-wide', BASE, () => map.set('width', new CSSKeywordValue('inherit'))), step('set custom keyword', BASE, () => map.set('--k', new CSSKeywordValue('x'))), step('set keyword case', BASE, () => map.set('display', new CSSKeywordValue('FLEX'))),
    step('set sum', BASE, () => map.set('width', new CSSMathSum(px(1), pct(2)))), step('set calc string', BASE, () => map.set('width', 'calc(10px + 5%)')), step('set min', BASE, () => map.set('width', new CSSMathMin(px(1), pct(2)))), step('set product', BASE, () => map.set('height', new CSSMathProduct(px(2), 3))), step('set negate', BASE, () => map.set('margin-top', new CSSMathNegate(px(4)))), step('set calc simplify', BASE, () => map.set('width', 'calc(1px + 2px)')),
    step('set wrong type', BASE, () => map.set('width', sec(1))), step('set two values', BASE, () => map.set('width', '1px', '2px')), step('set no values', BASE, () => map.set('width')), step('set bad name', BASE, () => map.set('foo', '1px')), step('set number for length', BASE, () => map.set('width', num(0))), step('set percent for number', BASE, () => map.set('opacity', pct(50))), step('set invalid string', BASE, () => map.set('width', 'red')),
    step('set unsupported from parse', BASE, () => map.set('align-items', CSSStyleValue.parse('align-items', 'center'))), step('set unsupported other property', BASE, () => map.set('justify-content', CSSStyleValue.parse('align-items', 'center'))), step('set color string', BASE, () => map.set('color', 'rgb(1, 2, 3)')), step('set color value', BASE, () => map.set('background-color', CSSStyleValue.parse('background-color', 'red'))),
    step('set transform', BASE, () => map.set('transform', new CSSTransformValue([new CSSTranslate(px(1), pct(2)), new CSSScale(2, 2)]))), step('set position', BASE, () => map.set('object-position', new CSSPositionValue(px(1), pct(2)))), step('set image from parse', BASE, () => map.set('background-image', CSSStyleValue.parse('background-image', 'url(a.png)'))),
    step('set unparsed', BASE, () => map.set('width', new CSSUnparsedValue([new CSSVariableReferenceValue('--w')]))), step('get unparsed', 'width: var(--w, 1px)', () => map.get('width')), step('set custom unparsed', BASE, () => map.set('--x', new CSSUnparsedValue(['1px ', 'solid']))), step('set custom string', BASE, () => map.set('--y', '  a  b ')), step('set custom unit', BASE, () => map.set('--z', px(1))),
    step('set list', BASE, () => map.set('transition-duration', sec(1), '2s, 300ms')), step('append list', 'transition-duration: 1s', () => map.append('transition-duration', sec(4))), step('append string', 'transition-duration: 1s', () => map.append('transition-duration', '5s, 6s')), step('append to missing', BASE, () => map.append('transition-duration', '5s')), step('append bad', 'transition-duration: 1s', () => map.append('transition-duration', px(1))), step('append none', BASE, () => map.append('foo')), step('append non list', BASE, () => map.append('width', px(1))), step('append custom', BASE, () => map.append('--x', 'a')),
    step('set list keyword', BASE, () => map.set('transition-property', 'opacity')), step('append css-wide', 'animation-name: inherit', () => map.append('animation-name', 'spin')), step('append var', 'animation-name: var(--n)', () => map.append('animation-name', 'spin')), step('set list css-wide pair', BASE, () => map.set('animation-name', 'inherit', 'spin')), step('set list values', BASE, () => map.set('animation-name', new CSSKeywordValue('a'), new CSSKeywordValue('B'))),
    step('set shorthand', BASE, () => map.set('margin', '1px 2px')), step('get shorthand', 'margin: 1px 2px', () => [map.get('margin'), map.getAll('margin').length]), step('set shorthand value', BASE, () => map.set('padding', CSSStyleValue.parse('padding', '3px'))), step('set shorthand unit', BASE, () => map.set('margin', px(5))), step('set shorthand invalid', BASE, () => map.set('margin', 'red')), step('set shorthand two', BASE, () => map.set('margin', '1px', '2px')),
    step('delete', BASE, () => map.delete('width')), step('delete missing', BASE, () => map.delete('height')), step('delete bad', BASE, () => map.delete('foo')), step('delete custom', BASE, () => map.delete('--brand')), step('delete alias', 'transform: none', () => map.delete('-webkit-transform')),
    step('forEach', BASE, () => { const seen = []; map.forEach(function (v, k, m) { seen.push([k, v.length, m === map, this === undefined]); }); return seen; }), step('keys values', BASE, () => [[...map.keys()], [...map.values()].map((v) => v.map(String))]),
    step('iterator shape', BASE, () => { const it = map.entries(); const p = Object.getPrototypeOf(it); return [Object.prototype.toString.call(it), Object.getOwnPropertyNames(p), Object.getPrototypeOf(p) === Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]())), it.next().done]; }),
    step('iterator snapshot', BASE, () => { const it = map.keys(); map.set('top', px(1)); return [...it].includes('top'); }),
    step('clear', BASE, () => map.clear()), step('clear empty', null, () => map.clear()),
    step('mutation records', null, () => { const records = []; const o = new MutationObserver((r) => records.push(...r)); o.observe(el, { attributes: true, attributeOldValue: true }); map.set('width', px(1)); map.set('height', px(2)); map.clear(); const r = o.takeRecords(); o.disconnect(); return r.map((x) => [x.attributeName, x.oldValue]); }),
    step('svg map', null, () => { const svg = document.createElementNS('http://www.w3.org/2000/svg', 'rect'); host.append(svg); svg.attributeStyleMap.set('fill', 'red'); return [svg.getAttribute('style'), String(svg.attributeStyleMap.get('fill'))]; }),
  ];

  // ---- CSSStyleRule.styleMap ---------------------------------------------------------------
  const sheet = document.getElementById('sheet').sheet;
  const rule = sheet.cssRules[0];
  const ruleSnapshot = () => ({ css: rule.style.cssText, size: rule.styleMap.size });
  const ruleStep = (label, f) => { let result; try { result = describe(f()); } catch (e) { result = { e: e.name, m: e.message }; } return [label, result, ruleSnapshot()]; };
  out.rule = [
    ruleStep('identity', () => [rule.styleMap === rule.styleMap, rule.styleMap instanceof StylePropertyMap, rule.styleMap === rule.style]),
    ruleStep('entries', () => [...rule.styleMap].map(([k, v]) => [k, v.map(describe)])),
    ruleStep('get', () => [rule.styleMap.get('width'), rule.styleMap.get('color'), rule.styleMap.get('--v'), rule.styleMap.get('transition-duration')]),
    ruleStep('set', () => rule.styleMap.set('width', px(42))),
    ruleStep('set reaches computed', () => document.getElementById('ruled').computedStyleMap().get('width')),
    ruleStep('style edit live', () => { rule.style.setProperty('height', '7px'); return rule.styleMap.get('height'); }),
    ruleStep('delete', () => rule.styleMap.delete('color')), ruleStep('append', () => rule.styleMap.append('transition-duration', sec(9))),
  ];

  // ---- computedStyleMap --------------------------------------------------------------------
  const plain = document.getElementById('plain');
  const plainMap = plain.computedStyleMap();
  out.computedShape = [
    ['identity', plainMap === plain.computedStyleMap(), plainMap instanceof StylePropertyMapReadOnly, plainMap instanceof StylePropertyMap],
    ['size', plainMap.size], ['first keys', [...plainMap.keys()].slice(0, 5)], ['last keys', [...plainMap.keys()].slice(-3)],
    ['disconnected', (() => { const d = document.createElement('div'); d.style.width = '5px'; const m = d.computedStyleMap(); return [m.size, String(m.get('width')), m.has('width'), [...m].length]; })()],
    ['bad name', E(() => plainMap.get('foo'))], ['missing custom', String(plainMap.get('--nope'))], ['shorthand margin', String(plainMap.get('margin'))], ['shorthand padding', String(plainMap.get('padding'))],
  ];
  out.computedPlain = [...plainMap].map(([k, v]) => [k, v.map((x) => [x.constructor.name, String(x)])]);
  // Every computable property re-parses its own initial serialization.
  out.parseInitial = [...plainMap].map(([k, v]) => [k, run(() => CSSStyleValue.parseAll(k, v.map(String).join(', ')).map((x) => [x.constructor.name, String(x)]))]);
  const styled = document.getElementById('styled');
  const SHOW = ['width', 'height', 'min-width', 'max-width', 'margin-top', 'margin-left', 'margin-right', 'margin-block-start', 'margin-inline-start', 'padding-left', 'padding-inline-end', 'inline-size', 'block-size', 'top', 'left', 'opacity', 'z-index', 'order', 'display', 'position', 'visibility', 'color', 'background-color', 'border-top-color', 'border-top-width', 'border-top-style', 'border-left-width', 'outline-color', 'caret-color', 'font-size', 'font-weight', 'line-height', 'letter-spacing', 'transition-duration', 'transition-delay', 'transition-property', 'animation-name', 'animation-duration', 'transform', 'translate', 'rotate', 'object-position', 'perspective-origin', 'transform-origin', 'background-image', 'background-size', 'border-top-left-radius', 'text-decoration-line', 'text-transform', 'text-indent', 'flex-grow', 'flex-basis', 'cursor', 'will-change', 'pointer-events', '--brand', '--size', '--inherited'];
  const showComputed = (node) => SHOW.map((name) => [name, run(() => node.computedStyleMap().getAll(name))]);
  out.computedStyled = showComputed(styled);
  out.computedChild = showComputed(document.getElementById('child'));
  out.computedEntriesTail = [...styled.computedStyleMap()].filter(([k]) => k.startsWith('--')).map(([k, v]) => [k, v.map(String)]);
  out.computedSize = [styled.computedStyleMap().size, document.getElementById('child').computedStyleMap().size];
  const full = (id) => [...document.getElementById(id).computedStyleMap()].map(([k, v]) => [k, v.map((x) => [x.constructor.name, String(x)])]);
  out.computedVaried = full('varied');
  out.computedVariedChild = full('varied-child');
  out.computedMisc = ['hidden', 'dialog', 'para', 'host'].map((id) => [id, String(document.getElementById(id).computedStyleMap().get('display'))]).concat([['body margin', String(document.body.computedStyleMap().get('margin-top')), String(document.body.computedStyleMap().get('margin'))], ['html display', String(document.documentElement.computedStyleMap().get('display'))], ['head display', String(document.head.computedStyleMap().get('display'))]]);
  return out;
})();
