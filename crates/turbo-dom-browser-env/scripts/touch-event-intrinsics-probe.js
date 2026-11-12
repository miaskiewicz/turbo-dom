(() => {
	const touch = new Touch({ identifier: 7, target: document.body });
	const results = [];
	for (const name of ["kind", "touches", "targetTouches", "changedTouches"]) {
		const previous = Object.getOwnPropertyDescriptor(Object.prototype, name);
		const hits = [];
		let result;
		try {
			Object.defineProperty(Object.prototype, name, {
				configurable: true,
				get() {
					hits.push("get");
					throw new Error("poisoned " + name);
				},
				set() {
					hits.push("set");
					throw new Error("poisoned " + name);
				},
			});
			const event = new TouchEvent("x", {
				touches: [touch],
				targetTouches: [touch],
				changedTouches: [touch],
				ctrlKey: true,
			});
			result = [
				event.touches[0] === touch,
				event.targetTouches[0] === touch,
				event.changedTouches[0] === touch,
				event.ctrlKey,
			];
		} catch (error) {
			result = error.name;
		} finally {
			if (previous) Object.defineProperty(Object.prototype, name, previous);
			else delete Object.prototype[name];
		}
		results.push([name, hits, result]);
	}
	return results;
})();
