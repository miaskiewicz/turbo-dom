import assert from "node:assert/strict";
import test from "node:test";
import { sharedProviderSnapshot } from "../src/runtime/css-shared-adapter.mjs";
import { sharedText, sharedUnits } from "../src/runtime/css-shared-codec.mjs";
const snapshot = () => ({
	roots: [{ id: 1, element: { native: true } }],
	used: { maxNodes: 1 },
	records: [{ id: 1, kind: "document", element: { native: true } }],
	nodeId() {
		throw Error("not retained");
	},
});
const policy = { maxDepth: 8, maxEntries: 32 };
test("shared provider context retains source data without native wrappers or functions", () => {
	const source = snapshot(),
		copy = sharedProviderSnapshot(source, policy);
	assert.deepEqual(copy, {
		roots: [{ id: 1 }],
		used: { maxNodes: 1 },
		records: [{ id: 1, kind: "document" }],
	});
	source.records[0].id = 99;
	assert.equal(copy.records[0].id, 1);
	assert.notEqual(copy.roots, source.roots);
});
test("provider quotas and unknown/accessor payloads preserve failure before getter invocation", () => {
	for (const bound of [
		{ maxDepth: 0, maxEntries: 32 },
		{ maxDepth: 8, maxEntries: 0 },
	])
		assert.throws(() => sharedProviderSnapshot(snapshot(), bound), RangeError);
	assert.throws(
		() => sharedProviderSnapshot(snapshot(), { maxDepth: -1, maxEntries: 32 }),
		TypeError,
	);
	let calls = 0;
	const record = {};
	Object.defineProperty(record, "payload", {
		enumerable: true,
		get() {
			calls++;
			throw Error("guest getter");
		},
	});
	assert.throws(
		() => sharedProviderSnapshot({ ...snapshot(), records: [record] }, policy),
		TypeError,
	);
	assert.equal(calls, 0);
	assert.throws(
		() =>
			sharedProviderSnapshot(
				{ ...snapshot(), records: [{ payload: () => 0 }] },
				policy,
			),
		TypeError,
	);
	assert.throws(
		() =>
			sharedProviderSnapshot(
				{ ...snapshot(), records: [{ payload: new Date() }] },
				policy,
			),
		TypeError,
	);
});
test("shared codec preserves lone UTF16 units and has no argument-spread length ceiling", () => {
	const value = [0xd800, 0, 0xdfff, 0x26, 0x1f];
	assert.deepEqual(sharedUnits(sharedText(value)), value);
	const long = Array(200000).fill(97);
	assert.deepEqual(sharedUnits(sharedText(long)), long);
});

