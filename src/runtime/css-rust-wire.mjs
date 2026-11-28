// Explicit JSON ABI transport for known private DTO producers, not arbitrary
// guest objects. Proxy reflection can trap; its original error propagates.
const parse = JSON.parse;
const stringify = JSON.stringify;
const scalarString = String;
// Snapshot every own descriptor, including symbols, before recursing. A Proxy
// may delete a key or change another value while its descriptors are read.
const ownKeys = Reflect.ownKeys;
const descriptor = Object.getOwnPropertyDescriptor;
function descriptors(value) {
  const fields = create(null);
  const names = ownKeys(value);
  for (let index = 0; index < names.length; index++) {
    const key = names[index];
    const field = descriptor(value, key);
    if (field !== undefined) fields[key] = field;
  }
  return fields;
}
const keys = Object.keys;
const create = Object.create;
let priorArrayFragment;
const isArray = Array.isArray;
const finite = Number.isFinite;
const apply = Reflect.apply;
const own = Object.hasOwn;
const SetConstructor = Set;
const setHas = Set.prototype.has;
const setAdd = Set.prototype.add;
const setDelete = Set.prototype.delete;
function invalid() {
	return new TypeError("turbo-dom: CSS wire requires scalar own-data DTOs");
}
function field(property, active, memo) {
	if (!own(property, "value")) throw invalid();
	return ownData(property.value, active, memo);
}
function rememberArray(properties, length, active, memo) {
 const previous = priorArrayFragment;
 let tokens = previous?.length === length ? undefined : ({ __proto__: null });
 for (let index = 0; index < length; index++) {
  if (!own(properties, index)) throw invalid();
  const encoded = field(properties[index], active, memo);
  const token = '' + encoded;
  if (tokens === undefined && previous.tokens[index] !== token) {
   tokens = copyPrefix(previous.tokens, index);
  }
  if (tokens !== undefined) tokens[index] = token;
 }
 return finishArray(previous, tokens, length);
}
function copyPrefix(previous, end) {
 const tokens = { __proto__: null };
 for (let index = 0; index < end; index++) tokens[index] = previous[index];
 return tokens;
}
function finishArray(previous, tokens, length) {
 if (tokens === undefined) return previous.wire;
 let wire = '[';
 for (let index = 0; index < length; index++) wire += (index === 0 ? '' : ',') + tokens[index];
 wire += ']';
 priorArrayFragment = wire.length <= 65536 ? { length, tokens, wire } : undefined;
 return wire;
}
function arrayData(properties, active, memo) {
 const length = properties.length.value;
 const names = keys(properties);
 if (names.length !== length + 1) throw invalid();
 if (typeof length === 'number' && length >= 512 && length <= 16384) return rememberArray(properties, length, active, memo);
 let result = '[';
 for (let index = 0; index < length; index++) {
  if (!own(properties, index)) throw invalid();
  const encoded = field(properties[index], active, memo);
  result += (index === 0 ? '' : ',') + encoded;
 }
 return result + ']';
}
function recordData(properties, active, memo) {
 let result = '{', separator = '';
 const names = keys(properties);
 for (let index = 0; index < names.length; index++) {
  const name = names[index];
  const property = properties[name];
  if (!property.enumerable) continue;
  const encoded = field(property, active, memo);
  result += separator + stringify(name) + ':' + encoded;
  separator = ',';
 }
 return result + '}';
}
function scalarData(value) {
	if (value === null) return "null";
	if (typeof value === "boolean") return value ? "true" : "false";
	if (typeof value === "string") return stringify(value);
	if (typeof value === "number" && finite(value)) return scalarString(value);
}
function ownData(value, active, memo) {
 const scalar = scalarData(value);
 if (scalar !== undefined) return scalar;
	if (typeof value !== "object") throw invalid();
	if (apply(setHas, active, [value]))
		throw new TypeError("turbo-dom: cyclic CSS wire DTO");
	apply(setAdd, active, [value]);
	try {
		const cached = memo?.read(value);
        if (cached !== undefined) return cached;
        const properties = descriptors(value);
        const wire = isArray(value)
            ? arrayData(properties, active, memo)
            : recordData(properties, active, memo);
        memo?.remember(value, wire);
        return wire;
	} finally {
		apply(setDelete, active, [value]);
	}
}
export function encodeCssWire(value) {
	return ownData(value, new SetConstructor());
}
export function decodeCssWire(value) {
	return parse(value);
}

