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
	global.__turboDomWorker = {
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
	};
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
	// Minimum structuredClone stub: a JSON round trip handles the common
	// page -> worker cases (plain data, nested objects/arrays). turbo-surf wires
	// the real HTML structured clone through its message-port machinery, where
	// typed arrays, Maps/Sets and transferables cross realms correctly.
	global.structuredClone = function structuredClone(value) {
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
