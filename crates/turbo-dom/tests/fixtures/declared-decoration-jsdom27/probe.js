(() => {
	const sheet = document.createElement("style");
	sheet.textContent = ".decoration-control { text-decoration: underline; }";
	document.head.appendChild(sheet);
	const cases = [
		["unset", "", ""],
		["strike", "text-decoration: line-through", ""],
		["underline", "text-decoration: underline", ""],
		["explicit-color", "text-decoration: underline red", ""],
		["explicit-style", "text-decoration: underline dotted blue", ""],
		["stylesheet", "", "decoration-control"],
		["inline-wins", "text-decoration: line-through", "decoration-control"],
		[
			"important",
			"text-decoration: line-through !important; text-decoration: underline",
			"",
		],
		[
			"longhands",
			"text-decoration-line: underline; text-decoration-color: red",
			"",
		],
		[
			"shorthand-then-longhand",
			"text-decoration: underline; text-decoration-color: red",
			"",
		],
		[
			"all-longhands",
			"text-decoration-line: underline; text-decoration-color: red; text-decoration-style: dotted; text-decoration-thickness: auto",
			"",
		],
	];
	const results = cases.map(([name, css, cls]) => {
		const span = document.createElement("span");
		span.className = cls;
		span.style.cssText = css;
		document.body.appendChild(span);
		const computed = getComputedStyle(span);
		const row = {
			name,
			value: computed.textDecoration,
			property: computed.getPropertyValue("text-decoration"),
		};
		span.remove();
		return row;
	});
	const span = document.createElement("span");
	span.style.textDecoration = "line-through";
	document.body.appendChild(span);
	results.push({
		name: "idl-setter",
		value: getComputedStyle(span).textDecoration,
		property: getComputedStyle(span).getPropertyValue("text-decoration"),
	});
	span.style.textDecoration = "underline";
	results.push({
		name: "idl-update",
		value: getComputedStyle(span).textDecoration,
		property: getComputedStyle(span).getPropertyValue("text-decoration"),
	});
	span.style.cssText = "text-decoration: line-through red";
	results.push({
		name: "css-text-replacement",
		value: getComputedStyle(span).textDecoration,
		property: getComputedStyle(span).getPropertyValue("text-decoration"),
	});
	span.style.removeProperty("text-decoration");
	results.push({
		name: "removed",
		value: getComputedStyle(span).textDecoration,
		property: getComputedStyle(span).getPropertyValue("text-decoration"),
	});
	span.remove();
	sheet.remove();
	return results;
})();
