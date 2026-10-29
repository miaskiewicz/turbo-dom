(async () => {
	const result = {},
		events = [],
		root = document.createElement("div");
	document.body.append(root);
	const summary = (record) => [
		record.type,
		record.target.nodeName,
		Array.from(record.addedNodes, (node) => node.nodeName),
		Array.from(record.removedNodes, (node) => node.nodeName),
		record.attributeName,
		record.oldValue,
	];
	class Parsed extends HTMLElement {
		static observedAttributes = ["data-v"];
		constructor() {
			super();
			events.push([
				"constructor",
				this.getAttribute("data-v"),
				this.isConnected,
			]);
		}
		attributeChangedCallback(name, old, value) {
			events.push(["attribute", old, value]);
		}
		connectedCallback() {
			events.push(["connected", this.isConnected]);
		}
	}
	customElements.define("x-parser-observer", Parsed);
	const template = document.createElement("template");
	const templateObserver = new MutationObserver(() => {});
	templateObserver.observe(template, { childList: true, subtree: true });
	templateObserver.observe(template.content, {
		childList: true,
		subtree: true,
	});
	template.innerHTML =
		'<x-parser-observer data-v="parsed"><b>text</b></x-parser-observer>';
	result.template = [
		events.slice(),
		template.content.firstChild.ownerDocument === document,
		templateObserver.takeRecords().map(summary),
	];
	const cloned = template.content.cloneNode(true);
	result.inertClone = events.slice();
	const rootObserver = new MutationObserver(() => {});
	rootObserver.observe(root, { childList: true, subtree: true });
	root.append(cloned);
	result.inserted = [
		events.slice(),
		root.firstChild instanceof Parsed,
		rootObserver.takeRecords().map(summary),
	];
	templateObserver.disconnect();
	rootObserver.disconnect();
	const table = document.createElement("table");
	table.innerHTML = "<tr><td>one</td></tr><tr><td>two</td></tr>";
	result.table = [
		table.innerHTML,
		table.firstChild.nodeName,
		table.querySelectorAll("td").length,
	];
	const select = document.createElement("select");
	select.innerHTML = "<div>ignored</div><option>A<option selected>B";
	result.select = [select.innerHTML, select.options.length, select.value];
	const formatting = document.createElement("div");
	formatting.innerHTML = "<b><i>one</b>two</i>";
	result.formatting = formatting.innerHTML;
	const foreign = document.createElement("div");
	foreign.innerHTML =
		"<svg><foreignObject><p>html</p></foreignObject><title>svg</title></svg>";
	result.foreign = Array.from(foreign.querySelectorAll("*"), (node) => [
		node.localName,
		node.namespaceURI,
	]);
	const range = document.createRange();
	range.selectNodeContents(table.querySelector("tbody"));
	const fragment = range.createContextualFragment("<tr><td>context</td></tr>");
	result.contextual = [
		fragment.firstChild.nodeName,
		fragment.firstChild.firstChild.nodeName,
		fragment.textContent,
	];
	const changing = document.createRange();
	changing.selectNodeContents(root);
	let changingConversions = 0;
	const changed = changing.createContextualFragment({
		toString() {
			changingConversions++;
			changing.selectNodeContents(table.firstChild);
			return "<tr><td>changed-context</td></tr>";
		},
	});
	result.convertedContext = [
		changingConversions,
		changed.firstChild.nodeName,
		changed.firstChild.firstChild.nodeName,
		changed.textContent,
	];
	const contexts = [];
	const parseContext = (label, context, source) => {
		const range = document.createRange();
		range.selectNodeContents(context);
		const expectedOwner =
			context.nodeType === 9 ? context : context.ownerDocument;
		const observed = new MutationObserver(() => {});
		observed.observe(context, {
			childList: true,
			attributes: true,
			subtree: true,
		});
		let conversions = 0;
		const fragment = range.createContextualFragment({
			toString() {
				conversions++;
				return source;
			},
		});
		const structure = (node) => [
			node.nodeName,
			node.namespaceURI ?? null,
			node.nodeType === 3 ? node.data : Array.from(node.childNodes, structure),
		];
		contexts.push([
			label,
			fragment.ownerDocument === expectedOwner,
			Array.from(fragment.childNodes, structure),
			observed.takeRecords().map(summary),
			conversions,
		]);
		observed.disconnect();
		return fragment;
	};
	const quirks =
		globalThis.quirksDocument ||
		new DOMParser().parseFromString("<body><div id=quirks></div>", "text/html");
	result.quirksMode = quirks.compatMode;
	parseContext(
		"quirks-document",
		quirks.body.firstChild,
		"<p>before<table><tr><td>cell</td></tr></table>",
	);
	parseContext(
		"standards-document",
		root,
		"<p>before<table><tr><td>cell</td></tr></table>",
	);
	parseContext("body", document.body, "<p>body</p>");
	parseContext("html", document.documentElement, "<p>html</p>");
	parseContext("table", table, "<tr><td>table</td></tr>");
	parseContext("tbody", table.firstChild, "<tr><td>tbody</td></tr>");
	const textRange = document.createRange();
	textRange.setStart(table.querySelector("td").firstChild, 1);
	const textFragment = textRange.createContextualFragment("<b>text-parent</b>");
	result.textContext = [
		textFragment.firstChild.nodeName,
		textFragment.textContent,
	];
	parseContext("select", select, "<option>one<option>two");
	parseContext("template", template, "<tr><td>template</td></tr>");
	parseContext("svg", foreign.querySelector("svg"), "<circle></circle>");
	parseContext(
		"foreignObject",
		foreign.querySelector("foreignObject"),
		"<p>html</p>",
	);
	const math = document.createElementNS(
		"http://www.w3.org/1998/Math/MathML",
		"annotation-xml",
	);
	math.setAttribute("encoding", "text/html");
	parseContext("math-html-integration", math, "<p>html</p>");
	const form = document.createElement("form"),
		formContext = document.createElement("div");
	form.append(formContext);
	parseContext(
		"form-ancestor",
		formContext,
		"<form id=nested><b>inner</b></form>",
	);
	const nestedTemplate = document.createElement("template");
	form.append(nestedTemplate);
	parseContext(
		"template-form-ancestor",
		nestedTemplate,
		"<form><b>template-form</b></form>",
	);
	const constructorCount = events.length;
	parseContext("custom-element-context", root.firstChild, "<i>content</i>");
	result.contextConstructors = events.length - constructorCount;
	const inert = document.implementation.createHTMLDocument();
	parseContext("inert-document", inert.body, "<p>inert</p>");
	parseContext(
		"active-noscript",
		root,
		"<noscript><b>fallback</b></noscript><script>void 0</script><i>after</i>",
	);
	parseContext(
		"inert-noscript",
		inert.body,
		"<noscript><b>fallback</b></noscript><script>void 0</script><i>after</i>",
	);
	parseContext(
		"template-inert-boundary",
		template.content,
		"<p>inert-template</p>",
	);
	let frame;
	if (!globalThis.child) {
		frame = document.createElement("iframe");
		document.body.append(frame);
		globalThis.child = frame.contentWindow;
	}
	parseContext("foreign-document", child.document.body, "<p>foreign</p>");
	if (frame) frame.remove();
	result.contexts = contexts;
	const scopedRegistry = new CustomElementRegistry(),
		scopedEvents = [];
	class ScopedParsed extends HTMLElement {
		static observedAttributes = ["data-v"];
		constructor() {
			super();
			scopedEvents.push([
				"constructor",
				this.getAttribute("data-v"),
				this.isConnected,
			]);
		}
		attributeChangedCallback(name, old, value) {
			scopedEvents.push(["attribute", old, value]);
		}
		connectedCallback() {
			scopedEvents.push(["connected", this.isConnected]);
		}
	}
	scopedRegistry.define("x-scoped-parser", ScopedParsed);
	const scopedHost = document.createElement("div");
	document.body.append(scopedHost);
	const scopedRoot = scopedHost.attachShadow({
		mode: "open",
		customElementRegistry: scopedRegistry,
	});
	scopedRoot.innerHTML = "<div></div>";
	const scopedContext = scopedRoot.firstChild,
		scopedRange = document.createRange();
	scopedRange.selectNodeContents(scopedContext);
	const scopedFragment = scopedRange.createContextualFragment(
		'<x-scoped-parser data-v="scoped"></x-scoped-parser>',
	);
	result.scopedFragmentBefore = [
		scopedFragment.firstChild instanceof ScopedParsed,
		scopedFragment.firstChild.customElementRegistry === scopedRegistry,
		scopedEvents.slice(),
	];
	scopedContext.append(scopedFragment);
	result.scopedFragmentAfter = [
		scopedContext.firstChild instanceof ScopedParsed,
		scopedEvents.slice(),
	];
	scopedEvents.length = 0;
	scopedRange.selectNodeContents(scopedRoot);
	const scopedRootFragment = scopedRange.createContextualFragment(
		'<x-scoped-parser data-v="root"></x-scoped-parser>',
	);
	result.scopedRootFragment = [
		scopedRootFragment.firstChild instanceof ScopedParsed,
		scopedRootFragment.firstChild.customElementRegistry === scopedRegistry,
		scopedEvents.slice(),
	];
	const scopedTemplate = document.createElement("template");
	scopedRoot.append(scopedTemplate);
	scopedEvents.length = 0;
	scopedRange.selectNodeContents(scopedTemplate);
	const scopedTemplateFragment = scopedRange.createContextualFragment(
		'<x-scoped-parser data-v="template"></x-scoped-parser>',
	);
	result.scopedTemplateFragment = [
		scopedTemplateFragment.ownerDocument ===
			scopedTemplate.content.ownerDocument,
		scopedTemplateFragment.firstChild instanceof ScopedParsed,
		scopedTemplateFragment.firstChild.customElementRegistry === null,
		scopedEvents.slice(),
	];
	scopedHost.remove();
	events.length = 0;
	const customRange = document.createRange();
	customRange.selectNodeContents(root);
	const customContextObserver = new MutationObserver(() => {});
	customContextObserver.observe(root, { childList: true, subtree: true });
	const customFragment = customRange.createContextualFragment(
		'<x-parser-observer data-v="fragment"></x-parser-observer><template><x-parser-observer data-v="nested"></x-parser-observer></template>',
	);
	result.customFragmentBefore = [
		events.slice(),
		customFragment.firstChild instanceof Parsed,
		customContextObserver.takeRecords().map(summary),
	];
	root.append(customFragment);
	result.customFragmentAfter = [
		events.slice(),
		customContextObserver.takeRecords().map(summary),
	];
	root.append(root.lastChild.content);
	result.customTemplateAfter = [
		events.slice(),
		customContextObserver.takeRecords().map(summary),
	];
	customContextObserver.disconnect();
	await Promise.resolve();
	await Promise.resolve();
	const order = [],
		calls = [];
	let first = true;
	const observer = new MutationObserver(function (records, self) {
		calls.push([this === observer, self === observer, records.map(summary)]);
		order.push("observer");
		if (first) {
			first = false;
			root.setAttribute("data-next", "nested");
			Promise.resolve().then(() => order.push("nested-promise"));
		}
	});
	observer.observe(root, { attributes: true, attributeOldValue: true });
	Promise.resolve().then(() => order.push("before"));
	root.setAttribute("data-start", "one");
	Promise.resolve().then(() => order.push("after"));
	await Promise.resolve();
	await Promise.resolve();
	await Promise.resolve();
	await Promise.resolve();
	result.reentrantObserver = [order, calls];
	observer.disconnect();
	const node = document.createElement("span");
	root.append(node);
	const transient = new MutationObserver(() => {});
	transient.observe(root, {
		attributes: true,
		attributeOldValue: true,
		subtree: true,
	});
	node.remove();
	node.setAttribute("data-v", "before");
	transient.observe(root, { attributes: true, subtree: true });
	node.setAttribute("data-v", "after");
	result.reobserveTransient = transient.takeRecords().map(summary);
	transient.disconnect();
	const delivered = [];
	const drained = new MutationObserver((records) =>
		delivered.push(records.map(summary)),
	);
	drained.observe(root, { attributes: true });
	root.setAttribute("data-drained", "first");
	result.drained = drained.takeRecords().map(summary);
	await Promise.resolve();
	result.drainedDelivery = delivered.slice();
	root.setAttribute("data-drained", "second");
	await Promise.resolve();
	result.secondDelivery = delivered.slice();
	drained.disconnect();
	root.remove();
	return result;
})();
