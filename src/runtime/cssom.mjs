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

import {
  wrapCssRule, wrapCssSheet, finishCssRuleConstruction, finishCssSheetConstruction, emptyCssRules, closedCssRules, beginCssMutation, endCssApiMutation, spliceCssRules,
  invalidateCssSheet, canOwnCssMap, replaceCssRules, privateCssRule, isObservedCssSheet,
} from './cssom-state.mjs';
export { cssSheetSnapshot } from './cssom-state.mjs';

const observedToken = Symbol('observed document CSS store');
const nativeRules = new WeakSet();
const nativeRuleHas = WeakSet.prototype.has, nativeRuleAdd = WeakSet.prototype.add;
const nativeRuleApply = Reflect.apply;
export function hasNativeCssRuleBrand(value) { return nativeRuleApply(nativeRuleHas, nativeRules, [value]); }
export function hasNativeCssSheetBrand(value) { return hasPlainSheetBrand(value); }
export class CSSStyleRule {
  constructor(cssText, token = undefined) {
    nativeRuleApply(nativeRuleAdd, nativeRules, [this]);
    const observed = token === observedToken;
    const rule = observed ? wrapCssRule(this) : this;
    rule.cssText = String(cssText);
    rule.type = 1; // CSSRule.STYLE_RULE
    if (observed) finishCssRuleConstruction(rule);
    return rule;
  }
  get selectorText() {
    const i = this.cssText.indexOf('{');
    return i < 0 ? '' : this.cssText.slice(0, i).trim();
  }
}

const rulePrototype = CSSStyleRule.prototype;
const objectPrototype = Object.prototype;
const ownDescriptor = Object.getOwnPropertyDescriptor;
const getPrototype = Object.getPrototypeOf;
const createObject = Object.create;
const cssString = String;
function exposedInsertedRule(cssText) {
  const target = createObject(rulePrototype);
  nativeRuleApply(nativeRuleAdd, nativeRules, [target]);
  const rule = wrapCssRule(target);
  rule.cssText = cssText;
  rule.type = 1;
  finishCssRuleConstruction(rule);
  return rule;
}
function insertedRule(value) {
  const nativeString = String === cssString;
  const cssText = String(value);
  // No inherited assignment callback can observe this internal record before its
  // first public array read. Customized constructor shapes keep the original path.
  if (!nativeString || typeof cssText !== 'string' || getPrototype(rulePrototype) !== objectPrototype ||
      ownDescriptor(rulePrototype, 'cssText') || ownDescriptor(rulePrototype, 'type') ||
      ownDescriptor(objectPrototype, 'cssText') || ownDescriptor(objectPrototype, 'type')) return exposedInsertedRule(cssText);
  const target = createObject(rulePrototype);
  nativeRuleApply(nativeRuleAdd, nativeRules, [target]);
  target.cssText = cssText;
  target.type = 1;
  return privateCssRule(target);
}

// Split a stylesheet string into top-level rule strings (brace-depth aware), used
// by replaceSync/constructor text. Mirrors cascade.mjs's depth scan but keeps the
// braces so each piece is a self-contained cssText.
function scanBlockEnd(css, start, n) {
  let depth = 1, k = start;
  while (k < n && depth > 0) { const ch = css[k]; if (ch === '{') depth++; else if (ch === '}') depth--; k++; }
  return k;
}

function splitTopLevelRules(css) {
  const out = [];
  const n = css.length;
  let i = 0;
  while (i < n) {
    let j = i;
    while (j < n && css[j] !== '{' && css[j] !== '}') j++;
    if (j >= n) break;
    if (css[j] === '}') { i = j + 1; continue; }
    const k = scanBlockEnd(css, j + 1, n);
    const piece = css.slice(i, k).trim();
    if (piece) out.push(piece);
    i = k;
  }
  return out;
}

let hasPlainSheetBrand;
export class CSSStyleSheet {
  static {
    hasPlainSheetBrand = (value) => value !== null && (typeof value === 'object' || typeof value === 'function') && #invalidate in value;
  }
  // ownerNode is the <style> element (null for constructed/adopted sheets).
  constructor(ownerNode = null, init = {}, token = undefined) {
    const observed = token === observedToken;
    const sheet = observed ? wrapCssSheet(this, ownerNode) : this;
    sheet.ownerNode = ownerNode;
    sheet.cssRules = observed ? emptyCssRules(sheet) : [];
    sheet.type = 'text/css';
    sheet.disabled = false;
    sheet.media = init.media != null ? init.media : [];
    sheet.title = init.title != null ? init.title : null;
    sheet.href = null;
    sheet.parentStyleSheet = null;
    sheet.ownerRule = null;
    if (observed) finishCssSheetConstruction(sheet);
    return sheet;
  }

  // legacy alias some libraries read
  get rules() { return this.cssRules; }

  insertRule(rule, index = 0) {
    if (index < 0 || index > this.cssRules.length) throw new RangeError('insertRule: index out of bounds');
    if (hasPlainSheetBrand(this) || !isObservedCssSheet(this)) {
      this.cssRules.splice(index, 0, new CSSStyleRule(rule));
      this.#invalidate();
      return index;
    }
    const list = closedCssRules(this);
    if (list) {
      const value = insertedRule(rule);
      const mutation = beginCssMutation(this, index, rule, list, value);
      let completed = false;
      try { spliceCssRules(list, index, 0, value); completed = true; }
      finally { endCssApiMutation(mutation, completed); }
    } else this.cssRules.splice(index, 0, new CSSStyleRule(rule, observedToken));
    invalidateCssSheet(this);
    return index;
  }

  deleteRule(index) {
    if (index < 0 || index >= this.cssRules.length) throw new RangeError('deleteRule: index out of bounds');
    if (hasPlainSheetBrand(this) || !isObservedCssSheet(this)) {
      this.cssRules.splice(index, 1);
      this.#invalidate();
      return;
    }
    const list = closedCssRules(this);
    if (list) {
      const mutation = beginCssMutation(this, index, undefined, list);
      let completed = false;
      try { spliceCssRules(list, index, 1); completed = true; }
      finally { endCssApiMutation(mutation, completed); }
    } else this.cssRules.splice(index, 1);
    invalidateCssSheet(this);
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
    if (hasPlainSheetBrand(this) || !isObservedCssSheet(this)) {
      this.cssRules = splitTopLevelRules(String(text)).map((r) => new CSSStyleRule(r));
      this.#invalidate();
      return;
    }
    const pieces = splitTopLevelRules(String(text));
    const trusted = canOwnCssMap(pieces);
    const rules = pieces.map((r) => new CSSStyleRule(r, observedToken));
    replaceCssRules(this, rules, trusted);
    invalidateCssSheet(this);
  }
  replace(text) { this.replaceSync(text); return Promise.resolve(this); }

  #invalidate() {
    const node = this.ownerNode;
    if (node && node.__touch) node.__touch();
  }
}

export function createObservedCssRule(text) { return new CSSStyleRule(text, observedToken); }
export function createObservedCssSheet(owner, init = {}) {
  return new CSSStyleSheet(owner, init, observedToken);
}

// Live StyleSheetList for document.styleSheets: a plain array (indexable +
// iterable, what emotion needs) with the spec's `item()` accessor bolted on.
export function styleSheetList(sheets) {
  const list = sheets.slice();
  list.item = (i) => list[i] ?? null;
  return list;
}
