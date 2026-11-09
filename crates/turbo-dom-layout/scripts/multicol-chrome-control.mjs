// Chrome-156 control for turbo-dom-layout's CSS multi-column geometry
// (#398 PR-B + PR-E).
//
// Captures the layout of the multicol containers Chrome 156 lays out, as a geometry
// oracle for the layout pass that lands in #398 PR-C and PR-E. The capture pattern
// follows `scripts/live-geometry-chrome-control.mjs` (same shape: one `boxes` array
// per case, one JSON file). PR-C's and PR-E's layout harness assert against this
// fixture; PR-B's harness only loaded it.
//
// PR-E adds six cases exercising `column-span: all` and the
// `break-{before,after,inside}` property family: the span-all child breaks the
// multicol flow, fills the content box's inline size, and Chrome balances the
// pre-span and post-span sub-flows separately (CSS Multicol 1 §6 "Spanning
// columns"); `break-before: column` / `break-after: column` force a column
// break at the start / after the end of a flowed child; `break-inside: avoid`
// pushes a would-be-fragmented child to a fresh column when it fits in one.
//
// Each case records, in document order:
//   - the multicol container's own content-box (`cid: "c"`): its bounding rect plus
//     content-box inset (clientLeft/Top + clientWidth/Height);
//   - each flowed child's bounding rect (`cid: "i0"`, `"i1"`, ...), with its
//     offsetParent-relative offsetLeft/Top so the column strip it landed in is
//     derivable (the container is `position: relative`; Chrome places overflowing
//     children at the column's x/y, so column strips are visible from the child
//     columns).
//
// Column strips are reported as `columns`: an array of `{x, width, y, height}` in
// the container's content-box space, derived from the children's positions — the
// unique set of (left, width) tuples in children order, each with the content-box
// top + content-box height (Chrome lays each column strip across the full content
// height).
//
// Usage: node crates/turbo-dom-layout/scripts/multicol-chrome-control.mjs [--chrome PATH] [--out PATH]
//   --chrome defaults to the repo's cached Chrome for Testing 156;
//   --out    defaults to crates/turbo-dom-layout/tests/fixtures/multicol_chrome156.json.
// Needs playwright-core: PLAYWRIGHT_CORE=/path/to/node_modules/playwright-core (default: a
// sibling turbo-surf checkout's node_modules/playwright-core).
import { writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../../..");
const { values } = parseArgs({ options: { chrome: { type: "string" }, out: { type: "string" } } });
const chromePath =
  values.chrome ||
  join(
    homedir(),
    ".cache/turbo-consolidation/chrome156/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  );
const out = resolve(values.out || join(repo, "crates/turbo-dom-layout/tests/fixtures/multicol_chrome156.json"));
const playwrightCore =
  process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);
const playwrightVersion = require(join(playwrightCore, "package.json")).version;

// A fixed-height, zero-margin block for predictable per-child geometry.
const BLK = (h, i) => `<div data-cid="i${i}" style="height:${h}px;background:#eef"></div>`;
// The container wrapper: position:relative so children's offsetLeft/Top read the
// container's content-box origin (so column x/y falls out of the children).
const DOC = (css, body) =>
  `<!doctype html><style>*{box-sizing:border-box;margin:0}body{margin:0;font:16px/20px monospace}${css}</style>${body}`;
const WRAP = (style, body) =>
  `<div data-cid="c" style="position:relative;${style}">${body}</div>`;

const stack = (heights) => heights.map((h, i) => BLK(h, i)).join("");

const CASES = [
  {
    id: "basic-2col-equal",
    html: DOC("", WRAP("columns:2;column-gap:0;width:400px;height:300px", stack([100, 100, 100, 100, 100, 100]))),
  },
  {
    id: "basic-3col-with-gap",
    html: DOC("", WRAP("columns:3;column-gap:16px;width:600px;height:300px", stack([90, 90, 90, 90, 90, 90, 90, 90, 90]))),
  },
  {
    id: "column-width-driven",
    html: DOC(
      "",
      WRAP("column-width:180px;column-gap:20px;width:500px", stack([60, 60, 60, 60, 60, 60])),
    ),
  },
  {
    id: "columns-shorthand",
    html: DOC(
      "",
      WRAP("columns:200px 3;column-gap:0;width:600px;height:200px", stack([50, 50, 50, 50, 50, 50, 50, 50])),
    ),
  },
  {
    id: "mixed-child-heights",
    html: DOC(
      "",
      WRAP("columns:2;column-gap:0;width:400px;height:200px", stack([60, 80, 40, 100, 50])),
    ),
  },
  {
    id: "overflow-at-column-height",
    html: DOC(
      "",
      WRAP("columns:2;column-gap:0;width:400px;height:150px", stack([100, 100, 100, 100])),
    ),
  },
  {
    id: "nested-multicol",
    html: DOC(
      "",
      WRAP(
        "columns:2;column-gap:0;width:400px;height:300px",
        `<div data-cid="i0" style="columns:2;column-gap:0;height:150px">${stack([40, 40, 40, 40]).replace(/i(\d+)/g, (_, n) => `in${n}`)}</div>` +
          BLK(100, 1) +
          BLK(100, 2),
      ),
    ),
  },
  {
    id: "floats-inside-column",
    html: DOC(
      "",
      WRAP(
        "columns:2;column-gap:0;width:400px;height:200px",
        `<div data-cid="i0" style="height:60px"><div data-cid="f" style="float:left;width:40px;height:30px;background:#fcc"></div>text</div>` +
          BLK(60, 1) +
          BLK(60, 2) +
          BLK(60, 3),
      ),
    ),
  },
  // PR-E: `column-span: all` breaks the flow — Chrome fragments the pre-span
  // children, places the span-all child at the full container inline size,
  // then flows the post-span children into a new column set below.
  {
    id: "span-all-simple",
    html: DOC(
      "",
      WRAP(
        "columns:2;column-gap:0;width:400px;height:400px",
        BLK(80, 0) +
          `<div data-cid="i1" style="column-span:all;height:40px;background:#efe"></div>` +
          BLK(80, 2),
      ),
    ),
  },
  // PR-E: `column-span: all` inside a balanced container — balance is per
  // sub-flow, so the pre-span children balance over their own pair of columns.
  {
    id: "span-all-balance",
    html: DOC(
      "",
      WRAP(
        "columns:2;column-gap:16px;width:416px;height:400px",
        stack([60, 60, 60, 60]).replace(/i(\d+)/g, (_, n) => `p${n}`) +
          `<div data-cid="s" style="column-span:all;height:40px;background:#efe"></div>` +
          stack([60, 60]).replace(/i(\d+)/g, (_, n) => `q${n}`),
      ),
    ),
  },
  // PR-E: `break-before: column` on i1 forces a new column at its start even
  // though i0 did not fill its column.
  {
    id: "break-before-column",
    html: DOC(
      "",
      WRAP(
        "columns:2;column-gap:0;width:400px;height:300px",
        BLK(80, 0) +
          `<div data-cid="i1" style="break-before:column;height:80px;background:#eef"></div>` +
          BLK(80, 2),
      ),
    ),
  },
  // PR-E: `break-after: column` on i0 forces a new column after i0, so i1
  // starts at the top of column 2 even though there was room in column 1.
  {
    id: "break-after-column",
    html: DOC(
      "",
      WRAP(
        "columns:2;column-gap:0;width:400px;height:300px",
        `<div data-cid="i0" style="break-after:column;height:80px;background:#eef"></div>` +
          BLK(80, 1) +
          BLK(80, 2),
      ),
    ),
  },
  // PR-E: `break-inside: avoid` on a child that would overrun the current
  // column pushes it into a fresh one rather than fragmenting.
  {
    id: "break-inside-avoid",
    html: DOC(
      "",
      WRAP(
        "columns:2;column-gap:0;width:400px;height:200px",
        BLK(150, 0) +
          `<div data-cid="i1" style="break-inside:avoid;height:80px;background:#eef"></div>` +
          BLK(60, 2),
      ),
    ),
  },
  // PR-E: `break-inside: avoid` on a child that fits in the current column
  // stays in place (no spurious column push).
  {
    id: "break-inside-avoid-fits-anyway",
    html: DOC(
      "",
      WRAP(
        "columns:2;column-gap:0;width:400px;height:200px",
        BLK(60, 0) +
          `<div data-cid="i1" style="break-inside:avoid;height:60px;background:#eef"></div>` +
          BLK(60, 2),
      ),
    ),
  },
  // miaskiewicz/turbo-dom#444: `position: absolute` kids must NOT occupy
  // column flow space. The abspos kid (i1) places relative to its
  // containing block (the multicol container, position:relative) at its
  // own CSS width/height, and the three in-flow kids pack as if i1 were
  // absent — Chrome's multicol routes out-of-flow kids around the column
  // flow entirely. The abspos kid's BCR must match its own CSS geometry,
  // not the column's `used_width` (also closes miaskiewicz/turbo-dom#461:
  // `measure_kid` was returning the multicol-container inline size for
  // abspos descendants).
  {
    id: "abspos-in-multicol",
    html: DOC(
      "",
      WRAP(
        "columns:2;column-gap:0;width:400px;height:300px",
        BLK(60, 0) +
          `<div data-cid="i1" style="position:absolute;left:10px;top:120px;width:80px;height:50px;background:#fcc"></div>` +
          BLK(60, 2) +
          BLK(60, 3),
      ),
    ),
  },
  // miaskiewicz/turbo-dom#427: `background` on a block that fragments
  // across the column gap. Chrome paints the background inside EACH
  // column fragment (never across the gap). `getBoundingClientRect` on
  // the fragmented block returns the union of its column fragments, so
  // the oracle's border-box is the union and `rects` records the
  // per-column pieces `getClientRects` reports.
  {
    id: "fragmented-background",
    html: DOC(
      "",
      WRAP(
        "columns:2;column-gap:16px;width:416px;height:200px",
        `<div data-cid="i0" style="height:300px;background:#cce"></div>`,
      ),
    ),
  },
  // #442 (a): content overflows past used_count under `overflow:visible` —
  // extra strips painted past the content box; the overflowing kid's
  // bounding rect sits at the overflow strip's x.
  {
    id: "overflow-visible-extra-strips",
    html: DOC(
      "",
      WRAP(
        "columns:2;column-gap:0;width:400px;height:100px;overflow:visible",
        stack([100, 100, 100, 100, 100, 100]),
      ),
    ),
  },
  // #442 (b): same flow under `overflow:hidden` — Chrome still lays the
  // overflow strips out past the content box, but paint-time clipping trims
  // them. `getBoundingClientRect()` reports the laid-out (pre-clip) x of the
  // overflowing kid; that is what the layout contract pins.
  {
    id: "overflow-hidden-extra-strips",
    html: DOC(
      "",
      WRAP(
        "columns:2;column-gap:0;width:400px;height:100px;overflow:hidden",
        stack([100, 100, 100, 100, 100, 100]),
      ),
    ),
  },
];

const browser = await chromium.launch({
  executablePath: chromePath,
  headless: true,
  args: ["--use-mock-keychain", "--password-store=basic"],
});
const version = browser.version();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 });
const result = [];
for (const testCase of CASES) {
  await page.setContent(testCase.html, { waitUntil: "load" });
  const data = await page.evaluate(() => {
    const container = document.querySelector('[data-cid="c"]');
    const cbox = container.getBoundingClientRect();
    const cstyle = getComputedStyle(container);
    const containerBox = {
      cid: "c",
      x: cbox.x,
      y: cbox.y,
      width: cbox.width,
      height: cbox.height,
      clientLeft: container.clientLeft,
      clientTop: container.clientTop,
      clientWidth: container.clientWidth,
      clientHeight: container.clientHeight,
      columnCount: cstyle.columnCount,
      columnWidth: cstyle.columnWidth,
      columnGap: cstyle.columnGap,
    };
    const items = [...container.querySelectorAll("[data-cid]")]
      .filter((el) => el !== container)
      .map((el) => {
        const b = el.getBoundingClientRect();
        // `getClientRects()` returns one rect per column fragment for a
        // multicol-fragmented box (Chrome 156). For a non-fragmenting box
        // it is a single rect equal to the BCR.
        const rects = [...el.getClientRects()].map((r) => ({
          x: r.x,
          y: r.y,
          width: r.width,
          height: r.height,
        }));
        return {
          cid: el.dataset.cid,
          x: b.x,
          y: b.y,
          width: b.width,
          height: b.height,
          offsetLeft: el.offsetLeft,
          offsetTop: el.offsetTop,
          offsetWidth: el.offsetWidth,
          offsetHeight: el.offsetHeight,
          rects,
        };
      });
    // Derive column strips from child x/width in document order: one strip per
    // unique (x, width) pair in that order. Strip y = container content-box top,
    // height = container content-box height (Chrome's column strips run the
    // full content height).
    const seen = new Set();
    const strips = [];
    for (const it of items) {
      const key = `${it.x}:${it.width}`;
      if (seen.has(key)) continue;
      seen.add(key);
      strips.push({ x: it.x - cbox.x - container.clientLeft, width: it.width });
    }
    const columnsOut = strips.map((s) => ({
      x: s.x,
      width: s.width,
      y: container.clientTop,
      height: container.clientHeight,
    }));
    return { container: containerBox, items, columns: columnsOut };
  });
  result.push({ id: testCase.id, html: testCase.html, ...data });
}
await browser.close();

const fixture = {
  generator: "crates/turbo-dom-layout/scripts/multicol-chrome-control.mjs",
  chrome: version,
  playwright_core: playwrightVersion,
  viewport: [1280, 800],
  cases: result,
};
writeFileSync(out, `${JSON.stringify(fixture, null, 1)}\n`);
console.log(`Chrome ${version}: ${result.length} multicol cases -> ${out}`);
