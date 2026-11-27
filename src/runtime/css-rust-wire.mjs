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
function field(property, active) {
	if (!own(property, "value")) throw invalid();
	return ownData(property.value, active);
}
function rememberArray(properties, length, active) {
 const previous = priorArrayFragment;
 let tokens = previous?.length === length ? undefined : ({ __proto__: null });
 for (let index = 0; index < length; index++) {
  if (!own(properties, index)) throw invalid();
  const encoded = field(properties[index], active);
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
function arrayData(properties, active) {
 const length = properties.length.value;
 const names = keys(properties);
 if (names.length !== length + 1) throw invalid();
 if (typeof length === 'number' && length >= 512 && length <= 16384) return rememberArray(properties, length, active);
 let result = '[';
 for (let index = 0; index < length; index++) {
  if (!own(properties, index)) throw invalid();
  const encoded = field(properties[index], active);
  result += (index === 0 ? '' : ',') + encoded;
 }
 return result + ']';
}
function recordData(properties, active) {
 let result = '{', separator = '';
 const names = keys(properties);
 for (let index = 0; index < names.length; index++) {
  const name = names[index];
  const property = properties[name];
  if (!property.enumerable) continue;
  const encoded = field(property, active);
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
function ownData(value, active) {
 const scalar = scalarData(value);
 if (scalar !== undefined) return scalar;
	if (typeof value !== "object") throw invalid();
	if (apply(setHas, active, [value]))
		throw new TypeError("turbo-dom: cyclic CSS wire DTO");
	apply(setAdd, active, [value]);
	try {
		const properties = descriptors(value);
		return isArray(value)
			? arrayData(properties, active)
			: recordData(properties, active);
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
