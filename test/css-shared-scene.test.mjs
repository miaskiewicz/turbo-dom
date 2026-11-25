import assert from "node:assert/strict";
import test from "node:test";
import { createSharedSceneProvider } from "../src/runtime/css-shared-scene.mjs";
const units = (value) => Array.from(value, (letter) => letter.charCodeAt(0));
function fixture() {
	return {
		snapshot: {
			roots: [{ id: 1 }],
			used: {},
			records: [
				{ id: 1, kind: "document", parent: null },
				{
					id: 2,
					kind: "element",
					parent: 1,
					tagUnits: units("div"),
					namespaceUnits: units("http://www.w3.org/1999/xhtml"),
					attrs: [],
				},
				{ id: 3, kind: "text", parent: 2, textUnits: units("text") },
			],
		},
		rows: [
			{
				id: 2,
				rendererValues: { display: units("block"), width: units("50%") },
			},
		],
		requested: [2],
		world: "world",
	};
}
const viewport = { width: 800, height: 600 };
test("Scene protocol derives primitive nodes from actual computed rows and caller policy", () => {
	let accepted, counts;
	const backend = {
		measureSceneV1(source) {
			accepted = JSON.parse(source);
			return JSON.stringify({
				world: accepted.scene.world,
				measurements: [{ id: 2, result: { status: "used_box", geometry: {} } }],
			});
		},
	};
	const provider = createSharedSceneProvider({
		selectBackend: () => backend,
		viewport,
		fonts: "native_html_system",
		limits: (context) => {
			counts = context.counts;
			return { callerPolicy: 1 };
		},
	});
	const source = fixture(),
		result = provider(source);
	assert.equal(result[0].id, 2);
	assert.equal(accepted.abiVersion, 1);
	assert.deepEqual(accepted.scene.viewport, viewport);
	assert.equal(accepted.scene.fonts, "native_html_system");
	assert.deepEqual(accepted.scene.limits, { callerPolicy: 1 });
	assert.deepEqual(counts, { properties: 2, styleUnits: 20 });
	assert.equal(accepted.scene.nodes[0].parent, null);
	assert.equal(accepted.scene.nodes[1].parent, 2);
	assert.deepEqual(
		accepted.scene.nodes[0].node.computed[1].valueUnits,
		units("50%"),
	);
});
test("provider exceptions, foreign world, missing style and unsupported attribute context stay honest", () => {
	const original = Error("original Scene exception");
	const backend = {
		measureSceneV1() {
			throw original;
		},
	};
	const provider = createSharedSceneProvider({
		selectBackend: () => backend,
		viewport,
		fonts: "bundled_html",
		limits: () => ({}),
	});
	assert.throws(
		() => provider(fixture()),
		(error) => error === original,
	);
	backend.measureSceneV1 = () =>
		JSON.stringify({ world: [0], measurements: [] });
	assert.throws(() => provider(fixture()), /CanonicalSceneWorldMismatch/);
	const missing = fixture();
	missing.rows = [];
	assert.throws(() => provider(missing), /MissingCanonicalComputedSceneStyle/);
	const namespace = fixture();
	namespace.snapshot.records[1].attrs = [{ prefix: "xlink" }];
	assert.throws(() => provider(namespace), /MissingOriginalAttributeNamespace/);
});
test("Scene own-data wire and node builders do not invoke after-import JSON/species/iterator/numeric hooks", () => {
	const source = fixture(),
		reply = JSON.stringify({ world: units("world"), measurements: [] });
	let calls = 0;
	const provider = createSharedSceneProvider({
		selectBackend: () => ({
			measureSceneV1() {
				calls++;
				return reply;
			},
		}),
		viewport,
		fonts: "bundled_html",
		limits: () => ({}),
	});
	const saved = {
		json: Object.getOwnPropertyDescriptor(Object.prototype, "toJSON"),
		numeric: Object.getOwnPropertyDescriptor(Array.prototype, "0"),
		species: Object.getOwnPropertyDescriptor(Array, Symbol.species),
		iterator: Object.getOwnPropertyDescriptor(Array.prototype, Symbol.iterator),
	};
	let traps = 0;
	const fail = () => {
		traps++;
		throw Error("private Scene trap");
	};
	let result;
	try {
		Object.defineProperty(Object.prototype, "toJSON", {
			configurable: true,
			value: fail,
		});
		Object.defineProperty(Array.prototype, "0", {
			configurable: true,
			set: fail,
			get: fail,
		});
		Object.defineProperty(Array, Symbol.species, {
			configurable: true,
			get: fail,
		});
		Object.defineProperty(Array.prototype, Symbol.iterator, {
			configurable: true,
			value: fail,
		});
		result = provider(source);
	} finally {
		const restore = (object, key, value) =>
			value
				? Object.defineProperty(object, key, value)
				: Reflect.deleteProperty(object, key);
		restore(Object.prototype, "toJSON", saved.json);
		restore(Array.prototype, "0", saved.numeric);
		restore(Array, Symbol.species, saved.species);
		restore(Array.prototype, Symbol.iterator, saved.iterator);
	}
	assert.deepEqual(result, []);
	assert.equal(calls, 1);
	assert.equal(traps, 0);
});

test("Scene rejects unsupported owners and capabilities and compares every world unit", () => {
	assert.throws(
		() =>
			createSharedSceneProvider({ selectBackend: null, limits: () => ({}) }),
		TypeError,
	);
	assert.throws(
		() =>
			createSharedSceneProvider({ selectBackend: () => ({}), limits: null }),
		TypeError,
	);
	const backend = {},
		provider = createSharedSceneProvider({
			selectBackend: () => backend,
			viewport,
			fonts: "bundled_html",
			limits: () => ({}),
		});
	const shadow = fixture();
	shadow.snapshot.roots.push({ id: 4 });
	assert.throws(() => provider(shadow), /MissingCanonicalShadowScene/);
	assert.throws(() => provider(fixture()), /MissingCanonicalSceneProvider/);
	backend.measureSceneV1 = () =>
		JSON.stringify({ world: units("worle"), measurements: [] });
	assert.throws(() => provider(fixture()), /CanonicalSceneWorldMismatch/);
});
