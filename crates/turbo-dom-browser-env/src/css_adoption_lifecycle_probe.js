(() => {
	const result = {},
		callbacks = [];
	let outerList, innerList;
	class Host extends HTMLElement {
		adoptedCallback() {
			callbacks.push([outerList.length, innerList.length]);
		}
	}
	customElements.define("x-sheet-lifecycle", Host);
	const host = document.createElement("x-sheet-lifecycle");
	document.body.append(host);
	const outer = host.attachShadow({ mode: "closed" }),
		nested = document.createElement("div");
	outer.append(nested);
	const inner = nested.attachShadow({ mode: "closed" });
	inner.innerHTML = "<style>#leaf{width:17px}</style><div id=leaf></div>";
	const parentSheet = new CSSStyleSheet();
	parentSheet.replaceSync("#leaf{width:29px}");
	const get = Object.getOwnPropertyDescriptor(
			ShadowRoot.prototype,
			"adoptedStyleSheets",
		).get,
		set = Object.getOwnPropertyDescriptor(
			ShadowRoot.prototype,
			"adoptedStyleSheets",
		).set;
	outer.adoptedStyleSheets = [parentSheet];
	inner.adoptedStyleSheets = [parentSheet, parentSheet];
	outerList = outer.adoptedStyleSheets;
	innerList = inner.adoptedStyleSheets;
	const snapshot = () => [
		outerList.length,
		innerList.length,
		get.call(outer) === outerList,
		get.call(inner) === innerList,
	];
	document.adoptNode(host);
	result.sameDocument = snapshot();
	document.body.append(host);
	host.remove();
	result.disconnected = snapshot();
	document.body.append(host);
	result.reconnected = snapshot();
	let hooks = 0;
	for (const root of [outer, inner])
		for (const name of ["adoptedStyleSheets", "ownerDocument"])
			Object.defineProperty(root, name, {
				configurable: true,
				get() {
					hooks++;
					throw Error("public adoption hook");
				},
			});
	const foreign =
		globalThis.__adoptionForeignDoc ||
		new DOMParser().parseFromString(
			"<!doctype html><html><body></body></html>",
			"text/html",
		);
	foreign.adoptNode(host);
	result.foreign = snapshot();
	let denied;
	try {
		innerList.push(parentSheet);
		denied = "allowed";
	} catch (error) {
		denied = error.name;
	}
	result.foreignOldSheet = denied;
	document.adoptNode(host);
	set.call(outer, [parentSheet]);
	set.call(inner, [parentSheet]);
	foreign.adoptNode(host);
	document.adoptNode(host);
	result.unreadRoundtrip = snapshot();
	if (!globalThis.child) {
		const frame = document.createElement("iframe");
		document.body.append(frame);
		globalThis.child = frame.contentWindow;
	}
	const childSheet = child.eval(
		"globalThis.lifecycleSheet=new CSSStyleSheet();lifecycleSheet.replaceSync('#leaf{width:37px}');lifecycleSheet",
	);
	child.document.adoptNode(host);
	set.call(inner, [childSheet]);
	result.foreignRealm = [...snapshot(), innerList[0] === childSheet];
	document.adoptNode(host);
	result.backToSource = snapshot();
	result.callbacks = callbacks;
	result.publicHooks = hooks;
	result.oldSheetIntact = parentSheet.cssRules[0].style.width;
	host.remove();
	return result;
})();
