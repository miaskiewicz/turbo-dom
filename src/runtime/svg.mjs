// Minimal SVG DOM-property wrappers. SVG IDL attributes are not plain strings/numbers
// like HTML reflected attributes — they're SVGAnimated{String,Length,Rect} objects with
// .baseVal / .animVal. Component code (and the libraries that render SVG icons) reads
// `el.className.baseVal`, `rect.width.baseVal.value`, `svg.viewBox.baseVal`, etc.; without
// these wrappers those reads throw "Cannot read properties of undefined (reading 'baseVal')".
//
// HONEST + LIVE: every wrapper reads/writes the underlying attribute on demand (no cached
// snapshot), so it always reflects the current attribute and never invents a value. There's
// no animation engine — animVal === baseVal. Only attached to real SVGElement instances
// (see dom.mjs), so HTML elements are completely unaffected.

// SVGLength — the unit-bearing value behind an SVGAnimatedLength. valueOf() returns the
// number so `rect.width.baseVal` used in arithmetic/comparisons coerces correctly, while
// `rect.width.baseVal.value` reads the same number explicitly.
export class SVGLength {
  constructor(host, attr) { this.__host = host; this.__attr = attr; }
  get value() { const v = parseFloat(this.__host.getAttribute(this.__attr)); return Number.isNaN(v) ? 0 : v; }
  set value(v) { this.__host.setAttribute(this.__attr, String(v)); }
  get valueInSpecifiedUnits() { return this.value; }
  get unitType() { return 1; } // SVG_LENGTHTYPE_NUMBER
  get valueAsString() { return this.__host.getAttribute(this.__attr) || '0'; }
  set valueAsString(s) { this.__host.setAttribute(this.__attr, String(s)); }
  valueOf() { return this.value; }
  toString() { return this.valueAsString; }
}

export class SVGAnimatedLength {
  constructor(host, attr) {
    const base = new SVGLength(host, attr);
    this.baseVal = base;
    this.animVal = base; // no animation engine — animated value tracks the base value
  }
}

export class SVGAnimatedString {
  constructor(host, attr) { this.__host = host; this.__attr = attr; }
  get baseVal() { return this.__host.getAttribute(this.__attr) || ''; }
  set baseVal(v) { this.__host.setAttribute(this.__attr, String(v)); }
  get animVal() { return this.baseVal; }
}

// SVGRect / SVGAnimatedRect — backs `viewBox`. Parses the four-number list lazily.
export class SVGRect {
  constructor(host, attr) { this.__host = host; this.__attr = attr; }
  #nums() {
    const raw = this.__host.getAttribute(this.__attr) || '';
    const parts = raw.split(/[\s,]+/).map(parseFloat);
    return [parts[0] || 0, parts[1] || 0, parts[2] || 0, parts[3] || 0];
  }
  get x() { return this.#nums()[0]; }
  get y() { return this.#nums()[1]; }
  get width() { return this.#nums()[2]; }
  get height() { return this.#nums()[3]; }
}

export class SVGAnimatedRect {
  constructor(host, attr) {
    const base = new SVGRect(host, attr);
    this.baseVal = base;
    this.animVal = base;
  }
}

// Attributes that surface as SVGAnimatedLength on SVG elements. Covers the common
// geometry set (shapes, gradients, markers); any not listed still works via getAttribute.
export const SVG_LENGTH_ATTRS = [
  'width', 'height', 'x', 'y', 'cx', 'cy', 'r', 'rx', 'ry',
  'x1', 'y1', 'x2', 'y2', 'dx', 'dy', 'fx', 'fy', 'refX', 'refY',
  'markerWidth', 'markerHeight', 'startOffset', 'textLength',
];
