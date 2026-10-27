// Chrome-156 control for turbo-dom-raster's `<img>` handling (P5b,
// miaskiewicz/turbo-dom#27):
//
// 1. An `<img src="data:…">` paints from its own URL — base64 or percent-encoded PNG, SVG,
//    BMP — with no caller-fetched bytes (the screenshot decodes it itself).
// 2. Only the parsed document's HTML `<img>` ELEMENTS are images: `<img` text inside an
//    attribute value, a `<script>` string, a comment or `<template>` content is not one, so
//    neither the lazy-image fill (`delazy_images`) nor the fetch list (`image_urls`) may act
//    on it.
//
// Every case is one page (viewport 1280x800, `body{margin:0;background:#00ffff}`). Chrome for
// Testing 156 records per `[data-cid]` box its `rect` and the colour class
// (R/G/B/Y/W/T(the backdrop)/?) at the centres of its four quadrants (`points`, `samples`),
// read from a screenshot, plus `images`: the attributes of every element of
// `document.images` (the document's `<img>` elements; template content is not in it).
// 3. An image whose pixels do not decode (miaskiewicz/turbo-dom#107): Chrome loads one it can
//    size (a frame-corrupt AVIF, a PNG cut inside IDAT, a GIF with an empty LZW stream) and
//    paints nothing (the backdrop shows through), but rejects an AVIF whose primary item
//    extent runs past the end of the data (error, naturalWidth 0). `natural` records every
//    `document.images` element's [naturalWidth, naturalHeight].
// Asserted by crates/turbo-dom-raster/tests/raster_img_chrome.rs.
//
// Usage: node scripts/raster-img-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156;
//   --out    defaults to crates/turbo-dom-raster/tests/fixtures/raster_img_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { crc32, deflateSync } from "node:zlib";
import { classOf, decodePng } from "./lib/png-samples.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({
  options: { chrome: { type: "string" }, out: { type: "string" } },
});
const chromePath =
  values.chrome ||
  join(
    homedir(),
    "Library/Caches/ms-playwright/chromium-1248/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const out = resolve(values.out || join(repo, "crates/turbo-dom-raster/tests/fixtures/raster_img_chrome156.json"));
const playwrightCore = process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

const W = 64;
const H = 40;
const pos = (x, y) => `position:absolute;left:${x}px;top:${y}px`;
const at = (x, y) => `${pos(x, y)};width:${W}px;height:${H}px`;
const DOC = (body) =>
  `<!DOCTYPE html><html><head><style>body{margin:0;background:#00ffff}</style></head><body>${body}</body></html>`;

/** A `W x H` solid RGB PNG. */
function png([r, g, b]) {
  const chunk = (type, data) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(type, "latin1"), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body));
    return Buffer.concat([len, body, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(W, 0);
  ihdr.writeUInt32BE(H, 4);
  ihdr.set([8, 2, 0, 0, 0], 8);
  const row = Buffer.concat([Buffer.from([0]), Buffer.from(Array.from({ length: W }, () => [r, g, b]).flat())]);
  const idat = deflateSync(Buffer.concat(Array.from({ length: H }, () => row)));
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  return Buffer.concat([sig, chunk("IHDR", ihdr), chunk("IDAT", idat), chunk("IEND", Buffer.alloc(0))]);
}

/** A `W x H` solid 24-bit BI_RGB BMP (bottom-up rows, 4-byte aligned). */
function bmp([r, g, b]) {
  const stride = Math.ceil((W * 3) / 4) * 4;
  const pixels = Buffer.alloc(stride * H);
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) pixels.set([b, g, r], y * stride + x * 3);
  const head = Buffer.alloc(54);
  head.write("BM", 0, "latin1");
  head.writeUInt32LE(54 + pixels.length, 2);
  head.writeUInt32LE(54, 10);
  head.writeUInt32LE(40, 14);
  head.writeInt32LE(W, 18);
  head.writeInt32LE(H, 22);
  head.writeUInt16LE(1, 26);
  head.writeUInt16LE(24, 28);
  head.writeUInt32LE(pixels.length, 34);
  return Buffer.concat([head, pixels]);
}

