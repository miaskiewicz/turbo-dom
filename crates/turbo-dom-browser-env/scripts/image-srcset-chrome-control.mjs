// Chrome-156 control for <img srcset/sizes> and <picture>/<source> selection (P5b-X7,
// miaskiewicz/turbo-dom#71). Serves src/html_image_srcset_cases.json's page at
// http://127.0.0.1:PORT/page/ (viewport 1024x768) and runs src/html_image_srcset_probe.js
// after its load event, three times:
//   load1 — every /img/<W>x<H>/<name>.png answers a W x H PNG, deviceScaleFactor 1;
//   load2 — the same at deviceScaleFactor 2;
//   none1 — every image request answers 404 ("nothing loads", turbo-test's case), DPR 1.
// The cases come from scripts/make-image-srcset-cases.py.
//
// Usage (from crates/turbo-dom-browser-env):
//   node scripts/image-srcset-chrome-control.mjs [--chrome PATH] [--out src/html_image_srcset_chrome156.json]
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../../../turbo-surf/node_modules/playwright-core).
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { deflateSync } from "node:zlib";

const crate = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(
    homedir(),
    "Library/Caches/ms-playwright/chromium-1248/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const out = resolve(values.out || join(crate, "src/html_image_srcset_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(crate, "../../../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");
const probe = readFileSync(join(crate, "src/html_image_srcset_probe.js"), "utf8");
const casesBytes = readFileSync(join(crate, "src/html_image_srcset_cases.json"));
const cases = JSON.parse(casesBytes);

const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc = (buf) => {
  let c = 0xffffffff;
  for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
const chunk = (type, data) => {
  const head = Buffer.alloc(8);
  head.writeUInt32BE(data.length, 0);
  head.write(type, 4, "latin1");
  const tail = Buffer.alloc(4);
  tail.writeUInt32BE(crc(Buffer.concat([head.subarray(4), data])), 0);
  return Buffer.concat([head, data, tail]);
};
/** A solid grey W x H 8-bit RGB PNG. */
const png = (w, h) => {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;
  ihdr[9] = 2;
  const row = Buffer.concat([Buffer.from([0]), Buffer.alloc(w * 3, 0x80)]);
  const raw = Buffer.concat(Array.from({ length: h }, () => row));
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
};

let loads = true;
const server = createServer((request, response) => {
  const path = new URL(request.url, "http://x").pathname;
  if (path === "/page/") {
    response.setHeader("Content-Type", "text/html");
    response.end(cases.html);
    return;
  }
  const match = /^\/img\/(\d+)x(\d+)\//.exec(path);
  if (loads && match) {
    response.setHeader("Content-Type", "image/png");
    response.end(png(Number(match[1]), Number(match[2])));
    return;
  }
  response.statusCode = 404;
  response.end();
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const url = `http://127.0.0.1:${server.address().port}/page/`;
const browser = await chromium.launch({ executablePath: chromePath, headless: true });
const result = {};
const errors = [];
try {
  for (const [label, dpr, load] of [["load1", 1, true], ["load2", 2, true], ["none1", 1, false]]) {
    loads = load;
    const context = await browser.newContext({
      viewport: { width: cases.viewport[0], height: cases.viewport[1] },
      deviceScaleFactor: dpr,
    });
    const page = await context.newPage();
    page.on("pageerror", (error) => errors.push({ label, error: String(error) }));
    await page.goto(url, { waitUntil: "load" });
    result[label] = await page.evaluate(probe);
    await context.close();
  }
} finally {
  await browser.close();
  await new Promise((done) => server.close(done));
}
const oracle = {
  browser: `Chrome/${browser.version()}`,
  generator: "scripts/image-srcset-chrome-control.mjs",
  casesSha256: sha(casesBytes),
  probeSha256: sha(probe),
  errors,
  result,
};
writeFileSync(out, `${JSON.stringify(oracle, null, 1)}\n`);
console.log(JSON.stringify({ browser: oracle.browser, errors: errors.length, out }));
