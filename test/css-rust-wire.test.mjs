import assert from "node:assert/strict";
import test from "node:test";
import { decodeCssWire, encodeCssWire } from "../src/runtime/css-rust-wire.mjs";

test("wire preserves UTF16 and ignores inherited serialization hooks", () => {
	const old = Object.prototype.toJSON;
	let calls = 0;
	Object.prototype.toJSON = () => {
		calls++;
		throw Error("inherited");
	};
	try {
		const value = Object.create({
			toJSON() {
				throw Error("prototype");
			},
		});
		value.text = [0xd800, 34, 92];
		assert.equal(encodeCssWire(value), '{"text":[55296,34,92]}');
		assert.equal(calls, 0);
	} finally {
		if (old === undefined) delete Object.prototype.toJSON;
		else Object.prototype.toJSON = old;
	}
});
test("wire refuses accessors without calling them and preserves provider errors", () => {
	let called = false;
	assert.throws(
		() =>
			encodeCssWire({
				get field() {
					called = true;
				},
			}),
		TypeError,
	);
	assert.equal(called, false);
	assert.deepEqual(decodeCssWire('{"text":[55296]}'), { text: [55296] });
});

test("factory accepts explicit codec hooks without extra warm bridges", async () => {
	const { createSharedComputedStyle } = await import(
		"../src/runtime/css-gcs-factory.mjs"
	);
	const document = {},
		element = {},
		token = {},
		environment = {};
	let encodes = 0,
		decodes = 0,
		resolves = 0,
		disposals = 0;
	const backend = {
		CompiledStylesV1: class {
			metadata() {
				return "{}";
			}
			prepare() {
				return {
					metadata: () => "{}",
					resolveBatch() {
						resolves++;
						return '{"targets":[{"id":1,"values":{"color":[114,101,100]},"custom":{}}]}';
					},
					dispose() {
						disposals++;
					},
				};
			}
			dispose() {
				disposals++;
			}
		},
	};
	const factory = createSharedComputedStyle({
		selectBackend: () => backend,
		readContext: () => ({
			document,
			version: 1,
			renderToken: token,
			environmentKey: environment,
			resultKey: null,
		}),
		captureSources: () => ({ status: "captured" }),
		compileRequest: () => ({}),
		prepareRequest: () => ({}),
		resolveRequest: () => ({ request: {}, resultId: 1 }),
		finalizeResult: (row) => row,
		encodeRequest(value) {
			encodes++;
			return encodeCssWire(value);
		},
		decodeResponse(value) {
			decodes++;
			return decodeCssWire(value);
		},
	});
	assert.equal(factory.get(element).color, "red");
	const firstDecodes = decodes;
	assert.equal(factory.get(element).color, "red");
	assert.equal(resolves, 1);
	assert.equal(decodes, firstDecodes);
	assert.ok(encodes > 0);
	factory.dispose();
	assert.equal(disposals, 2);
});

test("codec rejects unsupported and cyclic DTOs without inherited descriptor data", () => {
	const cyclic = {};
	cyclic.self = cyclic;
	for (const value of [
		cyclic,
		undefined,
		Number.NaN,
		Number.POSITIVE_INFINITY,
		{ x: undefined },
		[undefined],
		Array(1),
		{ x() {} },
	]) {
		assert.throws(() => encodeCssWire(value), TypeError);
	}
	const decorated = [];
	decorated.extra = 1;
	assert.throws(() => encodeCssWire(decorated), TypeError);
	const failure = new Error("proxy reflection");
	assert.throws(
		() =>
			encodeCssWire(
				new Proxy(
					{},
					{
						ownKeys() {
							throw failure;
						},
					},
				),
			),
		(error) => error === failure,
	);
	const hidden = Object.defineProperty({ kept: 1 }, "hidden", { value: 2 });
	assert.equal(encodeCssWire(hidden), '{"kept":1}');
});
test("captured intrinsics tolerate mutable Set/Array/Number/descriptor prototypes", () => {
	const has = Set.prototype.has,
		add = Set.prototype.add,
		remove = Set.prototype.delete;
	const iterator = Array.prototype[Symbol.iterator],
		number = Number.isFinite;
	const poison = () => {
		throw Error("poisoned intrinsic");
	};
	let output;
	try {
		Set.prototype.has = Set.prototype.add = Set.prototype.delete = poison;
		Array.prototype[Symbol.iterator] = poison;
		Number.isFinite = poison;
		Object.defineProperty(Object.prototype, "value", {
			configurable: true,
			get: poison,
		});
		output = encodeCssWire({
			text: [0xd800],
			number: 2,
			nested: null,
			flag: true,
		});
	} finally {
		Set.prototype.has = has;
		Set.prototype.add = add;
		Set.prototype.delete = remove;
		Array.prototype[Symbol.iterator] = iterator;
		Number.isFinite = number;
		delete Object.prototype.value;
	}
	assert.equal(output, '{"text":[55296],"number":2,"nested":null,"flag":true}');
});
