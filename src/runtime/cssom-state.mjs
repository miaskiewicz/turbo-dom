// CSS store metadata only: no parsing, cascade policy or mirrored DOM.
import { noteCssSourceChange } from './css-invalidation.mjs';

function weakTable() {
  const map = new WeakMap();
  return { get: map.get.bind(map), set: map.set.bind(map) };
}
const sheets = weakTable(),
  sheetTargets = weakTable();
const rules = weakTable(),
  ruleTargets = weakTable();
const arrays = weakTable(),
  arrayTargets = weakTable();
const descriptorOf = Object.getOwnPropertyDescriptor;
const reflectGet = Reflect.get;
const reflectSet = Reflect.set;
const reflectDefine = Reflect.defineProperty;
const reflectDelete = Reflect.deleteProperty;
const reflectPrototype = Reflect.setPrototypeOf;
const extensible = Object.isExtensible;
const prototypeOf = Object.getPrototypeOf;
const ARRAY = Array.prototype;
const OBJECT = Object.prototype;
const sameValue = Object.is;
const freezeCss = Object.freeze;
const ProxyCtor = Proxy;
const integer = Number.isInteger;
const invokeSplice = Function.prototype.call.bind(ARRAY.splice);
const splice = ARRAY.splice;
const map = ARRAY.map;
const ArrayCtor = Array;
const species = descriptorOf(ArrayCtor, Symbol.species).get;
const Weak = typeof WeakRef === 'function' ? WeakRef : null;
const deref = Weak ? Function.prototype.call.bind(Weak.prototype.deref) : null;
const UNKNOWN = Symbol('unknown CSS store field');
const references = weakTable();
const pendingRules = weakTable();

function data(object, name) {
  const descriptor = descriptorOf(object, name);
  return descriptor && 'value' in descriptor ? descriptor.value : UNKNOWN;
}
function sameDescriptor(before, after) {
  if (before === undefined || after === undefined) return before === after;
  return (
    sameValue(before.value, after.value) &&
    before.get === after.get &&
    before.set === after.set &&
    before.writable === after.writable &&
    before.enumerable === after.enumerable &&
    before.configurable === after.configurable
  );
}
function accessor(descriptor) {
  return descriptor !== undefined && !('value' in descriptor);
}
function indexKey(name) {
  if (typeof name !== 'string') return false;
  const index = +name;
  return integer(index) && index >= 0 && index < 4294967295 && `${index}` === name;
}
function reference(value) {
  if (!Weak || value === null || (typeof value !== 'object' && typeof value !== 'function')) return null;
  let ref = references.get(value);
  if (!ref) {
    ref = new Weak(value);
    references.set(value, ref);
  }
  return ref;
}
function read(reference) {
  return reference ? deref(reference) : undefined;
}
function mutable(value) {
  return value !== null && (typeof value === 'object' || typeof value === 'function');
}
function primitive(value) {
  return !mutable(value);
}
function primitiveArguments(first, second) {
  return primitive(first) && primitive(second);
}

function flush(state) {
  if (state.depth === 0) state.dirty = false;
}
function changed(state) {
  state.dirty = true;
  state.snapshot = null;
  if (!state.initializing) noteCssSourceChange();
  flush(state);
}

function begin(state) {
  if (state) state.depth++;
}
export function endCssMutation(state) {
  if (state) {
    state.depth--;
    flush(state);
  }
}
function unknown(state) {
  if (!state.unknown) {
    state.unknown = true;
    changed(state);
  }
}

function observedRule(state) {
  return !!state && state.trusted && !state.initializing && typeof data(state.target, 'cssText') === 'string' && !!Weak;
}
function adoptRule(value, owner) {
  if (pendingRules.get(value)) return;
  const state = rules.get(value);
  if (!observedRule(state)) {
    unknown(owner);
    return;
  }
  const first = read(state.owner);
  if (first && first !== owner) {
    state.shared = true;
    unknown(first);
  }
  if (state.shared) {
    unknown(owner);
    return;
  }
  if (!first) state.owner = reference(owner);
}
function ruleChanged(state) {
  if (typeof data(state.target, 'cssText') !== 'string') state.trusted = false;
  const owner = read(state.owner);
  if (!state.initializing || owner) noteCssSourceChange();
  if (owner) {
    if (!state.trusted) unknown(owner);
    changed(owner);
  }
}
function finishRule(state) {
  state.depth--;
  if (state.depth === 0 && state.dirty) {
    state.dirty = false;
    ruleChanged(state);
  }
}
function dirtyRule(state) {
  state.dirty = true;
  if (state.depth === 0) {
    state.dirty = false;
    ruleChanged(state);
  }
}

