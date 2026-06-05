// HTML serialization for innerHTML / outerHTML (WHATWG-ish fragment serializer).

const VOID = new Set([
  'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link',
  'meta', 'param', 'source', 'track', 'wbr',
]);

// Elements whose content is raw text (not escaped).
const RAW_TEXT = new Set(['style', 'script', 'xmp', 'iframe', 'noembed', 'noframes', 'plaintext']);

function escapeText(s) {
  return s.replace(/&/g, '&amp;').replace(/ /g, '&nbsp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
function escapeAttr(s) {
  return s.replace(/&/g, '&amp;').replace(/ /g, '&nbsp;').replace(/"/g, '&quot;');
}

function serializeNode(node, out) {
  switch (node.nodeType) {
    case 1: { // element
      const tag = node.localName;
      out.push('<' + tag);
      // __attrs is lazy: buffer-backed elements leave it undefined (store __attrIdx).
      // Build from the SoA on first touch, matching every other __attrs read site.
      const attrs = node.__attrs ?? (node.__attrs = node.__buildAttrs());
      for (const a of attrs) {
        const name = a.prefix ? `${a.prefix}:${a.name}` : a.name;
        out.push(` ${name}="${escapeAttr(a.value)}"`);
      }
      out.push('>');
      if (VOID.has(tag)) break;
      if (tag === 'template' && node.content) {
        serializeChildren(node.content, out);
      } else {
        serializeChildren(node, out);
      }
      out.push(`</${tag}>`);
      break;
    }
    case 3: { // text
      const parentTag = node.parentNode && node.parentNode.localName;
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
