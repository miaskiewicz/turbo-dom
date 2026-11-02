(async () => {
	const frame = document.createElement("iframe");
	const loaded = new Promise((resolve) => {
		frame.onload = resolve;
	});
	frame.srcdoc =
		'<!doctype html><body><span id=leaf style="font-size:10px">child</span>';
	document.body.append(frame);
	await loaded;
	const child = frame.contentWindow,
		doc = child.document,
		leaf = doc.getElementById("leaf");
	document.documentElement.style.cssText =
		"font-family:serif;font-size:24px;line-height:2";
	doc.documentElement.style.cssText =
		"font-family:monospace;font-size:18px;line-height:1.5";
	const read = (style) => [style.font, style.fontSize, style.lineHeight];
	const retained = getComputedStyle(leaf),
		childRetained = child.getComputedStyle(doc.body);
	const before = {
		borrowed: read(retained),
		own: read(child.getComputedStyle(leaf)),
	};
	document.body.append(document.adoptNode(leaf));
	const adopted = {
		retained: read(retained),
		own: read(getComputedStyle(leaf)),
	};
	frame.remove();
	const retired = {
		retained: read(childRetained),
		fresh: read(getComputedStyle(doc.body)),
		defaultView: doc.defaultView === null,
	};
	return { before, adopted, retired };
})();
