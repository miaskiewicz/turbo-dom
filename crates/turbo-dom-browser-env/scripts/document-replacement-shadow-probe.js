(() => {
	let openSeen = null,
		closedSeen = null;
	customElements.define(
		"x-shadow-review",
		class extends HTMLElement {
			constructor() {
				super();
				const root = this.shadowRoot;
				openSeen = {
					shadow: !!root,
					owner:
						this.ownerDocument === document && root?.ownerDocument === document,
					mode: root?.mode,
					clonable: root?.clonable,
					serializable: root?.serializable,
					delegatesFocus: root?.delegatesFocus,
					text: root?.querySelector("span").textContent,
				};
			}
		},
	);
	customElements.define(
		"x-shadow-child",
		class extends HTMLElement {
			constructor() {
				super();
				const root = this.getRootNode();
				closedSeen = {
					mode: root.mode,
					clonable: root.clonable,
					owner:
						this.ownerDocument === document && root.ownerDocument === document,
					hidden: root.host.shadowRoot === null,
					host: root.host.id,
				};
			}
		},
	);
	globalThis.inspectReplacement = () => {
		document.getElementById("open").shadowRoot.querySelector("#action").click();
		return {
			title: document.title,
			openSeen,
			closedSeen,
			handlerMark,
			order: shadowOrder,
			templates: document.querySelectorAll("template").length,
		};
	};
})();