const svg = (fill) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}"><rect width="${W}" height="${H}" fill="${fill}"/></svg>`;
const base64 = (mime, bytes) => `data:${mime};base64,${Buffer.from(bytes).toString("base64")}`;
const RED_PNG = base64("image/png", png([255, 0, 0]));
const GREEN_SVG = base64("image/svg+xml", svg("#00ff00"));
const BLUE_SVG_PERCENT = `data:image/svg+xml,${encodeURIComponent(svg("#0000ff"))}`;
const YELLOW_BMP = base64("image/bmp", bmp([255, 255, 0]));
/** `[start, end)` of the payload of the top-level box `kind` of an ISOBMFF file. */
function boxPayload(bytes, kind) {
  for (let off = 0; off + 8 <= bytes.length; ) {
    const size = bytes.readUInt32BE(off);
    if (bytes.toString("latin1", off + 4, off + 8) === kind) return [off + 8, off + size];
    off += size;
  }
  throw new Error(`no ${kind} box`);
}
// q.avif (64x40): its `mdat` holds only the primary item's AV1 stream.
const Q_AVIF = readFileSync(join(repo, "crates/turbo-dom-layout/tests/fixtures/images/formats/q.avif"));
const [mdatStart, mdatEnd] = boxPayload(Q_AVIF, "mdat");
const mid = mdatStart + Math.floor((mdatEnd - mdatStart) / 2);
// The colour stream's second half overwritten with 0xFF; the container is intact.
const AVIF_FRAME_CORRUPT = Buffer.from(Q_AVIF);
AVIF_FRAME_CORRUPT.fill(0xff, mid, mdatEnd);
// Cut in the middle of `mdat`: the item extent runs past the end of the data.
const AVIF_TRUNCATED = Q_AVIF.subarray(0, mid);
// The red PNG cut 8 bytes into its IDAT data.
const RED = png([255, 0, 0]);
const PNG_CUT_IDAT = RED.subarray(0, RED.indexOf("IDAT") + 4 + 8);
/** A `W x H` GIF (global table red/black) whose one image has an empty LZW stream. */
function gifEmptyLzw() {
  const lsd = Buffer.alloc(7);
  lsd.writeUInt16LE(W, 0);
  lsd.writeUInt16LE(H, 2);
  lsd.set([0x80, 0, 0], 4);
  const desc = Buffer.alloc(10);
  desc[0] = 0x2c;
  desc.writeUInt16LE(W, 5);
  desc.writeUInt16LE(H, 7);
  return Buffer.concat([
    Buffer.from("GIF89a", "latin1"),
    lsd,
    Buffer.from([255, 0, 0, 0, 0, 0]),
    desc,
    Buffer.from([2, 0]), // LZW minimum code size, then the block terminator: no data
    Buffer.from([0x3b]),
  ]);
}
const img = (cid, src, x, y) => `<img data-cid="${cid}" src="${src}" style="${pos(x, y)}">`;

