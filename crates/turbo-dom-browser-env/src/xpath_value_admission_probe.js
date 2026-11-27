(() => {
	const rows = [];
	for (const mode of ["control", "script", "style", "attribute"]) {
		const doc = document.implementation.createHTMLDocument("XPath admission");
		doc.body.innerHTML =
			'<main><div id="parent" class="MuiPickersInputBase-root"><input id="field"><div class="MuiPickersSectionList-root"></div></div></main>';
		if (mode === "attribute")
			doc.body.setAttribute("data-module", "x".repeat(1024 * 1024));
		else if (mode !== "control") {
			const module = doc.createElement(mode);
			if (mode === "script") module.type = "application/json";
			module.textContent =
				mode === "style"
					? "/*" + "x".repeat(1024 * 1024) + "*/"
					: "x".repeat(1024 * 1024);
			doc.head.appendChild(module);
		}
		const field = doc.getElementById("field");
		const date =
			'descendant-or-self::div[contains(@class,"MuiPickersSectionList-root")] | ancestor::*[contains(@class,"MuiPickersInputBase-root")][1]//div[contains(@class,"MuiPickersSectionList-root")]';
		rows.push({
			mode,
			css: doc.querySelector("#parent > #field") === field,
			parent: doc.evaluate("..", field, null, 9).singleNodeValue.id,
			count: doc.evaluate("count(.)", field, null, 1).numberValue,
			date: doc.evaluate(date, field, null, 7).snapshotLength,
		});
	}
	return rows;
})();
