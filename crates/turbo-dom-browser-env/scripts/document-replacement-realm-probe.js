(() => {
	const [target, caller] = document.querySelectorAll("iframe");
	caller.contentWindow.eval(
		`parent.frames[0].document.open();parent.frames[0].document.write('<p id=target>next</p><script>globalThis.mark=7<\\/script>');parent.frames[0].document.close();`,
	);
	const script = target.contentDocument.querySelector("script");
	if (target.contentWindow.mark !== 7)
		throw Error("replacement script did not run");
	return {
		targetRealm: script instanceof target.contentWindow.HTMLScriptElement,
		callerRealm: script instanceof caller.contentWindow.HTMLScriptElement,
		owner: script.ownerDocument === target.contentDocument,
		callerUnaffected: caller.contentDocument.body.textContent === "",
	};
})();
