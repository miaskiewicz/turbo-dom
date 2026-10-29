// Color canonicalization — browsers serialize computed/inline <color> values to
// rgb()/rgba(), never as the authored `#fff`/`white`. testing-library's
// `toHaveStyle` compares the element's computed value against the expected value
// AS STRINGS, after each is run through the DOM's own normalization — so to match
// a browser (and jsdom) both getComputedStyle (cascade.mjs) and inline style
// read-back (dom.mjs makeStyle) must canonicalize. This module is the shared core.
//
// PERF: only ever called from getComputedStyle + el.style reads (test/app time),
// and only for properties in COLOR_PROPS. Parse/query/match/event hot paths never
// touch it. Returns null when the value isn't a recognized color → caller keeps
// the original string (honest passthrough for url()/gradients/var()/keywords).
//
// Two modes, matching real browsers:
//   includeNamed=false (inline el.style): hex + rgb()/hsl() canonicalize; NAMED
//     keywords (`red`, `transparent`) stay as authored — Chrome keeps them inline.
//   includeNamed=true  (computed getComputedStyle): names resolve to rgb() too.

// Properties whose value is a <color> (longhands + the single-token shorthands we
// already expand). `background` is included for the bare-color MUI case
// (`background:#fff`); a non-color background value just fails to parse → passthrough.
export const COLOR_PROPS = new Set([
  'color', 'background-color', 'background', 'border-color',
  'border-top-color', 'border-right-color', 'border-bottom-color', 'border-left-color',
  'outline-color', 'text-decoration-color', 'column-rule-color', 'caret-color',
  'fill', 'stroke', 'stop-color', 'flood-color', 'lighting-color',
]);

// CSS named colors → #rrggbb (reused through hexToRgb). `transparent` is special
// (→ rgba(0,0,0,0)); both gate behind includeNamed.
const NAMED = {
  aliceblue: 'f0f8ff', antiquewhite: 'faebd7', aqua: '00ffff', aquamarine: '7fffd4',
  azure: 'f0ffff', beige: 'f5f5dc', bisque: 'ffe4c4', black: '000000',
  blanchedalmond: 'ffebcd', blue: '0000ff', blueviolet: '8a2be2', brown: 'a52a2a',
  burlywood: 'deb887', cadetblue: '5f9ea0', chartreuse: '7fff00', chocolate: 'd2691e',
  coral: 'ff7f50', cornflowerblue: '6495ed', cornsilk: 'fff8dc', crimson: 'dc143c',
  cyan: '00ffff', darkblue: '00008b', darkcyan: '008b8b', darkgoldenrod: 'b8860b',
  darkgray: 'a9a9a9', darkgreen: '006400', darkgrey: 'a9a9a9', darkkhaki: 'bdb76b',
  darkmagenta: '8b008b', darkolivegreen: '556b2f', darkorange: 'ff8c00', darkorchid: '9932cc',
  darkred: '8b0000', darksalmon: 'e9967a', darkseagreen: '8fbc8f', darkslateblue: '483d8b',
  darkslategray: '2f4f4f', darkslategrey: '2f4f4f', darkturquoise: '00ced1', darkviolet: '9400d3',
  deeppink: 'ff1493', deepskyblue: '00bfff', dimgray: '696969', dimgrey: '696969',
  dodgerblue: '1e90ff', firebrick: 'b22222', floralwhite: 'fffaf0', forestgreen: '228b22',
  fuchsia: 'ff00ff', gainsboro: 'dcdcdc', ghostwhite: 'f8f8ff', gold: 'ffd700',
  goldenrod: 'daa520', gray: '808080', green: '008000', greenyellow: 'adff2f',
  grey: '808080', honeydew: 'f0fff0', hotpink: 'ff69b4', indianred: 'cd5c5c',
  indigo: '4b0082', ivory: 'fffff0', khaki: 'f0e68c', lavender: 'e6e6fa',
  lavenderblush: 'fff0f5', lawngreen: '7cfc00', lemonchiffon: 'fffacd', lightblue: 'add8e6',
  lightcoral: 'f08080', lightcyan: 'e0ffff', lightgoldenrodyellow: 'fafad2', lightgray: 'd3d3d3',
  lightgreen: '90ee90', lightgrey: 'd3d3d3', lightpink: 'ffb6c1', lightsalmon: 'ffa07a',
  lightseagreen: '20b2aa', lightskyblue: '87cefa', lightslategray: '778899', lightslategrey: '778899',
  lightsteelblue: 'b0c4de', lightyellow: 'ffffe0', lime: '00ff00', limegreen: '32cd32',
  linen: 'faf0e6', magenta: 'ff00ff', maroon: '800000', mediumaquamarine: '66cdaa',
  mediumblue: '0000cd', mediumorchid: 'ba55d3', mediumpurple: '9370db', mediumseagreen: '3cb371',
  mediumslateblue: '7b68ee', mediumspringgreen: '00fa9a', mediumturquoise: '48d1cc', mediumvioletred: 'c71585',
  midnightblue: '191970', mintcream: 'f5fffa', mistyrose: 'ffe4e1', moccasin: 'ffe4b5',
  navajowhite: 'ffdead', navy: '000080', oldlace: 'fdf5e6', olive: '808000',
  olivedrab: '6b8e23', orange: 'ffa500', orangered: 'ff4500', orchid: 'da70d6',
  palegoldenrod: 'eee8aa', palegreen: '98fb98', paleturquoise: 'afeeee', palevioletred: 'db7093',
  papayawhip: 'ffefd5', peachpuff: 'ffdab9', peru: 'cd853f', pink: 'ffc0cb',
  plum: 'dda0dd', powderblue: 'b0e0e6', purple: '800080', rebeccapurple: '663399',
  red: 'ff0000', rosybrown: 'bc8f8f', royalblue: '4169e1', saddlebrown: '8b4513',
  salmon: 'fa8072', sandybrown: 'f4a460', seagreen: '2e8b57', seashell: 'fff5ee',
  sienna: 'a0522d', silver: 'c0c0c0', skyblue: '87ceeb', slateblue: '6a5acd',
  slategray: '708090', slategrey: '708090', snow: 'fffafa', springgreen: '00ff7f',
  steelblue: '4682b4', tan: 'd2b48c', teal: '008080', thistle: 'd8bfd8',
  tomato: 'ff6347', turquoise: '40e0d0', violet: 'ee82ee', wheat: 'f5deb3',
  white: 'ffffff', whitesmoke: 'f5f5f5', yellow: 'ffff00', yellowgreen: '9acd32',
};

