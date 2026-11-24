// Original UTF16 stylesheet/source controls; no external services.
export function probe() {
	const units = (s) =>
		Array.from({ length: s.length }, (_, i) => s.charCodeAt(i));
	const values = [
		["raw", "\ud800"],
		["repaired", "\ufffd"],
		["marker", "\u{f0000}\u{f0800}"],
	];
	for (const [id, value] of values) {
		const el = document.createElement("div");
		el.id = id;
		el.setAttribute("data-x", value);
		el.className = "x" + value;
		document.body.append(el);
	}
	const style = document.createElement("style");
	document.head.append(style);
	const selectors = [
		["quoted-raw", '[data-x="\ud800"]'],
		["quoted-escaped", '[data-x="\\d800 "]'],
		["quoted-marker", '[data-x="\u{f0000}\u{f0800}"]'],
		["identifier-raw", ".x\ud800"],
	].map(([id, selector]) => {
		style.textContent = "div{color:red}" + selector + "{color:blue}";
		return {
			id,
			selector: units(selector),
			query: Array.from(document.querySelectorAll(selector), (n) => n.id),
			styled: values.map(([id]) => [
				id,
				getComputedStyle(document.getElementById(id)).color,
			]),
			serialized: units(style.sheet.cssRules[1]?.selectorText || ""),
		};
	});
	const syntax = [
		'"a; } { ; b"',
		'"/* literal comment */"',
		'url("data:text/plain;a{b}:c")',
	].map((payload) => {
		style.textContent = "#raw{--token:" + payload + ";color:rgb(1, 2, 3)}";
		const computed = getComputedStyle(document.getElementById("raw"));
		return {
			payload,
			css: style.textContent,
			token: computed.getPropertyValue("--token"),
			color: computed.color,
		};
	});
	style.textContent = '#raw{color:/* note */red;--token:"/* literal */"}';
	const computed = getComputedStyle(document.getElementById("raw"));
	syntax.push({
		payload: '"/* literal */"',
		css: style.textContent,
		token: computed.getPropertyValue("--token"),
		color: computed.color,
	});
	return { selectors, syntax };
}
