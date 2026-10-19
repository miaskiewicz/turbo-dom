(() => {
	const units = (text) =>
		Array.from({ length: text.length }, (_, i) => text.charCodeAt(i));
	const fixtureHtml = `<section id="attribute-fixture"><div id="attribute-nest">
<input id="hidden" type="hidden" name="q" data-eq="AbC" data-word="Zero AbC End" data-dash="EN-us" data-start="AbCd" data-end="xxAbC" data-sub="xAbCy">
<textarea id="visible" name="q"></textarea><input id="upper" type="HIDDEN" name="q" data-eq="abc"><input id="real" name="q" data-eq="ABC">
<button id="button" type="BUTTON"></button><ol id="ordered" type="A"></ol>
<div id="literal" data-eq="i" data-escaped="a&quot;b]c" data-backslash="a\\b" data-name="sp ace"></div>
<div id="nonascii" data-eq="ÉAbÇ" data-token="ÉA éa"></div><div id="ascii" data-eq="éaBç"></div>
<div id="astral" data-eq="🔥AbC"></div><div id="empty" data-eq=""></div>
<div id="lone"></div><div id="replacement" data-eq="�AbC"></div>
<svg><a id="svg" type="HIDDEN" data-eq="AbC"></a></svg>
</div></section>`;
	document.body.innerHTML = fixtureHtml;
	const root = document.getElementById("attribute-fixture");
	const mutations = [
		{ id: "lone", name: "data-eq", valueUnits: [55296, 65, 98, 67] },
	];
	for (const row of mutations)
		document
			.getElementById(row.id)
			.setAttribute(row.name, String.fromCharCode(...row.valueUnits));
	const targets = [...root.querySelectorAll("[id]")].filter(
		(e) => e.id !== "attribute-nest",
	);
	const targetIds = targets.map((element) => element.id);
	for (const target of targets) {
		const child = document.createElement("span");
		child.id = `${target.id}-child`;
		if (!["INPUT", "TEXTAREA"].includes(target.tagName)) target.append(child);
	}
	const fragment = document.createDocumentFragment();
	fragment.append(root.cloneNode(true));
	const host = document.createElement("div");
	host.id = "attribute-shadow-host";
	document.body.append(host);
	const shadow = host.attachShadow({ mode: "open" });
	shadow.append(root.cloneNode(true));
	const rows = [];
	const add = (name, selector) =>
		rows.push({ name, selector, selectorUnits: units(selector) });
	for (const [name, op, value] of [
		["equal", "=", "abc"],
		["word", "~=", "abc"],
		["dash", "|=", "en"],
		["prefix", "^=", "ab"],
		["suffix", "$=", "abc"],
		["substring", "*=", "abc"],
	]) {
		const attribute = {
			equal: "eq",
			word: "word",
			dash: "dash",
			prefix: "start",
			suffix: "end",
			substring: "sub",
		}[name];
		for (const flag of ["", " i", " I"])
			add(
				`${name}-${flag.trim() || "default"}`,
				`[data-${attribute}${op}"${value}"${flag}]`,
			);
	}
	for (const [name, selector] of [
		[
			"common-query",
			'textarea[name="q"],input[name="q"]:not([type="hidden" i])',
		],
		["not-i", 'input[name="q"]:not([type="hidden" i])'],
		["implicit-html-type", '[type="hidden"]'],
		["upper-attribute", '[TYPE="hidden"]'],
		["ordered-type-lower", 'ol[type="a"]'],
		["ordered-type-i", 'ol[type="a" i]'],
		["button-type-default", 'button[type="button"]'],
		["svg-type-default", 'svg [type="hidden"]'],
		["explicit-type-i", '[type="hidden" i]'],
		["literal-i", '[data-eq="i"]'],
		["literal-space-i", '[data-name="sp ace" i]'],
		["unquoted-i-value", "[data-eq=i]"],
		["unquoted-i-flag", "[data-eq=abc i]"],
		["whitespace-tab", '[data-eq="abc"\ti\t]'],
		["whitespace-newline", '[data-eq="abc"\nI\n]'],
		["escaped-value", '[data-eq="\\41 bC" i]'],
		["escaped-attribute", '[data-\\65 q="abc" i]'],
		["escaped-flag", '[data-eq="abc" \\69]'],
		["escaped-quote-and-bracket", '[data-escaped="a\\"b]c" i]'],
		["escaped-backslash", '[data-backslash="a\\\\b" i]'],
		["nonascii-exact-capital", '[data-eq="ÉaBÇ" i]'],
		["nonascii-lower-is-distinct", '[data-eq="éAbç" i]'],
		["nonascii-token", '[data-token~="éA" i]'],
		["astral-ascii-fold", '[data-eq="🔥aBc" i]'],
		["lone-literal", '[data-eq="\ud800abc" i]'],
		["lone-escaped", '[data-eq="\\d800 abc" i]'],
		["replacement-literal", '[data-eq="�abc" i]'],
		["empty-equal-i", '[data-eq="" i]'],
		["empty-prefix-i", '[data-eq^="" i]'],
		["empty-suffix-i", '[data-eq$="" i]'],
		["empty-substring-i", '[data-eq*="" i]'],
		["unsupported-s", '[data-eq="abc" s]'],
		["unsupported-S", '[data-eq="abc" S]'],
		["s-not", ':not([data-eq="abc" s])'],
		["s-list", '[data-eq="abc" s],#real'],
		["s-is-forgiving", ':is([data-eq="abc" s],#real)'],
		["s-where-forgiving", ':where([data-eq="abc" s],#real)'],
		["s-escaped", '[data-eq="abc" \\73]'],
		["s-literal-value", '[data-eq="x s"]'],
		["duplicate-i", '[data-eq="abc" i i]'],
		["mixed-flags", '[data-eq="abc" i s]'],
		["unknown-flag", '[data-eq="abc" z]'],
		["flag-without-value", "[data-eq i]"],
		["joined-flag", '[data-eq="abc"ii]'],
		["comment-flag", '[data-eq="abc"/**/i]'],
		["comment-after-flag", '[data-eq="abc" i/**/]'],
	])
		add(name, selector);
	const attempt = (callback) => {
		try {
			return { value: callback() };
		} catch (error) {
			return { error: error.name };
		}
	};
	const query = (scope, selector) => ({
		first: attempt(() => scope.querySelector(selector)?.id || null),
		all: attempt(() =>
			[...scope.querySelectorAll(selector)].map((element) => element.id),
		),
	});
	const cases = rows.map((row) => ({
		...row,
		document: query(document, row.selector),
		element: query(root, row.selector),
		fragment: query(fragment, row.selector),
		shadow: query(shadow, row.selector),
		matches: targets.map((element) => ({
			id: element.id,
			result: attempt(() => element.matches(row.selector)),
		})),
		closest: targets.map((element) => ({
			id: element.id,
			result: attempt(
				() =>
					(element.firstElementChild || element).closest(row.selector)?.id ||
					null,
			),
		})),
	}));
	const xml = new DOMParser().parseFromString(
		'<root><input id="xml-upper" type="HIDDEN" data-eq="AbC"/><input id="xml-lower" type="hidden" data-eq="abc"/></root>',
		"text/xml",
	);
	const xmlCases = [
		'[type="hidden"]',
		'[type="hidden" i]',
		'[data-eq="abc"]',
		'[data-eq="abc" i]',
		'[TYPE="hidden" i]',
	].map((selector) => ({
		selector,
		selectorUnits: units(selector),
		...query(xml, selector),
		matches: [...xml.querySelectorAll("input")].map((element) => ({
			id: element.id,
			result: attempt(() => element.matches(selector)),
		})),
	}));
	return {
		fixtureHtml,
		fixtureHtmlUnits: units(fixtureHtml),
		mutations,
		targetIds,
		cases,
		xmlCases,
	};
})();
