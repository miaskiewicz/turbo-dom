// HTMLInputElement / HTMLTextAreaElement members that need no layout or file host: limited
// integer reflection, IDL-only indeterminate state, list, and Blink-exact stepping and typed
// value conversion. width/height (layout boxes) and files/webkitEntries (FileList) live in
// other modules; showPicker() is activation-gated in user_activation_gates.js; selection
// members and createValueRange (OpaqueRange) live in selection.js.
(function () {
  'use strict';
  const g = globalThis, ns = g[Symbol.for('ts')], native = ns.makeNativeFunction;
  const Native = g.__inputNative; delete g.__inputNative;
  const apply = Reflect.apply, define = Object.defineProperty, describe = Object.getOwnPropertyDescriptor;
  const StringValue = String, NumberValue = Number, DateValue = Date, TypeErrorValue = TypeError, Exception = g.DOMException;
  const isFiniteNumber = Number.isFinite, isNaNValue = Number.isNaN, floor = Math.floor, absolute = Math.abs;
  const dateTime = Date.prototype.getTime, regexTest = RegExp.prototype.test, replaceText = String.prototype.replace;
  const sliceText = String.prototype.slice, padText = String.prototype.padStart, charCode = String.prototype.charCodeAt;
  const splitText = String.prototype.split, joinRows = Array.prototype.join;
  const getAttribute = Element.prototype.getAttribute, setAttribute = Element.prototype.setAttribute, removeAttribute = Element.prototype.removeAttribute;
  const inputPrototype = g.HTMLInputElement.prototype, textareaPrototype = g.HTMLTextAreaElement.prototype;
  const checkInput = ns.domAccessor('nodeType', false, 'HTML:input'), checkTextarea = ns.domAccessor('nodeType', false, 'HTML:textarea');
  const localName = ns.domAccessor('localName', false, 'Element');
  const valueDescriptor = describe(inputPrototype, 'value'), textareaValue = describe(textareaPrototype, 'value');
  const attr = (element, name) => apply(getAttribute, element, [name]);
  const test = (pattern, text) => apply(regexTest, pattern, [text]);
  const pad = (number, width) => apply(padText, StringValue(number), [width, '0']);

  // Chromium's Decimal: an 18-digit coefficient whose alignment, products and quotients
  // truncate exactly as Blink's, so stepping matches Chrome digit for digit.
  const MAX_COEFFICIENT = 999999999999999999n, PRECISION = 18, EXPONENT_MAX = 1023, EXPONENT_MIN = -1023;
  const NORMAL = 0, INFINITE = 1, NOT_A_NUMBER = 2;
  const make = (negative, exponent, coefficient) => {
    if (exponent >= EXPONENT_MIN && exponent <= EXPONENT_MAX) while (coefficient > MAX_COEFFICIENT) { coefficient /= 10n; exponent++; }
    if (exponent > EXPONENT_MAX) return {kind: INFINITE, negative, exponent: 0, coefficient: 0n};
    if (exponent < EXPONENT_MIN) return {kind: NORMAL, negative, exponent: 0, coefficient: 0n};
    return {kind: NORMAL, negative, exponent, coefficient};
  };
  const NAN = {kind: NOT_A_NUMBER, negative: false, exponent: 0, coefficient: 0n};
  const infinity = (negative) => ({kind: INFINITE, negative, exponent: 0, coefficient: 0n});
  const integer = (value) => make(value < 0, 0, BigInt(absolute(value)));
  const finite = (value) => value.kind === NORMAL;
  const zero = (value) => value.kind === NORMAL && value.coefficient === 0n;
  const digits = (value) => { let count = 0; for (let power = 1n; value >= power; power *= 10n) count++; return count; };
  const scaleDown = (value, count) => count > 0 ? value / 10n ** BigInt(count) : value;
  const scaleUp = (value, count) => value * 10n ** BigInt(count);
  const align = (left, right) => {
    let exponent = left.exponent < right.exponent ? left.exponent : right.exponent, a = left.coefficient, b = right.coefficient;
    if (left.exponent > right.exponent) {
      const count = digits(a);
      if (count) {
        const shift = left.exponent - right.exponent, overflow = count + shift - PRECISION;
        if (overflow <= 0) a = scaleUp(a, shift); else { a = scaleUp(a, shift - overflow); b = scaleDown(b, overflow); exponent += overflow; }
      }
    } else if (left.exponent < right.exponent) {
      const count = digits(b);
      if (count) {
        const shift = right.exponent - left.exponent, overflow = count + shift - PRECISION;
        if (overflow <= 0) b = scaleUp(b, shift); else { b = scaleUp(b, shift - overflow); a = scaleDown(a, overflow); exponent += overflow; }
      }
    }
    return [exponent, a, b];
  };
  const special = (left, right) => left.kind === NOT_A_NUMBER || right.kind === NOT_A_NUMBER ? NAN : null;
  const add = (left, right) => {
    const nan = special(left, right); if (nan) return nan;
    if (left.kind === INFINITE && right.kind === INFINITE) return left.negative === right.negative ? left : NAN;
    if (left.kind === INFINITE) return left;
    if (right.kind === INFINITE) return right;
    const [exponent, a, b] = align(left, right), result = left.negative === right.negative ? a + b : a - b;
    if (left.negative && !right.negative && result === 0n) return make(false, exponent, 0n);
    return result >= 0n ? make(left.negative, exponent, result) : make(!left.negative, exponent, -result);
  };
  const subtract = (left, right) => {
    const nan = special(left, right); if (nan) return nan;
    if (left.kind === INFINITE && right.kind === INFINITE) return left.negative === right.negative ? NAN : left;
    if (left.kind === INFINITE) return left;
    if (right.kind === INFINITE) return infinity(!right.negative);
    const [exponent, a, b] = align(left, right), result = left.negative === right.negative ? a - b : a + b;
    if (left.negative && right.negative && result === 0n) return make(false, exponent, 0n);
    return result >= 0n ? make(left.negative, exponent, result) : make(!left.negative, exponent, -result);
  };
  const multiply = (left, right) => {
    const nan = special(left, right); if (nan) return nan;
    const negative = left.negative !== right.negative;
    if (left.kind === INFINITE || right.kind === INFINITE) return zero(left) || zero(right) ? NAN : infinity(negative);
    let exponent = left.exponent + right.exponent, work = left.coefficient * right.coefficient;
    while (work >= 18446744073709551616n) { work /= 10n; exponent++; }
    return make(negative, exponent, work);
  };
  const divide = (left, right) => {
    const nan = special(left, right); if (nan) return nan;
    const negative = left.negative !== right.negative;
    if (left.kind === INFINITE) return right.kind === INFINITE ? NAN : infinity(negative);
    if (right.kind === INFINITE) return make(negative, 0, 0n);
    if (right.coefficient === 0n) return left.coefficient === 0n ? NAN : infinity(negative);
    let exponent = left.exponent - right.exponent;
    if (left.coefficient === 0n) return make(negative, exponent, 0n);
    let remainder = left.coefficient, result = 0n;
    const divisor = right.coefficient;
    for (;;) {
      while (remainder < divisor && result < MAX_COEFFICIENT / 10n) { remainder *= 10n; result *= 10n; exponent--; }
      if (remainder < divisor) break;
      const quotient = remainder / divisor;
      if (result > MAX_COEFFICIENT - quotient) break;
      result += quotient; remainder %= divisor;
      if (!remainder) break;
    }
    if (remainder > divisor / 2n) result++;
    return make(negative, exponent, result);
  };
  // Blink compares through subtraction, so precision loss is shared with Chrome.
  const compare = (left, right) => {
    const result = subtract(left, right);
    if (result.kind === NOT_A_NUMBER) return NaN;
    if (result.kind === INFINITE) return result.negative ? -1 : 1;
    return result.coefficient === 0n ? 0 : result.negative ? -1 : 1;
  };
  const less = (left, right) => compare(left, right) < 0, greater = (left, right) => compare(left, right) > 0;
  const minimumOf = (left, right) => less(right, left) ? right : left, maximumOf = (left, right) => less(left, right) ? right : left;
  const magnitude = (value) => value.kind === NOT_A_NUMBER ? value : {kind: value.kind, negative: false, exponent: value.exponent, coefficient: value.coefficient};
  const negate = (value) => value.kind === NOT_A_NUMBER ? value : {kind: value.kind, negative: !value.negative, exponent: value.exponent, coefficient: value.coefficient};
  const multipleOfTen = (value, count) => !value || value % 10n ** BigInt(count) === 0n;
  const truncate = (value, positiveFraction, negativeFraction, adjust) => {
    if (!finite(value) || value.exponent >= 0) return value;
    const drop = -value.exponent;
    if (digits(value.coefficient) <= drop) return value.negative ? negativeFraction : positiveFraction;
    let result = scaleDown(value.coefficient, drop);
    if (adjust(value) && !multipleOfTen(value.coefficient, drop)) result++;
    return make(value.negative, 0, result);
  };
  const floorOf = (value) => truncate(value, integer(0), integer(-1), (item) => item.negative);
  const ceilOf = (value) => truncate(value, integer(1), integer(0), (item) => !item.negative);
  const roundOf = (value) => {
    if (!finite(value) || value.exponent >= 0) return value;
    const drop = -value.exponent;
    if (digits(value.coefficient) < drop) return integer(0);
    let result = scaleDown(value.coefficient, drop - 1);
    if (result % 10n >= 5n) result += 10n;
    return make(value.negative, 0, result / 10n);
  };
  const remainderOf = (value, divisor) => {
    const quotient = divide(value, divisor);
    return quotient.kind !== NORMAL ? quotient : subtract(value, multiply(quotient.negative ? ceilOf(quotient) : floorOf(quotient), divisor));
  };
  // Decimal::FromString's state machine: digits past the 18th only scale the exponent.
  const parseDecimal = (text) => {
    let state = 'start', negative = false, exponent = 0, exponentNegative = false, count = 0, afterDot = 0, extra = 0, accumulator = 0n;
    for (let index = 0; index < text.length; index++) {
      const code = apply(charCode, text, [index]), digit = code >= 48 && code <= 57, value = BigInt(code - 48);
      if (state === 'digit') {
        if (digit) { if (count < PRECISION) { count++; accumulator = accumulator * 10n + value; } else extra++; continue; }
        if (code === 46) { state = 'dot'; continue; }
        if (code === 69 || code === 101) { state = 'e'; continue; }
        return NAN;
      }
      if (state === 'dot' || state === 'dot-digit') {
        if (digit) { if (count < PRECISION) { count++; afterDot++; accumulator = accumulator * 10n + value; } state = 'dot-digit'; continue; }
        if (code === 69 || code === 101) { state = 'e'; continue; }
        return NAN;
      }
      if (state === 'e' || state === 'e-sign') {
        if (state === 'e' && (code === 43 || code === 45)) { exponentNegative = code === 45; state = 'e-sign'; continue; }
        if (digit) { exponent = code - 48; state = 'e-digit'; continue; }
        return NAN;
      }
      if (state === 'e-digit') {
        if (!digit) return NAN;
        exponent = exponent * 10 + code - 48;
        if (exponent > EXPONENT_MAX + PRECISION) return accumulator ? (exponentNegative ? make(false, 0, 0n) : infinity(negative)) : make(negative, 0, 0n);
        continue;
      }
      if (state === 'zero') {
        if (code === 48) continue;
        if (digit) { accumulator = value; count = 1; state = 'digit'; continue; }
        if (code === 46) { state = 'dot'; continue; }
        if (code === 69 || code === 101) { state = 'e'; continue; }
        return NAN;
      }
      if (digit && code !== 48) { accumulator = value; count = 1; state = 'digit'; continue; }
      if (state === 'start' && (code === 43 || code === 45)) { negative = code === 45; state = 'sign'; continue; }
      if (code === 48) { state = 'zero'; continue; }
      if (code === 46) { state = 'dot'; continue; }
      return NAN;
    }
    if (state === 'zero') return make(negative, 0, 0n);
    if (state !== 'digit' && state !== 'e-digit' && state !== 'dot-digit') return NAN;
    let resultExponent = (exponentNegative ? -exponent : exponent) - afterDot + extra;
    if (resultExponent < EXPONENT_MIN) return make(false, 0, 0n);
    const overflow = resultExponent - EXPONENT_MAX + 1;
    if (overflow > 0) {
      if (overflow + count - afterDot > PRECISION) return infinity(negative);
      resultExponent -= overflow; accumulator = scaleUp(accumulator, overflow);
    }
    return make(negative, resultExponent, accumulator);
  };
  const decimalText = (value) => {
    if (value.kind === INFINITE) return value.negative ? '-Infinity' : 'Infinity';
    if (value.kind === NOT_A_NUMBER) return 'NaN';
    let exponent = value.exponent, coefficient = value.coefficient;
    if (exponent < 0) {
      let last = 0n;
      while (digits(coefficient) > 15) { last = coefficient % 10n; coefficient /= 10n; exponent++; }
      if (last >= 5n) coefficient++;
      while (exponent < 0 && coefficient && coefficient % 10n === 0n) { coefficient /= 10n; exponent++; }
    }
    const text = StringValue(coefficient), sign = value.negative ? '-' : '';
    let length = text.length;
    const adjusted = exponent + length - 1;
    if (exponent <= 0 && adjusted >= -6) {
      if (!exponent) return sign + text;
      if (adjusted >= 0) return sign + apply(sliceText, text, [0, adjusted + 1]) + '.' + apply(sliceText, text, [adjusted + 1]);
      return sign + '0.' + apply(padText, '', [-adjusted - 1, '0']) + text;
    }
    while (length >= 2 && text[length - 1] === '0') length--;
    let result = sign + text[0];
    if (length >= 2) result += '.' + apply(sliceText, text, [1, length]);
    if (adjusted) result += (adjusted < 0 ? 'e' : 'e+') + adjusted;
    return result;
  };
  const fromDouble = (value) => isFiniteNumber(value) ? parseDecimal(StringValue(value)) : isNaNValue(value) ? NAN : infinity(value < 0);
  const toDouble = (value) => finite(value) ? NumberValue(decimalText(value)) : NaN;
  const DOUBLE_MAX = fromDouble(NumberValue.MAX_VALUE);
  // ParseToDecimalForNumberType / ParseToDoubleForNumberType.
  const numberShape = (text) => text !== '' && (text[0] === '-' || text[0] === '.' || text[0] >= '0' && text[0] <= '9') && text[text.length - 1] !== '.';
  const parseNumber = (text, fallback) => {
    if (text === null || !numberShape(text)) return fallback;
    const value = parseDecimal(text);
    if (!finite(value) || less(value, negate(DOUBLE_MAX)) || greater(value, DOUBLE_MAX)) return fallback;
    return zero(value) ? integer(0) : value;
  };
  const parseDouble = (text) => {
    if (!numberShape(text) || !test(/^-?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?$/, text)) return NaN;
    const value = NumberValue(text);
    return isFiniteNumber(value) ? value || 0 : NaN;
  };
  const numberText = (value) => !finite(value) ? '' : zero(value) ? (value.negative ? '-0' : '0') : decimalText(value);

  // DateComponents: proleptic Gregorian calendar bounded to 0001-01-01 .. 275760-09-13.
  const DAY = 86400000, MAX_YEAR = 275760, MAX_MS = 8640000000000000;
  const leap = (year) => year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const monthDays = (year, month) => [31, leap(year) ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][month - 1];
  const daysFromCivil = (year, month, day) => {
    const y = month <= 2 ? year - 1 : year, era = floor(y / 400), yoe = y - era * 400;
    const doy = floor((153 * (month + (month > 2 ? -3 : 9)) + 2) / 5) + day - 1, doe = yoe * 365 + floor(yoe / 4) - floor(yoe / 100) + doy;
    return era * 146097 + doe - 719468;
  };
  const civilFromDays = (days) => {
    const z = days + 719468, era = floor(z / 146097), doe = z - era * 146097;
    const yoe = floor((doe - floor(doe / 1460) + floor(doe / 36524) - floor(doe / 146096)) / 365);
    const doy = doe - (365 * yoe + floor(yoe / 4) - floor(yoe / 100)), mp = floor((5 * doy + 2) / 153);
    const month = mp + (mp < 10 ? 3 : -9);
    return [yoe + era * 400 + (month <= 2 ? 1 : 0), month, doy - floor((153 * mp + 2) / 5) + 1];
  };
  const weekday = (days) => ((days % 7) + 11) % 7;
  const firstWeekOffset = (year) => { let offset = 1 - weekday(daysFromCivil(year, 1, 1)); if (offset <= -4) offset += 7; return offset; };
  const maxWeek = (year) => { const day = weekday(daysFromCivil(year, 1, 1)); return day === 4 || day === 3 && leap(year) ? 53 : 52; };
  const withinDate = (year, month, day, time) => year >= 1 && (year < MAX_YEAR || month < 9 || month === 9 && (day < 13 || day === 13 && !time));
  // C round(): halves away from zero, exact for doubles beyond 2^52.
  const roundHalfAway = (value) => { const whole = floor(value), fraction = value - whole; return fraction > 0.5 || fraction === 0.5 && value > 0 ? whole + 1 : whole; };
  const positiveModulo = (value, divisor) => { const result = value % divisor; return result < 0 ? result + divisor : result; };
  const twoDigits = (text, at) => { const a = apply(charCode, text, [at]) - 48, b = apply(charCode, text, [at + 1]) - 48; return a >= 0 && a <= 9 && b >= 0 && b <= 9 ? a * 10 + b : -1; };
  // Each parser consumes from `at`, returning [components, end] or null.
  const parseYear = (text, at) => {
    let end = at;
    while (end < text.length && text[end] >= '0' && text[end] <= '9') end++;
    if (end - at < 4) return null;
    const year = NumberValue(apply(sliceText, text, [at, end]));
    return year >= 1 && year <= MAX_YEAR ? [year, end] : null;
  };
  const parseMonthAt = (text, at) => {
    const year = parseYear(text, at);
    if (!year || text[year[1]] !== '-') return null;
    const month = twoDigits(text, year[1] + 1);
    if (month < 1 || month > 12 || year[0] === MAX_YEAR && month > 9) return null;
    return [{year: year[0], month}, year[1] + 3];
  };
  const parseDateAt = (text, at) => {
    const month = parseMonthAt(text, at);
    if (!month || text[month[1]] !== '-') return null;
    const day = twoDigits(text, month[1] + 1), parts = month[0];
    if (day < 1 || day > monthDays(parts.year, parts.month) || !withinDate(parts.year, parts.month, day, 0)) return null;
    return [{year: parts.year, month: parts.month, day}, month[1] + 3];
  };
  const parseTimeAt = (text, at) => {
    const hour = twoDigits(text, at), minute = text[at + 2] === ':' ? twoDigits(text, at + 3) : -1;
    if (hour < 0 || hour > 23 || minute < 0 || minute > 59) return null;
    let end = at + 5, second = 0, millisecond = 0;
    if (text[end] === ':') {
      second = twoDigits(text, end + 1);
      if (second < 0 || second > 59) return null;
      end += 3;
      if (text[end] === '.') {
        let stop = end + 1;
        while (stop < text.length && text[stop] >= '0' && text[stop] <= '9') stop++;
        const fraction = stop - end - 1;
        if (fraction > 3) return null;
        if (fraction) { millisecond = NumberValue(apply(sliceText, apply(sliceText, text, [end + 1, stop]) + '00', [0, 3])); end = stop; }
      }
    }
    return [{hour, minute, second, millisecond}, end];
  };
  const whole = (parsed, text) => parsed && parsed[1] === text.length ? parsed[0] : null;
  const timeMs = (time) => ((time.hour * 60 + time.minute) * 60 + time.second) * 1000 + time.millisecond;
  const timeFromMs = (ms) => ({hour: floor(ms / 3600000), minute: floor(ms / 60000) % 60, second: floor(ms / 1000) % 60, millisecond: ms % 1000});
  const timeText = (time, format) => {
    const effective = time.millisecond ? 'millisecond' : format === 'none' && time.second ? 'second' : format;
    let text = pad(time.hour, 2) + ':' + pad(time.minute, 2);
    if (effective !== 'none') text += ':' + pad(time.second, 2);
    if (effective === 'millisecond') text += '.' + pad(time.millisecond, 3);
    return text;
  };
  const dateText = (date) => pad(date.year, 4) + '-' + pad(date.month, 2) + '-' + pad(date.day, 2);
  const dateFromMs = (ms) => { const [year, month, day] = civilFromDays(floor(ms / DAY)); return {year, month, day}; };
  // Per-type DateComponents conversions: string <-> components <-> ms (months for month).
  const TEMPORAL = {
    date: {
      parse: (text) => { const date = whole(parseDateAt(text, 0), text); return date && daysFromCivil(date.year, date.month, date.day) * DAY; },
      fromMs: (ms) => { ms = roundHalfAway(ms); if (ms > MAX_MS) return null; const date = dateFromMs(ms); return withinDate(date.year, date.month, date.day, 0) ? date : null; },
      text: (date) => dateText(date), date: true, step: [1, DAY, 'parsed'], base: 0, limits: [-62135596800000, MAX_MS],
    },
    month: {
      parse: (text) => { const month = whole(parseMonthAt(text, 0), text); return month && (month.year - 1970) * 12 + month.month - 1; },
      fromNumber: (months) => { months = roundHalfAway(months); const month = positiveModulo(months, 12), year = 1970 + (months - month) / 12; return year >= 1 && year <= MAX_YEAR && (year < MAX_YEAR || month < 9) ? {year, month: month + 1} : null; },
      fromMs: (ms) => { ms = roundHalfAway(ms); const date = dateFromMs(ms); return date.year >= 1 && date.year <= MAX_YEAR && (date.year < MAX_YEAR || date.month <= 9) ? {year: date.year, month: date.month} : null; },
      toMs: (months) => { const month = positiveModulo(months, 12); return daysFromCivil(1970 + (months - month) / 12, month + 1, 1) * DAY; },
      text: (month) => pad(month.year, 4) + '-' + pad(month.month, 2), date: true, step: [1, 1, 'parsed'], base: 0, limits: [-23628, 3285488],
    },
    week: {
      parse: (text) => {
        const year = parseYear(text, 0);
        if (!year || text[year[1]] !== '-' || text[year[1] + 1] !== 'W' || year[1] + 4 !== text.length) return null;
        const week = twoDigits(text, year[1] + 2);
        if (week < 1 || week > maxWeek(year[0]) || year[0] === MAX_YEAR && week > 37) return null;
        return (daysFromCivil(year[0], 1, 1) + firstWeekOffset(year[0]) + (week - 1) * 7) * DAY;
      },
      fromMs: (ms) => {
        ms = roundHalfAway(ms);
        const days = floor(ms / DAY);
        let year = civilFromDays(days)[0];
        if (year < 1 || year > MAX_YEAR) return null;
        const yearDay = days - daysFromCivil(year, 1, 1), offset = firstWeekOffset(year);
        let week;
        if (yearDay < offset) { year--; if (year <= 1) return null; week = maxWeek(year); }
        else {
          week = floor((yearDay - offset) / 7) + 1;
          if (week > maxWeek(year)) { year++; week = 1; }
          if (year > MAX_YEAR || year === MAX_YEAR && week > 37) return null;
        }
        return {year, week};
      },
      text: (week) => pad(week.year, 4) + '-W' + pad(week.week, 2), date: true, step: [1, 7 * DAY, 'parsed'], base: -259200000, limits: [-62135596800000, 8639999568000000],
    },
    time: {
      parse: (text) => { const time = whole(parseTimeAt(text, 0), text); return time && timeMs(time); },
      fromMs: (ms) => isFiniteNumber(ms) ? timeFromMs(floor(positiveModulo(ms, DAY))) : null,
      text: (time, format) => timeText(time, format), date: true, step: [60, 1000, 'scaled'], base: 0, limits: [0, 86399999],
    },
    'datetime-local': {
      parse: (text) => {
        const date = parseDateAt(text, 0);
        if (!date || text[date[1]] !== 'T' && text[date[1]] !== ' ') return null;
        const time = whole(parseTimeAt(text, date[1] + 1), text), parts = date[0];
        if (!time || !withinDate(parts.year, parts.month, parts.day, timeMs(time))) return null;
        return daysFromCivil(parts.year, parts.month, parts.day) * DAY + timeMs(time);
      },
      fromMs: (ms) => {
        ms = roundHalfAway(ms);
        const date = dateFromMs(ms), time = timeFromMs(positiveModulo(ms, DAY));
        return withinDate(date.year, date.month, date.day, timeMs(time)) && ms <= MAX_MS ? {date, time} : null;
      },
      text: (value, format) => dateText(value.date) + 'T' + timeText(value.time, format), date: false, step: [60, 1000, 'scaled'], base: 0, limits: [-62135596800000, MAX_MS],
    },
  };
  for (const name in TEMPORAL) { const spec = TEMPORAL[name]; if (!spec.fromNumber) spec.fromNumber = spec.fromMs; if (!spec.toMs) spec.toMs = (ms) => ms; }
  const NUMERIC = {step: [1, 1, 'real'], base: 0};

  const knownTypes = ['hidden', 'text', 'search', 'tel', 'url', 'email', 'password', 'date', 'month', 'week', 'time', 'datetime-local', 'number', 'range', 'color', 'checkbox', 'radio', 'file', 'submit', 'image', 'reset', 'button'];
  const lowerAscii = (text) => apply(replaceText, text, [/[A-Z]/g, (letter) => StringValue.fromCharCode(apply(charCode, letter, [0]) + 32)]);
  const typeOf = (element) => { const type = lowerAscii(attr(element, 'type') || ''); for (let index = 0; index < knownTypes.length; index++) if (knownTypes[index] === type) return type; return 'text'; };
  const valueOf = (element) => apply(valueDescriptor.get, element, []);
  const assignValue = (element, value) => apply(valueDescriptor.set, element, [value]);
  // ParseToNumber for each steppable type; NaN when the string is not a valid value.
  const parseFor = (type, text) => {
    if (text === null) return NAN;
    if (type === 'number' || type === 'range') return parseNumber(text, NAN);
    const parsed = TEMPORAL[type].parse(text);
    return parsed === null ? NAN : fromDouble(parsed);
  };

  // StepRange (CreateStepRange + ParseStep) for one control.
  const stepRange = (element, type) => {
    const spec = TEMPORAL[type] || NUMERIC, read = (name) => parseFor(type, attr(element, name));
    let base = read('min');
    if (!finite(base)) { base = read('value'); if (!finite(base)) base = integer(spec.base); }
    let minimum = read('min'), maximum = read('max');
    if (type === 'range') {
      if (!finite(minimum)) minimum = integer(0);
      if (!finite(maximum)) maximum = integer(100);
      if (less(maximum, minimum)) maximum = minimum;
    } else {
      if (!finite(minimum)) minimum = spec.limits ? integer(spec.limits[0]) : negate(DOUBLE_MAX);
      if (!finite(maximum)) maximum = spec.limits ? integer(spec.limits[1]) : DOUBLE_MAX;
    }
    const [fallback, scale, mode] = spec.step, text = attr(element, 'step');
    let step;
    if (text !== null && text !== '' && lowerAscii(text) === 'any') step = NAN;
    else {
      step = text === null ? NAN : parseNumber(text, NAN);
      if (!finite(step) || !greater(step, integer(0))) step = multiply(integer(fallback), integer(scale));
      else if (mode === 'real') step = multiply(step, integer(scale));
      else if (mode === 'parsed') step = multiply(maximumOf(roundOf(step), integer(1)), integer(scale));
      else step = maximumOf(roundOf(multiply(step, integer(scale))), integer(1));
    }
    return {base, minimum, maximum, step, real: mode === 'real'};
  };
  const TWO_53 = integer(9007199254740992), TWO_24 = integer(16777216), TEN_21 = make(false, 21, 1n);
  const mismatch = (range, value) => {
    if (!finite(range.step) || !finite(value)) return false;
    const distance = magnitude(subtract(value, range.base));
    if (!finite(distance) || greater(divide(distance, TWO_53), range.step)) return false;
    const remainder = magnitude(subtract(distance, multiply(range.step, roundOf(divide(distance, range.step)))));
    const tolerance = range.real ? divide(range.step, TWO_24) : integer(0);
    return less(tolerance, remainder) && less(remainder, subtract(range.step, tolerance));
  };
  const snappedMaximum = (range) => {
    const {base, step} = range;
    if (compare(subtract(base, step), base) === 0 || !finite(divide(base, step))) return NAN;
    let aligned = add(base, multiply(floorOf(divide(subtract(range.maximum, base), step)), step));
    if (greater(aligned, range.maximum)) aligned = subtract(aligned, step);
    return less(aligned, range.minimum) ? NAN : aligned;
  };
  const roundByStep = (range, value) => add(range.base, multiply(roundOf(divide(subtract(value, range.base), range.step)), range.step));
  const clampValue = (range, value) => {
    const inRange = maximumOf(range.minimum, minimumOf(value, range.maximum));
    if (!finite(range.step)) return inRange;
    const rounded = roundByStep(range, inRange);
    const clamped = greater(rounded, range.maximum) ? subtract(rounded, range.step) : less(rounded, range.minimum) ? add(rounded, range.step) : rounded;
    // A step wider than the range leaves the unaligned in-range value.
    return less(clamped, range.minimum) || greater(clamped, range.maximum) ? inRange : clamped;
  };
  // Serialize(Decimal) per type; temporal precision follows the allowed value step.
  const serialize = (element, type, value) => {
    if (type === 'number' || type === 'range') return numberText(value);
    if (!finite(value)) return '';
    return serializeComponents(element, type, TEMPORAL[type].fromNumber(toDouble(value)));
  };
  const serializeComponents = (element, type, components) => {
    if (!components) return '';
    let format = 'none';
    const step = stepRange(element, type).step;
    if (finite(step) && !zero(remainderOf(step, integer(60000)))) format = zero(remainderOf(step, integer(1000))) ? 'second' : 'millisecond';
    return TEMPORAL[type].text(components, format);
  };
  // Chromium's binding-layer exception prefixes.
  const execute = (name, message) => "Failed to execute '" + name + "' on 'HTMLInputElement': " + message;
  const assign = (owner, name, message) => "Failed to set the '" + name + "' property on '" + owner + "': " + message;
  const toNumber = (value, describeFailure) => {
    if (typeof value === 'symbol') throw new TypeErrorValue(describeFailure('Cannot convert a Symbol value to a number'));
    if (typeof value === 'bigint') throw new TypeErrorValue(describeFailure('Cannot convert a BigInt value to a number'));
    return +value;
  };
  const toText = (value, describeFailure) => {
    if (typeof value === 'symbol') throw new TypeErrorValue(describeFailure('Cannot convert a Symbol value to a string'));
    return StringValue(value);
  };
  function applyStep(element, name, count) {
    const type = typeOf(element);
    if (type !== 'number' && type !== 'range' && !TEMPORAL[type]) throw new Exception(execute(name, 'This form element is not steppable.'), 'InvalidStateError');
    const range = stepRange(element, type);
    if (!finite(range.step)) throw new Exception(execute(name, 'This form element does not have an allowed value step.'), 'InvalidStateError');
    let current = parseFor(type, valueOf(element));
    const invalid = !finite(current);
    if (invalid) current = integer(0);
    if (greater(range.minimum, range.maximum)) return;
    const alignedMaximum = snappedMaximum(range);
    if (!finite(alignedMaximum)) return;
    const {base, step} = range;
    let next = current;
    if (mismatch(range, current)) {
      if (count < 0) { next = add(base, multiply(floorOf(divide(subtract(next, base), step)), step)); count++; }
      else if (count > 0) { next = add(base, multiply(ceilOf(divide(subtract(next, base), step)), step)); count--; }
    }
    next = add(next, multiply(step, fromDouble(count)));
    if (!(compare(next, TEN_21) >= 0) && !mismatch(range, current)) next = roundByStep(range, next);
    if (greater(next, range.maximum)) next = alignedMaximum;
    else if (less(next, range.minimum)) next = add(base, multiply(ceilOf(divide(subtract(range.minimum, base), step)), step));
    if (!invalid && (count < 0 && less(current, next) || count > 0 && greater(current, next))) return;
    assignValue(element, serialize(element, type, next));
  }
  // Chromium value sanitization for range and the date/time family, plus the textarea API
  // value's newline normalization. Other types keep the compatibility sanitizer.
  ns.sanitizeControlValue = (element, type, value) => {
    const name = apply(localName, element, []);
    if (name === 'textarea') return apply(replaceText, value, [/\r\n?/g, '\n']);
    if (name !== 'input') return undefined;
    type = typeOf(element);
    // Text-like states strip line breaks; url and email also trim ASCII whitespace,
    // each comma-separated address for email[multiple].
    if (type === 'text' || type === 'search' || type === 'tel' || type === 'password') return apply(replaceText, value, [/[\r\n]/g, '']);
    if (type === 'url' || type === 'email') {
      value = apply(replaceText, value, [/[\r\n]/g, '']);
      const trim = (text) => apply(replaceText, text, [/^[\t\n\f\r ]+|[\t\n\f\r ]+$/g, '']);
      if (type === 'email' && attr(element, 'multiple') !== null) {
        const parts = apply(splitText, value, [',']);
        for (let index = 0; index < parts.length; index++) parts[index] = trim(parts[index]);
        return apply(joinRows, parts, [',']);
      }
      return trim(value);
    }
    if (type === 'number') return isNaNValue(parseDouble(value)) ? '' : value;
    if (type === 'color') return test(/^#[0-9a-fA-F]{6}$/, value) ? lowerAscii(value) : '#000000';
    if (type === 'range') {
      const range = stepRange(element, 'range');
      const fallback = add(range.minimum, divide(subtract(range.maximum, range.minimum), integer(2)));
      return numberText(clampValue(range, parseNumber(value, fallback)));
    }
    const spec = TEMPORAL[type];
    if (!spec) return undefined;
    const parsed = spec.parse(value);
    if (parsed === null) return '';
    return type === 'datetime-local' ? normalizedLocal(spec.fromMs(parsed)) : value;
  };
  // A valid normalized local date and time string: the shortest seconds/fraction form.
  const normalizedLocal = (value) => {
    let text = dateText(value.date) + 'T' + pad(value.time.hour, 2) + ':' + pad(value.time.minute, 2);
    if (value.time.second || value.time.millisecond) text += ':' + pad(value.time.second, 2);
    if (value.time.millisecond) text += '.' + apply(replaceText, pad(value.time.millisecond, 3), [/0+$/, '']);
    return text;
  };

  const accessor = (prototype, check, name, get, set) => define(prototype, name, {
    configurable: true, enumerable: true,
    get: native(function () { apply(check, this, []); return apply(get, this, []); }, 'get ' + name, 0),
    set: set ? native(function (value) { apply(check, this, []); apply(set, this, [value]); }, 'set ' + name, 1) : undefined,
  });
  const method = (prototype, check, name, fn) => define(prototype, name, {
    configurable: true, enumerable: true, writable: true,
    value: native(function () { apply(check, this, []); return apply(fn, this, arguments); }, name, 0),
  });
  const owners = [[inputPrototype, checkInput, 'HTMLInputElement'], [textareaPrototype, checkTextarea, 'HTMLTextAreaElement']];
  const reflectString = (prototype, check, owner, name, attribute) => accessor(prototype, check, name,
    function () { return attr(this, attribute) ?? ''; },
    function (value) { apply(setAttribute, this, [attribute, toText(value, (message) => assign(owner, name, message))]); });
  for (const [name, attribute] of [['align', 'align'], ['alt', 'alt'], ['dirName', 'dirname'], ['useMap', 'usemap']]) reflectString(inputPrototype, checkInput, 'HTMLInputElement', name, attribute);
  for (const [name, attribute] of [['wrap', 'wrap'], ['dirName', 'dirname']]) reflectString(textareaPrototype, checkTextarea, 'HTMLTextAreaElement', name, attribute);
  for (const name of ['incremental', 'webkitdirectory']) accessor(inputPrototype, checkInput, name,
    function () { return attr(this, name) !== null; },
    function (value) { if (value) apply(setAttribute, this, [name, '']); else apply(removeAttribute, this, [name]); });

  // Rules for parsing non-negative integers; null on failure.
  const nonNegative = (text) => {
    if (text === null) return null;
    let index = 0;
    while (index < text.length && test(/[\t\n\f\r ]/, text[index])) index++;
    const negative = text[index] === '-';
    if (negative || text[index] === '+') index++;
    const start = index;
    while (index < text.length && text[index] >= '0' && text[index] <= '9') index++;
    if (index === start) return null;
    const value = NumberValue(apply(sliceText, text, [start, index]));
    return negative && value !== 0 ? null : value;
  };
  for (const [prototype, check, owner] of owners) {
    for (const [name, attribute] of [['maxLength', 'maxlength'], ['minLength', 'minlength']]) accessor(prototype, check, name,
      function () { const value = nonNegative(attr(this, attribute)); return value === null || value > 2147483647 ? -1 : value; },
      function (value) {
        value = toNumber(value, (message) => assign(owner, name, message)) | 0;
        if (value < 0) throw new Exception(assign(owner, name, 'The value provided (' + value + ') is not positive or 0.'), 'IndexSizeError');
        apply(setAttribute, this, [attribute, StringValue(value)]);
      });
  }
  // Unsigned integers limited to positive numbers, with Chrome's default fallbacks.
  const positive = (prototype, check, owner, name, fallback, throwsOnZero) => accessor(prototype, check, name,
    function () { const value = nonNegative(attr(this, name)); return value === null || value < 1 || value > 2147483647 ? fallback : value; },
    function (value) {
      value = toNumber(value, (message) => assign(owner, name, message)) >>> 0;
      if (!value && throwsOnZero) throw new Exception(assign(owner, name, 'The value provided is 0, which is an invalid size.'), 'IndexSizeError');
      apply(setAttribute, this, [name, StringValue(value >= 1 && value <= 2147483647 ? value : fallback)]);
    });
  positive(inputPrototype, checkInput, 'HTMLInputElement', 'size', 20, true);
  positive(textareaPrototype, checkTextarea, 'HTMLTextAreaElement', 'cols', 20, false);
  positive(textareaPrototype, checkTextarea, 'HTMLTextAreaElement', 'rows', 2, false);
  accessor(textareaPrototype, checkTextarea, 'type', function () { return 'textarea'; });
  accessor(textareaPrototype, checkTextarea, 'textLength', function () { return apply(textareaValue.get, this, []).length; });

  accessor(inputPrototype, checkInput, 'indeterminate', function () { return Native('indeterminate', this); }, function (value) { Native('set-indeterminate', this, !!value); });
  // Types whose list attribute does not apply never expose a suggestions source.
  const listless = ['hidden', 'password', 'checkbox', 'radio', 'file', 'submit', 'image', 'reset', 'button'];
  accessor(inputPrototype, checkInput, 'list', function () {
    const type = typeOf(this);
    for (let index = 0; index < listless.length; index++) if (listless[index] === type) return null;
    return Native('list', this);
  });
  for (const [name, sign] of [['stepUp', 1], ['stepDown', -1]]) method(inputPrototype, checkInput, name, function (count) {
    count = count === undefined ? 1 : toNumber(count, (message) => execute(name, message)) | 0;
    applyStep(this, name, sign * count);
  });
  accessor(inputPrototype, checkInput, 'valueAsNumber', function () {
    const type = typeOf(this);
    if (type === 'number' || type === 'range') return parseDouble(valueOf(this));
    const spec = TEMPORAL[type];
    if (!spec) return NaN;
    const parsed = spec.parse(valueOf(this));
    return parsed === null ? NaN : parsed;
  }, function (value) {
    const describeFailure = (message) => assign('HTMLInputElement', 'valueAsNumber', message);
    value = toNumber(value, describeFailure);
    if (value === Infinity || value === -Infinity) throw new TypeErrorValue(describeFailure('The value provided is infinite.'));
    const type = typeOf(this);
    if (type === 'number') assignValue(this, isNaNValue(value) ? '' : StringValue(value));
    else if (type === 'range') assignValue(this, numberText(fromDouble(value)));
    else if (TEMPORAL[type]) assignValue(this, isNaNValue(value) ? '' : serializeComponents(this, type, TEMPORAL[type].fromNumber(value)));
    else throw new Exception(describeFailure('This input element does not support Number values.'), 'InvalidStateError');
  });
  accessor(inputPrototype, checkInput, 'valueAsDate', function () {
    const spec = TEMPORAL[typeOf(this)];
    if (!spec || !spec.date) return null;
    const parsed = spec.parse(valueOf(this));
    return parsed === null ? null : new DateValue(spec.toMs(parsed));
  }, function (value) {
    const describeFailure = (message) => assign('HTMLInputElement', 'valueAsDate', message);
    if (value !== null && value !== undefined && typeof value !== 'object' && typeof value !== 'function') throw new TypeErrorValue(describeFailure("Failed to convert value to 'object'."));
    let time = null;
    if (value !== null && value !== undefined) {
      try { time = apply(dateTime, value, []); } catch (_) { throw new TypeErrorValue(describeFailure('The provided value is not a Date.')); }
    }
    const type = typeOf(this), spec = TEMPORAL[type];
    if (!spec || !spec.date) throw new Exception(describeFailure('This input element does not support Date values.'), 'InvalidStateError');
    assignValue(this, time === null || isNaNValue(time) ? '' : serializeComponents(this, type, spec.fromMs(time)));
  });

  // Blink's StepRange facts for the private form-state modules: temporal validity flags and
  // :in-range/:out-of-range. null for a type that is not steppable. Range limitations are the
  // range type's defaults or a parseable min/max; only time supports a reversed range.
  const internals = g.__captureFormInternals;
  if (internals) {
    internals.indeterminate = (element) => Native('indeterminate', element);
    internals.rangeState = (element) => {
      const type = typeOf(element);
      if (type !== 'number' && type !== 'range' && !TEMPORAL[type]) return null;
      const range = stepRange(element, type), value = parseFor(type, valueOf(element));
      const hasMinimum = finite(parseFor(type, attr(element, 'min'))), hasMaximum = finite(parseFor(type, attr(element, 'max')));
      const reversed = type === 'time' && hasMinimum && hasMaximum && less(range.maximum, range.minimum);
      const below = finite(value) && less(value, range.minimum), above = finite(value) && greater(value, range.maximum);
      return {
        empty: valueOf(element) === '', finite: finite(value), limited: type === 'range' || hasMinimum || hasMaximum,
        underflow: reversed ? below && above : below, overflow: reversed ? below && above : above,
        mismatch: mismatch(range, value),
      };
    };
  }
})();
