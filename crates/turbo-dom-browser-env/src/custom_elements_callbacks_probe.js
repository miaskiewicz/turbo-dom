(async () => {
	if (!globalThis.child) {
		const frame = document.createElement("iframe");
		document.body.append(frame);
		globalThis.child = frame.contentWindow;
	}
	const result = {},
		parentEvents = [],
		childEvents = [];
	const parentError = (event) => {
		parentEvents.push([event.error?.message, event.error instanceof Error]);
		event.preventDefault();
	};
	const childError = child.eval(
		`event => { callbackErrors.push([event.error?.message, event.error instanceof Error]); event.preventDefault(); }`,
	);
	addEventListener("error", parentError);
	child.callbackErrors = childEvents;
	child.addEventListener("error", childError);
	try {
		child.eval(`globalThis.callbackRows=[];
      globalThis.CallbackElement=class extends HTMLElement {
        static observedAttributes=['data-v'];
        connectedCallback(){callbackRows.push(['connected',this.ownerDocument===document,Object.getPrototypeOf(this)===CallbackElement.prototype]);if(this.id==='throw-child')throw new Error('child-lifecycle');}
        disconnectedCallback(){callbackRows.push(['disconnected',this.ownerDocument===document]);}
        adoptedCallback(old,next){callbackRows.push(['adopted',old===document,next===document]);if(this.id==='reentrant'&&next!==document){document.body.append(this);}}
        attributeChangedCallback(name,old,value){callbackRows.push(['attribute',name,old,value,this.ownerDocument===document]);if(value==='throw')throw new Error('child-attribute');}
      };customElements.define('x-callback-realm',CallbackElement);`);
		const node = child.document.createElement("x-callback-realm");
		node.id = "throw-child";
		child.document.body.append(node);
		node.setAttribute("data-v", "throw");
		document.body.append(node);
		node.setAttribute("data-v", "parent");
		result.callbacks = child.callbackRows.slice();
		result.errors = [parentEvents.slice(), childEvents.slice()];
		node.remove();
		child.callbackRows.length = 0;
		const nested = child.document.createElement("x-callback-realm");
		nested.id = "reentrant";
		child.document.body.append(nested);
		child.callbackRows.length = 0;
		document.body.append(nested);
		result.reentrant = [
			nested.ownerDocument === child.document,
			nested.parentNode === child.document.body,
			child.callbackRows.slice(),
		];
		nested.remove();
		const foreignCallback = function () {
			result.borrowedCallback = [
				this.ownerDocument === child.document,
				this instanceof child.CallbackElement,
				document === globalThis.document,
			];
			throw new Error("parent-callback");
		};
		child.ForeignCallback = foreignCallback;
		child.eval(
			`class ForeignCallbacks extends HTMLElement {connectedCallback=undefined;} ForeignCallbacks.prototype.connectedCallback=ForeignCallback;customElements.define('x-foreign-callback',ForeignCallbacks);document.body.append(document.createElement('x-foreign-callback'));`,
		);
		result.borrowedErrors = [parentEvents.slice(), childEvents.slice()];
		const interleaved = [];
		class Interleaved extends child.HTMLElement {
			connectedCallback() {
				interleaved.push([
					this.id,
					"connected",
					this.ownerDocument === child.document,
				]);
			}
			disconnectedCallback() {
				interleaved.push([
					this.id,
					"disconnected",
					this.ownerDocument === child.document,
				]);
			}
			adoptedCallback() {
				interleaved.push([
					this.id,
					"adopted",
					this.ownerDocument === child.document,
				]);
				if (this.id === "interleave-a" && this.ownerDocument === document)
					child.document.body.append(this.peer);
			}
		}
		child.customElements.define("x-interleaved-callback", Interleaved);
		const a = child.document.createElement("x-interleaved-callback"),
			b = child.document.createElement("x-interleaved-callback");
		a.id = "interleave-a";
		b.id = "interleave-b";
		a.peer = b;
		child.document.body.append(a, b);
		interleaved.length = 0;
		document.body.append(a, b);
		result.interleaved = interleaved.slice();
		a.remove();
		b.remove();
		const attributes = [];
		class NestedAttributes extends HTMLElement {
			static observedAttributes = ["data-v"];
			attributeChangedCallback(name, old, value) {
				attributes.push([old, value]);
				if (value === "outer") {
					this.setAttribute(name, "nested");
					throw new Error("after-nested");
				}
			}
		}
		customElements.define("x-nested-attributes", NestedAttributes);
		const attribute = document.createElement("x-nested-attributes");
		attribute.setAttribute("data-v", "outer");
		attribute.setAttribute("data-v", "final");
		result.nestedAttributes = [attributes, parentEvents.slice()];
		return result;
	} finally {
		removeEventListener("error", parentError);
		child.removeEventListener("error", childError);
	}
})();
