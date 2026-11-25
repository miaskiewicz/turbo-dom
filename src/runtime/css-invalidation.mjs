// Opt-in connected-document CSS checkpoints. The broad DOM version remains the
// query/provenance authority; this records only audited mutation notifications.
// Mutable legacy media arrays/foreign records still require source fingerprints;
// viewport, compatibility and provider stamps remain independent.
const freezeCss = Object.freeze;
const cssArray = Array.isArray;
const cssInteger = Number.isSafeInteger;
const states = new WeakMap();
const stateGet = states.get.bind(states);
const stateHas = states.has.bind(states);
const stateSet = states.set.bind(states);
let armDocument = null;
let focusStatus = null;
let trackingStarted = false;
let cssSourceStamp = freezeCss({});
let cssSourceObserved = true;

// Installed once by Document static initialization. A hostile preload can only
// make that initialization fail; it cannot replace the genuine arm-only hook.
export function installCssTrackingArm(arm, focus) {
  if (armDocument !== null) throw new TypeError('turbo-dom: CSS document tracking is already initialized');
  armDocument = arm;
  focusStatus = focus;
}
const UNKNOWN = Symbol('unknown CSS ownership');
const CONNECTED = Symbol('connected CSS ownership');
const DETACHED = Symbol('detached CSS ownership');
const descriptorOf = Object.getOwnPropertyDescriptor;

function ownValue(object, name, missing = UNKNOWN) {
  // Exotic reflection cannot certify a cache proof. No public connection getter
  // is invoked, and an optional proof failure must not change DOM mutator errors.
  try {
    const descriptor = descriptorOf(object, name);
    if (descriptor === undefined) return missing;
    return 'value' in descriptor ? descriptor.value : UNKNOWN;
  } catch {
    return UNKNOWN;
  }
}

// This is a runtime fact, not activeElement's unfocused body fallback. Unknown
// native ownership/descriptor state stays distinct from a known absence of focus.
export function cssFocusTarget(document) {
  const focused = focusStatus ? focusStatus(document) : undefined;
  if (focused !== true) return focused === false ? null : undefined;
  const target = ownValue(document, '__active');
  return target === null || typeof target === 'object' ? target : undefined;
}

function currentIndex(node, document) {
  const index = ownValue(node, '__idx');
  if (typeof index !== 'number') return UNKNOWN;
  if (index < 0) return index;
  return ownValue(ownValue(document, '__cache'), String(index)) === node ? index : UNKNOWN;
}

function childEdge(parent, node) {
  const children = ownValue(parent, '__kids');
  if (!cssArray(children)) return false;
  const length = ownValue(children, 'length');
  if (!cssInteger(length)) return false;
  for (let index = 0; index < length; index++) {
    const child = ownValue(children, String(index));
    if (child === UNKNOWN) return false;
    if (child === node) return true;
  }
  return false;
}

function rootEdge(node, index, strict) {
  const shadow = ownValue(node, '__isShadowRoot', false);
  if (shadow === UNKNOWN) return UNKNOWN;
  if (!shadow) return index < 0 ? DETACHED : UNKNOWN;
  const host = ownValue(node, 'host');
  if (strict && ownValue(host, '__shadow') !== node) return UNKNOWN;
  return host;
}

function nextNode(node, document, strict) {
  if (node === document) return CONNECTED;
  if (ownValue(node, 'ownerDocument') !== document) return UNKNOWN;
  const index = currentIndex(node, document);
  if (index === UNKNOWN) return UNKNOWN;
  const parent = ownValue(node, 'parentNode');
  if (parent === UNKNOWN) return UNKNOWN;
  if (parent === null) return rootEdge(node, index, strict);
  if (strict && !childEdge(parent, node)) return UNKNOWN;
  return parent;
}

function connection(node, document, strict) {
  // Brent's cycle check visits the ordinary parent chain once without retaining
  // a per-node ownership mirror or allocating a traversal set.
  let anchor = node,
    span = 1,
    distance = 0;
  for (;;) {
    node = nextNode(node, document, strict);
    if (typeof node === 'symbol') return node;
    if (node === anchor) return UNKNOWN;
    distance++;
    if (distance === span) {
      anchor = node;
      span *= 2;
      distance = 0;
    }
  }
}

function advance(state, document, structural = true) {
  // Only returned checkpoints can be held by caches. Coalesce writes until the
  // next observed checkpoint, while retaining a separate CSS-source component.
  if (structural && !state.structureDirty) state.structure = freezeCss({});
  if (structural) state.structureDirty = true;
  state.token = null;
  state.version = document.__version;
  state.cssSourceStamp = cssSourceStamp;
}

// CSSOM edits may have unknown/shared owners. A global source stamp avoids new
// owner getters or an owner registry; unrelated documents may conservatively miss.
export function noteCssSourceChange() {
  if (!trackingStarted || !cssSourceObserved) return;
  cssSourceStamp = freezeCss({});
  cssSourceObserved = false;
}

export function ensureCssState(document) {
  if (!stateHas(document)) {
    if (armDocument === null) return null;
    armDocument(document);
    stateSet(document, {
      version: document.__version,
      token: null,
      structure: freezeCss({}),
      structureDirty: false,
      cssSourceStamp,
    });
    trackingStarted = true;
  }
  return cssStateSnapshot(document);
}

// A target outside the current connected tree receives no checkpoint. It must
// resolve without connected-world reuse unless independent target/state stamps
// establish validity. A full DOM version alone cannot certify dirty form state.
export function cssStateSnapshot(document, target = document) {
  const state = stateGet(document);
  if (!state) return null;
  if (state.version !== document.__version) advance(state, document);
  else if (state.cssSourceStamp !== cssSourceStamp) advance(state, document, false);
  if (connection(target, document, true) !== CONNECTED) return null;
  if (state.token === null) state.token = freezeCss({ structure: state.structure, cssSource: state.cssSourceStamp });
  cssSourceObserved = true;
  state.structureDirty = false;
  return state.token;
}

// Called AFTER the original broad-version update, with its exact prior value.
export function noteCssMutation(document, target, previousVersion) {
  const state = stateGet(document);
  if (!state) return;
  if (state.version !== previousVersion || connection(target, document, false) !== DETACHED) advance(state, document);
  state.version = document.__version;
}

// Dirty form state, focus, composition, adoption and document replacement can
// affect style without changing the existing broad version. Be conservative.
export function noteCssStateChange(document) {
  const state = stateGet(document);
  if (state) advance(state, document);
}
