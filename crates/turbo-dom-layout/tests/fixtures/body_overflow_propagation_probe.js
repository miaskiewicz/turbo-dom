(async () => {
	const cases = [];
	const add = (name, css, extra = {}) => cases.push({ name, css, ...extra });
	add("baseline", "");
	for (const target of ["body", "html"]) {
		for (const value of [
			"none",
			"paint",
			"style",
			"layout",
			"size",
			"inline-size",
			"content",
			"strict",
			"size style paint",
		])
			add(`${target}-contain-${value}`, `${target}{contain:${value}}`);
		for (const value of ["none", "contents"])
			add(`${target}-display-${value}`, `${target}{display:${value}}`);
		for (const value of ["auto", "hidden", "visible"])
			add(
				`${target}-content-visibility-${value}`,
				`${target}{content-visibility:${value}}`,
			);
		for (const value of [
			"normal",
			"inline-size",
			"size",
			"scroll-state",
			"inline-size scroll-state",
			"size scroll-state",
		])
			add(
				`${target}-container-type-${value}`,
				`${target}{container-type:${value}}`,
			);
	}
	for (const value of [
		"hidden",
		"clip",
		"visible hidden",
		"hidden visible",
		"visible clip",
	])
		add(`root-overflow-${value}`, `html{overflow:${value}}`);
	add("body-overflow-clip", "body{overflow:clip}");
	for (const mode of ["quirks", "frame"])
		for (const target of ["none", "body", "html"])
			add(
				`${mode}-${target}`,
				target === "none" ? "" : `${target}{contain:paint}`,
				{ [mode]: true },
			);
	for (const target of ["body", "html"])
		for (const display of ["contents", "inline"])
			add(
				`${target}-${display}-paint`,
				`${target}{display:${display};contain:paint}`,
			);
	const result = [];
	for (const item of cases) {
		let parent = document.body;
		let outer;
		if (item.frame) {
			outer = document.createElement("iframe");
			document.body.append(outer);
			parent = outer.contentDocument.body;
		}
		const f = document.createElement("iframe");
		f.style.cssText = "width:400px;height:300px;border:0";
		const loaded = new Promise((resolve) => (f.onload = resolve));
		f.srcdoc =
			(item.quirks ? "" : "<!doctype html>") +
			"<style>html{scrollbar-width:none}body{margin:8px;height:40px;overflow:hidden;scrollbar-width:none}button{display:block;height:30px;padding:0;border:0}" +
			item.css +
			'</style><body><div style="height:100px"></div><button id="target">Target</button>';
		parent.append(f);
		await loaded;
		const w = f.contentWindow,
			d = w.document,
			b = d.body,
			r = d.documentElement,
			t = d.getElementById("target");
		const before = {
			target: t.getBoundingClientRect().top,
			body: b.getBoundingClientRect().top,
		};
		b.scrollTop = 90;
		const read = {
			bodyScroll: b.scrollTop,
			scrollY: w.scrollY,
			bodyTop: b.getBoundingClientRect().top,
			targetTop: t.getBoundingClientRect().top,
			bodyHeight: b.clientHeight,
			bodyScrollHeight: b.scrollHeight,
			compat: d.compatMode,
			scrolling: d.scrollingElement?.tagName,
			bodyContain: w.getComputedStyle(b).contain,
			rootContain: w.getComputedStyle(r).contain,
		};
		b.scrollTop = 0;
		w.scrollTo(0, 0);
		t.scrollIntoView({ block: "nearest", inline: "nearest" });
		const into = {
			bodyScroll: b.scrollTop,
			scrollY: w.scrollY,
			targetTop: t.getBoundingClientRect().top,
		};
		result.push({ ...item, before, read, into });
		f.remove();
		outer?.remove();
	}
	return result;
})();
