// turbo-dom#385: the DedicatedWorkerGlobalScope bootstrap, installed onto a
// worker-marked context by `install_worker_global_scope`. The embedder
// (turbo-surf #203) wires three hooks through `__turboDomWorker` after the
// bootstrap runs, and delivers incoming messages via the Rust helper
// `dispatch_worker_message` -> `__turboDomWorker.dispatch`.
(function () {
	"use strict";
	const global = globalThis;
	// Review R2 V8 M1: capture every intrinsic the bootstrap uses at install
	// time, before any page script can reassign `globalThis.Set` or poison
	// `Array.prototype[@@iterator]`. The main-realm binding does this via
	// captured_intrinsics.js; the worker bootstrap now does the same.
	// turbo-dom#428: capture the native structured-clone binding BEFORE any
	// page script can tamper with `globalThis`. The binding is installed by
	// `worker_scope::install` right before this bootstrap runs; we hold it
	// in a closure and delete the global slot so the surface stays Chrome-shaped.
	const nativeStructuredClone = global.__turboDomStructuredClone;
	delete global.__turboDomStructuredClone;
	const SetType = global.Set;
	const MapType = global.Map;
	const SetAdd = SetType.prototype.add;
	const SetDel = SetType.prototype.delete;
	const SetHas = SetType.prototype.has;
	const SetSize = Object.getOwnPropertyDescriptor(SetType.prototype, "size").get;
	const SetForEach = SetType.prototype.forEach;
	const MapGet = MapType.prototype.get;
	const MapSet = MapType.prototype.set;
	const apply = Reflect.apply;
	Object.defineProperty(global, "self", {
		value: global,
		writable: true,
		configurable: true,
	});
	const listeners = new MapType();
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
			const list = apply(MapGet, listeners, [kind]);
			if (list && apply(SetSize, list, [])) {
				fired = true;
				// Snapshot via captured Set.prototype.forEach — avoids
				// `[...list]` which resolves Array.prototype[@@iterator]
				// at call time (R2 V8 M1).
				const snapshot = [];
				apply(SetForEach, list, [(fn) => {
					snapshot[snapshot.length] = fn;
				}]);
				for (let index = 0; index < snapshot.length; index++) {
					try {
						snapshot[index].call(global, event);
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
	// turbo-dom#428: real structured-clone semantics via V8's
	// ValueSerializer / ValueDeserializer (captured native binding above).
	// Date / Map / Set / RegExp / BigInt / typed arrays / cycles round-trip
	// faithfully; non-cloneable values (functions, symbols, WeakMap, ...)
	// throw DataCloneError from the native side. turbo-surf still wires its
	// own message-port path for transferables.
	if (typeof nativeStructuredClone === "function") {
		global.structuredClone = function structuredClone(value) {
			if (arguments.length === 0) {
				throw new TypeError(
					"structuredClone: 1 argument required, but only 0 present",
				);
			}
			return nativeStructuredClone(value);
		};
	} else {
		// Fallback (defensive — the native binding is installed in every
		// worker context this crate creates). If it is missing, structured
		// clone throws DataCloneError so a page cannot silently disagree with
		// Chrome.
		global.structuredClone = function structuredClone() {
			const err = new Error(
				"structuredClone: native binding unavailable",
			);
			err.name = "DataCloneError";
			throw err;
		};
	}
	global.addEventListener = function addEventListener(type, listener) {
		if (typeof listener !== "function") return;
		let list = apply(MapGet, listeners, [type]);
		if (!list) {
			list = new SetType();
			apply(MapSet, listeners, [type, list]);
		}
		apply(SetAdd, list, [listener]);
	};
	global.removeEventListener = function removeEventListener(type, listener) {
		const list = apply(MapGet, listeners, [type]);
		if (list) apply(SetDel, list, [listener]);
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
