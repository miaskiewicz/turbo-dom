// Geometry Interfaces stores coordinates independently of own properties on the wrapper.
const coordinates = new WeakMap();
const mutableCoordinates = new WeakMap();

function fromDictionary(Type, init) {
  if (init == null) return new Type();
  if (typeof init !== 'object' && typeof init !== 'function') throw new TypeError('Expected a DOMRectInit dictionary');
  const number = key => { const value = init[key]; return value === undefined ? 0 : +value; };
  const height = number('height'), width = number('width'), x = number('x'), y = number('y');
  return new Type(x, y, width, height);
}

class Coordinates {
  constructor(x, y, width, height) {
    this.x = +x; this.y = +y; this.width = +width; this.height = +height;
  }
  get left() { return Math.min(this.x, this.x + this.width); }
  get right() { return Math.max(this.x, this.x + this.width); }
  get top() { return Math.min(this.y, this.y + this.height); }
  get bottom() { return Math.max(this.y, this.y + this.height); }
}

export class DOMRectReadOnly {
  constructor(x = 0, y = 0, width = 0, height = 0) {
    coordinates.set(this, new Coordinates(x, y, width, height));
  }
  static fromRect(init = {}) { return fromDictionary(DOMRectReadOnly, init); }
  toJSON() {
    const { x, y, width, height, top, right, bottom, left } = coordinates.get(this);
    return { x, y, width, height, top, right, bottom, left };
  }
}

export class DOMRect extends DOMRectReadOnly {
  constructor(x = 0, y = 0, width = 0, height = 0) {
    super(x, y, width, height);
    mutableCoordinates.set(this, coordinates.get(this));
  }
  static fromRect(init = {}) { return fromDictionary(DOMRect, init); }
}

for (const key of ['x', 'y', 'width', 'height', 'top', 'right', 'bottom', 'left']) {
  Object.defineProperty(DOMRectReadOnly.prototype, key, {
    configurable: true, enumerable: true,
    get() { return coordinates.get(this)[key]; },
  });
}
for (const key of ['x', 'y', 'width', 'height']) {
  Object.defineProperty(DOMRect.prototype, key, {
    configurable: true, enumerable: true,
    get() { return mutableCoordinates.get(this)[key]; },
    set(value) { mutableCoordinates.get(this)[key] = +value; },
  });
}
for (const Type of [DOMRectReadOnly, DOMRect]) {
  Object.defineProperty(Type.prototype, Symbol.toStringTag, { configurable: true, value: Type.name });
  Object.defineProperty(Type, 'fromRect', { enumerable: true });
}
Object.defineProperty(DOMRectReadOnly.prototype, 'toJSON', { enumerable: true });