// Only the closed default profile passes this decoder the provider method name.
// Eligible UA bytes are parsed here, checked before exposure, and retained by
// weak identity. Every other DTO still takes the ordinary descriptor traversal.
const NativeWeakMap = WeakMap, NativeWeakSet = WeakSet;
const weakGet = WeakMap.prototype.get, weakSet = WeakMap.prototype.set;
const ownedHas = WeakSet.prototype.has, ownedAdd = WeakSet.prototype.add;
const freeze = Object.freeze, prototype = Object.getPrototypeOf, objectPrototype = Object.prototype;
function jsonField(value, name) {
 if (value === null || typeof value !== 'object') return undefined;
 const field = descriptor(value, name);
 return field === undefined ? undefined : field.value;
}
function unitArray(value) {
 if (!isArray(value)) return false;
 for (let index = 0; index < value.length; index++) {
  const unit = value[index];
  if (typeof unit !== 'number' || unit < 0 || unit > 65535 || unit % 1 !== 0) return false;
 }
 return true;
}
function initialFields(value) {
 return unitArray(jsonField(value, 'color')) && unitArray(jsonField(value, 'fontFamily')) &&
  unitArray(jsonField(value, 'fontSizeInput'));
}
function ownNames(value, names) {
 if (value === null || typeof value !== 'object') return false;
 for (let index = 0; index < names.length; index++) if (descriptor(value, names[index]) === undefined) return false;
 return true;
}
function privatePolicy(value) {
 return jsonField(value, 'abiVersion') === 1 &&
  ownNames(value, ['preferences','environment','snapshot','selectorFacts','scene','composition','fonts']) &&
  ownNames(jsonField(value, 'adapter'), ['compile','prepare','matching','contextualMatching','resolve','serialize','property','providerSnapshot']);
}
function privateUaText(value) {
 if (jsonField(value, 'abiVersion') !== 1 || !unitArray(jsonField(value, 'preferenceIdentity'))) return undefined;
 if (!initialFields(jsonField(value, 'initial'))) return undefined;
 const source = jsonField(value, 'userAgentSource');
 if (jsonField(source, 'abiVersion') !== 1 || !unitArray(jsonField(source, 'documentText'))) return undefined;
 const text = jsonField(source, 'text');
 return unitArray(text) ? text : undefined;
}
export function createOwnedJsonCssWire() {
 const owned = new NativeWeakSet();
 let fragments = new NativeWeakMap(), units = 0, eligible = true;
 function decline() { eligible = false; fragments = new NativeWeakMap(); units = 0; }
 const memo = {
  read(value) { return eligible ? apply(weakGet, fragments, [value]) : undefined; },
  remember(value, wire) {
   if (!eligible || !apply(ownedHas, owned, [value]) || units + wire.length > 65536) return;
   apply(weakSet, fragments, [value, wire]);
   units += wire.length;
  },
 };
 function encode(value) { return ownData(value, new SetConstructor(), memo); }
 return {
  decode(value, provider) {
   if (typeof value !== 'string') { decline(); return value; }
   const result = parse(value);
   if (provider === 'cssRuntimePolicyV1' && !privatePolicy(result)) decline();
   const text = eligible && provider === 'cssComputedPreferencesV1' ? privateUaText(result) : undefined;
   if (text !== undefined) { freeze(text); apply(ownedAdd, owned, [text]); }
   return result;
  },
  policyField(adapter, name) {
   if (typeof name !== 'string') { decline(); return; }
   if (prototype(adapter) !== objectPrototype || descriptor(objectPrototype, name) !== undefined) decline();
  },
  compile: encode,
  prepare: encode,
  reset() { fragments = new NativeWeakMap(); units = 0; },
 };
}

