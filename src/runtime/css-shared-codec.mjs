// Known private producers only. These data operations confer no DOM/Proxy authority.
import { encodeCssWire, decodeCssWire } from "./css-rust-wire.mjs";
const fromCode = String.fromCharCode,
	charCode = String.prototype.charCodeAt,
	apply = Reflect.apply;
const define = Object.defineProperty,
	descriptor = Object.getOwnPropertyDescriptor,
	keys = Object.keys;
const create = Object.create,
	isArray = Array.isArray,
	MapClass = Map,
	SetClass = Set;
const A = Array.prototype,
	M = Map.prototype,
	S = Set.prototype;
const mapGet = M.get,
	mapSet = M.set,
	mapHas = M.has,
	mapForEach = M.forEach;
const setAdd = S.add,
	setHas = S.has;
const methods = {
	arrayReduce: A.reduce,
	arrayFind: A.find,
	arraySome: A.some,
	arrayIncludes: A.includes,
	arrayJoin: A.join,
	mapGet,
	mapSet,
	mapHas,
	stringStartsWith: String.prototype.startsWith,
	stringSlice: String.prototype.slice,
};
function put(array, index, value) {
	define(array, index, {
		value,
		writable: true,
		enumerable: true,
		configurable: true,
	});
}
function push(array, value) {
	put(array, array.length, value);
	return array.length;
}
function indexedMap(source, callback) {
	const output = [];
	for (let index = 0; index < source.length; index++)
		push(output, callback(source[index], index, source));
	return output;
}
function indexedFilter(source, callback) {
	const output = [];
	for (let index = 0; index < source.length; index++)
		if (callback(source[index], index, source)) push(output, source[index]);
	return output;
}
function indexedSlice(source, start = 0) {
	const output = [];
	const from =
		start < 0 ? (source.length + start > 0 ? source.length + start : 0) : start;
	for (let index = from; index < source.length; index++)
		push(output, source[index]);
	return output;
}
function indexedFlatMap(source, callback) {
	const output = [];
	for (let index = 0; index < source.length; index++) {
		const row = callback(source[index], index, source);
		for (let child = 0; child < row.length; child++) push(output, row[child]);
	}
	return output;
}
const arrays = {
	arrayMap: indexedMap,
	arrayFilter: indexedFilter,
	arraySlice: indexedSlice,
	arrayFlatMap: indexedFlatMap,
};
export function sharedApply(name, receiver, ...args) {
	if (name === "arrayPush") return push(receiver, args[0]);
	if (arrays[name])
		return apply(arrays[name], null, sharedAppend([receiver], args));
	return apply(methods[name], receiver, args);
}
export function sharedUnits(value) {
	const output = [];
	for (let index = 0; index < value.length; index++)
		push(output, apply(charCode, value, [index]));
	return output;
}
export function sharedText(value) {
	let output = "";
	for (let index = 0; index < value.length; index++)
		output += fromCode(value[index]);
	return output;
}
export const sharedDecode = (value) =>
	typeof value === "string" ? decodeCssWire(value) : value;
export const sharedEncode = encodeCssWire;
export const sharedInvoke = apply,
	sharedDefine = define,
	sharedDescriptor = descriptor,
	sharedKeys = keys,
	sharedIsArray = isArray;
export function sharedEntries(value) {
	const output = [],
		names = keys(value);
	for (let index = 0; index < names.length; index++) {
		const name = names[index],
			field = descriptor(value, name);
		if (!field || !("value" in field))
			throw new TypeError("CSS shared data requires own fields");
		push(output, [name, field.value]);
	}
	return output;
}
export function sharedFromEntries(entries) {
	const output = create(null);
	for (let index = 0; index < entries.length; index++)
		define(output, entries[index][0], {
			value: entries[index][1],
			enumerable: true,
		});
	return output;
}
export function SharedMap(entries) {
	const output = new MapClass();
	if (entries)
		for (let index = 0; index < entries.length; index++)
			apply(mapSet, output, [entries[index][0], entries[index][1]]);
	return output;
}
export function sharedMapKeys(map) {
	const output = [];
	apply(mapForEach, map, [(_value, key) => push(output, key)]);
	return output;
}
export function sharedAppend(first, second) {
	const output = [];
	for (let index = 0; index < first.length; index++) push(output, first[index]);
	for (let index = 0; index < second.length; index++)
		push(output, second[index]);
	return output;
}
export function sharedUnique(source) {
	const found = new SetClass(),
		output = [];
	for (let index = 0; index < source.length; index++) {
		const value = source[index];
		if (!apply(setHas, found, [value])) {
			apply(setAdd, found, [value]);
			push(output, value);
		}
	}
	return output;
}
