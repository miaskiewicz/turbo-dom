(async () => {
	function buildCases() {
		const root =
			'<div id="root"><label>Schedule</label><div id="control" role="combobox">Control</div></div>';
		const common =
			"body{margin:8px}#root{position:relative;width:200px;height:56px}label{display:block;height:20px}#control{height:36px}";
		const cases = [
			{
				name: "propagated-body",
				css: "body{height:40px;overflow:hidden}#space{height:100px}",
				body: '<div id="space"></div>' + root,
				point: [108, 136],
			},
			{
				name: "independent-body-clip",
				css: "html{overflow:hidden}body{height:40px;overflow:hidden}#space{height:100px}",
				body: '<div id="space"></div>' + root,
				point: [108, 136],
			},
			{
				name: "contained-body-clip",
				css: "body{height:40px;overflow:hidden;contain:paint}#space{height:100px}",
				body: '<div id="space"></div>' + root,
				point: [108, 136],
			},
			{
				name: "contained-body-scrolled",
				css: "body{height:40px;overflow:hidden;contain:paint}#space{height:100px}",
				body: '<div id="space"></div>' + root,
				point: [108, 36],
				scroll: 100,
			},
			{
				name: "fixed-escapes-ordinary-clip",
				css: "#clip{height:40px;overflow:clip}#root{position:fixed;top:300px}",
				body: '<div id="clip">' + root + "</div>",
				point: [108, 328],
			},
			{
				name: "fixed-escapes-nested-clip",
				css: "#clip,#inner{height:40px;overflow:hidden}#root{position:fixed;top:300px}",
				body: '<div id="clip"><div id="inner">' + root + "</div></div>",
				point: [108, 328],
			},
			{
				name: "fixed-descendant-escapes-clip",
				css: "#clip{height:40px;overflow:clip}#fixed{position:fixed;top:300px}",
				body: '<div id="clip"><div id="fixed">' + root + "</div></div>",
				point: [108, 328],
			},
			{
				name: "fixed-inner-clip-stays",
				css: "#fixed{position:fixed;top:300px}#clip{height:20px;overflow:clip}",
				body: '<div id="fixed"><div id="clip">' + root + "</div></div>",
				point: [108, 328],
			},
			{
				name: "fixed-paint-containing-clip-stays",
				css: "#clip{height:40px;overflow:clip;contain:paint}#root{position:fixed;top:100px}",
				body: '<div id="clip">' + root + "</div>",
				point: [108, 128],
			},
			{
				name: "absolute-escapes-intermediate-clip",
				css: "body{position:relative}#clip{height:40px;overflow:clip}#root{position:absolute;top:100px}",
				body: '<div id="clip">' + root + "</div>",
				point: [108, 136],
			},
			{
				name: "absolute-containing-clip-stays",
				css: "#clip{height:40px;overflow:clip;position:relative}#root{position:absolute;top:100px}",
				body: '<div id="clip">' + root + "</div>",
				point: [108, 136],
			},
			{
				name: "ordinary-clip-stays",
				css: "#clip{height:40px;overflow:clip}#root{top:100px}",
				body: '<div id="clip">' + root + "</div>",
				point: [108, 136],
			},
			{
				name: "contents-does-not-clip",
				css: "#clip{display:contents;overflow:clip}#root{top:100px}",
				body: '<div id="clip">' + root + "</div>",
				point: [108, 136],
			},
			{
				name: "transformed-fixed-remains-unsupported",
				css: "#clip{height:40px;overflow:clip;transform:translate(0px)}#root{position:fixed;top:100px}",
				body: '<div id="clip">' + root + "</div>",
				point: [108, 136],
				unsupported: true,
			},
		];
		for (const [property, values] of [
			[
				"contain",
				["none", "size", "inline-size", "style", "layout", "content", "strict"],
			],
			["content-visibility", ["visible", "auto", "hidden"]],
			[
				"container-type",
				["normal", "inline-size", "size", "inline-size scroll-state"],
			],
		])
			for (const value of values)
				cases.push({
					name: `fixed-${property}-${value}`,
					css: `#clip{height:40px;overflow:clip;${property}:${value}}#root{position:fixed;top:100px}`,
					body: '<div id="clip">' + root + "</div>",
					point: [108, 128],
				});
		for (const [name, css] of [
			["inline-paint", "display:inline;contain:paint"],
			["inline-layout", "display:inline;contain:layout"],
			[
				"atomic-inline-paint",
				"display:inline-block;contain:paint;width:200px;height:40px",
			],
			["contents-paint", "display:contents;contain:paint"],
			[
				"blockified-inline-paint",
				"display:inline;position:absolute;contain:paint;width:200px;height:40px",
			],
			[
				"floated-inline-paint",
				"display:inline;float:left;contain:paint;width:200px;height:40px",
			],
		])
			cases.push({
				name: `fixed-${name}`,
				css: `#clip{height:40px;overflow:clip}#inline{${css}}#root{position:fixed;top:100px}`,
				body: '<div id="clip"><span id="inline">' + root + "</span></div>",
				point: [108, 128],
			});
		cases.push({
			name: "fixed-flex-item-paint",
			css: "#clip{height:40px;overflow:clip}#flex{display:flex}#inline{display:inline;contain:paint;width:200px;height:40px}#root{position:fixed;top:100px}",
			body:
				'<div id="clip"><div id="flex"><span id="inline">' +
				root +
				"</span></div></div>",
			point: [108, 128],
		});

		return cases.map((item) => ({ ...item, css: common + item.css }));
	}
	async function measure(item) {
		const css = item.css;
		const f = document.createElement("iframe");
		f.style.cssText = "width:1280px;height:800px;border:0";
		const loaded = new Promise((resolve) => (f.onload = resolve));
		f.srcdoc = "<!doctype html><style>" + css + "</style><body>" + item.body;
		document.body.append(f);
		await loaded;
		const w = f.contentWindow,
			d = w.document;
		if (item.scroll) d.body.scrollTop = item.scroll;
		const rect = d.getElementById("control").getBoundingClientRect();
		const hit = d.elementFromPoint(...item.point);
		const result = {
			...item,
			css,
			read: {
				hit: hit?.id || hit?.tagName,
				controlHit: hit?.id === "control",
				rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
				bodyScroll: d.body.scrollTop,
			},
		};
		f.remove();
		return result;
	}
	const result = [];
	for (const item of buildCases()) result.push(await measure(item));
	return result;
})();
