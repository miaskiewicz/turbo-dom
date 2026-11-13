import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const { values } = parseArgs({
	options: { chrome: { type: "string" }, out: { type: "string" } },
});
const chromePath =
	values.chrome ||
	join(
		homedir(),
		".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
	);
const require = createRequire(import.meta.url);
const { chromium } = require(
	process.env.PLAYWRIGHT_CORE ||
		resolve(repo, "../turbo-surf/node_modules/playwright-core"),
);
const probe = readFileSync(
	join(
		repo,
		"crates/turbo-dom-browser-env/scripts/touch-event-sequence-probe.js",
	),
	"utf8",
);
const browser = await chromium.launch({
	executablePath: chromePath,
	headless: true,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
try {
	const page = await browser.newPage();
	await page.setContent("<body>control</body>");
	const control = {
		browser: await browser.version(),
		probeSha256: createHash("sha256").update(probe).digest("hex"),
		result: await page.evaluate(probe),
	};
	writeFileSync(
		resolve(
			values.out ||
				join(
					repo,
					"crates/turbo-dom-browser-env/src/touch_event_sequence_chrome156.json",
				),
		),
		`${JSON.stringify(control, null, 2)}\n`,
	);
} finally {
	await browser.close();
}
