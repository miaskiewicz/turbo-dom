// Explicit JSON ABI transport for known private DTO producers, not arbitrary
// guest objects. Proxy reflection can trap; its original error propagates.
const parse = JSON.parse;
const stringify = JSON.stringify;
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
const setPrototypeOf = Object.setPrototypeOf;
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
function arrayData(properties, active) {
	const length = properties.length.value;
	const names = keys(properties);
	if (names.length !== length + 1) throw invalid();
	const result = setPrototypeOf([], null);
	for (let index = 0; index < length; index++) {
		if (!own(properties, index)) throw invalid();
		result[index] = field(properties[index], active);
	}
	return result;
}
function recordData(properties, active) {
	const names = keys(properties);
	for (let index = 0; index < names.length; index++) {
		const name = names[index];
		const property = properties[name];
		if (property.enumerable) properties[name] = field(property, active);
		else delete properties[name];
	}
	return properties;
}
function ownData(value, active) {
	if (value === null || typeof value === "string" || typeof value === "boolean")
		return value;
	if (typeof value === "number" && finite(value)) return value;
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
	return stringify(ownData(value, new SetConstructor()));
}
export function decodeCssWire(value) {
	return parse(value);
}