const clamp255 = (n) => (n < 0 ? 0 : n > 255 ? 255 : n) | 0;

// Serialize {r,g,b,a}. Browsers emit `rgb(r, g, b)` when fully opaque, else
// `rgba(r, g, b, a)` with the alpha trimmed (0.5, not 0.50). Both sides of every
// comparison route through here, so the exact alpha rounding only has to be
// internally consistent.
function rgb(r, g, b, a) {
  r = clamp255(r); g = clamp255(g); b = clamp255(b);
  if (a === undefined || a >= 1) return `rgb(${r}, ${g}, ${b})`;
  if (a < 0) a = 0;
  const as = String(Math.round(a * 1000) / 1000);
  return `rgba(${r}, ${g}, ${b}, ${as})`;
}

function hexToRgb(v) {
  const h = v.slice(1);
  const n = h.length;
  if (!/^[0-9a-fA-F]+$/.test(h)) return null;
  if (n === 3 || n === 4) {
    const r = parseInt(h[0] + h[0], 16), g = parseInt(h[1] + h[1], 16), b = parseInt(h[2] + h[2], 16);
    const a = n === 4 ? parseInt(h[3] + h[3], 16) / 255 : undefined;
    return rgb(r, g, b, a);
  }
  if (n === 6 || n === 8) {
    const r = parseInt(h.slice(0, 2), 16), g = parseInt(h.slice(2, 4), 16), b = parseInt(h.slice(4, 6), 16);
    const a = n === 8 ? parseInt(h.slice(6, 8), 16) / 255 : undefined;
    return rgb(r, g, b, a);
  }
  return null;
}

// Re-serialize rgb()/rgba() to canonical spacing. Percentage components (rare in
// CSS-in-JS output) → null (passthrough) rather than guessing.
function normalizeRgb(v) {
  const open = v.indexOf('(');
  if (open === -1 || v[v.length - 1] !== ')') return null;
  const parts = v.slice(open + 1, -1).split(/[,\s/]+/).filter(Boolean);
  if (parts.length < 3 || parts.length > 4) return null;
  const nums = [];
  for (let i = 0; i < parts.length; i++) {
    if (parts[i].indexOf('%') !== -1) return null;
    const f = parseFloat(parts[i]);
    if (Number.isNaN(f)) return null;
    nums.push(f);
  }
  return rgb(nums[0], nums[1], nums[2], nums[3]);
}

function hslToRgb(v) {
  const open = v.indexOf('(');
  if (open === -1 || v[v.length - 1] !== ')') return null;
  const parts = v.slice(open + 1, -1).split(/[,\s/]+/).filter(Boolean);
  if (parts.length < 3 || parts.length > 4) return null;
  let h = parseFloat(parts[0]);
  const s = parseFloat(parts[1]) / 100;
  const l = parseFloat(parts[2]) / 100;
  if (Number.isNaN(h) || Number.isNaN(s) || Number.isNaN(l)) return null;
  const a = parts[3] !== undefined ? parseFloat(parts[3]) : undefined;
  h = ((h % 360) + 360) % 360 / 360;
  const hue = (p, q, t) => {
    if (t < 0) t += 1; if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
  };
  let r, g, b;
  if (s === 0) { r = g = b = l; }
  else {
    const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
    const p = 2 * l - q;
    r = hue(p, q, h + 1 / 3); g = hue(p, q, h); b = hue(p, q, h - 1 / 3);
  }
  return rgb(Math.round(r * 255), Math.round(g * 255), Math.round(b * 255), a);
}

// Canonicalize one color value, or null if unrecognized (→ caller passthrough).
export function canonicalizeColor(value, includeNamed) {
  if (!value) return null;
  const v = value.trim();
  if (v[0] === '#') return hexToRgb(v);
  const lower = v.toLowerCase();
  if (lower.startsWith('rgb')) return normalizeRgb(v);
  if (lower.startsWith('hsl')) return hslToRgb(v);
  if (includeNamed) {
    if (lower === 'transparent') return 'rgba(0, 0, 0, 0)';
    const hex = NAMED[lower];
    if (hex) return hexToRgb('#' + hex);
  }
  return null;
}
