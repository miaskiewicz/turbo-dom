(() => {
	const frame = document.createElement("iframe");
	document.body.append(frame);
	const win = frame.contentWindow,
		doc = frame.contentDocument;
	win.eval(
		`globalThis.retiredRows=[];globalThis.RetiredElement=class extends HTMLElement{static observedAttributes=['data-v'];constructor(){super();retiredRows.push(['constructor',this.ownerDocument===document]);}connectedCallback(){retiredRows.push(['connected',this.isConnected]);}disconnectedCallback(){retiredRows.push(['disconnected',this.isConnected]);}attributeChangedCallback(name,old,value){retiredRows.push(['attribute',old,value]);}};customElements.define('x-retired-control',RetiredElement);`,
	);
	const registry = win.customElements,
		constructor = win.RetiredElement,
		existing = doc.createElement("x-retired-control");
	doc.body.append(existing);
	existing.setAttribute("data-v", "active");
	const active = [
		existing instanceof constructor,
		doc.customElementRegistry === registry,
		win.retiredRows.slice(),
	];
	const inert = doc.implementation.createHTMLDocument();
	const inertElement = inert.createElement("x-retired-control");
	const inertControl = [
		inert.customElementRegistry === null,
		inertElement instanceof constructor,
		inertElement.ownerDocument === inert,
	];
	const borrowedRows = [];
	win.parentAttribute = function (name, old, value) {
		borrowedRows.push([old, value, this.ownerDocument === doc]);
	};
	win.eval(
		`globalThis.BorrowedRetired=class extends HTMLElement{static observedAttributes=['data-v'];};BorrowedRetired.prototype.attributeChangedCallback=parentAttribute;customElements.define('x-retired-borrowed',BorrowedRetired);`,
	);
	const borrowed = doc.createElement("x-retired-borrowed");
	doc.body.append(borrowed);
	win.retiredRows.length = 0;
	frame.remove();
	const result = {
		active,
		inert: inertControl,
		retirement: [
			doc.defaultView === null,
			doc.customElementRegistry === registry,
			existing.ownerDocument === doc,
			existing instanceof constructor,
		],
		retirementRows: win.retiredRows.slice(),
	};
	win.retiredRows.length = 0;
	const created = doc.createElement("x-retired-control");
	doc.body.append(created);
	created.setAttribute("data-v", "retired");
	result.creation = [
		created instanceof constructor,
		created.ownerDocument === doc,
		created.isConnected,
		win.retiredRows.slice(),
	];
	win.retiredRows.length = 0;
	const container = doc.createElement("div");
	container.innerHTML =
		'<x-retired-control data-v="parsed"></x-retired-control>';
	doc.body.append(container);
	result.parsing = [
		container.firstChild instanceof constructor,
		container.firstChild.ownerDocument === doc,
		win.retiredRows.slice(),
	];
	win.retiredRows.length = 0;
	existing.setAttribute("data-v", "existing-retired");
	existing.remove();
	doc.body.append(existing);
	result.existingRetired = win.retiredRows.slice();
	win.retiredRows.length = 0;
	document.body.append(existing);
	existing.setAttribute("data-v", "adopted-active");
	result.existingAdopted = [
		existing.ownerDocument === document,
		existing instanceof constructor,
		win.retiredRows.slice(),
	];
	borrowed.setAttribute("data-v", "borrowed-retired");
	document.body.append(borrowed);
	borrowed.setAttribute("data-v", "borrowed-adopted");
	result.borrowedCallbacks = borrowedRows;
	win.retiredRows.length = 0;
	const range = doc.createRange();
	range.selectNodeContents(doc.body);
	const fragment = range.createContextualFragment(
		'<x-retired-control data-v="fragment"></x-retired-control>',
	);
	result.fragment = [
		fragment.firstChild instanceof constructor,
		fragment.ownerDocument === doc,
		win.retiredRows.slice(),
	];
	registry.upgrade(created);
	result.explicitUpgrade = [
		created instanceof constructor,
		win.retiredRows.slice(),
	];
	result.definitionRetained = registry.get("x-retired-control") === constructor;
	const queuedFrame = document.createElement("iframe");
	document.body.append(queuedFrame);
	const queuedWin = queuedFrame.contentWindow;
	queuedWin.stopDefinitionRealm = () => queuedFrame.remove();
	queuedWin.eval(
		`globalThis.queuedRows=[];class Queued extends HTMLElement{static observedAttributes=['a','b'];constructor(){super();queuedRows.push('constructor');}attributeChangedCallback(name){queuedRows.push(name);if(name==='a')stopDefinitionRealm();}connectedCallback(){queuedRows.push('connected');}};customElements.define('x-retired-queued',Queued);document.body.innerHTML='<x-retired-queued a="one" b="two"></x-retired-queued>';`,
	);
	result.queuedRetirement = queuedWin.queuedRows.slice();
	return result;
})();
