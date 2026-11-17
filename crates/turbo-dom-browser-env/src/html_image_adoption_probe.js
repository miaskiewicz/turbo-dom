async (peer, beginLoad) => {
	const image = peer.document.createElement("img");
	document.body.append(image);
	const root = HTMLImageElement.prototype,
		child = peer.HTMLImageElement.prototype;
	const read = (prototype, name) =>
		Object.getOwnPropertyDescriptor(prototype, name).get.call(image);
	const loading = beginLoad(image, true);
	const a = root.decode.call(image),
		b = child.decode.call(image);
	const promises = [
		[a instanceof Promise, a instanceof peer.Promise],
		[b instanceof Promise, b instanceof peer.Promise],
	];
	await Promise.all([loading, a, b]);
	const result = {
		owner: image.ownerDocument === document,
		originalPrototype: Object.getPrototypeOf(image) === child,
		root: [
			read(root, "complete"),
			read(root, "naturalWidth"),
			read(root, "naturalHeight"),
		],
		child: [
			read(child, "complete"),
			read(child, "naturalWidth"),
			read(child, "naturalHeight"),
		],
		currentSrcEqual: read(root, "currentSrc") === read(child, "currentSrc"),
		ownState: Object.hasOwn(image, "naturalWidth"),
		promises,
	};
	const broken = beginLoad(image, false);
	const rejected = [root.decode.call(image), child.decode.call(image)];
	await broken;
	result.errors = await Promise.all(
		rejected.map((promise, index) =>
			promise.then(
				() => "accepted",
				(error) => {
					return [
						error.name,
						error instanceof DOMException,
						error instanceof peer.DOMException,
						error instanceof Error,
						error instanceof peer.Error,
					];
				},
			),
		),
	);
	image.remove();
	return result;
};