const CASES = [
  // Item 1: `data:` images paint without caller-supplied bytes.
  { id: "data-png-base64", html: DOC(img("i", RED_PNG, 20, 10)) },
  { id: "data-svg-base64", html: DOC(img("i", GREEN_SVG, 20, 10)) },
  {
    id: "data-svg-percent-encoded",
    html: DOC(img("i", BLUE_SVG_PERCENT, 20, 10)),
  },
  { id: "data-bmp-base64", html: DOC(img("i", YELLOW_BMP, 20, 10)) },
  {
    // The URL scheme is case-insensitive.
    id: "data-uppercase-scheme",
    html: DOC(img("i", RED_PNG.replace(/^data:/, "DATA:"), 20, 10)),
  },
  {
    // Several images, one of them twice.
    id: "data-several",
    html: DOC(img("a", RED_PNG, 20, 10) + img("b", GREEN_SVG, 120, 10) + img("c", RED_PNG, 220, 10)),
  },
  {
    // A `data:` image in `<template>` content is not in the document: nothing paints.
    id: "data-in-template",
    html: DOC(`<template>${img("t", RED_PNG, 20, 10)}</template><div data-cid="d" style="${at(20, 10)}"></div>`),
  },
  // Item 2: `<img` text that is not an `<img>` element.
  {
    // An attribute value holding a lazy `<img` (double-quoted value, single-quoted inner
    // attributes): its `<div>` keeps its own style.
    id: "img-text-in-attribute",
    html: DOC(`<div data-cid="d" title="<img data-src='lazy.png'>" style="${at(20, 10)};background:#ff0000"></div>`),
  },
  {
    // Lazy `<img` text in a script string, a comment and template content, next to one
    // real lazy `<img>` element: only that element is an image.
    id: "img-text-in-script-comment-template",
    html: DOC(
      `<script>var s = "<img data-src='script.png'>";</script><!-- <img data-src="comment.png"> -->` +
        `<template><img data-src="template.png"></template>` +
        `<img data-src="real.png" alt="real"><div data-cid="d" style="${at(20, 10)};background:#0000ff"></div>`,
    ),
  },
  {
    // A real `<img>` whose alt text holds `<img` text, and a `data:` placeholder that
    // yields to a lazy URL.
    id: "img-text-in-alt",
    html: DOC(
      `<img alt="<img src='alt.png'>" data-src="hero.png"><img src="${RED_PNG}" data-src="lazy-hero.png">` +
        `<div data-cid="d" style="${at(20, 10)};background:#ffff00"></div>`,
    ),
  },
  // Item 3: images whose pixels do not decode.
  { id: "undecodable-avif-frame-corrupt", html: DOC(img("i", base64("image/avif", AVIF_FRAME_CORRUPT), 20, 10)) },
  { id: "undecodable-avif-truncated", html: DOC(img("i", base64("image/avif", AVIF_TRUNCATED), 20, 10)) },
  { id: "undecodable-png-cut-idat", html: DOC(img("i", base64("image/png", PNG_CUT_IDAT), 20, 10)) },
  { id: "undecodable-gif-empty-lzw", html: DOC(img("i", base64("image/gif", gifEmptyLzw()), 20, 10)) },
  { id: "undecodable-intact-avif", html: DOC(img("i", base64("image/avif", Q_AVIF), 20, 10)) },
];

const browser = await chromium.launch({
  executablePath: chromePath,
  headless: true,
});
const version = browser.version();
const page = await browser.newPage({
  viewport: { width: 1280, height: 800 },
  deviceScaleFactor: 1,
});
// The lazy URLs point nowhere: answer every non-data request with a 404.
await page.route("**/*", (route) => route.fulfill({ status: 404, body: "" }));
const cases = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  const { rects, images, natural } = await page.evaluate(async () => {
    for (const image of document.images) await image.decode().catch(() => {});
    return {
      rects: [...document.querySelectorAll("[data-cid]")].map((el) => {
        const r = el.getBoundingClientRect();
        return { cid: el.dataset.cid, rect: [r.x, r.y, r.width, r.height] };
      }),
      images: [...document.images].map((image) =>
        Object.fromEntries([...image.attributes].map((a) => [a.name, a.value])),
      ),
      natural: [...document.images].map((image) => [image.naturalWidth, image.naturalHeight]),
    };
  });
  const px = decodePng(await page.screenshot());
  const boxes = rects.map(({ cid, rect: [x, y, w, h] }) => {
    const points = [
      [x + w / 4, y + h / 4],
      [x + (3 * w) / 4, y + h / 4],
      [x + w / 4, y + (3 * h) / 4],
      [x + (3 * w) / 4, y + (3 * h) / 4],
    ].map(([a, b]) => [Math.floor(a), Math.floor(b)]);
    return {
      cid,
      rect: [x, y, w, h],
      points,
      samples: points.map(([a, b]) => classOf(px(a, b))).join(""),
    };
  });
  cases.push({ id: testCase.id, html: testCase.html, boxes, images, natural });
}
await browser.close();

const fixture = {
  generator: "scripts/raster-img-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  cases,
};
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${cases.length} cases -> ${out}`);
for (const c of cases) {
  const boxes = c.boxes.map((b) => `${b.cid}:${b.rect.map((v) => Math.round(v)).join(",")}:${b.samples}`).join("  ");
  console.log(c.id.padEnd(36), boxes, `images=${c.images.length}`, JSON.stringify(c.natural));
}
