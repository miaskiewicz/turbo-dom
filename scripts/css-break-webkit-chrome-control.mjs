// Chrome 156 control for turbo-dom#422 (break-* keyword acceptance) and turbo-dom#441
// (-webkit-* / -moz-* prefixed property acceptance). Runs a single page in the pinned Chrome 156
// headless shell and records, per declaration, CSS.supports() plus the serialized style value
// the browser keeps for that declaration.
//
// Usage: node scripts/css-break-webkit-chrome-control.mjs [--chrome PATH]
//   --chrome defaults to the frozen Chrome 156 headless shell (~/.cache/turbo-consolidation).
// Writes two fixtures:
//   crates/turbo-dom-layout/tests/fixtures/css_break_chrome156.json
//   crates/turbo-dom-browser-env/src/css_webkit_prefix_chrome156.json
// Needs playwright-core: PLAYWRIGHT_CORE (default: ../turbo-surf/node_modules/playwright-core).
import { writeFileSync } from "node:fs";
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
const outBreak = resolve(join(repo, "crates/turbo-dom-layout/tests/fixtures/css_break_chrome156.json"));
const outPrefix = resolve(join(repo, "crates/turbo-dom-browser-env/src/css_webkit_prefix_chrome156.json"));
const playwrightCore = process.env.PLAYWRIGHT_CORE || resolve(repo, "../turbo-surf/node_modules/playwright-core");
const require = createRequire(import.meta.url);
const { chromium } = require(playwrightCore);

// break-before / break-after keywords Chrome 156 may or may not accept (turbo-dom#422).
const BREAK_PROPS = ["break-before", "break-after", "break-inside"];
const BREAK_KEYWORDS = [
  "auto", "avoid", "always", "avoid-page", "avoid-column", "avoid-region",
  "page", "column", "region", "left", "right", "recto", "verso",
];

// Prefixed properties Chrome 156 may or may not accept (turbo-dom#441). Chosen to cover
// both the ones Lightning CSS recognises and the ones Chrome drops at parse time, so the
// audit is informative.
const PREFIXED_PROPS = [
  "-webkit-background-clip", "-webkit-mask-size", "-webkit-mask-image", "-webkit-mask-repeat",
  "-webkit-mask-position", "-webkit-mask-origin", "-webkit-mask-clip", "-webkit-text-fill-color",
  "-webkit-text-stroke", "-webkit-text-stroke-color", "-webkit-text-stroke-width",
  "-webkit-appearance", "-webkit-user-select", "-webkit-font-smoothing",
  "-webkit-tap-highlight-color", "-webkit-line-clamp", "-webkit-box-orient",
  "-webkit-box-direction", "-webkit-box-align", "-webkit-box-pack", "-webkit-box-flex",
  "-webkit-animation", "-webkit-transform", "-webkit-transition", "-webkit-filter",
  "-webkit-backdrop-filter", "-webkit-border-radius", "-webkit-flex", "-webkit-flex-direction",
  "-webkit-align-items", "-webkit-justify-content",
  "-moz-appearance", "-moz-user-select", "-moz-osx-font-smoothing",
];
const PREFIXED_VALUES = {
  "-webkit-background-clip": "text",
  "-webkit-mask-size": "cover",
  "-webkit-mask-image": "none",
  "-webkit-mask-repeat": "no-repeat",
  "-webkit-mask-position": "center",
  "-webkit-mask-origin": "border-box",
  "-webkit-mask-clip": "border-box",
  "-webkit-text-fill-color": "red",
  "-webkit-text-stroke": "1px red",
  "-webkit-text-stroke-color": "red",
  "-webkit-text-stroke-width": "1px",
  "-webkit-appearance": "none",
  "-webkit-user-select": "none",
  "-webkit-font-smoothing": "antialiased",
  "-webkit-tap-highlight-color": "transparent",
  "-webkit-line-clamp": "2",
  "-webkit-box-orient": "vertical",
  "-webkit-box-direction": "normal",
  "-webkit-box-align": "center",
  "-webkit-box-pack": "center",
  "-webkit-box-flex": "1",
  "-webkit-animation": "none",
  "-webkit-transform": "none",
  "-webkit-transition": "none",
  "-webkit-filter": "none",
  "-webkit-backdrop-filter": "none",
  "-webkit-border-radius": "4px",
  "-webkit-flex": "none",
  "-webkit-flex-direction": "row",
  "-webkit-align-items": "stretch",
  "-webkit-justify-content": "flex-start",
  "-moz-appearance": "none",
  "-moz-user-select": "none",
  "-moz-osx-font-smoothing": "grayscale",
};

const browser = await chromium.launch({ executablePath: chromePath });
try {
  const ctx = await browser.newContext();
  const page = await ctx.newPage();
  await page.setContent("<!doctype html><body><div id='t'></div>");
  const data = await page.evaluate(({ BREAK_PROPS, BREAK_KEYWORDS, PREFIXED_PROPS, PREFIXED_VALUES }) => {
    const el = document.getElementById("t");
    const breaks = [];
    for (const prop of BREAK_PROPS) {
      for (const kw of BREAK_KEYWORDS) {
        const supports2 = CSS.supports(prop, kw);
        el.removeAttribute("style");
        el.style.setProperty(prop, kw);
        const held = el.style.getPropertyValue(prop);
        breaks.push([prop, kw, supports2, held]);
      }
    }
    const prefixed = [];
    for (const prop of PREFIXED_PROPS) {
      const v = PREFIXED_VALUES[prop];
      const supports2 = CSS.supports(prop, v);
      el.removeAttribute("style");
      el.style.setProperty(prop, v);
      const held = el.style.getPropertyValue(prop);
      prefixed.push([prop, v, supports2, held]);
    }
    return { breaks, prefixed };
  }, { BREAK_PROPS, BREAK_KEYWORDS, PREFIXED_PROPS, PREFIXED_VALUES });
  writeFileSync(outBreak, JSON.stringify({ breaks: data.breaks }, null, 2) + "\n");
  writeFileSync(outPrefix, JSON.stringify({ prefixed: data.prefixed }, null, 2) + "\n");
  console.error(`wrote ${outBreak}`);
  console.error(`wrote ${outPrefix}`);
} finally {
  await browser.close();
}
