import { readFileSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({
	options: { chrome: { type: "string" }, out: { type: "string" } },
});
if (!values.chrome)
	throw new Error("Pass --chrome with the independent oracle executable");
const require = createRequire(import.meta.url);
const { chromium } = require(
	process.env.PLAYWRIGHT_CORE ||
		resolve(repo, "../turbo-surf/node_modules/playwright-core"),
);
const casesPath = resolve(
	repo,
	"crates/turbo-dom-layout/tests/fixtures/svg_custom_paint_cases.json",
);
const cases = JSON.parse(readFileSync(casesPath, "utf8"));
const browser = await chromium.launch({
	executablePath: values.chrome,
	headless: true,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
try {
	const page = await browser.newPage({ viewport: { width: 100, height: 100 } });
	for (const entry of cases) {
		entry.rgba = await page.evaluate(async (svg) => {
			const image = new Image();
			image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
			await image.decode();
			const canvas = document.createElement("canvas");
			canvas.width = canvas.height = 80;
			const context = canvas.getContext("2d");
			context.drawImage(image, 0, 0);
			return [...context.getImageData(40, 40, 1, 1).data];
		}, entry.svg);
	}
	const sha = (path) =>
		createHash("sha256").update(readFileSync(path)).digest("hex");
	writeFileSync(
		values.out ||
			resolve(
				repo,
				"crates/turbo-dom-layout/tests/fixtures/svg_custom_paint_chrome159.json",
			),
		`${JSON.stringify({ version: browser.version(), probeSHA256: sha(fileURLToPath(import.meta.url)), casesSHA256: sha(casesPath), cases }, null, 2)}\n`,
	);
} finally {
	await browser.close();
}
