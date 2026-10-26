// Web Animations: Element.animate/getAnimations, DocumentOrShadowRoot.getAnimations,
// document.timeline and the Animation, KeyframeEffect, AnimationEffect, AnimationTimeline,
// DocumentTimeline and AnimationPlaybackEvent interfaces over one timing model
// (https://drafts.csswg.org/web-animations-1/). Document timelines read the embedding's
// rendering-frame clock through a trusted host (ns.installAnimationHost): each frame runs
// "update animations and send events" before the animation-frame callbacks and commits
// pending animations after them. Keyframe effects compose into the computed-style
// snapshot (ns.animatedStyle). CSS animations/transitions are not modeled, so
// getAnimations() reports script animations only.
(function () {
  'use strict';
  const Native = globalThis.__animationsNative; delete globalThis.__animationsNative;
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction;
  const apply = Reflect.apply, construct = Reflect.construct, define = Object.defineProperty, create = Object.create;
  const ownKeys = Reflect.ownKeys, getOwnDescriptor = Object.getOwnPropertyDescriptor, setPrototype = Object.setPrototypeOf;
  const ErrorType = TypeError, Exception = g.DOMException, PromiseType = Promise, promiseThen = Promise.prototype.then;
  const EventType = g.Event, TargetType = g.EventTarget;
  const listen = g.EventTarget.prototype.addEventListener, unlisten = g.EventTarget.prototype.removeEventListener;
  const floor = Math.floor, round = Math.round, abs = Math.abs, min = Math.min, max = Math.max, sqrt = Math.sqrt;
  const sin = Math.sin, cos = Math.cos, tan = Math.tan, atan2 = Math.atan2, acos = Math.acos, PI = Math.PI;
  const isFinite = Number.isFinite, iteratorSymbol = Symbol.iterator;
  const realmDocument = g.document;
  const nodeTypeGetter = ns.domAccessor('nodeType', false, 'Node'), connectedGetter = ns.domAccessor('isConnected', false, 'Node');
  const ownerGetter = ns.domAccessor('ownerDocument', false, 'Node'), parentGetter = ns.domAccessor('parentElement', false, 'Node');
  const shadowMode = g.ShadowRoot && getOwnDescriptor(g.ShadowRoot.prototype, 'mode');
  const rootOf = g.Node.prototype.getRootNode, containsNode = g.Node.prototype.contains;
  const computedStyle = g.getComputedStyle;
  const store = () => { const map = new WeakMap(), get = WeakMap.prototype.get, set = WeakMap.prototype.set, has = WeakMap.prototype.has; return { get: key => apply(get, map, [key]), set: (key, value) => apply(set, map, [key, value]), has: key => apply(has, map, [key]) }; };
  const animations = store(), effects = store(), timelines = store(), events = store(), targets = store(), documentTimelines = store();
  const SetType = Set, setAdd = Set.prototype.add, setDelete = Set.prototype.delete, setValues = Set.prototype.values;
  const MapType = Map, mapGet = Map.prototype.get, mapSet = Map.prototype.set;
  const live = new SetType();
  const list = () => { const out = []; for (const value of apply(setValues, live, [])) out[out.length] = value; return out; };
  const push = (array, value) => { array[array.length] = value; return array; };
  const sortBy = (array, key) => { for (let i = 1; i < array.length; i++) { const item = array[i], k = key(item); let j = i - 1; while (j >= 0 && key(array[j]) > k) { array[j + 1] = array[j]; j--; } array[j + 1] = item; } return array; };

  // ---- conversions and Chrome's binding messages ------------------------------------------
  const string = value => `${value}`;
  const nodeType = value => { if (value === null || (typeof value !== 'object' && typeof value !== 'function')) return 0; try { return apply(nodeTypeGetter, value, []); } catch (error) { return 0; } };
  const isElement = value => nodeType(value) === 1;
  const objectLike = value => value !== null && (typeof value === 'object' || typeof value === 'function');
  const illegal = () => new ErrorType('Illegal invocation');
  const constructing = name => `Failed to construct '${name}': `;
  const executing = (operation, name) => `Failed to execute '${operation}' on '${name}': `;
  const setting = (property, name) => `Failed to set the '${property}' property on '${name}': `;
  const invalidState = (prefix, message) => new Exception(prefix + message, 'InvalidStateError');
  const finite = (value, prefix) => { const number = +value; if (!isFinite(number)) throw new ErrorType(prefix + 'The provided double value is non-finite.'); return number; };
  const member = (prefix, name, dictionary) => `${prefix}Failed to read the '${name}' property from '${dictionary}': `;
  const enumeration = (value, allowed, prefix, name, dictionary, type) => {
    const text = string(value);
    for (let i = 0; i < allowed.length; i++) if (allowed[i] === text) return text;
    throw new ErrorType(`${member(prefix, name, dictionary)}The provided value '${text}' is not a valid enum value of type ${type}.`);
  };
  const dictionary = (value, prefix, type) => {
    if (value === undefined || value === null) return null;
    if (!objectLike(value)) throw new ErrorType(`${prefix}The provided value is not of type '${type}'.`);
    return value;
  };
  const tick = PromiseType.resolve();
  const microtask = callback => { apply(promiseThen, tick, [callback]); };
  const handled = promise => { apply(promiseThen, promise, [undefined, () => {}]); };
  const abort = () => new Exception('The user aborted a request.', 'AbortError');

  // Number serialization at six significant digits (Blink's String::Number for CSS values).
  function format(value) {
    if (value === 0 || !isFinite(value)) return value === 0 ? '0' : string(value);
    let text = value.toPrecision(6), exponent = '';
    const e = text.indexOf('e');
    if (e >= 0) { exponent = text.slice(e); text = text.slice(0, e); }
    if (text.indexOf('.') >= 0) { while (text[text.length - 1] === '0') text = text.slice(0, -1); if (text[text.length - 1] === '.') text = text.slice(0, -1); }
    text += exponent;
    return text === '-0' ? '0' : text;
  }

  // ---- easing functions (CSS Easing 2) ------------------------------------------------------
  const NUMBER = /^[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?$/i;
  const PRESETS = { __proto__: null, ease: [0.25, 0.1, 0.25, 1], 'ease-in': [0.42, 0, 1, 1], 'ease-out': [0, 0, 0.58, 1], 'ease-in-out': [0.42, 0, 0.58, 1] };
  const linearEasing = { text: 'linear', evaluate: x => x };
  // gfx::CubicBezier, reproduced step for step so solved progress matches Chrome's values.
  function bezier(x1, y1, x2, y2) {
    const cx = 3 * x1, bx = 3 * (x2 - x1) - cx, ax = 1 - cx - bx, cy = 3 * y1, by = 3 * (y2 - y1) - cy, ay = 1 - cy - by;
    const sampleX = t => ((ax * t + bx) * t + cx) * t, sampleY = t => ((ay * t + by) * t + cy) * t, slope = t => (3 * ax * t + 2 * bx) * t + cx;
    const samples = []; for (let i = 0; i < 11; i++) samples[i] = sampleX(i * 0.1);
    let start, end;
    if (x1 > 0) start = y1 / x1; else if (!y1 && x2 > 0) start = y2 / x2; else if (!y1 && !y2) start = 1; else start = 0;
    if (x2 < 1) end = (y2 - 1) / (x2 - 1); else if (y2 === 1 && x1 < 1) end = (y1 - 1) / (x1 - 1); else if (y2 === 1 && y1 === 1) end = 1; else end = 0;
    const epsilon = 1e-7;
    const solve = x => {
      let t0 = 0, t1 = 0, t2 = x, x2v = 0, d2, i;
      for (i = 1; i < 11; i++) if (x <= samples[i]) { t1 = 0.1 * i; t0 = t1 - 0.1; t2 = t0 + (t1 - t0) * (x - samples[i - 1]) / (samples[i] - samples[i - 1]); break; }
      for (i = 0; i < 4; i++) { x2v = sampleX(t2) - x; if (abs(x2v) < epsilon) return t2; d2 = slope(t2); if (abs(d2) < epsilon) break; t2 -= x2v / d2; }
      if (abs(x2v) < epsilon) return t2;
      while (t0 < t1) { x2v = sampleX(t2); if (abs(x2v - x) < epsilon) return t2; if (x > x2v) t0 = t2; else t1 = t2; t2 = (t1 + t0) * 0.5; }
      return t2;
    };
    return x => x < 0 ? start * x : x > 1 ? 1 + end * (x - 1) : sampleY(solve(x));
  }
  function steps(count, position) {
    const jumps = position === 'jump-none' ? count - 1 : position === 'jump-both' ? count + 1 : count;
    const startOffset = position === 'start' || position === 'jump-start' || position === 'jump-both' ? 1 : 0;
    return (x, before) => {
      let step = floor(x * count) + startOffset;
      if (before && x * count - floor(x * count) === 0) step -= 1;
      if (x >= 0 && step < 0) step = 0;
      if (x <= 1 && step > jumps) step = jumps;
      return step / jumps;
    };
  }
  function linearPoints(body) {
    const parts = body.split(','), points = [];
    for (let i = 0; i < parts.length; i++) {
      const tokens = parts[i].trim().split(/\s+/);
      let output = null; const inputs = [];
      for (let j = 0; j < tokens.length; j++) {
        const token = tokens[j];
        if (NUMBER.test(token) && output === null && !inputs.length) output = +token;
        else if (NUMBER.test(token) && output === null && inputs.length && j === tokens.length - 1) output = +token;
        else if (/%$/.test(token) && NUMBER.test(token.slice(0, -1))) push(inputs, +token.slice(0, -1) / 100);
        else return null;
      }
      if (output === null || inputs.length > 2) return null;
      if (!inputs.length) push(points, { output, input: null });
      for (let k = 0; k < inputs.length; k++) push(points, { output, input: inputs[k] });
    }
    if (points.length < 2) return null;
    if (points[0].input === null) points[0].input = 0;
    if (points[points.length - 1].input === null) points[points.length - 1].input = max(1, ...points.map(point => point.input === null ? -Infinity : point.input));
    let largest = -Infinity;
    for (let i = 0; i < points.length; i++) if (points[i].input !== null) { if (points[i].input < largest) points[i].input = largest; largest = points[i].input; }
    for (let i = 1; i < points.length; i++) if (points[i].input === null) {
      let j = i; while (points[j].input === null) j++;
      const from = points[i - 1].input, to = points[j].input, span = j - i + 1;
      for (let k = i; k < j; k++) points[k].input = from + (to - from) * (k - i + 1) / span;
      i = j;
    }
    return points;
  }
  function parseEasing(source) {
    const text = string(source).trim(), lower = text.toLowerCase();
    if (lower === 'linear') return linearEasing;
    if (PRESETS[lower]) { const p = PRESETS[lower], evaluate = bezier(p[0], p[1], p[2], p[3]); return { text: lower, evaluate }; }
    if (lower === 'step-start') return { text: 'steps(1, start)', evaluate: steps(1, 'start') };
    if (lower === 'step-end') return { text: 'steps(1)', evaluate: steps(1, 'end') };
    let match = /^cubic-bezier\(([^()]*)\)$/i.exec(text);
    if (match) {
      const values = match[1].split(',').map(part => part.trim());
      if (values.length !== 4 || !values.every(value => NUMBER.test(value))) return null;
      const [x1, y1, x2, y2] = values.map(value => +value);
      if (x1 < 0 || x1 > 1 || x2 < 0 || x2 > 1) return null;
      return { text: `cubic-bezier(${format(x1)}, ${format(y1)}, ${format(x2)}, ${format(y2)})`, evaluate: bezier(x1, y1, x2, y2) };
    }
    match = /^steps\(([^()]*)\)$/i.exec(text);
    if (match) {
      const values = match[1].split(',').map(part => part.trim());
      if (values.length > 2 || !/^\+?\d+$/.test(values[0])) return null;
      const count = +values[0], position = values.length === 2 ? values[1].toLowerCase() : 'end';
      if (!/^(start|end|jump-start|jump-end|jump-none|jump-both)$/.test(position)) return null;
      if (count < 1 || (position === 'jump-none' && count < 2)) return null;
      const shown = position === 'end' || position === 'jump-end' ? `steps(${count})` : `steps(${count}, ${position})`;
      return { text: shown, evaluate: steps(count, position) };
    }
    match = /^linear\(([^()]*)\)$/i.exec(text);
    if (match) {
      const points = linearPoints(match[1]);
      if (!points) return null;
      const shown = 'linear(' + points.map(point => `${format(point.output)} ${format(point.input * 100)}%`).join(', ') + ')';
      return { text: shown, evaluate: x => {
        let a = 0;
        for (let i = 0; i < points.length; i++) if (points[i].input <= x) a = i;
        if (a === points.length - 1) a--;
        const from = points[a], to = points[a + 1];
        if (from.input === to.input) return to.output;
        return from.output + (x - from.input) / (to.input - from.input) * (to.output - from.output);
      } };
    }
    return null;
  }
  const easing = (value, prefix) => { const parsed = parseEasing(value); if (!parsed) throw new ErrorType(`${prefix}'${string(value)}' is not a valid value for easing`); return parsed; };

  // ---- computed value model: numbers, lengths, colors, transforms, discrete ---------------
  const UNIT_NUMBERS = { __proto__: null, opacity: 1, 'fill-opacity': 1, 'stroke-opacity': 1, 'flood-opacity': 1, 'stop-opacity': 1 };
  const INTEGERS = { __proto__: null, 'z-index': 1, order: 1, orphans: 1, widows: 1, 'column-count': 1 };
  const NUMBERS = { __proto__: null, 'flex-grow': 1, 'flex-shrink': 1, 'font-weight': 1, 'line-height': 1, 'stroke-miterlimit': 1 };
  const COLORS = { __proto__: null, color: 1, 'background-color': 1, 'border-top-color': 1, 'border-right-color': 1, 'border-bottom-color': 1, 'border-left-color': 1, 'outline-color': 1, 'text-decoration-color': 1, 'column-rule-color': 1, 'caret-color': 1, 'accent-color': 1, fill: 1, stroke: 1, 'flood-color': 1, 'lighting-color': 1, 'stop-color': 1, 'text-emphasis-color': 1 };
  const INHERITED = { __proto__: null, color: 1, visibility: 1, 'line-height': 1, 'font-weight': 1, 'letter-spacing': 1, 'word-spacing': 1, 'text-indent': 1, 'caret-color': 1, 'accent-color': 1, fill: 1, stroke: 1, 'fill-opacity': 1, 'stroke-opacity': 1, orphans: 1, widows: 1, cursor: 1, 'pointer-events': 1 };
  const INITIAL = { __proto__: null, opacity: '1', 'fill-opacity': '1', 'stroke-opacity': '1', 'flood-opacity': '1', 'stop-opacity': '1', transform: 'none', color: 'rgb(0, 0, 0)', 'background-color': 'rgba(0, 0, 0, 0)', 'z-index': 'auto', visibility: 'visible', display: 'inline', 'flex-grow': '0', 'flex-shrink': '1', 'font-weight': '400', 'line-height': 'normal', 'letter-spacing': 'normal', 'word-spacing': '0px', 'text-indent': '0px', width: 'auto', height: 'auto', top: 'auto', right: 'auto', bottom: 'auto', left: 'auto', 'outline-color': 'currentcolor', 'text-decoration-color': 'currentcolor', 'column-rule-color': 'currentcolor', 'caret-color': 'auto', 'accent-color': 'auto' };
  const lengthProperty = name => /(width|height|margin|padding|gap|radius|letter-spacing|word-spacing|text-indent)/.test(name) || /size$/.test(name) || /^inset/.test(name) || /^(top|right|bottom|left|flex-basis|outline-offset|column-width|column-gap)$/.test(name);
  const initialValue = name => INITIAL[name] !== undefined ? INITIAL[name] : /^border-(top|right|bottom|left)-color$/.test(name) ? 'currentcolor' : /^(margin|padding)-/.test(name) ? '0px' : /-radius$/.test(name) ? '0px' : '';

  const LENGTH = /^([+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)([a-z%]*)$/i;
  function parseNumber(text) { return NUMBER.test(text) ? +text : null; }
  function parseLength(text, context) {
    const match = LENGTH.exec(text);
    if (!match) return null;
    const value = +match[1], unit = match[2].toLowerCase();
    if (unit === '') return value === 0 ? { kind: 'length', value: 0, unit: 'px' } : null;
    if (unit === 'px') return { kind: 'length', value, unit: 'px' };
    if (unit === 'em') return { kind: 'length', value: value * context.fontSize, unit: 'px' };
    if (unit === 'rem') return { kind: 'length', value: value * 16, unit: 'px' };
    if (unit === 'in') return { kind: 'length', value: value * 96, unit: 'px' };
    if (unit === 'cm') return { kind: 'length', value: value * 96 / 2.54, unit: 'px' };
    if (unit === 'mm') return { kind: 'length', value: value * 96 / 25.4, unit: 'px' };
    if (unit === 'pt') return { kind: 'length', value: value * 4 / 3, unit: 'px' };
    if (unit === 'pc') return { kind: 'length', value: value * 16, unit: 'px' };
    if (unit === '%') return { kind: 'length', value, unit: '%' };
    return null;
  }
  // Colors in sRGB with straight alpha: hex, rgb()/hsl()/hwb() and named keywords.
  function hsl(h, s, l) {
    const f = n => { const k = (n + h / 30) % 12, a = s * min(l, 1 - l); return l - a * max(-1, min(k - 3, 9 - k, 1)); };
    return [f(0) * 255, f(8) * 255, f(4) * 255];
  }
  function hue(token) {
    const match = /^([+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)(deg|rad|grad|turn)?$/i.exec(token);
    if (!match) return null;
    const value = +match[1], unit = (match[2] || 'deg').toLowerCase();
    const degrees = unit === 'rad' ? value * 180 / PI : unit === 'grad' ? value * 0.9 : unit === 'turn' ? value * 360 : value;
    return ((degrees % 360) + 360) % 360;
  }
  function channel(token, scale) {
    if (/%$/.test(token)) { const number = parseNumber(token.slice(0, -1)); return number === null ? null : number / 100 * scale; }
    return parseNumber(token);
  }
  function parseColor(text, context) {
    const lower = text.trim().toLowerCase();
    if (lower === 'currentcolor') return context.currentColor();
    if (lower === 'transparent') return { kind: 'color', r: 0, g: 0, b: 0, a: 0 };
    let match = /^#([0-9a-f]{3,8})$/.exec(lower);
    if (match) {
      let hex = match[1];
      if (hex.length === 3 || hex.length === 4) hex = hex.replace(/./g, digit => digit + digit);
      if (hex.length !== 6 && hex.length !== 8) return null;
      const byte = i => parseInt(hex.slice(i, i + 2), 16);
      return { kind: 'color', r: byte(0), g: byte(2), b: byte(4), a: hex.length === 8 ? byte(6) / 255 : 1 };
    }
    match = /^(rgba?|hsla?|hwb)\((.*)\)$/.exec(lower);
    if (match) {
      let body = match[2].trim(), alpha = '1', parts;
      if (body.indexOf(',') >= 0) { parts = body.split(',').map(part => part.trim()); if (parts.length === 4) alpha = parts.pop(); }
      else { const slash = body.split('/'); if (slash.length > 2) return null; if (slash.length === 2) alpha = slash[1].trim(); parts = slash[0].trim().split(/\s+/); }
      if (parts.length !== 3) return null;
      const a = channel(alpha, 1);
      if (a === null) return null;
      let rgb;
      if (match[1][0] === 'r') { rgb = parts.map(part => channel(part, 255)); if (rgb.some(value => value === null)) return null; }
      else {
        const h = hue(parts[0]), x = channel(parts[1], 1), y = channel(parts[2], 1);
        if (h === null || x === null || y === null || !/%$/.test(parts[1]) && match[1] !== 'hwb' && body.indexOf(',') >= 0) return null;
        if (match[1] === 'hwb') { const white = x, black = y; if (white + black >= 1) { const gray = white / (white + black) * 255; rgb = [gray, gray, gray]; } else rgb = hsl(h, 1, 0.5).map(value => value / 255 * (1 - white - black) * 255 + white * 255); }
        else rgb = hsl(h, min(max(x, 0), 1), min(max(y, 0), 1));
      }
      const clamp = value => min(max(value, 0), 255);
      return { kind: 'color', r: clamp(rgb[0]), g: clamp(rgb[1]), b: clamp(rgb[2]), a: min(max(a, 0), 1) };
    }
    if (/^[a-z]+$/.test(lower)) { const named = Native('named', lower); return named ? parseColor(named, context) : null; }
    return null;
  }
  // CSS Color 4 legacy serialization: 8-bit channels, alpha at the shortest of two or three
  // decimals that round-trips the 8-bit alpha.
  function serializeAlpha(alpha) {
    const byte = round(alpha * 255), two = round(byte / 255 * 100) / 100;
    return format(round(two * 255) === byte ? two : round(byte / 255 * 1000) / 1000);
  }
  function serializeColor(color) {
    const r = round(color.r), gg = round(color.g), b = round(color.b);
    return color.a >= 1 ? `rgb(${r}, ${gg}, ${b})` : `rgba(${r}, ${gg}, ${b}, ${serializeAlpha(color.a)})`;
  }
  // Keyframe values keep named colors; other color syntaxes serialize as rgb()/rgba().
  function specifiedColor(text) {
    const lower = text.toLowerCase();
    if (/^[a-z]+$/.test(lower)) return lower;
    const color = parseColor(text, { currentColor: () => null });
    return color ? serializeColor(color) : text;
  }

  // Transform lists: each function normalized to a primitive with numeric arguments.
  function angle(token) {
    const match = /^([+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)(deg|rad|grad|turn)?$/i.exec(token);
    if (!match || (!match[2] && +match[1] !== 0)) return null;
    const value = +match[1], unit = (match[2] || 'deg').toLowerCase();
    return unit === 'rad' ? value : unit === 'grad' ? value * PI / 200 : unit === 'turn' ? value * 2 * PI : value * PI / 180;
  }
  function transformLength(token) { const length = parseLength(token, { fontSize: 16 }); return length && length.unit === 'px' ? length.value : null; }
  function scaleValue(token) { if (/%$/.test(token)) { const number = parseNumber(token.slice(0, -1)); return number === null ? null : number / 100; } return parseNumber(token); }
  function parseTransform(text) {
    const lower = text.trim();
    if (/^none$/i.test(lower)) return { kind: 'transform', list: [] };
    const list = [], pattern = /\s*([a-zA-Z0-9]+)\(([^()]*)\)\s*/y;
    let index = 0;
    while (index < lower.length) {
      pattern.lastIndex = index;
      const match = pattern.exec(lower);
      if (!match) return null;
      index = pattern.lastIndex;
      const name = match[1].toLowerCase(), args = match[2].split(',').map(part => part.trim());
      const fn = transformFunction(name, args);
      if (!fn) return null;
      push(list, fn);
    }
    return list.length ? { kind: 'transform', list } : null;
  }
  function transformFunction(name, args) {
    const lengths = values => { const out = values.map(transformLength); return out.some(value => value === null) ? null : out; };
    const count = (a, b) => args.length >= a && args.length <= b;
    let values;
    switch (name) {
      case 'translate': if (!count(1, 2) || !(values = lengths(args))) return null; return { type: 'translate', v: [values[0], values.length > 1 ? values[1] : 0, 0] };
      case 'translatex': if (!count(1, 1) || !(values = lengths(args))) return null; return { type: 'translate', v: [values[0], 0, 0] };
      case 'translatey': if (!count(1, 1) || !(values = lengths(args))) return null; return { type: 'translate', v: [0, values[0], 0] };
      case 'translatez': if (!count(1, 1) || !(values = lengths(args))) return null; return { type: 'translate', v: [0, 0, values[0]], three: true };
      case 'translate3d': if (!count(3, 3) || !(values = lengths(args))) return null; return { type: 'translate', v: values, three: true };
      case 'scale': case 'scalex': case 'scaley': case 'scalez': case 'scale3d': {
        values = args.map(scaleValue); if (values.some(value => value === null)) return null;
        if (name === 'scale' && count(1, 2)) return { type: 'scale', v: [values[0], values.length > 1 ? values[1] : values[0], 1] };
        if (name === 'scalex' && count(1, 1)) return { type: 'scale', v: [values[0], 1, 1] };
        if (name === 'scaley' && count(1, 1)) return { type: 'scale', v: [1, values[0], 1] };
        if (name === 'scalez' && count(1, 1)) return { type: 'scale', v: [1, 1, values[0]], three: true };
        if (name === 'scale3d' && count(3, 3)) return { type: 'scale', v: values, three: true };
        return null;
      }
      case 'rotate': case 'rotatez': { const a = count(1, 1) ? angle(args[0]) : null; return a === null ? null : { type: 'rotate', v: [0, 0, 1, a] }; }
      case 'rotatex': { const a = count(1, 1) ? angle(args[0]) : null; return a === null ? null : { type: 'rotate', v: [1, 0, 0, a], three: true }; }
      case 'rotatey': { const a = count(1, 1) ? angle(args[0]) : null; return a === null ? null : { type: 'rotate', v: [0, 1, 0, a], three: true }; }
      case 'rotate3d': { if (!count(4, 4)) return null; const axis = args.slice(0, 3).map(parseNumber), a = angle(args[3]); if (a === null || axis.some(value => value === null)) return null; return { type: 'rotate', v: [axis[0], axis[1], axis[2], a], three: true }; }
      case 'skew': { if (!count(1, 2)) return null; const x = angle(args[0]), y = args.length > 1 ? angle(args[1]) : 0; return x === null || y === null ? null : { type: 'skew', v: [x, y] }; }
      case 'skewx': { const x = count(1, 1) ? angle(args[0]) : null; return x === null ? null : { type: 'skew', v: [x, 0] }; }
      case 'skewy': { const y = count(1, 1) ? angle(args[0]) : null; return y === null ? null : { type: 'skew', v: [0, y] }; }
      case 'perspective': { if (!count(1, 1)) return null; if (/^none$/i.test(args[0])) return { type: 'perspective', v: [Infinity], three: true }; const d = transformLength(args[0]); return d === null || d < 0 ? null : { type: 'perspective', v: [d], three: true }; }
      case 'matrix': { values = args.map(parseNumber); if (args.length !== 6 || values.some(value => value === null)) return null; const [a, b, c, d, e, f] = values; return { type: 'matrix', m: [a, b, 0, 0, c, d, 0, 0, 0, 0, 1, 0, e, f, 0, 1] }; }
      case 'matrix3d': { values = args.map(parseNumber); if (args.length !== 16 || values.some(value => value === null)) return null; return { type: 'matrix', m: values, three: true }; }
    }
    return null;
  }
  // 4x4 matrices in column-major order (m[col * 4 + row]), as CSS matrix3d() lists them.
  const identity = () => [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1];
  function multiply(a, b) {
    const out = [];
    for (let col = 0; col < 4; col++) for (let row = 0; row < 4; row++) {
      let sum = 0; for (let k = 0; k < 4; k++) sum += a[k * 4 + row] * b[col * 4 + k];
      out[col * 4 + row] = sum;
    }
    return out;
  }
  function functionMatrix(fn) {
    const m = identity(), v = fn.v;
    switch (fn.type) {
      case 'translate': m[12] = v[0]; m[13] = v[1]; m[14] = v[2]; return m;
      case 'scale': m[0] = v[0]; m[5] = v[1]; m[10] = v[2]; return m;
      case 'skew': m[4] = tan(v[0]); m[1] = tan(v[1]); return m;
      case 'perspective': if (v[0] !== Infinity) m[11] = -1 / max(v[0], 1); return m;
      case 'matrix': return fn.m.slice();
      case 'rotate': {
        let [x, y, z] = v; const a = v[3], length = sqrt(x * x + y * y + z * z);
        if (!length) return m;
        x /= length; y /= length; z /= length;
        const s = sin(a), c = cos(a), t = 1 - c;
        return [t * x * x + c, t * x * y + s * z, t * x * z - s * y, 0, t * x * y - s * z, t * y * y + c, t * y * z + s * x, 0, t * x * z + s * y, t * y * z - s * x, t * z * z + c, 0, 0, 0, 0, 1];
      }
    }
    return m;
  }
  const listMatrix = list => { let m = identity(); for (let i = 0; i < list.length; i++) m = multiply(m, functionMatrix(list[i])); return m; };
  const flat = m => m[2] === 0 && m[3] === 0 && m[6] === 0 && m[7] === 0 && m[8] === 0 && m[9] === 0 && m[10] === 1 && m[11] === 0 && m[14] === 0 && m[15] === 1;
  function serializeTransform(value) {
    if (!value.list.length) return 'none';
    const m = listMatrix(value.list);
    if (flat(m)) return `matrix(${[m[0], m[1], m[4], m[5], m[12], m[13]].map(format).join(', ')})`;
    return `matrix3d(${m.map(format).join(', ')})`;
  }
  // CSS Transforms 2 matrix decomposition, interpolation and recomposition.
  function decompose(source) {
    const m = [[source[0], source[1], source[2], source[3]], [source[4], source[5], source[6], source[7]], [source[8], source[9], source[10], source[11]], [source[12], source[13], source[14], source[15]]];
    if (!m[3][3]) return null;
    for (let i = 0; i < 4; i++) for (let j = 0; j < 4; j++) m[i][j] /= source[15];
    const det3 = (a) => a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1]) - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0]) + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
    let perspective = [0, 0, 0, 1];
    if (m[0][3] || m[1][3] || m[2][3]) {
      const p = m.map(row => row.slice()); for (let i = 0; i < 3; i++) p[i][3] = 0; p[3][3] = 1;
      const inverse = invert([].concat(p[0], p[1], p[2], p[3]));
      if (!inverse) return null;
      const rhs = [m[0][3], m[1][3], m[2][3], m[3][3]];
      // perspective = rhs * transpose(inverse(p))
      perspective = [0, 1, 2, 3].map(col => rhs[0] * inverse[col * 4] + rhs[1] * inverse[col * 4 + 1] + rhs[2] * inverse[col * 4 + 2] + rhs[3] * inverse[col * 4 + 3]);
    } else if (!det3(m)) return null;
    const translate = [m[3][0], m[3][1], m[3][2]];
    const row = [m[0].slice(0, 3), m[1].slice(0, 3), m[2].slice(0, 3)];
    const length = v => sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
    const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    const combine = (a, b, sa, sb) => [a[0] * sa + b[0] * sb, a[1] * sa + b[1] * sb, a[2] * sa + b[2] * sb];
    const scale = [0, 0, 0], skew = [0, 0, 0];
    scale[0] = length(row[0]); row[0] = combine(row[0], row[0], 1 / scale[0], 0);
    skew[0] = dot(row[0], row[1]); row[1] = combine(row[1], row[0], 1, -skew[0]);
    scale[1] = length(row[1]); row[1] = combine(row[1], row[1], 1 / scale[1], 0); skew[0] /= scale[1];
    skew[1] = dot(row[0], row[2]); row[2] = combine(row[2], row[0], 1, -skew[1]);
    skew[2] = dot(row[1], row[2]); row[2] = combine(row[2], row[1], 1, -skew[2]);
    scale[2] = length(row[2]); row[2] = combine(row[2], row[2], 1 / scale[2], 0); skew[1] /= scale[2]; skew[2] /= scale[2];
    const cross = [row[1][1] * row[2][2] - row[1][2] * row[2][1], row[1][2] * row[2][0] - row[1][0] * row[2][2], row[1][0] * row[2][1] - row[1][1] * row[2][0]];
    if (dot(row[0], cross) < 0) for (let i = 0; i < 3; i++) { scale[i] = -scale[i]; row[i] = row[i].map(value => -value); }
    const quaternion = [0.5 * sqrt(max(1 + row[0][0] - row[1][1] - row[2][2], 0)), 0.5 * sqrt(max(1 - row[0][0] + row[1][1] - row[2][2], 0)), 0.5 * sqrt(max(1 - row[0][0] - row[1][1] + row[2][2], 0)), 0.5 * sqrt(max(1 + row[0][0] + row[1][1] + row[2][2], 0))];
    if (row[2][1] > row[1][2]) quaternion[0] = -quaternion[0];
    if (row[0][2] > row[2][0]) quaternion[1] = -quaternion[1];
    if (row[1][0] > row[0][1]) quaternion[2] = -quaternion[2];
    return { translate, scale, skew, perspective, quaternion };
  }
  function invert(m) {
    const inv = [], [a00, a01, a02, a03, a10, a11, a12, a13, a20, a21, a22, a23, a30, a31, a32, a33] = m;
    const b00 = a00 * a11 - a01 * a10, b01 = a00 * a12 - a02 * a10, b02 = a00 * a13 - a03 * a10, b03 = a01 * a12 - a02 * a11, b04 = a01 * a13 - a03 * a11, b05 = a02 * a13 - a03 * a12;
    const b06 = a20 * a31 - a21 * a30, b07 = a20 * a32 - a22 * a30, b08 = a20 * a33 - a23 * a30, b09 = a21 * a32 - a22 * a31, b10 = a21 * a33 - a23 * a31, b11 = a22 * a33 - a23 * a32;
    const det = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;
    if (!det) return null;
    const d = 1 / det;
    inv[0] = (a11 * b11 - a12 * b10 + a13 * b09) * d; inv[1] = (a02 * b10 - a01 * b11 - a03 * b09) * d; inv[2] = (a31 * b05 - a32 * b04 + a33 * b03) * d; inv[3] = (a22 * b04 - a21 * b05 - a23 * b03) * d;
    inv[4] = (a12 * b08 - a10 * b11 - a13 * b07) * d; inv[5] = (a00 * b11 - a02 * b08 + a03 * b07) * d; inv[6] = (a32 * b02 - a30 * b05 - a33 * b01) * d; inv[7] = (a20 * b05 - a22 * b02 + a23 * b01) * d;
    inv[8] = (a10 * b10 - a11 * b08 + a13 * b06) * d; inv[9] = (a01 * b08 - a00 * b10 - a03 * b06) * d; inv[10] = (a30 * b04 - a31 * b02 + a33 * b00) * d; inv[11] = (a21 * b02 - a20 * b04 - a23 * b00) * d;
    inv[12] = (a11 * b07 - a10 * b09 - a12 * b06) * d; inv[13] = (a00 * b09 - a01 * b07 + a02 * b06) * d; inv[14] = (a31 * b01 - a30 * b03 - a32 * b00) * d; inv[15] = (a20 * b03 - a21 * b01 + a22 * b00) * d;
    return inv;
  }
  function recompose(d) {
    let m = identity();
    for (let i = 0; i < 4; i++) m[i * 4 + 3] = d.perspective[i];
    const translate = identity(); translate[12] = d.translate[0]; translate[13] = d.translate[1]; translate[14] = d.translate[2];
    m = multiply(m, translate);
    const [x, y, z, w] = d.quaternion;
    const rotation = [1 - 2 * (y * y + z * z), 2 * (x * y + z * w), 2 * (x * z - y * w), 0, 2 * (x * y - z * w), 1 - 2 * (x * x + z * z), 2 * (y * z + x * w), 0, 2 * (x * z + y * w), 2 * (y * z - x * w), 1 - 2 * (x * x + y * y), 0, 0, 0, 0, 1];
    m = multiply(m, rotation);
    if (d.skew[2]) { const s = identity(); s[9] = d.skew[2]; m = multiply(m, s); }
    if (d.skew[1]) { const s = identity(); s[8] = d.skew[1]; m = multiply(m, s); }
    if (d.skew[0]) { const s = identity(); s[4] = d.skew[0]; m = multiply(m, s); }
    const scale = identity(); scale[0] = d.scale[0]; scale[5] = d.scale[1]; scale[10] = d.scale[2];
    return multiply(m, scale);
  }
  function interpolateMatrix(from, to, p) {
    const a = decompose(from), b = decompose(to);
    if (!a || !b) return p < 0.5 ? from : to;
    const lerp = (x, y) => x.map((value, i) => value + (y[i] - value) * p);
    let product = a.quaternion[0] * b.quaternion[0] + a.quaternion[1] * b.quaternion[1] + a.quaternion[2] * b.quaternion[2] + a.quaternion[3] * b.quaternion[3];
    product = max(min(product, 1), -1);
    let quaternion;
    if (abs(product) >= 1) quaternion = a.quaternion.slice();
    else {
      const sign = product < 0 ? -1 : 1, theta = acos(abs(product)), w = sin(p * theta) / sqrt(1 - product * product);
      const scaleA = cos(p * theta) - abs(product) * w;
      quaternion = a.quaternion.map((value, i) => value * scaleA + sign * b.quaternion[i] * w);
    }
    return recompose({ translate: lerp(a.translate, b.translate), scale: lerp(a.scale, b.scale), skew: lerp(a.skew, b.skew), perspective: lerp(a.perspective, b.perspective), quaternion });
  }
  const identityOf = fn => fn.type === 'scale' ? { type: 'scale', v: [1, 1, 1], three: fn.three } : fn.type === 'rotate' ? { type: 'rotate', v: [fn.v[0], fn.v[1], fn.v[2], 0], three: fn.three } : fn.type === 'skew' ? { type: 'skew', v: [0, 0] } : fn.type === 'perspective' ? { type: 'perspective', v: [Infinity], three: true } : fn.type === 'matrix' ? { type: 'matrix', m: identity() } : { type: 'translate', v: [0, 0, 0], three: fn.three };
  function sameAxis(a, b) {
    const la = sqrt(a[0] * a[0] + a[1] * a[1] + a[2] * a[2]), lb = sqrt(b[0] * b[0] + b[1] * b[1] + b[2] * b[2]);
    if (!la || !lb) return la === lb;
    return abs(a[0] / la - b[0] / lb) < 1e-9 && abs(a[1] / la - b[1] / lb) < 1e-9 && abs(a[2] / la - b[2] / lb) < 1e-9;
  }
  function interpolateFunction(a, b, p) {
    if (a.type === 'matrix' || b.type === 'matrix' || (a.type === 'rotate' && !sameAxis(a.v, b.v))) return { type: 'matrix', m: interpolateMatrix(functionMatrix(a), functionMatrix(b), p), three: a.three || b.three };
    if (a.type === 'perspective') { const ia = a.v[0] === Infinity ? 0 : 1 / max(a.v[0], 1), ib = b.v[0] === Infinity ? 0 : 1 / max(b.v[0], 1), inverse = ia + (ib - ia) * p; return { type: 'perspective', v: [inverse ? 1 / inverse : Infinity], three: true }; }
    if (a.type === 'rotate') { const axis = sqrt(a.v[0] * a.v[0] + a.v[1] * a.v[1] + a.v[2] * a.v[2]) ? a.v : b.v; return { type: 'rotate', v: [axis[0], axis[1], axis[2], a.v[3] + (b.v[3] - a.v[3]) * p], three: a.three || b.three }; }
    return { type: a.type, v: a.v.map((value, i) => value + (b.v[i] - value) * p), three: a.three || b.three };
  }
  function interpolateTransform(from, to, p) {
    let a = from.list, b = to.list;
    if (!a.length && !b.length) return from;
    if (!a.length) a = b.map(identityOf);
    if (!b.length) b = a.map(identityOf);
    let prefix = 0;
    while (prefix < min(a.length, b.length) && a[prefix].type === b[prefix].type) prefix++;
    const out = [];
    for (let i = 0; i < prefix; i++) push(out, interpolateFunction(a[i], b[i], p));
    if (prefix < max(a.length, b.length)) push(out, { type: 'matrix', m: interpolateMatrix(listMatrix(a.slice(prefix)), listMatrix(b.slice(prefix)), p) });
    return { kind: 'transform', list: out };
  }
  function accumulateFunction(a, b) {
    if (a.type === 'scale') return { type: 'scale', v: a.v.map((value, i) => value + b.v[i] - 1), three: a.three || b.three };
    if (a.type === 'rotate') return sameAxis(a.v, b.v) ? { type: 'rotate', v: [a.v[0], a.v[1], a.v[2], a.v[3] + b.v[3]], three: a.three || b.three } : null;
    if (a.type === 'translate' || a.type === 'skew') return { type: a.type, v: a.v.map((value, i) => value + b.v[i]), three: a.three || b.three };
    return null;
  }

  // Typed value for a property's text; anything else animates discretely.
  function parseValue(property, text, context) {
    if (text === null || text === undefined) return null;
    text = string(text).trim();
    if (property === 'transform') return parseTransform(text) || { kind: 'discrete', text };
    if (COLORS[property]) return parseColor(text, context) || { kind: 'discrete', text };
    if (UNIT_NUMBERS[property] || INTEGERS[property] || NUMBERS[property]) {
      let number = parseNumber(text);
      if (number === null && UNIT_NUMBERS[property] && /%$/.test(text)) { const percent = parseNumber(text.slice(0, -1)); if (percent !== null) number = percent / 100; }
      if (property === 'font-weight' && number === null) number = text === 'normal' ? 400 : text === 'bold' ? 700 : null;
      if (number !== null) return { kind: 'number', value: number };
      if (property === 'line-height') { const length = parseLength(text, context); if (length && length.unit === 'px') return length; }
      return { kind: 'discrete', text };
    }
    if (lengthProperty(property)) return parseLength(text, context) || { kind: 'discrete', text };
    return { kind: 'discrete', text };
  }
  function serializeValue(property, value) {
    switch (value.kind) {
      case 'number': {
        let number = value.value;
        if (UNIT_NUMBERS[property]) number = min(max(number, 0), 1);
        if (INTEGERS[property]) number = number < 0 ? -round(-number) : round(number);
        if (property === 'font-weight') number = min(max(number, 1), 1000);
        if ((property === 'flex-grow' || property === 'flex-shrink') && number < 0) number = 0;
        return format(number);
      }
      case 'length': return format(value.value) + value.unit;
      case 'color': return serializeColor(value);
      case 'transform': return serializeTransform(value);
    }
    return value.text;
  }
  function interpolate(property, from, to, p) {
    if (from.kind === to.kind && from.kind !== 'discrete' && (from.kind !== 'length' || from.unit === to.unit)) {
      if (from.kind === 'number') return { kind: 'number', value: from.value + (to.value - from.value) * p };
      if (from.kind === 'length') return { kind: 'length', value: from.value + (to.value - from.value) * p, unit: from.unit };
      if (from.kind === 'transform') return interpolateTransform(from, to, p);
      // Premultiplied sRGB interpolation.
      const alpha = from.a + (to.a - from.a) * p;
      if (alpha <= 0) return { kind: 'color', r: 0, g: 0, b: 0, a: 0 };
      const mix = key => (from[key] * from.a + (to[key] * to.a - from[key] * from.a) * p) / alpha;
      return { kind: 'color', r: min(max(mix('r'), 0), 255), g: min(max(mix('g'), 0), 255), b: min(max(mix('b'), 0), 255), a: min(alpha, 1) };
    }
    if (property === 'visibility') {
      const fromText = serializeValue(property, from), toText = serializeValue(property, to);
      if (p > 0 && p < 1 && (fromText === 'visible' || toText === 'visible')) return { kind: 'discrete', text: 'visible' };
      return p < 0.5 ? from : to;
    }
    if (property === 'display') {
      const fromText = serializeValue(property, from), toText = serializeValue(property, to);
      if (fromText === 'none' && toText !== 'none') return p <= 0 ? from : to;
      if (toText === 'none' && fromText !== 'none') return p >= 1 ? to : from;
    }
    return p < 0.5 ? from : to;
  }
  function compose(property, underlying, value, operation) {
    if (operation === 'replace' || !underlying || underlying.kind !== value.kind || value.kind === 'discrete') return value;
    if (value.kind === 'number') return { kind: 'number', value: underlying.value + value.value };
    if (value.kind === 'length') return underlying.unit === value.unit ? { kind: 'length', value: underlying.value + value.value, unit: value.unit } : value;
    if (value.kind === 'color') return { kind: 'color', r: min(underlying.r + value.r, 255), g: min(underlying.g + value.g, 255), b: min(underlying.b + value.b, 255), a: min(underlying.a + value.a, 1) };
    if (value.kind === 'transform') {
      if (operation === 'accumulate' && underlying.list.length === value.list.length) {
        const out = [];
        for (let i = 0; i < value.list.length; i++) { const fn = underlying.list[i].type === value.list[i].type ? accumulateFunction(underlying.list[i], value.list[i]) : null; if (!fn) { out.length = 0; break; } push(out, fn); }
        if (out.length || !value.list.length) return { kind: 'transform', list: out };
      }
      return { kind: 'transform', list: underlying.list.concat(value.list) };
    }
    return value;
  }

  // ---- keyframes ----------------------------------------------------------------------------
  const COMPOSITE = ['replace', 'add', 'accumulate'], COMPOSITE_AUTO = ['replace', 'add', 'accumulate', 'auto'];
  const expand = (property, value, emit) => { if (typeof ns.expandCssShorthand === 'function') ns.expandCssShorthand(property, value, emit); };
  // Keyframe offsets: numbers, numeric/percentage strings; timeline-range offsets need a
  // scroll/view timeline and Typed OM, neither of which a document timeline models.
  function offsetValue(value, prefix, dictionaryName) {
    if (value === undefined || value === null) return null;
    if (typeof value === 'number') { if (!isFinite(value)) throw new ErrorType(`${member(prefix, 'offset', dictionaryName)}The provided double value is non-finite.`); return value; }
    const text = string(value).trim();
    if (NUMBER.test(text)) return +text;
    if (/%$/.test(text) && NUMBER.test(text.slice(0, -1))) return +text.slice(0, -1) / 100;
    throw new ErrorType(`${prefix}timeline offset must be of the form [timeline-range-name] <percentage>`);
  }
  function propertyNames(input) {
    const names = [], keys = ownKeys(input);
    for (let i = 0; i < keys.length; i++) {
      const key = keys[i];
      if (typeof key !== 'string') continue;
      const descriptor = getOwnDescriptor(input, key);
      if (!descriptor || !descriptor.enumerable) continue;
      const css = Native('property', key);
      if (css !== null) push(names, { idl: key, css });
    }
    return sortBy(names, name => name.idl);
  }
  // https://drafts.csswg.org/web-animations-1/#process-a-keyframes-argument
  function processKeyframes(input, prefix) {
    if (input === null || input === undefined) return [];
    const method = input[iteratorSymbol];
    const frames = [];
    if (method !== undefined && method !== null) {
      if (typeof method !== 'function') throw new ErrorType(`${prefix}The object must have a callable @@iterator property.`);
      const iterator = apply(method, input, []);
      if (!objectLike(iterator)) throw new ErrorType('Result of the Symbol.iterator method is not an object');
      const next = iterator.next;
      for (;;) {
        const step = apply(next, iterator, []);
        if (!objectLike(step)) throw new ErrorType('Iterator result is not an object');
        if (step.done) break;
        const item = step.value;
        if (item !== undefined && item !== null && !objectLike(item)) throw new ErrorType(`${prefix}Keyframes must be objects, or null or undefined`);
        const source = item === undefined || item === null ? {} : item;
        const composite = source.composite, easingText = source.easing, offset = source.offset;
        const frame = { offset: offsetValue(offset, prefix, 'BaseKeyframe'), easing: easingText === undefined ? 'linear' : string(easingText), composite: composite === undefined ? 'auto' : enumeration(composite, COMPOSITE_AUTO, prefix, 'composite', 'BaseKeyframe', 'CompositeOperationOrAuto'), values: [] };
        const names = propertyNames(source);
        for (let i = 0; i < names.length; i++) push(frame.values, { idl: names[i].idl, css: names[i].css, text: string(source[names[i].idl]) });
        push(frames, frame);
      }
    } else {
      const composite = input.composite, easingText = input.easing, offset = input.offset;
      const listOf = (value, convert) => { if (objectLike(value) && value[iteratorSymbol] !== undefined) { const out = []; for (const item of value) push(out, convert(item)); return { list: true, values: out }; } return { list: false, values: [convert(value)] }; };
      const composites = composite === undefined ? { list: false, values: [] } : listOf(composite, value => enumeration(value, COMPOSITE_AUTO, prefix, 'composite', 'BasePropertyIndexedKeyframe', 'CompositeOperationOrAuto'));
      const easings = easingText === undefined ? { list: false, values: [] } : listOf(easingText, value => string(value));
      const offsets = offset === undefined || offset === null ? { list: false, values: [] } : listOf(offset, value => offsetValue(value, prefix, 'BasePropertyIndexedKeyframe'));
      const names = propertyNames(input), byOffset = [];
      for (let i = 0; i < names.length; i++) {
        const raw = input[names[i].idl];
        const values = objectLike(raw) && raw[iteratorSymbol] !== undefined ? listOf(raw, value => string(value)).values : [string(raw)];
        for (let j = 0; j < values.length; j++) {
          const computed = values.length === 1 ? 1 : j / (values.length - 1);
          let frame = null;
          for (let k = 0; k < byOffset.length; k++) if (byOffset[k].computed === computed) frame = byOffset[k];
          if (!frame) push(byOffset, frame = { computed, offset: null, easing: 'linear', composite: 'auto', values: [] });
          push(frame.values, { idl: names[i].idl, css: names[i].css, text: values[j] });
        }
      }
      sortBy(byOffset, frame => frame.computed);
      for (let i = 0; i < byOffset.length && i < offsets.values.length; i++) byOffset[i].offset = offsets.values[i];
      for (let i = 0; i < byOffset.length && easings.values.length; i++) byOffset[i].easing = easings.values[i % easings.values.length];
      for (let i = 0; i < byOffset.length && composites.values.length; i++) byOffset[i].composite = composites.values[i % composites.values.length];
      for (let i = 0; i < easings.values.length; i++) easing(easings.values[i], prefix);
      for (let i = 0; i < byOffset.length; i++) push(frames, byOffset[i]);
    }
    let previous = -Infinity;
    for (let i = 0; i < frames.length; i++) if (frames[i].offset !== null && (frames[i].offset < 0 || frames[i].offset > 1)) throw new ErrorType(`${prefix}Offsets must be null or in the range [0,1].`);
    for (let i = 0; i < frames.length; i++) if (frames[i].offset !== null) { if (frames[i].offset < previous) throw new ErrorType(`${prefix}Offsets must be monotonically non-decreasing.`); previous = frames[i].offset; }
    for (let i = 0; i < frames.length; i++) { const parsed = easing(frames[i].easing, prefix); frames[i].easing = parsed.text; frames[i].easingFn = parsed.evaluate; }
    // Values Chrome cannot parse for the property are dropped; the rest keep their specified form.
    for (let i = 0; i < frames.length; i++) {
      const kept = [];
      for (let j = 0; j < frames[i].values.length; j++) {
        const entry = frames[i].values[j];
        let text = Native('value', entry.css, entry.text);
        if (text === null) continue;
        if (INTEGERS[entry.css] && !/^([+-]?\d+|auto|inherit|initial|unset|revert|revert-layer)$/.test(text) && !/^var\(/.test(text)) continue;
        if (COLORS[entry.css]) text = specifiedColor(text);
        else if (UNIT_NUMBERS[entry.css] && /%$/.test(text) && NUMBER.test(text.slice(0, -1))) text = format(+text.slice(0, -1) / 100);
        push(kept, { idl: entry.idl, css: entry.css, text });
      }
      frames[i].values = kept;
    }
    computeOffsets(frames);
    return frames;
  }
  function computeOffsets(frames) {
    for (let i = 0; i < frames.length; i++) frames[i].computed = frames[i].offset;
    if (!frames.length) return;
    if (frames.length > 1 && frames[0].computed === null) frames[0].computed = 0;
    if (frames[frames.length - 1].computed === null) frames[frames.length - 1].computed = 1;
    for (let i = 1; i < frames.length; i++) if (frames[i].computed === null) {
      let j = i; while (frames[j].computed === null) j++;
      const from = frames[i - 1].computed, to = frames[j].computed, span = j - i + 1;
      for (let k = i; k < j; k++) frames[k].computed = from + (to - from) * (k - i + 1) / span;
      i = j;
    }
  }
  // Property-specific keyframes per animated longhand; longhands beat shorthands in a frame.
  function propertyFrames(frames) {
    const byProperty = create(null), order = [];
    for (let i = 0; i < frames.length; i++) {
      const frame = frames[i], values = create(null), explicit = create(null);
      for (let j = 0; j < frame.values.length; j++) {
        const entry = frame.values[j];
        let expanded = false;
        expand(entry.css, entry.text, (longhand, value) => { expanded = true; if (!explicit[longhand]) values[longhand] = value; });
        if (!expanded) { values[entry.css] = entry.text; explicit[entry.css] = true; }
      }
      for (const property in values) {
        if (!byProperty[property]) { byProperty[property] = []; push(order, property); }
        push(byProperty[property], { offset: frame.computed, text: values[property], composite: frame.composite, easing: frame.easingFn });
      }
    }
    return { byProperty, order };
  }

  // ---- timing -------------------------------------------------------------------------------
  const FILL = ['none', 'forwards', 'backwards', 'both', 'auto'], DIRECTION = ['normal', 'reverse', 'alternate', 'alternate-reverse'];
  function readTiming(input, prefix, name, optional) {
    const out = create(null);
    const read = key => input === null ? undefined : input[key];
    let value;
    if ((value = read('delay')) !== undefined) out.delay = finite(value, member(prefix, 'delay', name)); else if (!optional) out.delay = 0;
    if ((value = read('direction')) !== undefined) out.direction = enumeration(value, DIRECTION, prefix, 'direction', name, 'PlaybackDirection'); else if (!optional) out.direction = 'normal';
    if ((value = read('duration')) !== undefined) out.duration = typeof value === 'number' ? value : string(value); else if (!optional) out.duration = 'auto';
    if ((value = read('easing')) !== undefined) out.easing = string(value); else if (!optional) out.easing = 'linear';
    if ((value = read('endDelay')) !== undefined) out.endDelay = finite(value, member(prefix, 'endDelay', name)); else if (!optional) out.endDelay = 0;
    if ((value = read('fill')) !== undefined) out.fill = enumeration(value, FILL, prefix, 'fill', name, 'FillMode'); else if (!optional) out.fill = 'auto';
    if ((value = read('iterationStart')) !== undefined) out.iterationStart = finite(value, member(prefix, 'iterationStart', name)); else if (!optional) out.iterationStart = 0;
    if ((value = read('iterations')) !== undefined) out.iterations = +value; else if (!optional) out.iterations = 1;
    return out;
  }
  // Blink's TimingInput order: iterationStart, iterations, duration, then easing.
  function validateTiming(timing, prefix) {
    if (timing.iterationStart !== undefined && timing.iterationStart < 0) throw new ErrorType(`${prefix}iterationStart must be non-negative`);
    if (timing.iterations !== undefined && !(timing.iterations >= 0)) throw new ErrorType(`${prefix}iterationCount must be non-negative`);
    if (timing.duration !== undefined && (typeof timing.duration === 'number' ? !(timing.duration >= 0) : timing.duration !== 'auto')) throw new ErrorType(`${prefix}duration must be non-negative or auto`);
    if (timing.easing !== undefined) { const parsed = easing(timing.easing, prefix); timing.easing = parsed.text; timing.easingFn = parsed.evaluate; }
  }
  // Effect timing state at a local time (web-animations-1 §4.8-4.11).
  function timingAt(timing, local, backwards) {
    const duration = timing.duration === 'auto' ? 0 : timing.duration;
    const active = duration === 0 || timing.iterations === 0 ? 0 : duration * timing.iterations;
    const end = max(timing.delay + active + timing.endDelay, 0);
    const state = { duration, active, end, phase: 'idle', activeTime: null, iteration: null, progress: null };
    if (local === null) return state;
    const beforeActive = max(min(timing.delay, end), 0), activeAfter = max(min(timing.delay + active, end), 0);
    const phase = local < beforeActive || (backwards && local === beforeActive) ? 'before' : local > activeAfter || (!backwards && local === activeAfter) ? 'after' : 'active';
    state.phase = phase;
    const fill = timing.fill === 'auto' ? 'none' : timing.fill;
    let activeTime = null;
    if (phase === 'before') { if (fill === 'backwards' || fill === 'both') activeTime = max(local - timing.delay, 0); }
    else if (phase === 'active') activeTime = local - timing.delay;
    else if (fill === 'forwards' || fill === 'both') activeTime = max(min(local - timing.delay, active), 0);
    state.activeTime = activeTime;
    if (activeTime === null) return state;
    const overall = duration === 0 ? (phase === 'before' ? timing.iterationStart : timing.iterationStart + timing.iterations) : activeTime / duration + timing.iterationStart;
    let simple = overall === Infinity ? timing.iterationStart % 1 : overall % 1;
    if (simple === 0 && (phase === 'active' || phase === 'after') && activeTime === active && timing.iterations !== 0) simple = 1;
    const iteration = phase === 'after' && timing.iterations === Infinity ? Infinity : simple === 1 ? floor(overall) - 1 : floor(overall);
    let d = iteration;
    if (timing.direction === 'alternate-reverse') d += 1;
    const forwards = timing.direction === 'normal' ? true : timing.direction === 'reverse' ? false : d % 2 === 0;
    const directed = forwards ? simple : 1 - simple;
    const before = (phase === 'before' && forwards) || (phase === 'after' && !forwards);
    state.iteration = iteration;
    state.progress = timing.easingFn(directed, before);
    return state;
  }

  // ---- timelines and the host frame clock -------------------------------------------------
  let host = null, lastClock = -Infinity, frameRequested = false;
  const queue = [];
  const clock = () => {
    if (!host) return null;
    const time = +host.now();
    if (time !== time) return null;
    if (time > lastClock) lastClock = time;
    return lastClock;
  };
  const timelineRecord = value => { const record = objectLike(value) ? timelines.get(value) : undefined; if (!record) throw illegal(); return record; };
  const timelineActive = timeline => { if (!timeline || !host) return false; const record = timelines.get(timeline); return record.document === realmDocument && Native('active', record.document); };
  function timelineTime(timeline) {
    if (!timelineActive(timeline)) return null;
    const now = clock();
    return now === null ? null : now - timelines.get(timeline).origin;
  }
  function documentTimeline(document) {
    let timeline = documentTimelines.get(document);
    if (!timeline) { timeline = construct(TimelineDelegate, [], DocumentTimeline); timelines.set(timeline, { document, origin: 0 }); documentTimelines.set(document, timeline); }
    return timeline;
  }
  const defaultTimeline = () => documentTimeline(realmDocument);
  function requestFrame() {
    if (frameRequested || !host) return;
    frameRequested = true;
    host.frame(frameHandler);
  }
  const frameHandler = { __proto__: null, update: frameUpdate, dispatch: frameDispatch, commit: frameCommit };

  // ---- animations ---------------------------------------------------------------------------
  let sequence = 0;
  function deferred(resolved, value) {
    let resolve, reject;
    const promise = new PromiseType((a, b) => { resolve = a; reject = b; });
    const record = { promise, resolve, reject, state: 'pending' };
    if (resolved) { record.state = 'resolved'; resolve(value); }
    return record;
  }
  const settle = (record, value) => { if (record.state !== 'pending') return; record.state = 'resolved'; record.resolve(value); };
  const reject = record => { if (record.state === 'pending') { record.state = 'rejected'; record.reject(abort()); } handled(record.promise); };
  const animationRecord = value => { const record = objectLike(value) ? animations.get(value) : undefined; if (!record) throw illegal(); return record; };
  const effectRecord = value => { const record = objectLike(value) ? effects.get(value) : undefined; if (!record) throw illegal(); return record; };
  const effectiveRate = record => record.pendingRate !== null ? record.pendingRate : record.rate;
  const effectEnd = record => record.effect ? timingAt(effects.get(record.effect).timing, null, false).end : 0;
  const applyPendingRate = record => { if (record.pendingRate !== null) { record.rate = record.pendingRate; record.pendingRate = null; } };
  function currentTime(record, ignoreHold) {
    if (!ignoreHold && record.hold !== null) return record.hold;
    if (record.start === null) return null;
    const time = timelineTime(record.timeline);
    return time === null ? null : (time - record.start) * record.rate;
  }
  function playState(record) {
    const time = currentTime(record);
    if (time === null && record.start === null && record.task === null) return 'idle';
    if (record.task === 'pause' || (record.start === null && record.task !== 'play')) return 'paused';
    const rate = effectiveRate(record);
    if (time !== null && ((rate > 0 && time >= effectEnd(record)) || (rate < 0 && time <= 0))) return 'finished';
    return 'running';
  }
  const effectState = record => record.effect ? timingAt(effects.get(record.effect).timing, currentTime(record), record.rate < 0) : null;
  function relevant(record) {
    if (!record.effect || record.replace === 'removed') return false;
    const state = effectState(record);
    if (state.activeTime !== null) return true;
    if (state.phase === 'active' && playState(record) !== 'finished') return true;
    return (record.rate > 0 && state.phase === 'before') || (record.rate < 0 && state.phase === 'after');
  }
  function track(animation) {
    apply(setAdd, live, [animation]);
    const record = animations.get(animation), effect = record.effect ? effects.get(record.effect) : null;
    if (effect && effect.target) index(animation, null, effect.target);
    schedule();
  }
  function wantsFrame(record) {
    if (!timelineActive(record.timeline)) return false;
    if (record.task !== null) return true;
    if (playState(record) !== 'running') return false;
    return record.rate > 0 ? effectEnd(record) !== Infinity : record.rate < 0;
  }
  function schedule() {
    if (queue.length) return requestFrame();
    for (const animation of apply(setValues, live, [])) if (wantsFrame(animations.get(animation))) return requestFrame();
  }
  function prune() {
    for (const animation of list()) {
      const record = animations.get(animation);
      if (record.task !== null || record.finishTask || playState(record) === 'running' || relevant(record)) continue;
      apply(setDelete, live, [animation]);
      const effect = record.effect ? effects.get(record.effect) : null;
      if (effect && effect.target) index(animation, effect.target, null);
    }
  }
  function playbackEvent(type, current, timeline) {
    const event = construct(EventType, [type, { __proto__: null }], AnimationPlaybackEvent);
    events.set(event, { currentTime: current, timelineTime: timeline });
    return event;
  }
  function queueEvent(animation, record, event, time) {
    if (record.timeline && timelines.get(record.timeline).document) { push(queue, { animation, event, time }); requestFrame(); return; }
    const dispatch = () => Native('dispatch', animation, event);
    if (host) host.task(dispatch); else microtask(dispatch);
  }
  function finishNotify(animation, record) {
    if (playState(record) !== 'finished') return;
    settle(record.finished, animation);
    const time = timelineTime(record.timeline);
    const end = record.rate < 0 ? 0 : effectEnd(record);
    const scheduled = record.start !== null && record.rate ? record.start + end / record.rate : time;
    queueEvent(animation, record, playbackEvent('finish', currentTime(record), time), scheduled);
  }
  // https://drafts.csswg.org/web-animations-1/#update-an-animations-finished-state
  function updateFinished(animation, record, seek, synchronous) {
    const unconstrained = currentTime(record, !seek);
    if (unconstrained !== null && record.start !== null && record.task === null) {
      const end = effectEnd(record);
      if (record.rate > 0 && unconstrained >= end) record.hold = seek ? unconstrained : record.previousTime === null ? end : max(record.previousTime, end);
      else if (record.rate < 0 && unconstrained <= 0) record.hold = seek ? unconstrained : record.previousTime === null ? 0 : min(record.previousTime, 0);
      else if (record.rate !== 0 && timelineActive(record.timeline)) {
        if (seek && record.hold !== null) record.start = timelineTime(record.timeline) - record.hold / record.rate;
        record.hold = null;
      }
    }
    record.previousTime = currentTime(record);
    const finished = playState(record) === 'finished';
    if (finished && record.finished.state === 'pending') {
      if (synchronous) { if (record.finishTask) record.finishTask.cancelled = true; record.finishTask = null; finishNotify(animation, record); }
      else if (!record.finishTask) {
        const task = { cancelled: false };
        record.finishTask = task;
        microtask(() => { if (task.cancelled) return; record.finishTask = null; finishNotify(animation, record); });
      }
    }
    if (!finished && record.finished.state === 'resolved') record.finished = deferred(false);
  }
  function resetPendingTasks(animation, record) {
    if (record.task === null) return;
    record.task = null;
    applyPendingRate(record);
    reject(record.ready);
    record.ready = deferred(true, animation);
  }
  function silentlySetCurrentTime(record, seek, prefix) {
    if (seek === null) { if (currentTime(record) !== null) throw new ErrorType(`${prefix}currentTime may not be changed from resolved to unresolved`); return false; }
    const active = timelineActive(record.timeline);
    if (record.hold !== null || record.start === null || !active || record.rate === 0) record.hold = seek;
    else record.start = timelineTime(record.timeline) - seek / record.rate;
    if (!active) record.start = null;
    record.previousTime = null;
    return true;
  }
  function setCurrentTime(animation, record, seek, prefix) {
    if (!silentlySetCurrentTime(record, seek, prefix)) return;
    if (record.task === 'pause') { record.hold = seek; applyPendingRate(record); record.start = null; record.task = null; settle(record.ready, animation); }
    updateFinished(animation, record, true, false);
  }
  // https://drafts.csswg.org/web-animations-1/#playing-an-animation-section
  function play(animation, record, rewind, prefix) {
    const abortedPause = record.task === 'pause', rate = effectiveRate(record), time = currentTime(record), end = effectEnd(record);
    let pendingReady = false, seek = null;
    if (rewind) {
      if (rate >= 0 && (time === null || time < 0 || time >= end)) seek = 0;
      else if (rate < 0 && (time === null || time <= 0 || time > end)) {
        if (end === Infinity) throw invalidState(prefix, 'Cannot play reversed Animation with infinite target effect end.');
        seek = end;
      }
    }
    if (seek === null && record.start === null && time === null) seek = 0;
    if (seek !== null) record.hold = seek;
    if (record.hold !== null) record.start = null;
    if (record.task !== null) { record.task = null; pendingReady = true; }
    if (record.hold === null && seek === null && !abortedPause && record.pendingRate === null) return;
    if (!pendingReady) record.ready = deferred(false);
    record.task = 'play';
    updateFinished(animation, record, false, false);
  }
  function pause(animation, record, prefix) {
    if (record.task === 'pause' || playState(record) === 'paused') return;
    let seek = null;
    if (currentTime(record) === null) {
      if (effectiveRate(record) >= 0) seek = 0;
      else { if (effectEnd(record) === Infinity) throw invalidState(prefix, 'Cannot play reversed Animation with infinite target effect end.'); seek = effectEnd(record); }
    }
    if (seek !== null) record.hold = seek;
    let pendingReady = false;
    if (record.task === 'play') { record.task = null; pendingReady = true; }
    if (!pendingReady) record.ready = deferred(false);
    record.task = 'pause';
    updateFinished(animation, record, false, false);
  }
  function commitPending(animation, record, ready) {
    const task = record.task;
    record.task = null;
    if (task === 'play') {
      if (record.hold !== null) {
        applyPendingRate(record);
        record.start = record.rate === 0 ? ready : ready - record.hold / record.rate;
        if (record.rate !== 0) record.hold = null;
      } else if (record.start !== null && record.pendingRate !== null) {
        const match = (ready - record.start) * record.rate;
        applyPendingRate(record);
        if (record.rate === 0) record.hold = match;
        record.start = record.rate === 0 ? ready : ready - match / record.rate;
      }
    } else {
      if (record.start !== null && record.hold === null) record.hold = (ready - record.start) * record.rate;
      applyPendingRate(record);
      record.start = null;
    }
    settle(record.ready, animation);
    updateFinished(animation, record, false, false);
  }
  function cancel(animation, record) {
    if (playState(record) !== 'idle') {
      resetPendingTasks(animation, record);
      reject(record.finished);
      record.finished = deferred(false);
      queueEvent(animation, record, playbackEvent('cancel', null, timelineTime(record.timeline)), null);
    }
    record.hold = null;
    record.start = null;
  }
  function setEffect(animation, record, effect) {
    if (effect === record.effect) return;
    if (effect !== null) { const previous = effects.get(effect).animation; if (previous) setEffect(previous, animations.get(previous), null); }
    if (record.effect) { const old = effects.get(record.effect); old.animation = null; index(animation, old.target, null); }
    record.effect = effect;
    if (effect) { const next = effects.get(effect); next.animation = animation; index(animation, null, next.target); }
    updateFinished(animation, record, false, false);
  }
  // Animations by effect target, for getAnimations() and the effect stack.
  function index(animation, from, to) {
    if (from) { const entries = targets.get(from); if (entries) { const at = entries.indexOf(animation); if (at >= 0) entries.splice(at, 1); } }
    if (to) { let entries = targets.get(to); if (!entries) targets.set(to, entries = []); if (entries.indexOf(animation) < 0) push(entries, animation); }
  }
  // https://drafts.csswg.org/web-animations-1/#removing-replaced-animations
  function replaceable(record) {
    return record.replace === 'active' && record.effect && timelines.get(record.timeline || {}) && playState(record) === 'finished' && effects.get(record.effect).target && effectState(record).activeTime !== null;
  }
  function removeReplaced() {
    const candidates = sortBy(list().filter(animation => { const record = animations.get(animation); return record.timeline && replaceable(record); }), animation => animations.get(animation).seq);
    for (let i = 0; i < candidates.length; i++) {
      const record = animations.get(candidates[i]), effect = effects.get(record.effect);
      const properties = effect.frames.order;
      if (!properties.length) continue;
      let covered = true;
      for (let p = 0; p < properties.length && covered; p++) {
        let replaced = false;
        for (let j = i + 1; j < candidates.length && !replaced; j++) {
          const later = effects.get(animations.get(candidates[j]).effect);
          if (later.target !== effect.target || later.pseudo !== effect.pseudo || !later.frames.byProperty[properties[p]]) continue;
          replaced = true;
        }
        covered = replaced;
      }
      if (!covered) continue;
      record.replace = 'removed';
      queueEvent(candidates[i], record, playbackEvent('remove', currentTime(record), timelineTime(record.timeline)), timelineTime(record.timeline));
    }
  }
  function frameUpdate() {
    frameRequested = false;
    const ordered = sortBy(list(), animation => animations.get(animation).seq);
    for (let i = 0; i < ordered.length; i++) { const record = animations.get(ordered[i]); if (timelineActive(record.timeline)) updateFinished(ordered[i], record, false, false); }
    removeReplaced();
  }
  function frameDispatch() {
    const pending = queue.splice(0, queue.length);
    sortBy(pending, entry => entry.time === null ? -Infinity : entry.time);
    for (let i = 0; i < pending.length; i++) Native('dispatch', pending[i].animation, pending[i].event);
  }
  function frameCommit() {
    const ordered = sortBy(list(), animation => animations.get(animation).seq);
    for (let i = 0; i < ordered.length; i++) {
      const record = animations.get(ordered[i]);
      if (record.task !== null && timelineActive(record.timeline)) commitPending(ordered[i], record, timelineTime(record.timeline));
    }
    prune();
    schedule();
  }

  // ---- effect stack and computed style ------------------------------------------------------
  function fontSize(read) { const match = /^([\d.]+)px$/.exec(read('font-size')); return match ? +match[1] : 16; }
  // The animated value of one property on an element through its effect stack.
  function stackValue(stack, property, base, context) {
    let value = base;
    for (let i = 0; i < stack.length; i++) {
      const entry = stack[i], frames = entry.effect.frames.byProperty[property];
      if (!frames) continue;
      value = effectValue(entry, frames, property, value, context);
    }
    return value;
  }
  // https://drafts.csswg.org/web-animations-1/#the-effect-value-of-a-keyframe-animation-effect
  function effectValue(entry, frames, property, underlying, context) {
    const progress = entry.progress;
    if (progress === null) return underlying;
    const effectComposite = entry.effect.composite;
    const keyframe = frame => frame.value || (frame.value = compose(property, underlying, parseValue(property, frame.text, context), frame.composite === 'auto' ? effectComposite : frame.composite));
    const resolved = frames.map(frame => ({ offset: frame.offset, text: frame.text, composite: frame.composite, easing: frame.easing, value: null }));
    if (resolved[0].offset !== 0) resolved.unshift({ offset: 0, easing: x => x, value: underlying });
    if (resolved[resolved.length - 1].offset !== 1) push(resolved, { offset: 1, easing: x => x, value: underlying });
    let start, end;
    const zeros = resolved.filter(frame => frame.offset === 0), ones = resolved.filter(frame => frame.offset === 1);
    if (progress < 0 && zeros.length > 1) return keyframe(zeros[0]);
    if (progress >= 1 && ones.length > 1) return keyframe(ones[ones.length - 1]);
    start = 0;
    if (progress >= 0) for (let i = 0; i < resolved.length; i++) if (resolved[i].offset <= progress && resolved[i].offset < 1) start = i;
    end = min(start + 1, resolved.length - 1);
    const from = resolved[start], to = resolved[end];
    const distance = to.offset === from.offset ? 0 : (progress - from.offset) / (to.offset - from.offset);
    const eased = from.easing(distance, false);
    return interpolate(property, keyframe(from), keyframe(to), eased);
  }
  function effectStack(element, limit) {
    const entries = targets.get(element);
    if (!entries || !entries.length) return null;
    const stack = [];
    for (let i = 0; i < entries.length; i++) {
      const animation = entries[i], record = animations.get(animation);
      if (!record.effect || (record.replace === 'removed' && animation !== limit)) continue;
      const effect = effects.get(record.effect);
      if (effect.target !== element || effect.pseudo !== null) continue;
      if (limit && record.seq > animations.get(limit).seq) continue;
      const state = timingAt(effect.timing, currentTime(record), record.rate < 0);
      if (state.activeTime === null) continue;
      push(stack, { record, effect, progress: state.progress });
    }
    return stack.length ? sortBy(stack, entry => entry.record.seq) : null;
  }
  function stackResult(stack, read, inherited) {
    const properties = [];
    for (let i = 0; i < stack.length; i++) for (let j = 0; j < stack[i].effect.frames.order.length; j++) { const name = stack[i].effect.frames.order[j]; if (properties.indexOf(name) < 0) push(properties, name); }
    const at = properties.indexOf('color');
    if (at > 0) { properties.splice(at, 1); properties.unshift('color'); }
    let current = null;
    const underlyingText = property => { const text = read(property); if (text !== '') return text; if (INHERITED[property]) { const value = inherited(property); if (value) return value; } return initialValue(property); };
    const context = {
      fontSize: fontSize(read),
      currentColor: () => current || parseColor(underlyingText('color'), { currentColor: () => ({ kind: 'color', r: 0, g: 0, b: 0, a: 1 }) }),
    };
    const out = [];
    for (let i = 0; i < properties.length; i++) {
      const property = properties[i];
      const colorContext = property === 'color' ? { fontSize: context.fontSize, currentColor: () => parseColor(inherited('color') || 'rgb(0, 0, 0)', { currentColor: () => null }) } : context;
      const base = parseValue(property, underlyingText(property), colorContext) || { kind: 'discrete', text: '' };
      const value = stackValue(stack, property, base, colorContext);
      if (property === 'color' && value.kind === 'color') current = value;
      push(out, [property, serializeValue(property, value)]);
    }
    return out;
  }
  let suppressStyle = false;
  ns.animatedStyle = function (element, read, inherited) {
    if (suppressStyle) return null;
    const stack = effectStack(element, null);
    return stack ? stackResult(stack, read, inherited) : null;
  };

  // ---- interfaces ---------------------------------------------------------------------------
  const method = (object, name, fn, length) => define(object, name, { value: make(fn, name, length), writable: true, enumerable: true, configurable: true });
  const accessor = (object, name, get, set) => define(object, name, { get: make(get, 'get ' + name, 0), set: set ? make(set, 'set ' + name, 1) : undefined, enumerable: true, configurable: true });
  function finishInterface(constructor, delegate, name, parent, parentPrototype) {
    constructor.prototype = delegate.prototype;
    define(delegate.prototype, 'constructor', { value: constructor, writable: true, configurable: true });
    define(constructor, 'prototype', { writable: false });
    define(delegate.prototype, Symbol.toStringTag, { value: name, configurable: true });
    if (parent) setPrototype(constructor, parent);
    if (parentPrototype) setPrototype(delegate.prototype, parentPrototype);
    define(g, name, { value: constructor, writable: true, configurable: true });
  }
  // Own-key order as Chrome's IDL lists them, constructor and @@toStringTag last.
  function reorder(prototype, names) {
    const all = names.concat(['constructor']);
    for (let i = 0; i < all.length; i++) { const descriptor = getOwnDescriptor(prototype, all[i]); if (!descriptor) continue; delete prototype[all[i]]; define(prototype, all[i], descriptor); }
    const tag = getOwnDescriptor(prototype, Symbol.toStringTag); delete prototype[Symbol.toStringTag]; define(prototype, Symbol.toStringTag, tag);
  }
  const requireNew = (target, name) => { if (!target) throw new ErrorType(`${constructing(name)}Please use the 'new' operator, this DOM object constructor cannot be called as a function.`); };
  const illegalConstructor = name => { const delegate = function () { requireNew(new.target, name); throw new ErrorType(`${constructing(name)}Illegal constructor`); }; return delegate; };

  // AnimationTimeline / DocumentTimeline
  const AnimationTimelineDelegate = illegalConstructor('AnimationTimeline');
  const AnimationTimeline = make(AnimationTimelineDelegate, 'AnimationTimeline', 0, true, true, false);
  finishInterface(AnimationTimeline, AnimationTimelineDelegate, 'AnimationTimeline', null, null);
  accessor(AnimationTimelineDelegate.prototype, 'currentTime', function () { timelineRecord(this); return timelineTime(this); });
  accessor(AnimationTimelineDelegate.prototype, 'duration', function () { timelineRecord(this); return null; });
  const TimelineDelegate = function DocumentTimeline(options) {
    requireNew(new.target, 'DocumentTimeline');
    const prefix = constructing('DocumentTimeline');
    const input = dictionary(options, prefix, 'DocumentTimelineOptions');
    const value = input === null ? undefined : input.originTime;
    const origin = value === undefined ? 0 : finite(value, member(prefix, 'originTime', 'DocumentTimelineOptions'));
    timelines.set(this, { document: realmDocument, origin });
  };
  const DocumentTimeline = make(TimelineDelegate, 'DocumentTimeline', 0, true, true, false);
  finishInterface(DocumentTimeline, TimelineDelegate, 'DocumentTimeline', AnimationTimeline, AnimationTimelineDelegate.prototype);

  // AnimationEffect / KeyframeEffect
  const AnimationEffectDelegate = illegalConstructor('AnimationEffect');
  const AnimationEffect = make(AnimationEffectDelegate, 'AnimationEffect', 0, true, true, false);
  finishInterface(AnimationEffect, AnimationEffectDelegate, 'AnimationEffect', null, null);
  const ownerTouched = effect => { if (effect.animation) { const record = animations.get(effect.animation); updateFinished(effect.animation, record, false, false); track(effect.animation); } };
  method(AnimationEffectDelegate.prototype, 'getTiming', function () {
    const timing = effectRecord(this).timing;
    return { delay: timing.delay, direction: timing.direction, duration: timing.duration, easing: timing.easing, endDelay: timing.endDelay, fill: timing.fill, iterationStart: timing.iterationStart, iterations: timing.iterations };
  }, 0);
  method(AnimationEffectDelegate.prototype, 'getComputedTiming', function () {
    const effect = effectRecord(this), timing = effect.timing;
    const record = effect.animation ? animations.get(effect.animation) : null;
    const local = record ? currentTime(record) : null;
    const state = timingAt(timing, local, record ? record.rate < 0 : false);
    return { delay: timing.delay, direction: timing.direction, duration: state.duration, easing: timing.easing, endDelay: timing.endDelay, fill: timing.fill === 'auto' ? 'none' : timing.fill, iterationStart: timing.iterationStart, iterations: timing.iterations, activeDuration: state.active, currentIteration: state.iteration, endTime: state.end, localTime: local, progress: state.progress };
  }, 0);
  method(AnimationEffectDelegate.prototype, 'updateTiming', function (timing) {
    const effect = effectRecord(this), prefix = executing('updateTiming', 'AnimationEffect');
    const input = dictionary(timing, prefix, 'OptionalEffectTiming');
    const update = readTiming(input, prefix, 'OptionalEffectTiming', true);
    validateTiming(update, prefix);
    for (const key in update) effect.timing[key] = update[key];
    ownerTouched(effect);
  }, 0);
  function readEffectOptions(options, prefix) {
    // (unrestricted double or KeyframeEffectOptions): dictionaries for objects and nullish.
    if (options !== undefined && options !== null && !objectLike(options)) {
      const timing = readTiming(null, prefix, 'EffectTiming', false);
      timing.duration = +options;
      return { timing, composite: 'replace', pseudo: null, input: null };
    }
    const input = options === undefined || options === null ? null : options;
    const timing = readTiming(input, prefix, 'EffectTiming', false);
    let composite = 'replace', pseudo = null, value;
    if (input !== null && (value = input.composite) !== undefined) composite = enumeration(value, COMPOSITE, prefix, 'composite', 'KeyframeEffectOptions', 'CompositeOperation');
    if (input !== null && (value = input.pseudoElement) !== undefined && value !== null) pseudo = string(value);
    return { timing, composite, pseudo, input };
  }
  function pseudoElement(value, prefix) {
    if (value === null) return null;
    const normalized = Native('pseudo', value);
    if (normalized === null) throw new Exception(`${prefix}${value} is a syntactically invalid pseudo-element`, 'SyntaxError');
    return normalized;
  }
  function createEffect(effect, target, keyframes, options, prefix, extra) {
    const parsed = readEffectOptions(options, prefix);
    const more = extra ? extra(parsed.input) : null;
    validateTiming(parsed.timing, prefix);
    const pseudo = pseudoElement(parsed.pseudo, prefix);
    const frames = processKeyframes(keyframes, prefix);
    const record = { target, pseudo, composite: parsed.composite, timing: parsed.timing, keyframes: frames, frames: propertyFrames(frames), animation: null };
    effects.set(effect, record);
    return more;
  }
  const KeyframeEffectDelegate = function KeyframeEffect(target, keyframes, options) {
    requireNew(new.target, 'KeyframeEffect');
    const prefix = constructing('KeyframeEffect');
    if (!arguments.length) throw new ErrorType(`${prefix}1 argument required, but only 0 present.`);
    if (arguments.length === 1) {
      const source = objectLike(target) ? effects.get(target) : undefined;
      if (!source || !(source.frames)) throw new ErrorType(`${prefix}parameter 1 is not of type 'KeyframeEffect'.`);
      const timing = create(null); for (const key in source.timing) timing[key] = source.timing[key];
      effects.set(this, { target: source.target, pseudo: source.pseudo, composite: source.composite, timing, keyframes: source.keyframes, frames: source.frames, animation: null });
      return;
    }
    if (target !== null && target !== undefined && !isElement(target)) throw new ErrorType(`${prefix}parameter 1 is not of type 'Element'.`);
    if (keyframes !== null && keyframes !== undefined && !objectLike(keyframes)) throw new ErrorType(`${prefix}parameter 2 is not of type 'object'.`);
    createEffect(this, target === undefined ? null : target, keyframes, options, prefix);
  };
  const KeyframeEffect = make(KeyframeEffectDelegate, 'KeyframeEffect', 1, true, true, false);
  finishInterface(KeyframeEffect, KeyframeEffectDelegate, 'KeyframeEffect', AnimationEffect, AnimationEffectDelegate.prototype);
  const KP = KeyframeEffectDelegate.prototype;
  accessor(KP, 'target', function () { return effectRecord(this).target; }, function (value) {
    const effect = effectRecord(this);
    if (value !== null && value !== undefined && !isElement(value)) throw new ErrorType(`${setting('target', 'KeyframeEffect')}Failed to convert value to 'Element'.`);
    const next = value === undefined ? null : value;
    if (effect.animation) index(effect.animation, effect.target, next);
    effect.target = next;
    ownerTouched(effect);
  });
  accessor(KP, 'pseudoElement', function () { return effectRecord(this).pseudo; }, function (value) {
    const effect = effectRecord(this);
    effect.pseudo = pseudoElement(value === null ? null : string(value), setting('pseudoElement', 'KeyframeEffect'));
    ownerTouched(effect);
  });
  accessor(KP, 'composite', function () { return effectRecord(this).composite; }, function (value) {
    const effect = effectRecord(this), text = string(value);
    if (COMPOSITE.indexOf(text) >= 0) effect.composite = text;
  });
  method(KP, 'getKeyframes', function () {
    const effect = effectRecord(this), out = [];
    for (let i = 0; i < effect.keyframes.length; i++) {
      const frame = effect.keyframes[i], object = { offset: frame.offset, easing: frame.easing, composite: frame.composite };
      for (let j = 0; j < frame.values.length; j++) object[frame.values[j].idl] = frame.values[j].text;
      object.computedOffset = frame.computed;
      push(out, object);
    }
    return out;
  }, 0);
  method(KP, 'setKeyframes', function (keyframes) {
    const effect = effectRecord(this), prefix = executing('setKeyframes', 'KeyframeEffect');
    if (!arguments.length) throw new ErrorType(`${prefix}1 argument required, but only 0 present.`);
    if (keyframes !== null && keyframes !== undefined && !objectLike(keyframes)) throw new ErrorType(`${prefix}parameter 1 is not of type 'object'.`);
    const frames = processKeyframes(keyframes, prefix);
    effect.keyframes = frames; effect.frames = propertyFrames(frames);
    ownerTouched(effect);
  }, 1);

  // AnimationPlaybackEvent
  const eventRecord = value => { const record = objectLike(value) ? events.get(value) : undefined; if (!record) throw illegal(); return record; };
  const PlaybackDelegate = function AnimationPlaybackEvent(type, init) {
    requireNew(new.target, 'AnimationPlaybackEvent');
    const prefix = constructing('AnimationPlaybackEvent');
    if (!arguments.length) throw new ErrorType(`${prefix}1 argument required, but only 0 present.`);
    const name = string(type);
    const input = dictionary(init, prefix, 'AnimationPlaybackEventInit');
    const read = key => input === null ? undefined : input[key];
    const eventInit = { __proto__: null, bubbles: !!read('bubbles'), cancelable: !!read('cancelable'), composed: !!read('composed') };
    const time = (key) => { const value = read(key); return value === undefined || value === null ? null : finite(value, member(prefix, key, 'AnimationPlaybackEventInit')); };
    const current = time('currentTime'), timeline = time('timelineTime');
    const event = construct(EventType, [name, eventInit], new.target);
    events.set(event, { currentTime: current, timelineTime: timeline });
    return event;
  };
  const AnimationPlaybackEvent = make(PlaybackDelegate, 'AnimationPlaybackEvent', 1, true, true, false);
  finishInterface(AnimationPlaybackEvent, PlaybackDelegate, 'AnimationPlaybackEvent', EventType, EventType.prototype);
  accessor(PlaybackDelegate.prototype, 'currentTime', function () { return eventRecord(this).currentTime; });
  accessor(PlaybackDelegate.prototype, 'timelineTime', function () { return eventRecord(this).timelineTime; });

  // Animation
  function newAnimation(object, effect, timeline) {
    const record = { timeline, effect: null, start: null, hold: null, rate: 1, pendingRate: null, task: null, ready: null, finished: deferred(false), previousTime: null, id: '', replace: 'active', seq: ++sequence, finishTask: null, handlers: create(null) };
    record.ready = deferred(true, object);
    animations.set(object, record);
    if (effect) setEffect(object, record, effect);
    return record;
  }
  const AnimationDelegate = function Animation(effect, timeline) {
    requireNew(new.target, 'Animation');
    const prefix = constructing('Animation');
    if (effect !== undefined && effect !== null && !(objectLike(effect) && effects.has(effect))) throw new ErrorType(`${prefix}parameter 1 is not of type 'AnimationEffect'.`);
    if (timeline !== undefined && timeline !== null && !(objectLike(timeline) && timelines.has(timeline))) throw new ErrorType(`${prefix}parameter 2 is not of type 'AnimationTimeline'.`);
    const object = construct(TargetType, [], new.target);
    newAnimation(object, effect === undefined ? null : effect, timeline === undefined ? defaultTimeline() : timeline);
    return object;
  };
  const Animation = make(AnimationDelegate, 'Animation', 0, true, true, false);
  finishInterface(Animation, AnimationDelegate, 'Animation', TargetType, TargetType.prototype);
  const AP = AnimationDelegate.prototype;
  const mutate = (animation, record, operation) => { const result = operation(); track(animation); return result; };
  accessor(AP, 'effect', function () { return animationRecord(this).effect; }, function (value) {
    const record = animationRecord(this);
    if (value !== null && value !== undefined && !(objectLike(value) && effects.has(value))) throw new ErrorType(`${setting('effect', 'Animation')}Failed to convert value to 'AnimationEffect'.`);
    mutate(this, record, () => setEffect(this, record, value === undefined ? null : value));
  });
  accessor(AP, 'timeline', function () { return animationRecord(this).timeline; }, function (value) {
    const record = animationRecord(this);
    if (value !== null && value !== undefined && !(objectLike(value) && timelines.has(value))) throw new ErrorType(`${setting('timeline', 'Animation')}Failed to convert value to 'AnimationTimeline'.`);
    const next = value === undefined ? null : value;
    if (next === record.timeline) return;
    record.timeline = next;
    if (record.start !== null) record.hold = null;
    mutate(this, record, () => updateFinished(this, record, false, false));
  });
  accessor(AP, 'startTime', function () { return animationRecord(this).start; }, function (value) {
    const record = animationRecord(this), animation = this;
    const start = value === null ? null : finite(value, setting('startTime', 'Animation'));
    mutate(animation, record, () => {
      if (timelineTime(record.timeline) === null && start !== null) record.hold = null;
      const previous = currentTime(record);
      applyPendingRate(record);
      record.start = start;
      if (start !== null) { if (record.rate !== 0) record.hold = null; } else record.hold = previous;
      if (record.task !== null) { record.task = null; settle(record.ready, animation); }
      updateFinished(animation, record, true, false);
    });
  });
  accessor(AP, 'currentTime', function () { return currentTime(animationRecord(this)); }, function (value) {
    const record = animationRecord(this), prefix = setting('currentTime', 'Animation');
    const seek = value === null ? null : finite(value, prefix);
    mutate(this, record, () => setCurrentTime(this, record, seek, prefix));
  });
  accessor(AP, 'playbackRate', function () { return animationRecord(this).rate; }, function (value) {
    const record = animationRecord(this), prefix = setting('playbackRate', 'Animation');
    const rate = finite(value, prefix);
    mutate(this, record, () => {
      record.pendingRate = null;
      const previous = currentTime(record);
      record.rate = rate;
      if (previous !== null) setCurrentTime(this, record, previous, prefix);
    });
  });
  accessor(AP, 'playState', function () { return playState(animationRecord(this)); });
  accessor(AP, 'replaceState', function () { return animationRecord(this).replace; });
  accessor(AP, 'pending', function () { return animationRecord(this).task !== null; });
  accessor(AP, 'id', function () { return animationRecord(this).id; }, function (value) { const record = animationRecord(this); record.id = string(value); });
  for (const type of ['finish', 'cancel', 'remove']) {
    const name = 'on' + type;
    const callback = make(function (event) { const current = animations.get(this).handlers[name]; if (typeof current === 'function') apply(current, this, [event]); }, name, 1);
    accessor(AP, name, function () { const record = animationRecord(this); return record.handlers[name] || null; }, function (value) {
      const record = animationRecord(this), next = objectLike(value) ? value : null, previous = record.handlers[name] || null;
      if (next && !previous) apply(listen, this, [type, callback]);
      if (!next && previous) apply(unlisten, this, [type, callback]);
      record.handlers[name] = next;
    });
  }
  // Promise attributes report a bad receiver as a rejected promise, as Web IDL requires.
  const promised = (receiver, key) => { const record = objectLike(receiver) ? animations.get(receiver) : undefined; return record ? record[key].promise : PromiseType.reject(illegal()); };
  accessor(AP, 'finished', function () { return promised(this, 'finished'); });
  accessor(AP, 'ready', function () { return promised(this, 'ready'); });
  method(AP, 'cancel', function () { const record = animationRecord(this); mutate(this, record, () => cancel(this, record)); }, 0);
  method(AP, 'commitStyles', function () {
    const record = animationRecord(this), prefix = executing('commitStyles', 'Animation');
    const effect = record.effect ? effects.get(record.effect) : null;
    if (!effect || !effect.target || effect.pseudo !== null) throw new Exception(`${prefix}Animation not associated with a styled element`, 'NoModificationAllowedError');
    const element = effect.target;
    if (!rendered(element)) throw new Exception(`${prefix}Target element is not rendered.`, 'InvalidStateError');
    const stack = effectStack(element, this) || [];
    if (record.replace === 'removed' && !stack.some(entry => entry.record === record)) {
      const state = timingAt(effect.timing, currentTime(record), record.rate < 0);
      if (state.activeTime !== null) push(stack, { record, effect, progress: state.progress });
    }
    if (!stack.length) return;
    sortBy(stack, entry => entry.record.seq);
    const base = baseStyle(element);
    const values = stackResult(stack, base.read, base.inherited);
    const style = ns.makeInlineStyleDeclaration(element);
    for (let i = 0; i < values.length; i++) if (effect.frames.byProperty[values[i][0]]) style.setProperty(values[i][0], values[i][1]);
  }, 0);
  method(AP, 'finish', function () {
    const record = animationRecord(this), prefix = executing('finish', 'Animation');
    const rate = effectiveRate(record), end = effectEnd(record);
    if (rate === 0) throw invalidState(prefix, 'Cannot finish Animation with a playbackRate of 0.');
    if (rate > 0 && end === Infinity) throw invalidState(prefix, 'Cannot finish Animation with an infinite target effect end.');
    mutate(this, record, () => {
      applyPendingRate(record);
      const limit = record.rate > 0 ? end : 0;
      silentlySetCurrentTime(record, limit, prefix);
      if (record.start === null && timelineActive(record.timeline)) record.start = timelineTime(record.timeline) - limit / record.rate;
      if (record.task === 'pause' && record.start !== null) { record.hold = null; record.task = null; settle(record.ready, this); }
      if (record.task === 'play' && record.start !== null) { record.task = null; settle(record.ready, this); }
      updateFinished(this, record, true, true);
    });
  }, 0);
  method(AP, 'pause', function () { const record = animationRecord(this); mutate(this, record, () => pause(this, record, executing('pause', 'Animation'))); }, 0);
  method(AP, 'persist', function () { animationRecord(this).replace = 'persisted'; }, 0);
  method(AP, 'play', function () { const record = animationRecord(this); mutate(this, record, () => play(this, record, true, executing('play', 'Animation'))); }, 0);
  method(AP, 'reverse', function () {
    const record = animationRecord(this), prefix = executing('reverse', 'Animation');
    if (!timelineActive(record.timeline)) throw invalidState(prefix, 'Cannot reverse an animation with no active timeline');
    mutate(this, record, () => {
      const original = record.pendingRate;
      record.pendingRate = -effectiveRate(record);
      try { play(this, record, true, prefix); } catch (error) { record.pendingRate = original; throw error; }
    });
  }, 0);
  method(AP, 'updatePlaybackRate', function (rate) {
    const record = animationRecord(this), prefix = executing('updatePlaybackRate', 'Animation');
    if (!arguments.length) throw new ErrorType(`${prefix}1 argument required, but only 0 present.`);
    const value = finite(rate, prefix);
    mutate(this, record, () => {
      const previous = playState(record);
      record.pendingRate = value;
      if (record.task !== null) return;
      if (previous === 'idle' || previous === 'paused' || currentTime(record) === null) applyPendingRate(record);
      else if (previous === 'finished') {
        const unconstrained = currentTime(record, true), time = timelineTime(record.timeline);
        if (unconstrained !== null && time !== null) record.start = value === 0 ? time : time - unconstrained / value;
        applyPendingRate(record);
        updateFinished(this, record, false, false);
      } else play(this, record, false, prefix);
    });
  }, 1);
  accessor(AP, 'overallProgress', function () {
    const record = animationRecord(this), time = currentTime(record);
    if (!record.effect || time === null) return null;
    const end = effectEnd(record);
    if (end === 0) return time < 0 ? 0 : 1;
    return min(max(time / end, 0), 1);
  });
  reorder(AP, ['effect', 'timeline', 'startTime', 'currentTime', 'playbackRate', 'playState', 'replaceState', 'pending', 'id', 'onfinish', 'oncancel', 'onremove', 'finished', 'ready', 'cancel', 'commitStyles', 'finish', 'pause', 'persist', 'play', 'reverse', 'updatePlaybackRate', 'overallProgress']);
  reorder(KP, ['target', 'pseudoElement', 'composite', 'getKeyframes', 'setKeyframes']);
  reorder(AnimationEffectDelegate.prototype, ['getComputedTiming', 'getTiming', 'updateTiming']);
  reorder(AnimationTimelineDelegate.prototype, ['currentTime', 'duration']);
  reorder(PlaybackDelegate.prototype, ['currentTime', 'timelineTime']);

  function rendered(element) {
    if (!apply(connectedGetter, element, [])) return false;
    for (let node = element; node && nodeType(node) === 1; node = apply(parentGetter, node, [])) { suppressStyle = true; try { if (apply(computedStyle, g, [node]).display === 'none') return false; } finally { suppressStyle = false; } }
    return true;
  }
  function baseStyle(element) {
    suppressStyle = true;
    let style, parent;
    try { style = apply(computedStyle, g, [element]); parent = element.parentElement ? apply(computedStyle, g, [element.parentElement]) : null; } finally { suppressStyle = false; }
    const values = new MapType(), parents = new MapType();
    const read = name => { let value = apply(mapGet, values, [name]); if (value === undefined) { suppressStyle = true; try { value = style.getPropertyValue(name); } finally { suppressStyle = false; } apply(mapSet, values, [name, value]); } return value; };
    const inherited = name => { if (!parent) return ''; let value = apply(mapGet, parents, [name]); if (value === undefined) { suppressStyle = true; try { value = parent.getPropertyValue(name); } finally { suppressStyle = false; } apply(mapSet, parents, [name, value]); } return value; };
    return { read, inherited };
  }

  // Element.animate / getAnimations and DocumentOrShadowRoot.getAnimations
  const byComposite = list => sortBy(list, animation => animations.get(animation).seq);
  function collect(test) {
    const out = [];
    for (const animation of apply(setValues, live, [])) {
      const record = animations.get(animation);
      if (!relevant(record)) continue;
      const effect = effects.get(record.effect);
      if (effect.target && test(effect)) push(out, animation);
    }
    return byComposite(out);
  }
  const elementPrototype = g.Element.prototype;
  method(elementPrototype, 'animate', function (keyframes, options) {
    if (!isElement(this)) throw illegal();
    const prefix = executing('animate', 'Element');
    if (!arguments.length) throw new ErrorType(`${prefix}1 argument required, but only 0 present.`);
    if (keyframes !== null && keyframes !== undefined && !objectLike(keyframes)) throw new ErrorType(`${prefix}parameter 1 is not of type 'object'.`);
    const effect = construct(function () {}, [], KeyframeEffect);
    // KeyframeAnimationOptions members convert with the dictionary, before keyframes.
    const animationOptions = createEffect(effect, this, keyframes, options, prefix, input => {
      const result = { id: '', timeline: undefined, hasTimeline: false, range: [] };
      if (input === null) return result;
      let value;
      if ((value = input.id) !== undefined) result.id = string(value);
      if ((value = input.rangeEnd) !== undefined) push(result.range, value);
      if ((value = input.rangeStart) !== undefined) push(result.range, value);
      if ((value = input.timeline) !== undefined) {
        result.hasTimeline = true;
        if (value !== null && !(objectLike(value) && timelines.has(value))) throw new ErrorType(`${member(prefix, 'timeline', 'KeyframeAnimationOptions')}Failed to convert value to 'AnimationTimeline'.`);
        result.timeline = value;
      }
      return result;
    });
    for (let i = 0; i < animationOptions.range.length; i++) rangeValue(animationOptions.range[i], prefix);
    const timeline = animationOptions.hasTimeline ? animationOptions.timeline : documentTimeline(apply(ownerGetter, this, []) || realmDocument);
    const animation = construct(TargetType, [], Animation);
    const record = newAnimation(animation, effect, timeline);
    record.id = animationOptions.id;
    // Blink plays only through a timeline; an explicit null timeline leaves the animation idle.
    if (timeline !== null) play(animation, record, true, prefix);
    track(animation);
    return animation;
  }, 1);
  // Timeline ranges only position scroll and view timelines; their offsets need Typed OM.
  function rangeValue(value, prefix) {
    if (typeof value === 'string' && value.trim().toLowerCase() === 'normal') return;
    if (objectLike(value)) return;
    const text = string(value).trim();
    if (!/^(cover|contain|entry|exit|entry-crossing|exit-crossing|scroll)(\s+[+-]?(?:\d+\.?\d*|\.\d+)(?:%|px))?$/i.test(text)) throw new ErrorType(`${prefix}Animation range must be a name <length-percent> pair`);
  }
  method(elementPrototype, 'getAnimations', function (options) {
    if (!isElement(this)) throw illegal();
    const prefix = executing('getAnimations', 'Element');
    const input = dictionary(options, prefix, 'GetAnimationsOptions');
    const subtree = input !== null && !!input.subtree;
    const element = this;
    if (!apply(connectedGetter, element, [])) return [];
    // Pseudo-element targets count only once the pseudo-element is generated; without a
    // layout tree none is, so subtree queries skip them as Chrome does for content-less ones.
    return collect(effect => effect.pseudo === null && (effect.target === element || (subtree && apply(containsNode, element, [effect.target]))));
  }, 0);
  const root = node => apply(rootOf, node, []);
  accessor(g.Document.prototype, 'timeline', function () { if (nodeType(this) !== 9) throw illegal(); return documentTimeline(this); });
  method(g.Document.prototype, 'getAnimations', function () {
    if (nodeType(this) !== 9) throw illegal();
    const document = this;
    return collect(effect => root(effect.target) === document);
  }, 0);
  if (g.ShadowRoot) method(g.ShadowRoot.prototype, 'getAnimations', function () {
    try { apply(shadowMode.get, this, []); } catch (error) { throw illegal(); }
    const shadow = this;
    return collect(effect => root(effect.target) === shadow);
  }, 0);

  // The embedding's rendering-frame host. A runtime with its own frame scheduler installs it
  // once; otherwise a timer-driven default keeps document timelines advancing.
  const installHost = value => { host = value; lastClock = -Infinity; schedule(); };
  ns.installAnimationHost = value => { delete ns.installAnimationHost; installHost(value); };
  const timer = typeof g.setTimeout === 'function' ? g.setTimeout : null;
  const performance = g.performance, perfNow = performance && typeof performance.now === 'function' ? performance.now : null;
  if (timer && perfNow) {
    let frameTime = null;
    const now = () => apply(perfNow, performance, []);
    host = {
      now: () => frameTime !== null ? frameTime : now(),
      frame(handler) {
        apply(timer, g, [() => {
          frameTime = now(); handler.update();
          apply(timer, g, [() => { handler.dispatch(); handler.commit(); frameTime = null; }, 0]);
        }, 16]);
      },
      task(callback) { apply(timer, g, [callback, 0]); },
    };
  }
})();