const ruleHandler = {
  set(target, name, value, receiver) {
    const state = ruleTargets.get(target);
    if (state.initializing) return reflectSet(target, name, value, receiver);
    state.depth++;
    const before = descriptorOf(target, name);
    try {
      const ok = reflectSet(target, name, value, receiver);
      if (ok && name === 'cssText' && (accessor(before) || !sameDescriptor(before, descriptorOf(target, name))))
        dirtyRule(state);
      return ok;
    } finally {
      finishRule(state);
    }
  },
  defineProperty(target, name, descriptor) {
    const state = ruleTargets.get(target);
    if (state.initializing) return reflectDefine(target, name, descriptor);
    const before = descriptorOf(target, name);
    const ok = reflectDefine(target, name, descriptor);
    if (ok && name === 'cssText' && !sameDescriptor(before, descriptorOf(target, name)))
      dirtyRule(ruleTargets.get(target));
    return ok;
  },
  deleteProperty(target, name) {
    const before = descriptorOf(target, name);
    const ok = reflectDelete(target, name);
    if (ok && name === 'cssText' && before !== undefined) dirtyRule(ruleTargets.get(target));
    return ok;
  },
  setPrototypeOf(target, prototype) {
    const statePrototype = prototypeOf(target);
    const ok = reflectPrototype(target, prototype);
    if (ok && prototype !== statePrototype) {
      const state = ruleTargets.get(target);
      state.trusted = false;
      dirtyRule(state);
    }
    return ok;
  },
};
export function wrapCssRule(target) {
  const proxy = new ProxyCtor(target, ruleHandler);
  const state = { target, trusted: true, owner: null, shared: false, depth: 0, dirty: false, initializing: true };
  rules.set(proxy, state);
  ruleTargets.set(target, state);
  return proxy;
}

export function finishCssRuleConstruction(proxy) {
  rules.get(proxy).initializing = false;
}
export function finishCssSheetConstruction(proxy) {
  const state = sheets.get(proxy);
  state.initializing = false;
  state.snapshot = null;
}

function arrayOwner(state) {
  return state.active ? read(state.owner) : null;
}
function arrayEntry(state, owner, name) {
  if (name !== 'length' && !indexKey(name)) return;
  const descriptor = descriptorOf(state.target, name);
  if (!descriptor) return;
  if (!descriptor.writable || (name !== 'length' && !descriptor.configurable)) state.canonicalWritable = false;
  if (name === 'length') return;
  if (!('value' in descriptor)) loseArrayTrust(state, owner);
  else adoptRule(descriptor.value, owner);
}
function holeAfterEdit(state, name, beforeLength, deleted) {
  if (name === 'length') return state.target.length > beforeLength;
  if (!indexKey(name)) return false;
  return +name > beforeLength || (deleted && +name < state.target.length);
}
function arrayChanged(state, name, beforeLength, deleted = false) {
  const owner = arrayOwner(state);
  if (!owner) return;
  arrayEntry(state, owner, name);
  if (holeAfterEdit(state, name, beforeLength, deleted)) {
    if (owner.canonicalDepth) owner.pendingHole = true;
    else loseArrayTrust(state, owner);
  }
  changed(owner);
}

function loseArrayTrust(state, owner) {
  state.trusted = false;
  unknown(owner);
}

