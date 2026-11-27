// Independent localhost Hit clip ownership oracle. No application services.
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const { values } = parseArgs({
	options: { chrome: { type: "string" }, out: { type: "string" } },
});
const chrome =
	values.chrome ||
	join(
		homedir(),
		".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell",
	);
const out =
	values.out ||
	join(
		repo,
		"crates/turbo-dom-layout/tests/fixtures/hit_clip_ownership_chrome159.json",
	);
const { chromium } = createRequire(import.meta.url)(
	process.env.PLAYWRIGHT_CORE ||
		resolve(repo, "../turbo-surf/node_modules/playwright-core"),
);
const probe = readFileSync(
	join(
		repo,
		"crates/turbo-dom-layout/tests/fixtures/hit_clip_ownership_probe.js",
	),
	"utf8",
);
const server = createServer((_request, response) => {
	response.writeHead(200, { "content-type": "text/html" });
	response.end("<!doctype html><title>Independent own-box control</title>");
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
let browser;
try {
	browser = await chromium.launch({
		executablePath: chrome,
		headless: true,
		args: ["--use-mock-keychain", "--password-store=basic"],
	});
	const page = await browser.newPage();
	await page.goto(`http://127.0.0.1:${server.address().port}/`);
	const result = await page.evaluate(probe);
	writeFileSync(
		out,
		JSON.stringify(
			{
				browser: await browser.version(),
				probeSha256: createHash("sha256").update(probe).digest("hex"),
				result,
			},
			null,
			2,
		) + "\n",
	);
	console.log(out);
} finally {
	await browser?.close();
	await new Promise((resolve) => server.close(resolve));
}
