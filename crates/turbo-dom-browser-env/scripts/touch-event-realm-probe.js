(() => {
	const fields = [
		"touches",
		"targetTouches",
		"changedTouches",
		"altKey",
		"metaKey",
		"ctrlKey",
		"shiftKey",
	];
	const own = fields.map((name) => {
		const get = Object.getOwnPropertyDescriptor(TouchEvent.prototype, name).get;
		try {
			const value = get.call(foreignTouchEvent);
			return [
				name,
				name.endsWith("Touches") || name === "touches"
					? [
							value.length,
							value.item(0).identifier,
							[...value][0] === foreignTouch,
						]
					: value,
			];
		} catch (error) {
			return [name, error.name];
		}
	});
	const invalid = fields.map((name) => {
		const get = Object.getOwnPropertyDescriptor(TouchEvent.prototype, name).get;
		return [{}, foreignTouch, foreignTouchEvent.touches, new Event("x")].map(
			(value) => {
				try {
					get.call(value);
					return "accepted";
				} catch (error) {
					return error.name;
				}
			},
		);
	});
	let listBrand;
	try {
		const list = Object.getOwnPropertyDescriptor(
			TouchEvent.prototype,
			"touches",
		).get.call(foreignTouchEvent);
		listBrand = [
			Object.getOwnPropertyDescriptor(TouchList.prototype, "length").get.call(
				list,
			),
			TouchList.prototype.item.call(list, 0) === foreignTouch,
		];
	} catch (error) {
		listBrand = error.name;
	}
	const descriptor = Object.getOwnPropertyDescriptor(
		TouchEvent.prototype,
		"touches",
	);
	return { own, invalid, listBrand, readonly: descriptor.set === undefined };
})();
