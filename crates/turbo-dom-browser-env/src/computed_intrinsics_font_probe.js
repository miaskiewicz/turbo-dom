(() => {
	const read = getComputedStyle;
	const fields = [
		"font",
		"fontFamily",
		"fontSize",
		"fontStyle",
		"fontWeight",
		"fontStretch",
		"fontVariantCaps",
		"lineHeight",
	];
	const values = (element) => {
		const style = read(element),
			result = {};
		for (const field of fields) result[field] = style[field];
		result.properties = [
			style.getPropertyValue("font"),
			style.getPropertyValue("font-size"),
			style.getPropertyValue("line-height"),
		];
		return result;
	};
	const root = document.documentElement;
	root.style.cssText = "";
	document.body.style.cssText = "";
	const defaults = values(document.body);
	root.style.fontSize = "16px";
	root.style.fontFamily = "serif";
	const cases = [
		["initial", "", ""],
		["hidden", "display:none", ""],
		["longhands", "font-family:monospace;font-size:20px;line-height:1.5", ""],
		[
			"shorthand",
			'font:italic small-caps bold 20px/150% "Example Family",serif',
			"",
		],
		["quoted-comma", 'font-family:"A,B",serif;font-size:18px', ""],
		[
			"unitless-inheritance",
			"font-size:20px;line-height:1.5",
			"font-size:10px",
		],
		[
			"percentage-inheritance",
			"font-size:20px;line-height:150%",
			"font-size:10px",
		],
		["length-inheritance", "font-size:20px;line-height:30px", "font-size:10px"],
		["em-line-height", "font-size:20px;line-height:1.25em", "font-size:10px"],
		["zero-line-height", "font-size:20px;line-height:0", ""],
		["font-size-em", "font-size:20px", "font-size:2em"],
		["font-size-rem", "font-size:20px", "font-size:2rem"],
		["font-size-percent", "font-size:20px", "font-size:150%"],
		["font-size-zero", "font-size:20px", "font-size:0"],
		[
			"fractional-font",
			"font-size:13px;line-height:1.3333333",
			"font-size:133.333333%",
		],
		["initial-values", "font:italic bold 24px/2 monospace", "font:initial"],
		["inherit-values", "font:italic bold 24px/2 monospace", "font:inherit"],
		["unset-values", "font:italic bold 24px/2 monospace", "font:unset"],
		["revert-values", "font:italic bold 24px/2 monospace", "font:revert"],
		[
			"revert-layer-values",
			"font:italic bold 24px/2 monospace",
			"font:revert-layer",
		],
		["lighter", "font-weight:700", "font-weight:lighter"],
		["bolder", "font-weight:400", "font-weight:bolder"],
		["stretch", "font:20px serif;font-stretch:condensed", ""],
		["oblique", "font-style:oblique", ""],
		["negative-font-calc", "font-size:calc(1px - 10px)", ""],
		["negative-line-calc", "line-height:calc(1px - 10px)", ""],
		["variant-after-font", "font:20px serif;font-variant:small-caps", ""],
		[
			"calc-line-inheritance",
			"font-size:20px;line-height:calc(1em + 10px)",
			"font-size:10px",
		],
		["calc-font-size", "font-size:20px", "font-size:calc(1em + 2px)"],
		[
			"scalar-calc-line-inheritance",
			"font-size:20px;line-height:calc(1 + 1)",
			"font-size:10px",
		],
		["mixedcase-em", "font-size:20px;line-height:1.5EM", "font-size:2eM"],
		["mixedcase-rem-px", "font-size:20px", "font-size:2ReM;line-height:2pX"],
		["mixedcase-pt-pc", "font-size:20px", "font-size:18Pt;line-height:1pC"],
		[
			"mixedcase-absolute-calc",
			"",
			"font-size:CALC(1IN + 2.54CM + 25.4MM + 101.6Q + 6PC + 72PT + 1PX);line-height:CALC(1eM + 1pX)",
		],
		["uppercase-shorthand", "font:ITALIC SMALL-CAPS BOLD 20PX/2 SERIF", ""],
		[
			"mixedcase-inherit",
			"font:italic bold 20px/1.5 monospace",
			"font:InHeRiT",
		],
		[
			"uppercase-normal-smaller",
			"font:italic bold 20px/2 serif",
			"font-style:NORMAL;font-weight:NORMAL;font-variant-caps:NORMAL;line-height:NORMAL;font-size:SMALLER;font-family:MoNoSpAcE",
		],
		["uppercase-initial", "font:italic bold 24px/2 monospace", "font:INITIAL"],
		["uppercase-revert-layer", "font:20px/1.5 monospace", "font:REVERT-LAYER"],
		[
			"mixedcase-family",
			"",
			'font-family:"MiXeD Family", mOnOsPaCe;font-size:18PX',
		],
	];
	const rows = [];
	for (const [name, parentStyle, childStyle] of cases) {
		const parent = document.createElement("div"),
			child = document.createElement("span");
		parent.style.cssText = parentStyle;
		child.style.cssText = childStyle;
		parent.append(child);
		document.body.append(parent);
		rows.push({ name, parentStyle, childStyle, value: values(child) });
		parent.remove();
	}
	const detached = document.createElement("div");
	detached.style.font = "italic 24px/2 monospace";
	const parent = document.createElement("div"),
		child = document.createElement("span");
	parent.style.cssText = "font-size:20px;line-height:1.5;font-family:monospace";
	parent.append(child);
	document.body.append(parent);
	const retained = read(child);
	const live = [retained.font, retained.lineHeight];
	child.style.fontSize = "10px";
	live.push(retained.font, retained.lineHeight);
	parent.style.lineHeight = "125%";
	live.push(retained.font, retained.lineHeight);
	child.style.fontSize = "12px";
	live.push(retained.font, retained.lineHeight);
	let conversions = 0;
	const coerced = retained.getPropertyValue({
		toString() {
			conversions++;
			child.style.fontSize = "14px";
			return "font-size";
		},
	});
	const error = (fn) => {
		try {
			fn();
			return null;
		} catch (value) {
			return value.name;
		}
	};
	const wrongReceiver = error(() =>
		CSSStyleDeclaration.prototype.getPropertyValue.call({}, "font"),
	);
	const symbol = error(() => retained.getPropertyValue(Symbol("font")));
	let invalidConversions = 0;
	const invalidReceiverConversion = error(() =>
		CSSStyleDeclaration.prototype.getPropertyValue.call(
			{},
			{
				toString() {
					invalidConversions++;
					return "font";
				},
			},
		),
	);
	const invalidReceiverSymbol = error(() =>
		CSSStyleDeclaration.prototype.getPropertyValue.call({}, Symbol("font")),
	);
	parent.remove();
	const disconnectedRetained = [retained.font, retained.lineHeight];
	const inert = new DOMParser().parseFromString(
		"<!doctype html><body><b>inert</b>",
		"text/html",
	);
	const template = document.createElement("template");
	template.innerHTML = "<span>inert template</span>";
	document.body.append(template);
	const host = document.createElement("div");
	host.style.cssText = "font-size:20px;line-height:1.5;font-family:monospace";
	const shadow = host.attachShadow({ mode: "open" }),
		leaf = document.createElement("span");
	leaf.style.fontSize = "10px";
	shadow.append(leaf);
	document.body.append(host);
	const lifecycle = {
		disconnectedRetained,
		inert: values(inert.body),
		template: values(template.content.firstChild),
		shadow: values(leaf),
	};
	const slotted = document.createElement("span");
	const slot = document.createElement("slot");
	slot.style.cssText = "font-size:30px;line-height:2;font-family:serif";
	shadow.append(slot);
	host.append(slotted);
	lifecycle.slotted = values(slotted);
	slot.style.fontSize = "25px";
	lifecycle.slottedLive = values(slotted);
	const manualHost = document.createElement("div"),
		manualLeaf = document.createElement("span"),
		manualSlot = document.createElement("slot");
	manualHost.style.cssText = "font-size:20px;font-family:monospace";
	const manualRoot = manualHost.attachShadow({
		mode: "closed",
		slotAssignment: "manual",
	});
	manualSlot.style.cssText = "font-size:30px;line-height:2;font-family:serif";
	manualRoot.append(manualSlot);
	manualHost.append(manualLeaf);
	document.body.append(manualHost);
	manualSlot.assign(manualLeaf);
	lifecycle.manualSlot = values(manualLeaf);
	manualHost.remove();
	const unassigned = document.createElement("span");
	unassigned.slot = "missing";
	host.append(unassigned);
	const fallback = document.createElement("span"),
		fallbackDescendant = document.createElement("b");
	fallback.append(fallbackDescendant);
	slot.append(fallback);
	lifecycle.unassigned = values(unassigned);
	lifecycle.suppressedFallback = values(fallback);
	lifecycle.suppressedDescendant = values(fallbackDescendant);
	host.remove();
	template.remove();
	return {
		lifecycle,
		defaults,
		rows,
		detached: values(detached),
		live,
		coercion: [coerced, conversions],
		wrongReceiver,
		invalidReceiverConversion,
		invalidReceiverSymbol,
		invalidConversions,
		symbol,
	};
})();
