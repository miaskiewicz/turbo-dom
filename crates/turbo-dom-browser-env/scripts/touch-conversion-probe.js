(() => {
	const trace = [];
	const target = document.body;
	const init = new Proxy(
		{ identifier: 7, target },
		{
			get(object, key) {
				trace.push(String(key));
				return Reflect.get(object, key);
			},
		},
	);
	const touch = new Touch(init);
	const boxed = [];
	for (const field of ["identifier", "clientX", "radiusX"]) {
		try {
			new Touch({ identifier: 1, target, [field]: Object(1n) });
			boxed.push("accepted");
		} catch (error) {
			boxed.push(error.name);
		}
	}
	const second = [];
	const changing = {
		get identifier() {
			second.push("identifier");
			return second.length === 1 ? 4 : undefined;
		},
		get target() {
			second.push("target");
			return target;
		},
	};
	const changed = new Touch(changing);
	return {
		trace,
		identifier: touch.identifier,
		boxed,
		changing: [changed.identifier, second],
	};
})();
