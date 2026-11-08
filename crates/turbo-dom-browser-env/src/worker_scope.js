// turbo-dom#385: the DedicatedWorkerGlobalScope bootstrap, installed onto a
// worker-marked context by `install_worker_global_scope`. The embedder
// (turbo-surf #203) wires three hooks through `__turboDomWorker` after the
// bootstrap runs, and delivers incoming messages via the Rust helper
// `dispatch_worker_message` -> `__turboDomWorker.dispatch`.
(function () {
	"use strict";
	const global = globalThis;
	Object.defineProperty(global, "self", {
		value: global,
		writable: true,
		configurable: true,
	});
	const listeners = new Map();
	const handlers = { message: null, messageerror: null, error: null };
	let postHook = null;
	let closeHook = null;
	let importHook = null;
	// Review R1 M4: lock the embedder-facing namespace so page JS in the
	// worker cannot overwrite the whole slot (whole-takeover) nor
	// replace individual hooks (per-method intercept of incoming
	// messages). `Object.freeze` makes the inner methods non-writable +
	// non-configurable, and the `defineProperty` below seats it
	// non-writable + non-configurable + non-enumerable on `globalThis`.
	// Chrome's real `DedicatedWorkerGlobalScope` has no such namespace;
	// keeping it inert is extra defense for turbo-surf's trusted-code
	// model.
	const turboDomWorker = Object.freeze({
		setPostMessage(fn) {
			postHook = typeof fn === "function" ? fn : null;
		},
		setClose(fn) {
			closeHook = typeof fn === "function" ? fn : null;
		},
		setImportScripts(fn) {
			importHook = typeof fn === "function" ? fn : null;
		},
		dispatch(kind, event) {
			let fired = false;
			const slot = handlers[kind];
			if (slot) {
				fired = true;
				try {
					slot.call(global, event);
				} catch (_err) {}
			}
			const list = listeners.get(kind);
			if (list && list.size) {
				fired = true;
				for (const fn of [...list]) {
					try {
						fn.call(global, event);
					} catch (_err) {}
				}
			}
			return fired;
		},
	});
	Object.defineProperty(global, "__turboDomWorker", {
		value: turboDomWorker,
		writable: false,
		configurable: false,
		enumerable: false,
	});
	global.postMessage = function postMessage(message, transfer) {
		if (postHook) return postHook.call(global, message, transfer);
	};
	global.close = function close() {
		if (closeHook) return closeHook.call(global);
	};
	global.importScripts = function importScripts(...urls) {
		if (!importHook)
			throw new TypeError(
				"importScripts: no loader registered on this worker context",
			);
		return importHook.apply(global, urls);
	};
	// Review R1 M5: a POJO-only stub that THROWS DataCloneError on
	// anything it cannot faithfully round-trip. The real
	// `structuredClone` honours Date / Map / Set / BigInt / typed
	// arrays / cycles / undefined / Infinity / NaN; silently dropping
	// them (what the previous JSON round-trip did) made a page
	// feature-detection succeed while behaviour disagreed with Chrome.
	// turbo-surf wires the real HTML structured clone through its
	// message-port machinery; this stub is just a well-behaved marker.
	// Full structured-clone semantics is turbo-dom#422 (follow-up).
	function dataCloneError(detail) {
		const err = new Error(`${detail} is not cloneable (structuredClone stub)`);
		err.name = "DataCloneError";
		return err;
	}
	function assertCloneable(value, seen) {
		if (value === null) return;
		const t = typeof value;
		if (t === "string" || t === "boolean") return;
		if (t === "number") {
			if (Number.isFinite(value)) return;
			throw dataCloneError(String(value));
		}
		if (t === "undefined") throw dataCloneError("undefined");
		if (t === "bigint") throw dataCloneError("BigInt");
		if (t === "symbol") throw dataCloneError("Symbol");
		if (t === "function") throw dataCloneError("Function");
		// From here on `value` is an object. Reject non-POJO built-ins
		// whose JSON round-trip would lose type.
		if (Array.isArray(value)) {
			if (seen.has(value)) throw dataCloneError("cyclic reference");
			seen.add(value);
			for (let i = 0; i < value.length; i++) assertCloneable(value[i], seen);
			return;
		}
		const tag = Object.prototype.toString.call(value);
		if (tag !== "[object Object]") throw dataCloneError(tag);
		const proto = Object.getPrototypeOf(value);
		if (proto !== null && proto !== Object.prototype) {
			throw dataCloneError("non-POJO object");
		}
		if (seen.has(value)) throw dataCloneError("cyclic reference");
		seen.add(value);
		for (const key of Object.keys(value)) assertCloneable(value[key], seen);
	}
	global.structuredClone = function structuredClone(value) {
		assertCloneable(value, new Set());
		return JSON.parse(JSON.stringify(value));
	};
	global.addEventListener = function addEventListener(type, listener) {
		if (typeof listener !== "function") return;
		let list = listeners.get(type);
		if (!list) {
			list = new Set();
			listeners.set(type, list);
		}
		list.add(listener);
	};
	global.removeEventListener = function removeEventListener(type, listener) {
		const list = listeners.get(type);
		if (list) list.delete(listener);
	};
	global.dispatchEvent = function dispatchEvent(event) {
		return global.__turboDomWorker.dispatch(event && event.type, event);
	};
	for (const key of ["onmessage", "onmessageerror", "onerror"]) {
		const slot = key.slice(2);
		Object.defineProperty(global, key, {
			get() {
				return handlers[slot];
			},
			set(value) {
				handlers[slot] = typeof value === "function" ? value : null;
			},
			configurable: true,
		});
	}
	function illegalConstructor() {
		throw new TypeError("Illegal constructor");
	}
	function brand(name, parent) {
		const fn = function BrandCtor() {
			illegalConstructor();
		};
		Object.defineProperty(fn, "name", { value: name });
		if (parent) Object.setPrototypeOf(fn.prototype, parent.prototype);
		return fn;
	}
	const EventTarget = brand("EventTarget", null);
	const WorkerGlobalScope = brand("WorkerGlobalScope", EventTarget);
	const DedicatedWorkerGlobalScope = brand(
		"DedicatedWorkerGlobalScope",
		WorkerGlobalScope,
	);
	Object.defineProperty(DedicatedWorkerGlobalScope.prototype, "PERSISTENT", {
		value: 1,
	});
	Object.defineProperty(DedicatedWorkerGlobalScope.prototype, "TEMPORARY", {
		value: 0,
	});
	for (const [name, ctor] of [
		["EventTarget", EventTarget],
		["WorkerGlobalScope", WorkerGlobalScope],
		["DedicatedWorkerGlobalScope", DedicatedWorkerGlobalScope],
	]) {
		Object.defineProperty(global, name, {
			value: ctor,
			writable: true,
			configurable: true,
		});
	}
	// Minimum event brands: a full IDL-faithful install is a follow-up
	// (DedicatedWorkerGlobalScope IDL members beyond the core hook surface).
	for (const name of [
		"Event",
		"MessageEvent",
		"ErrorEvent",
		"PromiseRejectionEvent",
	]) {
		if (typeof global[name] === "undefined")
			global[name] = brand(name, EventTarget);
	}
})();
