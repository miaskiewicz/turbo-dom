// Chrome-156 control for legacy `kern` tables in web fonts (turbo-dom#240): which `kern`
// tables Chrome applies to an @font-face font with no GPOS. Four variants cross the outline
// format with the `kern` table version:
//   cff-v1  WarsawGothic-BnBV.otf as shipped (CFF, Apple `kern` version 1.0)
//   cff-v0  the same font with its `kern` table rewritten to the OpenType version 0 header
//   tt-v0   LiberationSerif-Regular.ttf with GPOS hidden (its `kern` table is version 0)
//   tt-v1   the same with its `kern` table rewritten to the Apple version 1.0 header
// The pair data (format 0) is untouched; only the table and subtable headers change, and
// a hidden table has its tag renamed to `zzzz` (an unknown table, dropped by OTS).
// Per variant and text: canvas measureText with fontKerning normal and none, and the
// HTML Range width of the text (DSF 2), with font-kerning normal and none.
// Asserted by crates/turbo-dom-layout/tests/web_font_kern_chrome.rs.
//
// Usage: node scripts/web-font-kern-chrome-control.mjs [--chrome PATH]
//   --chrome defaults to the frozen Chrome 156.0.8078.47 headless shell (mock keychain).
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(homedir(), ".cache/turbo-consolidation/chrome156/chrome-headless-shell-mac-arm64/chrome-headless-shell");
const playwrightCore = process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
const fonts = join(repo, "crates/turbo-dom-layout/assets/fonts");

function tables(buf) {
  const n = buf.readUInt16BE(4);
  const out = {};
  for (let i = 0; i < n; i++) {
    const at = 12 + 16 * i;
    out[buf.toString("latin1", at, at + 4)] = { at, offset: buf.readUInt32BE(at + 8), length: buf.readUInt32BE(at + 12) };
  }
  return out;
}
function hide(buf, tag) {
  const out = Buffer.from(buf);
  const t = tables(out)[tag];
  if (t) out.write("zzzz", t.at, "latin1");
  return out;
}
// Rewrite the `kern` table header in place-compatible form and append it as a new table
// (the directory entry is repointed; the old bytes stay as unreferenced padding).
function rewriteKern(buf, toVersion) {
  const t = tables(buf).kern;
  const kern = buf.subarray(t.offset, t.offset + t.length);
  const subs = [];
  if (kern.readUInt16BE(0) === 0) {
    let at = 4;
    for (let i = 0, n = kern.readUInt16BE(2); i < n; i++) {
      const len = kern.readUInt16BE(at + 2);
      const coverage = kern.readUInt16BE(at + 4);
      subs.push({ format: coverage >> 8, horizontal: (coverage & 1) === 1, data: kern.subarray(at + 6, at + len) });
      at += len;
    }
  } else {
    let at = 8;
    for (let i = 0, n = kern.readUInt32BE(4); i < n; i++) {
      const len = kern.readUInt32BE(at);
      const coverage = kern.readUInt16BE(at + 4);
      subs.push({ format: coverage & 0xff, horizontal: (coverage & 0x8000) === 0, data: kern.subarray(at + 8, at + len) });
      at += len;
    }
  }
  const parts = [];
  if (toVersion === 0) {
    const head = Buffer.alloc(4);
    head.writeUInt16BE(subs.length, 2);
    parts.push(head);
    for (const s of subs) {
      const h = Buffer.alloc(6);
      h.writeUInt16BE(s.data.length + 6, 2);
      h.writeUInt16BE((s.format << 8) | (s.horizontal ? 1 : 0), 4);
      parts.push(h, s.data);
    }
  } else {
    const head = Buffer.alloc(8);
    head.writeUInt32BE(0x00010000, 0);
    head.writeUInt32BE(subs.length, 4);
    parts.push(head);
    for (const s of subs) {
      const h = Buffer.alloc(8);
      h.writeUInt32BE(s.data.length + 8, 0);
      h.writeUInt16BE((s.horizontal ? 0 : 0x8000) | s.format, 4);
      parts.push(h, s.data);
    }
  }
  const table = Buffer.concat(parts);
  const pad = (b) => Buffer.concat([b, Buffer.alloc((4 - (b.length % 4)) % 4)]);
  const base = pad(Buffer.from(buf));
  const out = Buffer.concat([base, pad(table)]);
  out.writeUInt32BE(base.length, t.at + 8);
  out.writeUInt32BE(table.length, t.at + 12);
  return out;
}
const warsaw = readFileSync(join(fonts, "WarsawGothic-BnBV.otf"));
const liberation = hide(readFileSync(join(fonts, "liberation-serif/LiberationSerif-Regular.ttf")), "GPOS");
const VARIANTS = {
  "cff-v1": warsaw,
  "cff-v0": rewriteKern(warsaw, 0),
  "tt-v0": liberation,
  "tt-v1": rewriteKern(liberation, 1),
};
const css = Object.entries(VARIANTS)
  .map(([name, bytes]) => `@font-face{font-family:"${name}";src:url(data:font/sfnt;base64,${bytes.toString("base64")})}`)
  .join("");
const TEXTS = ["AVATAR Wave", "To Yo, LT", "WAVE"];
const SIZE = 48;

const server = createServer((_, res) => {
  res.writeHead(200, { "content-type": "text/html" });
  res.end(`<!doctype html><html><head><style>${css}body{margin:0}</style></head><body></body></html>`);
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
try {
  const browser = await chromium.launch({
    executablePath: chromePath,
    headless: true,
    args: ["--use-mock-keychain", "--password-store=basic", "--force-device-scale-factor=2"],
  });
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  const result = await page.evaluate(
    async ({ names, texts, size }) => {
      const out = [];
      for (const name of names) {
        const loaded = await document.fonts.load(`${size}px "${name}"`);
        const ctx = document.createElement("canvas").getContext("2d");
        ctx.font = `${size}px "${name}"`;
        for (const text of texts) {
          const row = { variant: name, loaded: loaded.length, text };
          for (const kerning of ["normal", "none"]) {
            ctx.fontKerning = kerning;
            row[`canvas_${kerning}`] = ctx.measureText(text).width;
            const div = document.createElement("div");
            div.setAttribute("style", `font-family:"${name}";font-size:${size}px;font-kerning:${kerning};width:2000px`);
            div.textContent = text;
            document.body.append(div);
            const r = document.createRange();
            r.selectNodeContents(div);
            row[`html_${kerning}`] = r.getBoundingClientRect().width;
            div.remove();
          }
          out.push(row);
        }
      }
      return out;
    },
    { names: Object.keys(VARIANTS), texts: TEXTS, size: SIZE },
  );
  const control = {
    browser: `Chrome/${browser.version()}`,
    collectorSha256: sha256(readFileSync(fileURLToPath(import.meta.url))),
    variantSha256: Object.fromEntries(Object.entries(VARIANTS).map(([n, b]) => [n, sha256(b)])),
    size: SIZE,
    errors,
    result,
  };
  const out = join(repo, "crates/turbo-dom-layout/tests/fixtures/web-font-kern/chrome156.json");
  writeFileSync(out, `${JSON.stringify(control, null, 1)}\n`);
  console.log(out);
  for (const r of result) console.log(r.variant, r.loaded, r.text, r.canvas_normal, r.canvas_none, r.html_normal, r.html_none);
  await browser.close();
} finally {
  server.close();
}
