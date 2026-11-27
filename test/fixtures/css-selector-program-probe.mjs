// Mechanical selector matching controls; no application fixtures or live network.
export function probe(cases) {
	const sheet = document.createElement("style");
	document.head.append(sheet);
	const scope = cases.scope.map((row) => {
		document.body.innerHTML = cases.html;
		sheet.textContent = `*{color:rgb(1,2,3)}@scope(.root){${row.selector}{color:rgb(4,5,6)!important}}`;
		const target = document.querySelector(
			row.target === "root" ? ".root" : ".target",
		);
		return {
			selector: row.selector,
			matches: getComputedStyle(target).color === "rgb(4, 5, 6)",
		};
	});
	document.body.innerHTML =
		"<div id=raw></div><div id=repaired></div><div id=marker></div>";
	const records = [
		["raw", "\ud800"],
		["repaired", "\ufffd"],
		["marker", "\u{f0000}\u{f0800}"],
	];
	for (const [id, value] of records) {
		const element = document.getElementById(id);
		element.setAttribute("data-x", value);
		element.className = "x" + value;
	}
	const units = cases.units.map((selector) => {
		sheet.textContent = `*{color:rgb(1,2,3)}${selector}{color:rgb(4,5,6)!important}`;
		return {
			selector,
			matches: records
				.filter(
					([id]) =>
						getComputedStyle(document.getElementById(id)).color ===
						"rgb(4, 5, 6)",
				)
				.map(([id]) => id),
		};
	});
	return { scope, units };
}
