(() => {
	const cases = [
		{
			name: "default-style",
			css: "",
			html: "<style id=subject>/* module */</style>",
			eligible: true,
		},
		{
			name: "default-script",
			css: "",
			html: "<script id=subject type=application/json>{}</script>",
			eligible: true,
		},
		{
			name: "inline-block",
			css: "",
			html: '<style id=subject style="display:block;width:40px;height:20px">/* module */</style>',
			eligible: false,
		},
		{
			name: "rule-block",
			css: "#subject{display:block;width:40px;height:20px}",
			html: "<style id=subject>/* module */</style>",
			eligible: false,
		},
		{
			name: "important-block",
			css: "#subject{display:block!important;width:40px;height:20px}",
			html: '<style id=subject style="display:none">/* module */</style>',
			eligible: false,
		},
		{
			name: "inline-important-none",
			css: "#subject{display:block}",
			html: '<style id=subject style="display:none!important">/* module */</style>',
			eligible: false,
		},
		{
			name: "contents",
			css: "",
			html: '<div id=subject style="display:contents"><span style="display:block;width:40px;height:20px">child</span></div>',
			eligible: false,
		},
		{
			name: "ancestor-only",
			css: "",
			html: '<div style="display:none"><div id=subject style="display:block;width:40px;height:20px">child</div></div>',
			eligible: false,
		},
		{
			name: "literal-none",
			css: "style{display:none}",
			html: "<style id=subject>/* module */</style>",
			eligible: true,
		},
		{
			name: "irrelevant-type",
			css: ".parent > span{display:block}",
			html: "<div class=parent><style id=subject>/* module */</style></div>",
			eligible: true,
		},
		{
			name: "irrelevant-pseudo",
			css: 'style::before{display:block;content:"x"}',
			html: "<style id=subject>/* module */</style>",
			eligible: true,
		},
		{
			name: "media-invariant",
			css: "@media(min-width:1px){.other{display:block}}",
			html: "<style id=subject>/* module */</style>",
			eligible: true,
		},
		{
			name: "media-potential",
			css: "@media(max-width:0px){style{display:block}}",
			html: "<style id=subject>/* module */</style>",
			eligible: false,
		},
		{
			name: "supports-potential",
			css: "@supports(not-a-property:invalid){style{display:block}}",
			html: "<style id=subject>/* module */</style>",
			eligible: false,
		},
		{
			name: "container-override",
			css: "@container(min-width:1px){style{display:block;width:40px;height:20px}}",
			html: '<div style="container-type:inline-size;width:100px"><style id=subject>/* module */</style></div>',
			eligible: false,
		},
		{
			name: "starting-style",
			css: "@starting-style{style{display:block}}",
			html: "<style id=subject>/* module */</style>",
			eligible: false,
		},
	];
	const rows = [];
	for (const item of cases) {
		const sheet = document.createElement("style");
		sheet.textContent = item.css;
		document.head.appendChild(sheet);
		const root = document.createElement("div");
		root.innerHTML = item.html;
		document.body.appendChild(root);
		const element = root.querySelector("#subject"),
			style = getComputedStyle(element),
			rect = element.getBoundingClientRect();
		rows.push({
			...item,
			display: style.display,
			position: style.position,
			width: rect.width,
			height: rect.height,
			clientRects: element.getClientRects().length,
		});
		root.remove();
		sheet.remove();
	}
	return rows;
})();
