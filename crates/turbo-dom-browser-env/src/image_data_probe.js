(() => {
	const attempt = (fn) => {
		try {
			return fn();
		} catch (e) {
			return { error: e.name };
		}
	};
	const describe = (image) => ({
		width: image.width,
		height: image.height,
		colorSpace: image.colorSpace,
		pixelFormat: image.pixelFormat,
		data: Array.from(image.data),
		tag: Object.prototype.toString.call(image),
		instance: image instanceof ImageData,
		own: Object.keys(image),
	});
	const cases = {};
	for (const [name, args] of [
		["dimensions", [2, 1]],
		["fractional", [2.9, 1.8]],
		["modulo", [4294967297, 1]],
		["zeroWidth", [0, 1]],
		["zeroHeight", [1, 0]],
		["nan", [NaN, 1]],
		["infinity", [Infinity, 1]],
		["missing", [1]],
		["array", [new Uint8ClampedArray([1, 2, 3, 255]), 1, 1]],
		["inferHeight", [new Uint8ClampedArray(8), 1]],
		["short", [new Uint8ClampedArray(3), 1]],
		["empty", [new Uint8ClampedArray(0), 1]],
		["mismatch", [new Uint8ClampedArray(8), 1, 1]],
		["badWidth", [new Uint8ClampedArray(8), 3]],
		["wrongArray", [new Uint8Array(4), 1]],
		["plain", [[1, 2, 3, 4], 1]],
		["null", [null, 1]],
		["p3", [1, 1, { colorSpace: "display-p3" }]],
		["badSpace", [1, 1, { colorSpace: "no" }]],
		["nullSettings", [1, 1, null]],
		["badSettings", [1, 1, 3]],
		["badFormat", [1, 1, { pixelFormat: "no" }]],
		[
			"arrayWrongFormat",
			[new Uint8ClampedArray(4), 1, 1, { pixelFormat: "rgba-float16" }],
		],
	])
		cases[name] = attempt(() => describe(new ImageData(...args)));
	if (typeof Float16Array === "function") {
		cases.floatAllocated = attempt(() =>
			describe(new ImageData(1, 1, { pixelFormat: "rgba-float16" })),
		);
		cases.floatArray = attempt(() =>
			describe(
				new ImageData(new Float16Array([1, 0.5, 0, 1]), 1, 1, {
					pixelFormat: "rgba-float16",
				}),
			),
		);
		cases.floatDefault = attempt(() =>
			describe(new ImageData(new Float16Array(4), 1)),
		);
	}
	if (typeof ArrayBuffer.prototype.resize === "function")
		cases.resizable = attempt(() =>
			describe(
				new ImageData(
					new Uint8ClampedArray(new ArrayBuffer(4, { maxByteLength: 8 })),
					1,
				),
			),
		);
	const bytes = new Uint8ClampedArray([1, 2, 3, 255]);
	cases.identity = attempt(() => {
		const image = new ImageData(bytes, 1);
		bytes[0] = 9;
		return [image.data === bytes, image.data[0]];
	});
	cases.call = attempt(() => ImageData(1, 1));
	cases.interface = {
		length: ImageData.length,
		keys: Object.keys(ImageData.prototype),
		tag: Object.prototype.toString.call(ImageData.prototype),
		prototypeWritable: Object.getOwnPropertyDescriptor(ImageData, "prototype")
			.writable,
	};
	cases.dataDescriptor = attempt(() => {
		const i = new ImageData(1, 1),
			d = Object.getOwnPropertyDescriptor(i, "data");
		return {
			get: typeof d.get,
			set: typeof d.set,
			writable: d.writable,
			enumerable: d.enumerable,
			configurable: d.configurable,
		};
	});
	cases.borrow = attempt(() =>
		Object.getOwnPropertyDescriptor(ImageData.prototype, "width").get.call({
			width: 1,
		}),
	);
	cases.inheritedBrand = attempt(() =>
		Object.getOwnPropertyDescriptor(ImageData.prototype, "width").get.call(
			Object.create(new ImageData(1, 1)),
		),
	);
	cases.proxyBrand = attempt(() =>
		Object.getOwnPropertyDescriptor(ImageData.prototype, "width").get.call(
			new Proxy(new ImageData(1, 1), {}),
		),
	);
	cases.subclass = attempt(() => {
		class I extends ImageData {}
		const i = new I(1, 1);
		return [i instanceof I, i instanceof ImageData, i.width];
	});
	const ctx = document.createElement("canvas").getContext("2d");
	cases.created = attempt(() => describe(ctx.createImageData(2, 1)));
	cases.createdClone = attempt(() => {
		const src = new ImageData(bytes, 1);
		const clone = ctx.createImageData(src);
		return [
			clone instanceof ImageData,
			clone.data !== src.data,
			Array.from(clone.data),
		];
	});
	cases.createdNegative = attempt(() => describe(ctx.createImageData(-2, -1)));
	cases.put = attempt(() => {
		const i = new ImageData(new Uint8ClampedArray([100, 20, 40, 255]), 1);
		ctx.putImageData(i, 0, 0);
		i.data[0] = 5;
		return describe(ctx.getImageData(0, 0, 1, 1));
	});
	cases.contextDimensions = [Infinity, NaN, 4294967297].map((value) => [
		attempt(() => ctx.createImageData(value, 1)),
		attempt(() => ctx.getImageData(value, 0, 1, 1)),
		attempt(() => ctx.getImageData(0, 0, value, 1)),
	]);
	cases.putNonFinite = [Infinity, NaN, -Infinity, 1e30].map((value) =>
		attempt(() => ctx.putImageData(new ImageData(1, 1), value, 0)),
	);
	cases.dirtyNonFinite = attempt(() =>
		ctx.putImageData(new ImageData(1, 1), 0, 0, 0, 0, Infinity, 1),
	);
	cases.fakePut = attempt(() =>
		ctx.putImageData(
			{ data: new Uint8ClampedArray(4), width: 1, height: 1 },
			0,
			0,
		),
	);
	cases.view = attempt(() => {
		const a = new Uint8ClampedArray(new ArrayBuffer(12), 4, 4);
		a.set([11, 12, 13, 255]);
		const i = new ImageData(a, 1);
		return [i.data === a, describe(i)];
	});
	cases.spoof = attempt(() => {
		const a = new Uint8ClampedArray([8, 9, 10, 255]);
		Object.defineProperty(a, Symbol.toStringTag, {
			value: "not a typed array",
		});
		return describe(new ImageData(a, 1));
	});
	cases.accessors = [
		"width",
		"height",
		"data",
		"colorSpace",
		"pixelFormat",
	].map((key) => {
		const d = Object.getOwnPropertyDescriptor(ImageData.prototype, key);
		return [
			key,
			d.get.name,
			d.get.length,
			d.enumerable,
			d.configurable,
			attempt(() => d.get.call({})),
		];
	});
	cases.methodShape = ["createImageData", "getImageData", "putImageData"].map(
		(key) => [
			key,
			ctx[key].name,
			ctx[key].length,
			attempt(() => ctx[key].call({}, 1, 1, 1, 1)),
		],
	);
	cases.cloneFloat = attempt(() =>
		describe(
			ctx.createImageData(new ImageData(1, 1, { pixelFormat: "rgba-float16" })),
		),
	);
	cases.cloneP3 = attempt(() =>
		describe(
			ctx.createImageData(new ImageData(1, 1, { colorSpace: "display-p3" })),
		),
	);
	cases.borrowContext = attempt(() => {
		const a = document.createElement("canvas").getContext("2d"),
			b = document.createElement("canvas").getContext("2d");
		a.putImageData.call(
			b,
			new ImageData(new Uint8ClampedArray([9, 10, 11, 255]), 1),
			0,
			0,
		);
		return Array.from(b.getImageData(0, 0, 1, 1).data);
	});
	cases.alpha = [0, 2, 6, 77, 128, 255].map((alpha) => {
		ctx.putImageData(
			new ImageData(new Uint8ClampedArray([201, 77, 13, alpha]), 1),
			3,
			3,
		);
		return Array.from(ctx.getImageData(3, 3, 1, 1).data);
	});
	cases.dirty = attempt(() => {
		const i = new ImageData(
			new Uint8ClampedArray([7, 8, 9, 255, 20, 30, 40, 255]),
			2,
		);
		ctx.putImageData(i, 0, 0, 1, 0, 1, 1);
		return Array.from(ctx.getImageData(0, 0, 2, 1).data);
	});
	return cases;
})();
