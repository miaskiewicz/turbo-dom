(() => {
	const canvas = document.createElement("canvas");
	const output = {
		defaults: [canvas.width, canvas.height],
		attributes: [],
		assigned: [],
		resets: [],
	};
	for (const text of [
		"",
		"-1",
		"12px",
		"+8",
		" 9 ",
		"1.5",
		"4294967296",
		"9999999999999999999999",
	]) {
		canvas.setAttribute("width", text);
		output.attributes.push([text, canvas.width, canvas.getAttribute("width")]);
	}
	for (const value of [
		0,
		-1,
		3.9,
		2147483647,
		2147483648,
		4294967295,
		4294967297,
		Number.NaN,
		Number.POSITIVE_INFINITY,
		null,
		"7",
	]) {
		canvas.width = value;
		output.assigned.push([
			String(value),
			canvas.width,
			canvas.getAttribute("width"),
		]);
	}
	const actions = [
		(cv) => {
			const value = cv.width;
			cv.width = value;
		},
		(cv) => {
			const value = cv.height;
			cv.height = value;
		},
		(cv) => cv.setAttribute("width", "2"),
		(cv) => cv.setAttributeNS(null, "width", "2"),
		(cv) => cv.removeAttribute("width"),
		(cv) => {
			cv.getAttributeNode("width").value = "2";
		},
		(cv) => {
			const a = document.createAttribute("height");
			a.value = "2";
			cv.setAttributeNode(a);
		},
		(cv) => cv.setAttributeNS("urn:other", "width", "2"),
		(cv) => cv.removeAttributeNS(null, "height"),
		(cv) => cv.removeAttributeNode(cv.getAttributeNode("width")),
		(cv) => {
			cv.getAttributeNode("height").nodeValue = "2";
		},
		(cv) => {
			cv.getAttributeNode("height").textContent = "2";
		},
	];
	for (let i = 0; i < actions.length; i++) {
		const cv = document.createElement("canvas");
		cv.width = 2;
		cv.height = 2;
		const ctx = cv.getContext("2d");
		ctx.fillStyle = "red";
		ctx.lineWidth = 9;
		ctx.setLineDash([1, 2]);
		ctx.save();
		ctx.putImageData(
			new ImageData(new Uint8ClampedArray([12, 34, 56, 255]), 1),
			0,
			0,
		);
		actions[i](cv);
		ctx.restore();
		output.resets.push([
			i,
			cv.width,
			cv.height,
			ctx.fillStyle,
			ctx.lineWidth,
			ctx.getLineDash(),
			Array.from(ctx.getImageData(0, 0, 1, 1).data),
		]);
	}
	const clean = document.createElement("canvas");
	const cleanCtx = clean.getContext("2d");
	cleanCtx.putImageData(
		new ImageData(new Uint8ClampedArray([12, 34, 56, 255]), 1),
		0,
		0,
	);
	clean.removeAttribute("width");
	clean.removeAttributeNS(null, "height");
	output.missingRemoval = Array.from(cleanCtx.getImageData(0, 0, 1, 1).data);
	output.qualifiedNamespace = [];
	for (const action of [
		(cv) => cv.setAttribute("width", "3"),
		(cv) => cv.removeAttribute("width"),
	]) {
		const cv = document.createElement("canvas");
		cv.setAttributeNS("urn:other", "width", "2");
		const ctx = cv.getContext("2d");
		ctx.putImageData(
			new ImageData(new Uint8ClampedArray([12, 34, 56, 255]), 1),
			0,
			0,
		);
		action(cv);
		output.qualifiedNamespace.push([
			cv.width,
			Array.from(ctx.getImageData(0, 0, 1, 1).data),
		]);
	}
	output.ownOverrides = [];
	for (const [name, value] of [
		["lineWidth", 17],
		["globalAlpha", 0.25],
		["font", "23px serif"],
	]) {
		const cv = document.createElement("canvas");
		const ctx = cv.getContext("2d");
		Object.defineProperty(ctx, name, {
			value,
			writable: true,
			configurable: true,
		});
		cv.width = 300;
		output.ownOverrides.push([name, ctx[name]]);
	}
	output.descriptors = ["width", "height"].map((name) => {
		const d = Object.getOwnPropertyDescriptor(
			HTMLCanvasElement.prototype,
			name,
		);
		return [
			name,
			d.enumerable,
			d.configurable,
			d.get.name,
			d.set.name,
			d.get.length,
			d.set.length,
		];
	});
	return output;
})();
