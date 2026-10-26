// Build src/typed_om_properties.json, the CSS property registry Typed OM reads, from Blink's own
// property metadata at the Chrome 156 tag plus the measured Chrome 156 registry control.
//
//   node scripts/typed-om-registry.mjs cases --json5 css_properties.json5 --out src/typed_om_registry_cases.json
//   node scripts/typed-om-chrome-control.mjs --probe src/typed_om_registry_probe.js \
//     --cases src/typed_om_registry_cases.json --out src/typed_om_registry_chrome156.json
//   node scripts/typed-om-registry.mjs build --json5 css_properties.json5 \
//     --keywords css_value_keywords.json5 --control src/typed_om_registry_chrome156.json \
//     --out src/typed_om_properties.json
//
// Both json5 files come from https://chromium.googlesource.com/chromium/src/+/refs/tags/156.0.8078.47/
// third_party/blink/renderer/core/css/ (css_properties.json5, css_value_keywords.json5).
// From Blink: property names, aliases, shorthands' longhands, `inherited`, `typedom_types`,
// `keywords`/`typedom_keywords`, `typedom_custom_ident`, list `separator`, logical property groups
// and the CSSValueID keyword table. From the control: which names this build enables, the
// computable longhands in computedStyleMap() order with their computed values on an element no
// UA rule styles (the property's initial value or the root's inherited value), and the UA
// `display` and `unicode-bidi` of HTML elements.
// Run from crates/turbo-dom-browser-env: src/ holds the binding's probes, cases and Chrome
// controls (the files its tests and `test_support` read).
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { parseArgs } from "node:util";

const { positionals, values } = parseArgs({
  allowPositionals: true,
  options: { json5: { type: "string" }, keywords: { type: "string" }, control: { type: "string" }, out: { type: "string" } },
});
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");
// Blink's json5 is an object literal with comments and trailing commas.
const loadJson5 = (path) => {
  const text = readFileSync(resolve(path), "utf8");
  return new Function(`return (${text.slice(text.indexOf("{"))})`)();
};
const properties = loadJson5(values.json5).data;
const candidates = [];
for (const property of properties) {
  candidates.push(property.name);
  for (const alias of [].concat(property.aliases || [])) candidates.push(alias);
}

if (positionals[0] === "cases") {
  const html = `<!doctype html><html><head><script type="application/json" id="candidates">${JSON.stringify(candidates)}</script></head><body><x-initial id="probe"></x-initial><div id="host"></div></body></html>`;
  writeFileSync(resolve(values.out), `${JSON.stringify([{ id: "typed-om-registry", html }])}\n`);
} else if (positionals[0] === "build") {
  const controlBytes = readFileSync(resolve(values.control));
  const control = JSON.parse(controlBytes);
  if (control.browser !== "Chrome/156.0.8078.47" || control.errors.length) throw Error("unexpected control");
  const measured = control.result[0].result;
  const enabled = new Set(measured.enabled);
  const initial = new Map(measured.computed.map(([name, value]) => [name, value]));
  const keywordBytes = readFileSync(resolve(values.keywords));
  const keywordTable = loadJson5(values.keywords).data.map((entry) => (typeof entry === "string" ? entry : entry.name).toLowerCase());
  const byName = new Map(properties.map((property) => [property.name, property]));
  const out = {};
  for (const property of properties) {
    const names = [property.name, ...[].concat(property.aliases || [])];
    for (const name of names) {
      if (!enabled.has(name)) continue;
      const target = property.alias_for ? byName.get(property.alias_for) : property;
      const record = {};
      if (name !== property.name || property.alias_for) record.alias = property.alias_for || property.name;
      if (target.longhands) record.longhands = target.longhands;
      if (target.inherited) record.inherited = 1;
      if (target.typedom_types) record.types = target.typedom_types;
      const keywords = target.typedom_keywords || target.keywords;
      if (keywords && keywords.length) record.keywords = keywords.map((keyword) => keyword.toLowerCase());
      if (target.typedom_custom_ident) record.customIdent = 1;
      if (target.separator) record.separator = target.separator;
      if (target.logical_property_group) record.group = [target.logical_property_group.name, target.logical_property_group.resolver, target.logical_property_group.is_logical ? 1 : 0];
      if (target.is_property === false) record.descriptor = 1;
      out[name] = record;
    }
  }
  const computable = measured.computed.map(([name]) => name);
  for (const name of computable) {
    if (!out[name]) throw Error(`computable ${name} is not an enabled json5 property`);
    out[name].initial = initial.get(name);
  }
  const registry = {
    source: {
      browser: control.browser,
      controlSha256: sha(controlBytes),
      cssPropertiesSha256: sha(readFileSync(resolve(values.json5))),
      cssValueKeywordsSha256: sha(keywordBytes),
    },
    computable,
    display: Object.fromEntries(measured.display),
    unicodeBidi: Object.fromEntries(measured.unicodeBidi.filter(([, value]) => value !== 'normal')),
    keywords: keywordTable,
    properties: out,
  };
  writeFileSync(resolve(values.out), `${JSON.stringify(registry)}\n`);
  console.log(JSON.stringify({ properties: Object.keys(out).length, computable: computable.length, keywords: keywordTable.length }));
} else {
  throw Error("usage: typed-om-registry.mjs cases|build ...");
}