// Experimental request-state transport records private encoded segments during
// the original traversal. A warm read constructs only its compact wire.
const defineWireField = Object.defineProperty;
function appendWire(array, value) {
 defineWireField(array, array.length, {value,enumerable:true,writable:true,configurable:true});
}
function appendSegment(state, value) {
 state.segments[state.segmentCount++] = value;
}
function findContext(contexts, full) {
 for (let index = 0; index < (contexts?.length ?? 0); index++) {
  if (contexts[index] === full) return index;
 }
 return -1;
}
function contextWire(value, state) {
 const full = ownData(value, state.active), index = state.contexts.length;
 appendWire(state.contexts, full);
 const reference = findContext(state.previous, full);
 if (reference === -1) state.reusable = false;
 appendWire(state.references, reference);
 appendWire(state.selfReferences, findContext(state.contexts, full));
 appendSegment(state, index);
}
function requestField(property, role, state) {
 if (!own(property, 'value')) throw invalid();
 if (role === 4) return contextWire(property.value, state);
 requestData(property.value, role, state);
}
function requestRecord(properties, role, state) {
 appendSegment(state, '{');
 let separator = '';
 const names = keys(properties);
 for (let index = 0; index < names.length; index++) {
  const name = names[index], property = properties[name];
  if (!property.enumerable) continue;
  const child = role === 1 && name === 'targets' ? 2 : role === 3 && name === 'context' ? 4 : 0;
  appendSegment(state, separator + stringify(name) + ':');
  requestField(property, child, state);
  separator = ',';
 }
 appendSegment(state, '}');
}
function requestArray(properties, state) {
 const length = properties.length.value;
 if (keys(properties).length !== length + 1) throw invalid();
 appendSegment(state, '[');
 for (let index = 0; index < length; index++) {
  if (!own(properties, index)) throw invalid();
  if (index > 0) appendSegment(state, ',');
  requestField(properties[index], 3, state);
 }
 appendSegment(state, ']');
}
function requestData(value, role, state) {
 if (role === 0) return appendSegment(state, ownData(value, state.active));
 const scalar = scalarData(value);
 if (scalar !== undefined) return appendSegment(state, scalar);
 if (typeof value !== 'object') throw invalid();
 if (apply(setHas, state.active, [value])) throw new TypeError('turbo-dom: cyclic CSS wire DTO');
 apply(setAdd, state.active, [value]);
 try {
  const properties = descriptors(value), array = isArray(value);
  requestContainer(properties, role, state, array);
 } finally { apply(setDelete, state.active, [value]); }
}
function requestContainer(properties, role, state, array) {
 if (array) {
  if (role === 2) return requestArray(properties, state);
  return appendSegment(state, arrayData(properties, state.active));
 }
 if (role !== 2) return requestRecord(properties, role, state);
 appendSegment(state, recordData(properties, state.active));
}
function renderWire(state, references) {
 let wire = '';
 for (let index = 0; index < state.segmentCount; index++) {
  const segment = state.segments[index];
  wire += typeof segment === 'string' ? segment : references ? scalarString(references[segment]) : state.contexts[segment];
 }
 return wire;
}
export function encodeCssRequestState(value, previous) {
 const state = { __proto__: null, active:new SetConstructor(),contexts:[],references:[],selfReferences:[],segments:{__proto__:null},segmentCount:0,previous,reusable:previous!==undefined };
 requestData(value, 1, state);
 const compact = renderWire(state, state.reusable ? state.references : state.selfReferences);
 let full;
 return { __proto__:null, get full(){return full ??= renderWire(state);}, compact,contexts:state.contexts,reusable:state.reusable };
}
