import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { createCssBackendResolver } from "../src/runtime/css-backend.mjs";

class CompiledStylesV1 {}
const capability = () => ({ cssAbiVersion: 1, CompiledStylesV1 });
const selection = (mode = "auto", binding = null, revision = 1) => ({
	mode,
	binding,
	revision,
});

test("CSS capability resolution is lazy and memoizes the selected descriptor", () => {
	let loads = 0;
	const binding = capability();
	const resolver = createCssBackendResolver({
		loadNative() {
			loads++;
			return binding;
		},
	});
	assert.equal(loads, 0);
	const first = resolver.resolve(selection());
	assert.equal(first.CompiledStylesV1, CompiledStylesV1);
	assert.equal(first.cssAbiVersion, 1);
	assert.equal(Object.isFrozen(first), true);
	assert.notEqual(first, binding);
	assert.equal(resolver.resolve(selection()), first);
	assert.equal(loads, 1);
});

test("injected optional CSS capability wins without requiring parser methods", () => {
	let calls = 0;
	const resolver = createCssBackendResolver({
		loadNative() {
			calls++;
		},
		loadWasm() {
			calls++;
		},
	});
	for (const mode of ["native", "wasm", "auto"]) {
		assert.equal(
			resolver.resolve(selection(mode, capability())).CompiledStylesV1,
			CompiledStylesV1,
		);
	}
	assert.equal(calls, 0);
});

test("minimal parser injection remains valid and CSS selection is independent", () => {
	const parser = { parse() {}, parseBuffer() {}, parseFragment() {} };
	let nativeLoads = 0;
	let wasmLoads = 0;
	const resolver = createCssBackendResolver({
		loadNative() {
			nativeLoads++;
			return parser;
		},
		loadWasm() {
			wasmLoads++;
			return capability();
		},
	});
	assert.equal(
		resolver.resolve(selection("auto", parser)).CompiledStylesV1,
		CompiledStylesV1,
	);
	assert.equal(nativeLoads, 1);
	assert.equal(wasmLoads, 1);
	assert.deepEqual(Object.keys(parser), [
		"parse",
		"parseBuffer",
		"parseFragment",
	]);
});

test("forced modes never cross-load and absent hosts retain an explicit null capability", () => {
	const calls = [];
	const resolver = createCssBackendResolver({
		loadNative() {
			calls.push("native");
			return null;
		},
		loadWasm() {
			calls.push("wasm");
			return capability();
		},
	});
	assert.equal(resolver.resolve(selection("native")), null);
	assert.deepEqual(calls, ["native"]);
	assert.equal(
		resolver.resolve(selection("wasm")).CompiledStylesV1,
		CompiledStylesV1,
	);
	assert.deepEqual(calls, ["native", "wasm"]);
	const unavailable = createCssBackendResolver();
	for (const mode of ["native", "wasm", "auto"])
		assert.equal(unavailable.resolve(selection(mode)), null);
});

test("auto falls through explicit absence and memoizes absent results", () => {
	const calls = [];
	const resolver = createCssBackendResolver({
		loadNative() {
			calls.push("native");
		},
		loadWasm() {
			calls.push("wasm");
			return null;
		},
	});
	assert.equal(resolver.resolve(selection()), null);
	assert.equal(resolver.resolve(selection()), null);
	assert.deepEqual(calls, ["native", "wasm"]);
	assert.equal(resolver.resolve(selection("auto", null, 2)), null);
	assert.deepEqual(calls, ["native", "wasm", "native", "wasm"]);
});

test("mode, binding identity, revision and explicit reset invalidate the one-entry memo", () => {
	let loads = 0;
	const resolver = createCssBackendResolver({
		loadNative() {
			loads++;
			return capability();
		},
	});
	const parser = {};
	const first = resolver.resolve(selection("native", parser));
	assert.equal(resolver.resolve(selection("native", parser)), first);
	assert.notEqual(resolver.resolve(selection("native", {})), first);
	assert.equal(loads, 2);
	resolver.resolve(selection("auto", parser));
	resolver.resolve(selection("auto", parser, 2));
	assert.equal(loads, 4);
	resolver.reset();
	resolver.resolve(selection("auto", parser, 2));
	assert.equal(loads, 5);
});

test("reset does not dispose caller-owned capabilities or model instances", () => {
	let disposals = 0;
	class Model {
		dispose() {
			disposals++;
		}
	}
	const binding = {
		cssAbiVersion: 1,
		CompiledStylesV1: Model,
		dispose() {
			disposals++;
		},
	};
	const resolver = createCssBackendResolver();
	const descriptor = resolver.resolve(selection("auto", binding));
	const model = new descriptor.CompiledStylesV1();
	resolver.reset();
	assert.equal(disposals, 0);
	model.dispose();
	binding.dispose();
	assert.equal(disposals, 2);
});

test("invalid mode and incomplete or incompatible ABI descriptors throw explicitly", () => {
	let loads = 0;
	const resolver = createCssBackendResolver({
		loadNative() {
			loads++;
			return capability();
		},
	});
	for (const mode of [undefined, null, "", "jsdom", "WASM"]) {
		assert.throws(
			() => resolver.resolve({ mode, binding: null, revision: 1 }),
			/CSS backend mode/,
		);
	}
	assert.equal(loads, 0);
	for (const binding of [
		{ cssAbiVersion: 2, CompiledStylesV1 },
		{ CompiledStylesV1 },
		{ cssAbiVersion: 1 },
		{ cssAbiVersion: 1, CompiledStylesV1: {} },
	])
		assert.throws(
			() => resolver.resolve(selection("auto", binding)),
			/CSS backend ABI/,
		);
});

