// Adapters that drive happy-dom and jsdom through the SAME html5lib-tests gate,
// so conformance is measured apples-to-apples against fast-dom.
//
// Each adapter parses a fixture (document or fragment-in-context) into a real DOM,
// then serializes it with one generic standard-DOM walker into html5lib dump format.

import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);

const SVG_NS = 'http://www.w3.org/2000/svg';
const MATHML_NS = 'http://www.w3.org/1998/Math/MathML';

const indent = (depth) => '| ' + '  '.repeat(depth);

function nsShort(uri) {
  if (uri === SVG_NS) return 'svg';
  if (uri === MATHML_NS) return 'math';
  return '';
}

// Render one standard-DOM node into html5lib lines.
function serializeDomNode(node, depth, out) {
  switch (node.nodeType) {
    case 1: {
      // element
      const ns = nsShort(node.namespaceURI);
      const local = node.localName;
      out.push(`${indent(depth)}<${ns ? ns + ' ' : ''}${local}>`);

      const attrs = Array.from(node.attributes).map((a) => ({
        display: a.prefix ? `${a.prefix} ${a.localName}` : a.name,
        value: a.value,
      }));
      attrs.sort((x, y) => (x.display < y.display ? -1 : x.display > y.display ? 1 : 0));
      for (const a of attrs) out.push(`${indent(depth + 1)}${a.display}="${a.value}"`);

      // <template> content lives in a separate fragment, printed as `content`
      if (local === 'template' && node.content) {
        out.push(`${indent(depth + 1)}content`);
        for (const c of node.content.childNodes) serializeDomNode(c, depth + 2, out);
      } else {
        for (const c of node.childNodes) serializeDomNode(c, depth + 1, out);
      }
      break;
    }
    case 3: // text
      out.push(`${indent(depth)}"${node.data}"`);
      break;
    case 8: // comment
      out.push(`${indent(depth)}<!-- ${node.data} -->`);
      break;
    case 10: { // doctype
      const pub = node.publicId ?? '';
      const sys = node.systemId ?? '';
      out.push(
        pub === '' && sys === ''
          ? `${indent(depth)}<!DOCTYPE ${node.name}>`
          : `${indent(depth)}<!DOCTYPE ${node.name} "${pub}" "${sys}">`
      );
      break;
    }
    case 11: // fragment
      for (const c of node.childNodes) serializeDomNode(c, depth, out);
      break;
    default:
      // processing instruction etc — html parsing turns these into comments, rarely hit
      break;
  }
}

function serializeDomRoot(root) {
  const out = [];
  for (const c of root.childNodes) serializeDomNode(c, 0, out);
  return out.join('\n');
}

// Build a context element for fragment parsing and set its innerHTML.
function fragmentRoot(document, fragmentContext, html) {
  let ns = '';
  let local = fragmentContext;
  const sp = fragmentContext.indexOf(' ');
  if (sp !== -1) {
    const pre = fragmentContext.slice(0, sp);
    local = fragmentContext.slice(sp + 1);
    if (pre === 'svg') ns = SVG_NS;
    else if (pre === 'math') ns = MATHML_NS;
  }
  const ctx = ns ? document.createElementNS(ns, local) : document.createElement(local);
  ctx.innerHTML = html;
  // <template> context exposes parsed nodes on .content
  return ctx.localName === 'template' && ctx.content ? ctx.content : ctx;
}

export function makeHappyDomAdapter() {
  // lazy import so the gate works without happy-dom installed
  const { Window } = require('happy-dom');
  const window = new Window();
  const { document, DOMParser } = window;
  return (t) => {
    if (t.fragmentContext != null) {
      return serializeDomRoot(fragmentRoot(document, t.fragmentContext, t.data));
    }
    const doc = new DOMParser().parseFromString(t.data, 'text/html');
    return serializeDomRoot(doc);
  };
}

export function makeJsdomAdapter() {
  const { JSDOM } = require('jsdom');
  const window = new JSDOM('').window;
  const { document, DOMParser } = window;
  return (t) => {
    if (t.fragmentContext != null) {
      return serializeDomRoot(fragmentRoot(document, t.fragmentContext, t.data));
    }
    const doc = new DOMParser().parseFromString(t.data, 'text/html');
    return serializeDomRoot(doc);
  };
}

export const _internal = { serializeDomRoot, serializeDomNode, nsShort };
