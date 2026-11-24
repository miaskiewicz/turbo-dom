// Run offline in a page with the JSON cases as the argument.
export function probe(cases) {
	document.body.innerHTML = "<div id=parent><div id=target></div></div>";
	const style = document.createElement("style");
	document.head.append(style);
	const target = document.getElementById("target");
	return cases.map((c) => {
		style.textContent = `#parent{${String.fromCharCode(...c.parentUnits)}}#target{${String.fromCharCode(...c.ownUnits)}}`;
		target.style.cssText = "";
		for (const entry of c.ownValues || []) {
			target.style.setProperty(entry.name, String.fromCharCode(...entry.units));
		}
		const computed = getComputedStyle(target);
		return {
			id: c.id,
			values: Object.fromEntries(
				c.properties.map((name) => {
					const value = computed.getPropertyValue(name);
					return [
						name,
						Array.from({ length: value.length }, (_, i) => value.charCodeAt(i)),
					];
				}),
			),
		};
	});
}