test("loader, getter and nested-module errors propagate unchanged without fallback or error caching", () => {
	for (const error of [
		new Error("initialization failed"),
		Object.assign(new Error("Cannot find module 'nested-dependency'"), {
			code: "MODULE_NOT_FOUND",
		}),
		"original thrown value",
	]) {
		let fallback = 0;
		let nativeLoads = 0;
		const resolver = createCssBackendResolver({
			loadNative() {
				nativeLoads++;
				throw error;
			},
			loadWasm() {
				fallback++;
				return capability();
			},
		});
		assert.throws(
			() => resolver.resolve(selection()),
			(value) => value === error,
		);
		assert.throws(
			() => resolver.resolve(selection()),
			(value) => value === error,
		);
		assert.equal(nativeLoads, 2);
		assert.equal(fallback, 0);
		const binding = {
			get cssAbiVersion() {
				throw error;
			},
		};
		assert.throws(
			() => resolver.resolve(selection("auto", binding)),
			(value) => value === error,
		);
	}
	const error = new Error("WASM init");
	const resolver = createCssBackendResolver({
		loadNative() {
			return null;
		},
		loadWasm() {
			throw error;
		},
	});
	assert.throws(
		() => resolver.resolve(selection()),
		(value) => value === error,
	);
	const incompatible = createCssBackendResolver({
		loadNative() {
			return { cssAbiVersion: 0 };
		},
		loadWasm() {
			assert.fail("ABI errors must not fall through");
		},
	});
	assert.throws(() => incompatible.resolve(selection()), /CSS backend ABI/);
});

test("the resolver loads and operates in a realm with no Node or WebAssembly capability", () => {
	const source = readFileSync(
		new URL("../src/runtime/css-backend.mjs", import.meta.url),
		"utf8",
	);
	const program = `const vm = require('node:vm'); (async()=>{const context=vm.createContext({WebAssembly:undefined});const module=new vm.SourceTextModule(${JSON.stringify(source)},{context});await module.link(()=>{throw new Error('unexpected dependency')});await module.evaluate();const resolver=module.namespace.createCssBackendResolver();if(resolver.resolve({mode:'auto',binding:null,revision:1})!==null)throw new Error('unexpected capability');if(vm.runInContext('typeof process',context)!=='undefined')throw new Error('Node leaked');if(vm.runInContext('typeof WebAssembly',context)!=='undefined')throw new Error('WASM leaked');})().catch(error=>{console.error(error);process.exitCode=1});`;
	const result = spawnSync(
		process.execPath,
		["--experimental-vm-modules", "-e", program],
		{ encoding: "utf8" },
	);
	assert.equal(result.status, 0, result.stderr);
});
test('optional scene capability is retained, read once and never substituted across modes', () => {
  const measureSceneV1 = () => 'scene';
  let reads = 0;
  const binding = { ...capability(), get measureSceneV1() { reads++; return measureSceneV1; } };
  const resolver = createCssBackendResolver({loadNative:()=>binding,loadWasm(){throw Error('not wasm');}});
  const first = resolver.resolve({mode:'native',binding:null,revision:0});
  assert.equal(first.measureSceneV1,measureSceneV1);
  assert.equal(reads,1);
  assert.equal(resolver.resolve({mode:'native',binding:null,revision:0}),first);
  assert.equal(reads,1);
  for(const measureSceneV1 of [null,1,{}]) {
    assert.throws(()=>createCssBackendResolver().resolve({mode:'auto',binding:{...capability(),measureSceneV1},revision:0}),TypeError);
  }
  const error = {};
  assert.throws(()=>createCssBackendResolver().resolve({mode:'auto',binding:{...capability(),get measureSceneV1(){throw error;}},revision:0}),e=>e===error);
});
test('optional canonical UA source function is discovered lazily without extending minimum binding',()=>{
  let reads=0,calls=0;
  const cssUserAgentSourceV1=()=>{calls++;return JSON.stringify({abiVersion:1,text:[65],fullscreenText:[66]});};
  const binding={...capability(),get cssUserAgentSourceV1(){reads++;return cssUserAgentSourceV1;}};
  const resolver=createCssBackendResolver({loadWasm:()=>binding,loadNative(){throw Error('forced wasm');}});
  const selected=resolver.resolve({mode:'wasm',binding:null,revision:0});
  assert.equal(selected.cssUserAgentSourceV1,cssUserAgentSourceV1);assert.equal(reads,1);assert.equal(calls,0);
  assert.equal(selected.cssUserAgentSourceV1(),JSON.stringify({abiVersion:1,text:[65],fullscreenText:[66]}));assert.equal(calls,1);
  for(const cssUserAgentSourceV1 of [null,{},1])assert.throws(()=>createCssBackendResolver().resolve({mode:'auto',binding:{...capability(),cssUserAgentSourceV1},revision:0}),TypeError);
  const error={};assert.throws(()=>createCssBackendResolver().resolve({mode:'auto',binding:{...capability(),get cssUserAgentSourceV1(){throw error;}},revision:0}),value=>value===error);
});
