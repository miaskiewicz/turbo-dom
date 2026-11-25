// Offline CSS selector-program oracle; no application services or live network.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { probe } from "../test/fixtures/css-selector-program-probe.mjs";

const repo = fileURLToPath(new URL("../", import.meta.url));
const { values } = parseArgs({
	options: { chrome: { type: "string" }, out: { type: "string" } },
});
const chrome =
	values.chrome ||
	join(
		homedir(),
		".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
	);
const { chromium } = createRequire(import.meta.url)(
	process.env.PLAYWRIGHT_CORE ||
		resolve(repo, "../turbo-surf/node_modules/playwright-core"),
);
const prefix = join(repo, "test/fixtures");
const cases = readFileSync(
	join(prefix, "css-selector-program-cases.json"),
	"utf8",
);
const source = readFileSync(
	join(prefix, "css-selector-program-probe.mjs"),
	"utf8",
);
const browser = await chromium.launch({
	executablePath: chrome,
	headless: true,
	args: ["--use-mock-keychain", "--password-store=basic"],
});
try {
	const version = browser.version();
	if (version !== "159.0.8154.6")
		throw new Error(
			`This frozen fixture requires Chromium159.0.8154.6; received ${version}. Pass the matching --chrome path.`,
		);
	const page = await browser.newPage();
	const result = await page.evaluate(probe, JSON.parse(cases));
	const sha = (text) => createHash("sha256").update(text).digest("hex");
	const out = values.out || join(prefix, "css_selector_program_chrome159.json");
	writeFileSync(
		out,
		JSON.stringify(
			{
				browser: version,
				probeSha256: sha(source),
				casesSha256: sha(cases),
				result,
			},
			null,
			2,
		) + "\n",
	);
	console.log(out);
} finally {
	await browser.close();
}