function exposedRule(value) {
  let proxy = pendingRules.get(value);
  if (!proxy) return value;
  if (proxy === true) {
    proxy = wrapCssRule(value);
    finishCssRuleConstruction(proxy);
    pendingRules.set(value, proxy);
  }
  return proxy;
}
function exposeArrayRule(state, name, value) {
  if (!indexKey(name) || !pendingRules.get(value)) return value;
  const property = descriptorOf(state.target, name);
  if (!property || property.value !== value) return value;
  const proxy = exposedRule(value);
  reflectDefine(state.target, name, { ...property, value: proxy });
  const owner = arrayOwner(state);
  if (owner) adoptRule(proxy, owner);
  return proxy;
}
// Only the CSSOM constructor's unexposed records use this mechanical marker.
export function privateCssRule(target) {
  pendingRules.set(target, true);
  return target;
}
const arrayHandler = {
  get(target, name, receiver) {
    if (name === 'length') return target.length;
    return exposeArrayRule(arrayTargets.get(target), name, reflectGet(target, name, receiver));
  },
  getOwnPropertyDescriptor(target, name) {
    const property = descriptorOf(target, name);
    if (property && 'value' in property) {
      const value = exposeArrayRule(arrayTargets.get(target), name, property.value);
      return { ...property, value };
    }
    return property;
  },
  set(target, name, value, receiver) {
    const state = arrayTargets.get(target),
      owner = arrayOwner(state);
    const before = descriptorOf(target, name),
      length = target.length;
    begin(owner);
    try {
      const ok = reflectSet(target, name, value, receiver);
      if (ok && (accessor(before) || !sameDescriptor(before, descriptorOf(target, name))))
        arrayChanged(state, name, length);
      return ok;
    } finally {
      endCssMutation(owner);
    }
  },
  defineProperty(target, name, descriptor) {
    exposeArrayRule(arrayTargets.get(target), name, data(target, name));
    const before = descriptorOf(target, name),
      length = target.length;
    const ok = reflectDefine(target, name, descriptor);
    if (ok && !sameDescriptor(before, descriptorOf(target, name))) arrayChanged(arrayTargets.get(target), name, length);
    return ok;
  },
  deleteProperty(target, name) {
    const before = descriptorOf(target, name),
      length = target.length;
    const ok = reflectDelete(target, name);
    if (ok && before !== undefined) arrayChanged(arrayTargets.get(target), name, length, true);
    return ok;
  },
  setPrototypeOf(target, prototype) {
    const before = prototypeOf(target),
      ok = reflectPrototype(target, prototype);
    if (ok && before !== prototype) {
      const state = arrayTargets.get(target);
      state.trusted = false;
      const owner = arrayOwner(state);
      if (owner) {
        unknown(owner);
        changed(owner);
      }
    }
    return ok;
  },
};

function ownedRules(owner, target) {
  const state = { target, owner: reference(owner), active: true, trusted: !!Weak, canonicalWritable: true };
  const proxy = new ProxyCtor(target, arrayHandler);
  arrays.set(proxy, state);
  arrayTargets.set(target, state);
  for (let index = 0; index < target.length; index++) adoptRule(target[index], owner);
  return proxy;
}
function refreshRules(state) {
  const value = data(state.target, 'cssRules');
  const active = arrays.get(value);
  if (state.active && state.active !== active) state.active.active = false;
  state.active = active?.owner && read(active.owner) === state ? active : null;
  if (!state.active) unknown(state);
}
function sheetChanged(state, name) {
  if (name === 'cssRules') refreshRules(state);
  changed(state);
}
const sheetHandler = {
  set(target, name, value, receiver) {
    const state = sheetTargets.get(target),
      before = descriptorOf(target, name);
    begin(state);
    try {
      const ok = reflectSet(target, name, value, receiver);
      if (ok && (accessor(before) || !sameDescriptor(before, descriptorOf(target, name)))) sheetChanged(state, name);
      return ok;
    } finally {
      endCssMutation(state);
    }
  },
  defineProperty(target, name, descriptor) {
    const before = descriptorOf(target, name),
      ok = reflectDefine(target, name, descriptor);
    if (ok && !sameDescriptor(before, descriptorOf(target, name))) sheetChanged(sheetTargets.get(target), name);
    return ok;
  },
  deleteProperty(target, name) {
    const before = descriptorOf(target, name),
      ok = reflectDelete(target, name);
    if (ok && before !== undefined) sheetChanged(sheetTargets.get(target), name);
    return ok;
  },
  setPrototypeOf(target, prototype) {
    const before = prototypeOf(target),
      ok = reflectPrototype(target, prototype);
    if (ok && before !== prototype) {
      const state = sheetTargets.get(target);
      unknown(state);
      changed(state);
    }
    return ok;
  },
};

export function wrapCssSheet(target, owner) {
  const state = {
    target,
    owner: reference(owner),
    unknown: !Weak,
    depth: 0,
    dirty: false,
    snapshot: null,
    active: null,
    initializing: true,
    canonicalDepth: 0,
    pendingHole: false,
  };
  const proxy = new ProxyCtor(target, sheetHandler);
  sheets.set(proxy, state);
  sheetTargets.set(target, state);
  return proxy;
}

