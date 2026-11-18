import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
const { values } = parseArgs({
	options: { chrome: { type: "string" }, out: { type: "string" } },
});
const require = createRequire(import.meta.url);
const { chromium } = require(
	process.env.PLAYWRIGHT_CORE ||
		resolve(
			dirname(fileURLToPath(import.meta.url)),
			"../../turbo-surf/node_modules/playwright-core",
		),
);
const source = readFileSync(
	new URL(
		"../crates/turbo-dom-layout/tests/aspect_absolute_cases.json",
		import.meta.url,
	),
	"utf8",
);
const browser = await chromium.launch({
	executablePath:
		values.chrome ||
		resolve(
			homedir(),
			".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
		),
	headless: true,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
try {
	const page = await browser.newPage({
		viewport: { width: 1000, height: 900 },
	});
	const cases = [];
	for (const c of JSON.parse(source)) {
		await page.setContent(c.html);
		cases.push({
			name: c.name,
			rects: await page.evaluate(() =>
				Object.fromEntries(
					["parent", "child", "grand"]
						.filter((id) => document.getElementById(id))
						.map((id) => {
							const r = document.getElementById(id).getBoundingClientRect();
							return [id, [r.x, r.y, r.width, r.height]];
						}),
				),
			),
		});
	}
	writeFileSync(
		values.out ||
			new URL(
				"../crates/turbo-dom-layout/tests/aspect_absolute_chrome156.json",
				import.meta.url,
			),
		`${JSON.stringify(
			{
				browser: await browser.version(),
				sourceSha256: createHash("sha256").update(source).digest("hex"),
				cases,
			},
			null,
			2,
		)}\n`,
	);
} finally {
	await browser.close();
}
