export const cases = [
	{
		name: "feature-cards",
		selectors: [
			".MuiGrid-container",
			".MuiGrid-container > .MuiGrid-root",
			"h4",
		],
		source:
			"website-global/apps/web/src/components/Blocks/FeatureVariantBlock.tsx CardVariant + UI FeatureCard.tsx",
	},
	{
		name: "product-cards",
		selectors: [
			".MuiGrid-container",
			".MuiGrid-container > .MuiGrid-root",
			"h3",
		],
		source:
			"website-global/apps/web/src/components/Blocks/FeatureVariantBlock.tsx ProductVariant",
	},
	{
		name: "partner-logos",
		selectors: [".MuiContainer-root", "img", "button"],
		source: "website-global homepage partner logos (nested MUI Stack)",
	},
	{
		name: "resources",
		selectors: [".MuiContainer-root", "h2", "h4", "img"],
		source: "website-global homepage resource cards (nested MUI Stack)",
	},
	{
		name: "mui-width-expression",
		selectors: [".grid", ".item", ".card", ".badge", ".body"],
		source: "Reduced exact MUI Grid width expression from both card components",
	},
	{
		name: "nested-grid-tracks",
		selectors: [".stack", ".grid", ".card", ".image", ".content"],
		source:
			"Reduced resource-card CSS: full-width repeat(3,1fr) grid inside a centered flex column",
	},
	{
		name: "mui-width-additive",
		selectors: [".grid", ".item", ".card", ".badge", ".body"],
		source: "Same reduction with algebraically equivalent additive calc",
	},
	{
		name: "nested-flex-padding",
		selectors: [".grid", ".item", ".card", ".badge", ".body"],
		source:
			"Width arithmetic removed: isolates nested flex grow, definite height and padding",
	},
];

// This function is serialized into each engine, so it has no host dependencies.
export function measure(selectors) {
	return selectors.flatMap((selector) =>
		Array.from(document.querySelectorAll(selector), (element, index) => {
			const box = element.getBoundingClientRect();
			return {
				key: `${selector}:${index}`,
				rect: [box.x, box.y, box.width, box.height],
			};
		}),
	);
}

export function geometryDiff(chrome, native, tolerance = 1) {
	const actual = new Map(native.map((entry) => [entry.key, entry.rect]));
	const differences = chrome.flatMap(({ key, rect }) => {
		const other = actual.get(key);
		if (
			other &&
			rect.every((value, index) => Math.abs(value - other[index]) <= tolerance)
		)
			return [];
		return [{ key, chromium: rect, native: other ?? null }];
	});
	const expected = new Set(chrome.map((entry) => entry.key));
	for (const { key, rect } of native) {
		if (!expected.has(key))
			differences.push({ key, chromium: null, native: rect });
	}
	return differences;
}