test("after-import poisoned prototypes cannot intercept private DTO construction or transport", async () => {
	const codec = await import("../src/runtime/css-shared-codec.mjs");
	const saved = {
		json: Object.getOwnPropertyDescriptor(Object.prototype, "toJSON"),
		numeric: Object.getOwnPropertyDescriptor(Array.prototype, "0"),
		species: Object.getOwnPropertyDescriptor(Array, Symbol.species),
		iterator: Object.getOwnPropertyDescriptor(Array.prototype, Symbol.iterator),
		mapIterator: Object.getOwnPropertyDescriptor(
			Map.prototype,
			Symbol.iterator,
		),
	};
	let calls = 0;
	const poison = () => {
		calls++;
		throw Error("prototype trap");
	};
	let encoded, owned, mapped, unique, keys;
	try {
		Object.defineProperty(Object.prototype, "toJSON", {
			configurable: true,
			value: poison,
		});
		Object.defineProperty(Array.prototype, "0", {
			configurable: true,
			set: poison,
			get: poison,
		});
		Object.defineProperty(Array, Symbol.species, {
			configurable: true,
			get: poison,
		});
		Object.defineProperty(Array.prototype, Symbol.iterator, {
			configurable: true,
			value: poison,
		});
		Object.defineProperty(Map.prototype, Symbol.iterator, {
			configurable: true,
			value: poison,
		});
		const source = {
			roots: [{ id: 1, element: { native: true } }],
			used: { maxNodes: 1 },
			records: [{ id: 1, kind: "document", element: { native: true } }],
		};
		owned = sharedProviderSnapshot(source, policy);
		mapped = codec.sharedApply("arrayMap", [1, 2], (value) => value + 1);
		const filtered = codec.sharedApply(
			"arrayFilter",
			mapped,
			(value) => value > 1,
		);
		const sliced = codec.sharedApply("arraySlice", filtered, 0);
		const flat = codec.sharedApply("arrayFlatMap", sliced, (value) => [
			value,
			value,
		]);
		unique = codec.sharedUnique(flat);
		keys = codec.sharedMapKeys(
			new codec.SharedMap([
				["a", 1],
				["b", 2],
			]),
		);
		encoded = codec.sharedEncode({
			dto: owned,
			units: codec.sharedUnits("\ud800"),
			values: codec.sharedAppend(unique, keys),
		});
	} finally {
		const restore = (object, key, value) =>
			value
				? Object.defineProperty(object, key, value)
				: Reflect.deleteProperty(object, key);
		restore(Object.prototype, "toJSON", saved.json);
		restore(Array.prototype, "0", saved.numeric);
		restore(Array, Symbol.species, saved.species);
		restore(Array.prototype, Symbol.iterator, saved.iterator);
		restore(Map.prototype, Symbol.iterator, saved.mapIterator);
	}
	assert.equal(calls, 0);
	assert.deepEqual(mapped, [2, 3]);
	assert.deepEqual(unique, [2, 3]);
	assert.deepEqual(keys, ["a", "b"]);
	const result = JSON.parse(encoded);
	assert.deepEqual(result.dto, owned);
	assert.deepEqual(result.units, [0xd800]);
	assert.deepEqual(result.values, [2, 3, "a", "b"]);
});

test("trusted-capture boundary does not claim reflective Proxy trap avoidance", () => {
	const original = Error("source reflection failure");
	const proxy = new Proxy(
		{ id: 1 },
		{
			ownKeys() {
				throw original;
			},
		},
	);
	assert.throws(
		() => sharedProviderSnapshot({ ...snapshot(), records: [proxy] }, policy),
		(error) => error === original,
	);
});

test("adapter requires explicit caller capabilities and policies, without inventing runtime defaults", async () => {
	const { createSharedCssAdapter } = await import(
		"../src/runtime/css-shared-adapter.mjs"
	);
	const no = () => {},
		base = {
			selectBackend: () => null,
			readContext: no,
			captureSnapshot: no,
			measureScene: no,
			profile: {
				environment: no,
				targetContext: no,
				scopeDepth: no,
				world: no,
				backendIdentity: no,
			},
			limits: {
				compile: no,
				prepare: no,
				matching: no,
				resolve: no,
				serialize: no,
				property: no,
				providerSnapshot: no,
			},
		};
	for (const name of [
		"selectBackend",
		"readContext",
		"captureSnapshot",
		"measureScene",
	])
		assert.throws(
			() => createSharedCssAdapter({ ...base, [name]: null }),
			(error) => error instanceof TypeError && error.message.includes(name),
		);
	for (const name of [
		"compile",
		"prepare",
		"matching",
		"resolve",
		"serialize",
		"property",
		"providerSnapshot",
	])
		assert.throws(
			() =>
				createSharedCssAdapter({
					...base,
					limits: { ...base.limits, [name]: null },
				}),
			(error) => error instanceof TypeError && error.message.includes(name),
		);
	for (const name of [
		"environment",
		"targetContext",
		"scopeDepth",
		"world",
		"backendIdentity",
	])
		assert.throws(
			() =>
				createSharedCssAdapter({
					...base,
					profile: { ...base.profile, [name]: null },
				}),
			(error) => error instanceof TypeError && error.message.includes(name),
		);
	const adapter = createSharedCssAdapter(base);
	try {
		assert.throws(() => adapter.get({}), /backend unavailable/);
	} finally {
		adapter.dispose();
	}
});

