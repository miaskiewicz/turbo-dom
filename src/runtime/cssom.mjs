// Minimal CSSOM — just enough for CSS-in-JS engines (emotion / styled-components /
// MUI) to inject rules at runtime without throwing, AND to feed those rules into
// the partial cascade (cascade.mjs) so getComputedStyle resolves them.
//
// emotion's `sheetForTag` reads `tag.sheet` and `document.styleSheets`, then calls
// `sheet.insertRule(rule, index)` per CSS rule. It only needs insertRule to not
// throw and `cssRules.length` to be readable — but we go one cheap step further:
// insertRule bumps Document.__version (via the owner <style>'s __touch), so the
// version-keyed style index in cascade.mjs rebuilds and picks the rules up.
//
// HONEST: this is a rule STORE, not a CSS engine. No serialization normalization,
// no @media evaluation, no specificity beyond what cascade.mjs already does. Rules
// are kept as their raw cssText; cascade.mjs parses them the same way it parses a
// <style>'s textContent.

export class CSSStyleRule {
  constructor(cssText) {
    this.cssText = String(cssText);
    this.type = 1; // CSSRule.STYLE_RULE
  }
  get selectorText() {
    const i = this.cssText.indexOf('{');
    return i < 0 ? '' : this.cssText.slice(0, i).trim();
  }
}

// Split a stylesheet string into top-level rule strings (brace-depth aware), used
// by replaceSync/constructor text. Mirrors cascade.mjs's depth scan but keeps the
// braces so each piece is a self-contained cssText.
function splitTopLevelRules(css) {
  const out = [];
  const n = css.length;
  let i = 0;
  while (i < n) {
    let j = i;
    while (j < n && css[j] !== '{' && css[j] !== '}') j++;
    if (j >= n) break;
    if (css[j] === '}') { i = j + 1; continue; }
    let depth = 1, k = j + 1;
    while (k < n && depth > 0) { const ch = css[k]; if (ch === '{') depth++; else if (ch === '}') depth--; k++; }
    const piece = css.slice(i, k).trim();
    if (piece) out.push(piece);
    i = k;
  }
  return out;
}

export class CSSStyleSheet {
  // ownerNode is the <style> element (null for constructed/adopted sheets).
  constructor(ownerNode = null, init = {}) {
    this.ownerNode = ownerNode;
    this.cssRules = [];
    this.type = 'text/css';
    this.disabled = false;
    this.media = init.media != null ? init.media : [];
    this.title = init.title != null ? init.title : null;
    this.href = null;
    this.parentStyleSheet = null;
    this.ownerRule = null;
  }

  // legacy alias some libraries read
  get rules() { return this.cssRules; }

  insertRule(rule, index = 0) {
    if (index < 0 || index > this.cssRules.length) throw new RangeError('insertRule: index out of bounds');
    this.cssRules.splice(index, 0, new CSSStyleRule(rule));
    this.#invalidate();
    return index;
  }

  deleteRule(index) {
    if (index < 0 || index >= this.cssRules.length) throw new RangeError('deleteRule: index out of bounds');
    this.cssRules.splice(index, 1);
    this.#invalidate();
  }

  // legacy IE-era APIs (some older libs still call these)
  addRule(selector = '', style = '', index) {
    const i = index === undefined ? this.cssRules.length : index;
    this.insertRule(`${selector} { ${style} }`, i);
    return -1; // spec'd legacy return
  }
  removeRule(index = 0) { this.deleteRule(index); }

  // constructable-stylesheet API (adoptedStyleSheets); replaceSync is synchronous.
  replaceSync(text) {
    this.cssRules = splitTopLevelRules(String(text)).map((r) => new CSSStyleRule(r));
    this.#invalidate();
  }
  replace(text) { this.replaceSync(text); return Promise.resolve(this); }

  // Bump the owning Document's version so cascade.mjs's version-keyed style index
  // (and any getComputedStyle cache) rebuilds. Constructed sheets (no ownerNode)
  // have nothing to invalidate until adopted — a no-op, which is correct.
  #invalidate() {
    const n = this.ownerNode;
    if (n && n.__touch) n.__touch();
  }
}

// Live StyleSheetList for document.styleSheets: a plain array (indexable +
// iterable, what emotion needs) with the spec's `item()` accessor bolted on.
export function styleSheetList(sheets) {
  const list = sheets.slice();
  list.item = (i) => list[i] ?? null;
  return list;
}
