// HTML serialization for innerHTML / outerHTML (WHATWG-ish fragment serializer).

// HTML "serializes as void" (Chrome 156 includes the legacy basefont / bgsound / frame / keygen).
const VOID = new Set([
  'area', 'base', 'basefont', 'bgsound', 'br', 'col', 'embed', 'frame', 'hr', 'img', 'input',
  'keygen', 'link', 'meta', 'param', 'source', 'track', 'wbr',
]);

// Elements whose content is raw text (not escaped).
const RAW_TEXT = new Set(['style', 'script', 'xmp', 'iframe', 'noembed', 'noframes', 'plaintext']);

function escapeText(s) {
  return s.replace(/&/g, '&amp;').replace(/ /g, '&nbsp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
// attribute values also escape < and > (Chrome 156)
function escapeAttr(s) {
  return s.replace(/&/g, '&amp;').replace(/ /g, '&nbsp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

// The element tag (HTML / SVG / MathML elements serialize their local name, other
// namespaces the qualified name; a createElementNS prefix is kept on all of them —
// Chrome 156: <h:br>, <svg:g>).
function elementTag(node, html) {
  if (node.__prefix !== undefined) return `${node.__prefix}:${node.localName}`;
  const ns = node.__ns;
  return html || ns === 'svg' || ns === 'math' ? node.localName : node.tagName;
}
function serializeAttrs(node, out) {
  // __attrs is lazy: buffer-backed elements leave it undefined (store __attrIdx).
  // Build from the SoA on first touch, matching every other __attrs read site.
  const attrs = node.__attrs ?? (node.__attrs = node.__buildAttrs());
  for (const a of attrs) {
    const name = a.prefix ? `${a.prefix}:${a.name}` : a.name;
    out.push(` ${name}="${escapeAttr(a.value)}"`);
  }
}
// An element: its tag, attributes and children. Only HTML elements can be void or
// <template>, keyed on the local name.
function serializeElement(node, out) {
  const html = node.__ns === '';
  const local = node.localName;
  const tag = elementTag(node, html);
  out.push('<' + tag);
  serializeAttrs(node, out);
  out.push('>');
  if (html && VOID.has(local)) return;
  const kidsHost = html && local === 'template' && node.content ? node.content : node;
  serializeChildren(kidsHost, out);
  out.push(`</${tag}>`);
}

function serializeNode(node, out) {
  switch (node.nodeType) {
    case 1: // element
      serializeElement(node, out);
      break;
    case 3: { // text
      // raw text keys on the parent's prefix-free local name (Chrome 156: <h:script> escapes)
      const parent = node.parentNode, parentTag = parent && parent.__prefix === undefined && parent.localName;
      out.push(RAW_TEXT.has(parentTag) ? node.data : escapeText(node.data));
      break;
    }
    case 8: // comment
      out.push(`<!--${node.data}-->`);
      break;
    case 10: // doctype
      out.push(`<!DOCTYPE ${node.name}>`);
      break;
    case 11: // fragment
      serializeChildren(node, out);
      break;
  }
}

function serializeChildren(node, out) {
  for (const c of node.childNodes) serializeNode(c, out);
}

export function serializeInner(node) {
  const out = [];
  if (node.nodeType === 1 && node.localName === 'template' && node.content) {
    serializeChildren(node.content, out);
  } else {
    serializeChildren(node, out);
  }
  return out.join('');
}

export function serializeOuter(node) {
  const out = [];
  serializeNode(node, out);
  return out.join('');
}