test("owned DTO copy preserves scalar/array/null fields and rejects unstable producer descriptors", () => {
	const record = { id: 1, payload: [null, true, false, 0, "text"] },
		source = { ...snapshot(), records: [record] };
	assert.deepEqual(
		sharedProviderSnapshot(source, policy).records[0].payload,
		record.payload,
	);
	for (const bound of [
		{ maxDepth: undefined, maxEntries: 32 },
		{ maxDepth: 8, maxEntries: -1 },
		{ maxDepth: 8, maxEntries: NaN },
	])
		assert.throws(() => sharedProviderSnapshot(source, bound), TypeError);
	const wrapped = { id: 1 };
	Object.defineProperty(wrapped, "element", {
		enumerable: true,
		get() {
			throw Error("wrapper getter must never be read");
		},
	});
	assert.deepEqual(
		sharedProviderSnapshot({ ...snapshot(), records: [wrapped] }, policy)
			.records,
		[{ id: 1 }],
	);
	let descriptors = 0;
	const unstable = new Proxy(
		{ id: 1 },
		{
			getOwnPropertyDescriptor(target, key) {
				descriptors++;
				return descriptors === 1
					? Reflect.getOwnPropertyDescriptor(target, key)
					: undefined;
			},
		},
	);
	assert.throws(
		() =>
			sharedProviderSnapshot({ ...snapshot(), records: [unstable] }, policy),
		/snapshot accessor/,
	);
});

test("indexed transport helpers preserve empty/partial arrays and dangerous own property names", async () => {
	const codec = await import("../src/runtime/css-shared-codec.mjs");
	assert.deepEqual(codec.sharedUnits(""), []);
	assert.equal(codec.sharedText([]), "");
	assert.deepEqual(codec.sharedApply("arraySlice", [1, 2, 3]), [1, 2, 3]);
	assert.deepEqual(codec.sharedApply("arraySlice", [1, 2, 3], -1), [3]);
	assert.deepEqual(codec.sharedApply("arraySlice", [1], -99), [1]);
	assert.deepEqual(
		codec.sharedApply("arrayFilter", [1, 2], () => false),
		[],
	);
	assert.deepEqual(
		codec.sharedApply("arrayFlatMap", [1], () => []),
		[],
	);
	assert.deepEqual(codec.sharedAppend([], []), []);
	assert.deepEqual(codec.sharedUnique([]), []);
	const object = codec.sharedFromEntries([
		["__proto__", [0xd800]],
		["constructor", null],
	]);
	assert.equal(Object.getPrototypeOf(object), null);
	assert.deepEqual(object.__proto__, [0xd800]);
	assert.equal(object.constructor, null);
	assert.deepEqual(codec.sharedEntries(object), [
		["__proto__", [0xd800]],
		["constructor", null],
	]);
	let calls = 0;
	const accessor = {};
	Object.defineProperty(accessor, "value", {
		enumerable: true,
		get() {
			calls++;
			return 1;
		},
	});
	assert.throws(() => codec.sharedEntries(accessor), TypeError);
	assert.equal(calls, 0);
	assert.equal(codec.sharedDecode(object), object);
});

test("nested provider data consumes the cumulative quota and descriptor uncertainty stays rejected", () => {
	assert.throws(
		() => sharedProviderSnapshot(snapshot(), { maxDepth: 8, maxEntries: 3 }),
		/snapshot entries/,
	);
	let calls = 0;
	const changing = new Proxy(
		{ value: 1 },
		{
			getOwnPropertyDescriptor(target, key) {
				calls++;
				return calls === 1
					? Reflect.getOwnPropertyDescriptor(target, key)
					: undefined;
			},
		},
	);
	assert.throws(
		() => sharedProviderSnapshot({ ...snapshot(), used: changing }, policy),
		/snapshot accessor/,
	);
});
