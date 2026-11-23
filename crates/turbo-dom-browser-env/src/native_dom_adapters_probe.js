(() => {
	const result = {},
		initialLength = history.length;
	const parser = new DOMParser();
	const document = parser.parseFromString(
		"<p>hi</p><script>globalThis.parserRan=true</script>",
		"text/html",
	);
	result.html = [
		document.querySelector("p").textContent,
		document.contentType,
		document.defaultView === null,
		document.location === null,
		globalThis.parserRan === undefined,
	];
	const xml = parser.parseFromString("<r><x/></r>", "application/xml");
	result.xml = [
		xml.documentElement.localName,
		xml.querySelector("x").parentNode === xml.documentElement,
		xml.contentType,
		xml.defaultView === null,
	];
	const state = { n: 1 };
	history.replaceState(state, "", "/adapters?x=1");
	state.n = 9;
	result.replace = [
		location.pathname,
		location.search,
		location.hash,
		history.state.n,
		history.length - initialLength,
	];
	history.pushState({ n: 2 }, "", "#two");
	result.push = [
		location.pathname,
		location.search,
		location.hash,
		history.state.n,
		history.length - initialLength,
	];
	result.errors = [];
	for (const call of [
		() => history.pushState({}, "", "https://elsewhere.invalid/"),
		() => history.replaceState({}, "", "http://["),
		() => history.pushState(() => {}, "", "/bad"),
		() => history.pushState({}),
		() => history.replaceState.call({}, {}, "", "/bad"),
		() => parser.parseFromString(""),
		() => parser.parseFromString("", "TEXT/HTML"),
		() => parser.parseFromString.call({}, "", "text/html"),
	]) {
		try {
			call();
			result.errors.push("accepted");
		} catch (error) {
			result.errors.push(error.name);
		}
	}
	result.afterErrors = [
		location.pathname,
		location.hash,
		history.state.n,
		history.length - initialLength,
	];
	const cycle = { map: new Map([["key", 3]]) };
	cycle.self = cycle;
	history.replaceState(cycle, "");
	result.clone = [
		history.state !== cycle,
		history.state.self === history.state,
		history.state.map.get("key"),
		history.state === history.state,
	];
	result.interfaces = [
		Object.prototype.toString.call(history),
		history instanceof History,
		Object.prototype.toString.call(parser),
		parser instanceof DOMParser,
		DOMParser.prototype.parseFromString.length,
		History.prototype.pushState.length,
	];
	history.scrollRestoration = "manual";
	let invalidRestoration;
	try {
		history.scrollRestoration = "invalid";
	} catch (error) {
		invalidRestoration = error.name;
	}
	result.restoration = [history.scrollRestoration, invalidRestoration ?? null];
	return result;
})();
