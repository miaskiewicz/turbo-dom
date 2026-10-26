// Chrome-156 control for turbo-dom's selector engine (consolidation S3a, miaskiewicz/turbo-dom#18).
//
// Records, for every case below, what Chrome for Testing 156 says about each selector:
//   - `valid`: whether `document.querySelector(sel)` accepts it (no SyntaxError),
//   - `matches`: `el.matches(sel)` for every element (document order, template content
//     excluded — it is a separate fragment in Chrome), as a 0/1 string,
//   - `qsa`: `document.querySelectorAll(sel)` as indices into that element list,
//   - `winner`: getComputedStyle(el).order for every element (a 0/1 string) when the case
//     document gets one extra `<style>SEL { order: 1 }</style>` as the last child of
//     <head> (the sentinel; 0 = the rule did not apply).
// Asserted by crates/turbo-dom/tests/cascade_selectors_chrome.rs.
//
// Usage: node scripts/cascade-selectors-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the Playwright-cached Chrome for Testing 156
//            (~/Library/Caches/ms-playwright/chromium-1248/...);
//   --out    defaults to crates/turbo-dom/tests/fixtures/cascade_selectors_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core (default: a
// sibling turbo-surf checkout's node_modules/playwright-core).
import { writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(
    homedir(),
    "Library/Caches/ms-playwright/chromium-1248/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const out = resolve(values.out || join(repo, "crates/turbo-dom/tests/fixtures/cascade_selectors_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

// `bug` names the miaskiewicz/turbo-dom#18 entry a case covers (the S3a bug list), or
// "guard" for nearby behaviour that must NOT change.
const DOC = (body, head = "") => `<!DOCTYPE html><html><head>${head}</head><body>${body}</body></html>`;
const CASES = [
  {
    id: "escapes",
    bug: "escapes",
    html: DOC(
      `<div class="md:flex" id="a"><p class="a.b">x</p></div><div id="123" class="10 @sm"><span class="md">y</span></div>`,
    ),
    selectors: [
      ".md\\:flex",
      "div.md\\:flex",
      "#\\61",
      "\\70",
      "\\64 iv",
      ".a\\.b",
      "#\\31 23",
      ".\\31 0",
      ".\\@sm",
      ".md\\:flex > \\70",
      ":is(.md\\:flex) p",
      ":not(.md\\:flex)",
      "[class=\"md:flex\"]",
      "[class=md\\:flex]",
      // guards
      ".md",
      "#a",
      "p",
      "div > p",
    ],
  },
  {
    id: "non-ascii",
    bug: "non-ascii",
    html: DOC(
      `<div class="café"><span class="日本 x">a</span></div><p id="ñ" class="caf">b</p><日本>c</日本>`,
    ),
    selectors: [".café", ".日本", "#ñ", "日本", "div.café > span.日本", ".日本.x", ":not(.café)", ".caf", "[class=café]"],
  },
  {
    id: "namespace",
    bug: "namespace",
    html: DOC(`<p class="x">a</p><svg><rect></rect></svg><div><p>b</p></div>`),
    selectors: ["*|p", "*|*", "*|p.x", "div > *|p", "*|rect", "|p", "svg|rect", ":is(*|p)", "p", "*"],
  },
  {
    id: "empty",
    bug: "empty",
    html: DOC(
      `<div id="c"><!--comment--></div><div id="s"> </div><div id="t">x</div><div id="m"><!--a-->x</div>` +
        `<div id="e"></div><div id="k"><span></span></div><template id="tp"></template><template id="tq"><p>x</p></template>` +
        `<div id="b"><?pi x?></div>`,
    ),
    selectors: [":empty", "div:empty", "template:empty", "#c:empty", "#s:empty", "#tq:empty", ":not(:empty)", "span:empty"],
  },
  {
    id: "read-only-read-write",
    bug: "read-only-read-write",
    html: DOC(
      `<div id="d">d</div><p>p</p>` +
        `<input id="text"><input id="ro" readonly><input id="dis" disabled><input id="disro" disabled readonly>` +
        `<input id="search" type="search"><input id="email" type="email"><input id="num" type="number"><input id="date" type="date">` +
        `<input id="pw" type="password"><input id="bogus" type="bogus">` +
        `<input id="cb" type="checkbox"><input id="radio" type="radio"><input id="hidden" type="hidden"><input id="range" type="range">` +
        `<input id="color" type="color"><input id="file" type="file"><input id="submit" type="submit"><input id="btn" type="button">` +
        `<textarea id="ta"></textarea><textarea id="taro" readonly></textarea><textarea id="tadis" disabled></textarea>` +
        `<select id="sel"><option>o</option></select><button id="b">b</button>` +
        `<fieldset disabled><input id="fsin"><legend><input id="legin"></legend></fieldset>` +
        `<fieldset disabled><legend><input id="firstlegin"></legend><input id="fsin2"></fieldset>` +
        `<div id="ce" contenteditable><span id="cechild">x</span><input id="cein" type="checkbox"></div>` +
        `<div id="cef" contenteditable="false">x</div><div contenteditable="true"><b id="cenest" contenteditable="false">x</b></div>` +
        `<div id="cepo" contenteditable="plaintext-only">x</div><div contenteditable><svg id="cesvg"><rect></rect></svg><select id="cesel"></select></div>` +
        `<svg id="svg"><rect></rect></svg><math><mi>x</mi></math>`,
    ),
    selectors: [":read-only", ":read-write", "div:read-only", "p:read-write", "input:read-write", "input:read-only", "textarea:read-write", "select:read-only", "button:read-only", "span:read-write", ":not(:read-write)"],
  },
  {
    id: "nth-of-s",
    bug: "nth-of-s",
    html: DOC(
      `<ul><li id="l1" class="x">1</li><li id="l2">2</li><li id="l3" class="x">3</li><li id="l4" class="x y">4</li><p id="l5" class="x">5</p></ul>`,
    ),
    selectors: [
      ":nth-child(2 of .x)",
      "li:nth-child(odd of .x)",
      ":nth-child(1 of li.y)",
      ":nth-last-child(1 of .x)",
      ":nth-last-child(2 of li)",
      ":nth-child(1 of p, .y)",
      ":nth-child(2n+1 of li)",
      ":nth-child(-n+2 of .x)",
      ":nth-child(2 of.x)",
      "li:not(:nth-child(1 of .x))",
      // guards
      ":nth-child(2)",
      "li:nth-child(2n+1)",
      ":nth-last-child(2)",
      "li:nth-of-type(odd)",
    ],
  },
  {
    id: "link",
    bug: "link",
    html: DOC(
      `<a id="a1" href="/x">a</a><a id="a2">no href</a><a id="a3" href="">empty</a>` +
        `<map name="m"><area id="ar" href="/y"><area id="ar2"></map><span id="s" href="/z">span</span>` +
        `<svg><a id="sa" href="/x"><rect></rect></a><a id="sx" xlink:href="/y"></a><a id="sn"></a></svg>`,
      `<link id="lk" rel="stylesheet" href="data:text/css,">`,
    ),
    selectors: [":link", ":any-link", "a:link", "area:any-link", "link:link", ":link:not(a)", ":visited", "a:not(:link)", "span:link", "a[href]"],
  },
  {
    id: "guards",
    bug: "guard",
    html: DOC(
      `<div class="a b" id="g"><p class="x">1</p><p>2</p><input type="checkbox" checked><input required><input disabled></div>` +
        `<section><h2>t</h2><p>after</p></section>`,
    ),
    selectors: [
      ".a.b",
      "#g > p",
      "p:first-child",
      "p:not(.x)",
      ":is(.x, h2)",
      "[type=\"checkbox\"]",
      ":checked",
      ":disabled",
      "input:enabled",
      ":required",
      "input:optional",
      "div:has(> p.x)",
      "h2 + p",
      "h2 ~ p",
      ":root",
      "p:hover",
      "p:empty",
      "*",
    ],
  },
];

const browser = await chromium.launch({ executablePath: chromePath, headless: true });
const version = browser.version();
const page = await browser.newPage();
const result = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "domcontentloaded" });
  const recorded = await page.evaluate((selectors) => {
    const elements = [];
    const walk = (el) => {
      elements.push(el);
      for (const child of el.children) walk(child);
    };
    walk(document.documentElement);
    const rows = selectors.map((selector) => {
      let valid = true;
      try {
        document.querySelector(selector);
      } catch {
        valid = false;
      }
      return {
        selector,
        valid,
        matches: valid ? elements.map((el) => (el.matches(selector) ? "1" : "0")).join("") : "0".repeat(elements.length),
        qsa: valid ? [...document.querySelectorAll(selector)].map((el) => elements.indexOf(el)) : [],
      };
    });
    // The sentinel sheet is a <style> appended as the LAST child of <head> only while the
    // winners are read (matches/qsa above see the case document unchanged); the Rust test
    // reproduces exactly that document for its cascade check.
    const sheet = document.createElement("style");
    document.head.appendChild(sheet);
    for (const row of rows) {
      sheet.textContent = `${row.selector} { order: 1 }`;
      row.winner = elements.map((el) => getComputedStyle(el).order).join("");
    }
    sheet.remove();
    return { elements: elements.map((el) => el.localName), rows };
  }, testCase.selectors);
  result.push({ id: testCase.id, bug: testCase.bug, html: testCase.html, elements: recorded.elements, selectors: recorded.rows });
}
await browser.close();
const fixture = {
  generator: "scripts/cascade-selectors-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  sentinel: "order",
  cases: result,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${result.length} cases, ${result.reduce((n, c) => n + c.selectors.length, 0)} selectors -> ${out}`);
