(() => {
	const touch = new Touch({ identifier: 1, target: document.body });
	const indices = [
		"0",
		"1",
		"4294967294",
		"4294967295",
		"4294967296",
		"-1",
		"01",
	];
	const keys = indices.map((key) => {
		const list = new TouchEvent("x", { touches: [touch] }).touches;
		const other = new Touch({ identifier: 2, target: document.body });
		const set = Reflect.set(list, key, other);
		const value = list[key] === touch;
		const deleted = Reflect.deleteProperty(list, key);
		const defined = Reflect.defineProperty(list, key, {
			value: other,
			configurable: true,
		});
		return [
			key,
			set,
			value,
			deleted,
			defined,
			list[key] === touch,
			list[key] === other,
			Reflect.getOwnPropertyDescriptor(list, key) && {
				configurable: Reflect.getOwnPropertyDescriptor(list, key).configurable,
				writable: Reflect.getOwnPropertyDescriptor(list, key).writable,
				enumerable: Reflect.getOwnPropertyDescriptor(list, key).enumerable,
			},
		];
	});
	const descriptors = [
		{ value: touch },
		{ writable: true },
		{ writable: false },
		{ configurable: false },
		{ enumerable: false },
		{
			get() {
				return touch;
			},
		},
	].map((descriptor) => {
		const list = new TouchEvent("x", { touches: [touch] }).touches;
		let result;
		try {
			result = Reflect.defineProperty(list, "0", descriptor);
		} catch (e) {
			result = e.name;
		}
		const after = Object.getOwnPropertyDescriptor(list, "0");
		return [result, after.writable, after.configurable, after.enumerable];
	});
	const list = new TouchEvent("x", { touches: [touch] }).touches;
	const enumeration = [
		Object.keys(list),
		Reflect.ownKeys(list),
		"0" in list,
		"1" in list,
	];
	return { keys, descriptors, enumeration };
})();
