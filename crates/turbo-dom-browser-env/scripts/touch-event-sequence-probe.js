(() => {
	const trace = [];
	const init = new Proxy(
		{},
		{
			get(object, key) {
				if (
					typeof key === "string" &&
					["touches", "targetTouches", "changedTouches"].includes(key)
				)
					trace.push(key);
				return Reflect.get(object, key);
			},
		},
	);
	new TouchEvent("x", init);
	const conversion = [];
	const one = new Touch({ identifier: 1, target: document.body });
	const data = {};
	for (const key of ["touches", "targetTouches", "changedTouches"])
		Object.defineProperty(data, key, {
			get() {
				conversion.push(key);
				return {
					[Symbol.iterator]() {
						conversion.push(key + ":iterator");
						return [one][Symbol.iterator]();
					},
				};
			},
		});
	const event = new TouchEvent("x", data);
	const failureTrace = [];
	const failing = {};
	for (const key of ["touches", "targetTouches", "changedTouches"])
		Object.defineProperty(failing, key, {
			get() {
				failureTrace.push(key);
				if (key === "changedTouches") throw new RangeError("first");
				return [];
			},
		});
	let failure;
	try {
		new TouchEvent("x", failing);
	} catch (error) {
		failure = error.name;
	}
	return {
		trace,
		conversion,
		failureTrace,
		failure,
		lengths: [
			event.touches.length,
			event.targetTouches.length,
			event.changedTouches.length,
		],
	};
})();