export function closedCssRules(receiver) {
  const state = sheets.get(receiver);
  if (!state?.active?.trusted) return null;
  const list = data(state.target, 'cssRules');
  if (
    !canOwnCssMap(state.active.target) ||
    descriptorOf(ARRAY, 'splice')?.value !== splice ||
    descriptorOf(state.active.target, 'splice')
  )
    return null;
  return list;
}
function matchesClosedRules(state, first, second, list) {
  return !!state?.active?.trusted && primitiveArguments(first, second) && list === data(state.target, 'cssRules');
}
export function beginCssMutation(receiver, first, second, list, inserted) {
  const state = sheets.get(receiver);
  if (!matchesClosedRules(state, first, second, list)) return null;
  // A private record plus primitive inputs proves construction ran no guest
  // callbacks since closedCssRules checked the exact array/species/splice shape.
  if (!pendingRules.get(inserted) && !canOwnCssMap(state.active.target)) return null;
  const length = state.active.target.length;
  if (descriptorOf(ARRAY, `${length}`) || descriptorOf(OBJECT, `${length}`)) return null;
  state.canonicalDepth++;
  begin(state);
  return state;
}
export function endCssApiMutation(state, completed) {
  if (!state) return;
  state.canonicalDepth--;
  if (state.pendingHole && !completed) loseArrayTrust(state.active, state);
  state.pendingHole = false;
  endCssMutation(state);
}
function canonicalSplice(state, owner, index, count, value) {
  const length = state.target.length;
  if (count === 0) invokeSplice(state.target, index, count, value);
  else invokeSplice(state.target, index, count);
  if (state.target.length !== length) {
    if (count === 0) adoptRule(value, owner);
    changed(owner);
  }
}
export function spliceCssRules(list, index, count, value) {
  const state = arrays.get(list),
    owner = state && arrayOwner(state);
  if (
    owner?.canonicalDepth &&
    state.trusted &&
    state.canonicalWritable &&
    owner.active === state &&
    extensible(state.target)
  ) {
    canonicalSplice(state, owner, index, count, value);
  } else if (count === 0) invokeSplice(list, index, count, exposedRule(value));
  else invokeSplice(list, index, count);
}

export function invalidateCssSheet(receiver) {
  const state = sheets.get(receiver);
  if (!state) throw new TypeError('Receiver must be an instance of class CSSStyleSheet');
  // Owned arrays already report their actual writes. Legacy/custom stores still
  // need the original method's conservative successful-operation notification.
  if (!state.active) changed(state);
  const node = receiver.ownerNode;
  if (node && node.__touch) node.__touch();
}
export function canOwnCssMap(pieces) {
  return (
    prototypeOf(pieces) === ARRAY &&
    !descriptorOf(pieces, 'constructor') &&
    !descriptorOf(pieces, 'map') &&
    descriptorOf(ARRAY, 'map')?.value === map &&
    descriptorOf(ARRAY, 'constructor')?.value === ArrayCtor &&
    descriptorOf(ArrayCtor, Symbol.species)?.get === species
  );
}
export function emptyCssRules(receiver) {
  return ownedRules(sheets.get(receiver), []);
}
export function replaceCssRules(receiver, value, trusted) {
  const state = sheets.get(receiver);
  receiver.cssRules = state && trusted ? ownedRules(state, value) : value;
}

function knownMetadata(state, owner, disabled, media) {
  return !state.initializing && owner !== UNKNOWN && disabled !== UNKNOWN && media !== UNKNOWN;
}
function observedRules(state) {
  return !state.initializing && !state.unknown && !!state.active?.trusted;
}
// A cached metadata object is not a text fingerprint for legacy/customized stores.
// Callers must inspect the explicit trust flags before reusing compiled sources.
export function cssSheetSnapshot(sheet) {
  const state = sheets.get(sheet);
  if (!state) return null;
  if (state.snapshot) return state.snapshot;
  if (state.active?.trusted) {
    const list = state.active.target;
    for (let index = 0; index < list.length; index++) {
      exposeArrayRule(state.active, `${index}`, list[index]);
    }
  }
  const ownerNode = data(state.target, 'ownerNode'),
    disabled = data(state.target, 'disabled'),
    media = data(state.target, 'media');
  const metadataTrusted = knownMetadata(state, ownerNode, disabled, media);
  state.snapshot = freezeCss({
    token: freezeCss({}),
    ownerNode,
    disabled,
    media,
    metadataTrusted,
    ownerStable: metadataTrusted && read(state.owner) === ownerNode,
    rulesTrusted: observedRules(state),
    mediaNeedsFingerprint: mutable(media),
  });
  return state.snapshot;
}

export function isObservedCssSheet(sheet) {
  return !!sheets.get(sheet);
}
