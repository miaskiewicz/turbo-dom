// jsdom 27.4.0 control for browser-env's MediaMode::Jsdom (the media, transition and
// animation behaviour vitest's jsdom environment has): runs a probe (an expression, possibly
// a promise) in a fresh JSDOM and records its result.
// Usage: JSDOM=/path/to/node_modules/jsdom node scripts/jsdom-media-control.cjs PROBE > OUT.json
const fs = require('node:fs'), crypto = require('node:crypto'), path = require('node:path');
const jsdomPath = process.env.JSDOM || path.resolve(__dirname, '../../ui-design-components-wt-vitest/node_modules/jsdom');
const { JSDOM } = require(jsdomPath);
const probe = fs.readFileSync(process.argv[2], 'utf8');
const dom = new JSDOM('<!doctype html><html><head></head><body></body></html>', { runScripts: 'outside-only', pretendToBeVisual: true });
Promise.resolve(dom.window.eval(probe)).then((result) => {
  const jsdom = require(path.join(jsdomPath, 'package.json')).version;
  console.log(JSON.stringify({ jsdom, probeSha256: crypto.createHash('sha256').update(probe).digest('hex'), result }, null, 2));
  dom.window.close();
});
