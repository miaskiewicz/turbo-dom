(async () => {
	const cases = [
		["auto", ""],
		["opaque-one", "opacity:1"],
		["static-z", "position:static;z-index:0"],
		["zero", "z-index:0"],
		["opacity-half", "opacity:.5"],
		["identity-transform", "transform:translateX(0px)"],
		["fixed", "position:fixed"],
		["sticky", "position:sticky"],
		["isolation", "isolation:isolate"],
		["contain", "contain:paint"],
		["will-change", "will-change:opacity"],
		["css-opacity", "", "css"],
		["css-important", "opacity:1!important", "css"],
		["waapi-opacity", "", "waapi"],
		["waapi-important", "opacity:1!important", "waapi"],
		["css-width", "", "width"],
	];
	const results = [];
	for (const [name, subjectStyle, effect] of cases) {
		const frame = document.createElement("iframe");
		const ready = new Promise((resolve) => (frame.onload = resolve));
		const css = `html,body{margin:0}#subject{position:relative;width:100px;height:100px;${subjectStyle}}#inner{position:absolute;inset:0;z-index:100}#cover{position:absolute;left:0;top:0;width:100px;height:100px;z-index:1}`;
		const animation =
			effect === "css"
				? "#subject{animation:a 1s linear both}@keyframes a{from{opacity:1}to{opacity:1}}"
				: effect === "width"
					? "#subject{animation:a 1s linear both}@keyframes a{from{width:100px}to{width:100px}}"
					: "";
		const body =
			'<div id="subject"><div id="inner"></div></div><div id="cover"></div>';
		frame.srcdoc = `<!doctype html><style>${css}${animation}</style>${body}`;
		document.body.append(frame);
		await ready;
		const win = frame.contentWindow,
			doc = frame.contentDocument;
		if (effect === "waapi")
			doc
				.querySelector("#subject")
				.animate([{ opacity: 1 }, { opacity: 1 }], {
					duration: 1000,
					fill: "both",
				});
		for (const effect of doc.getAnimations()) {
			effect.pause();
			effect.currentTime = 500;
		}
		const rects = ["subject", "inner", "cover"].map((id) => {
			const r = doc.getElementById(id).getBoundingClientRect();
			return [r.x, r.y, r.width, r.height];
		});
		results.push({
			name,
			subjectStyle,
			effect: effect || null,
			css,
			body,
			hit: doc.elementFromPoint(50, 50)?.id,
			rects,
			opacity: win.getComputedStyle(doc.getElementById("subject")).opacity,
		});
		frame.remove();
	}
	return results;
})();
