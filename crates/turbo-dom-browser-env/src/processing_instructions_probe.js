// HTML processing instructions (miaskiewicz/turbo-dom#135): Chrome 156 tokenizes `<?target
// data?>` as a ProcessingInstruction node when the target is an ASCII letter followed by ASCII
// alphanumerics, `-` or `_` and is not `xml` (any case); everything else after `<?` stays a bogus
// comment, and an instruction cut short by the end of input emits nothing. Each case is parsed
// through every DOM parsing entry point (innerHTML, insertAdjacentHTML, createContextualFragment,
// setHTMLUnsafe, setHTML, Document.parseHTMLUnsafe / parseHTML; DOMParser is the embedder's and
// reaches the same document parse) and read back as node descriptions plus the serialization.
(async () => {
  const cases = [
    '<?pi x?>', '<?xml version="1.0">', '<?>', '<? x?>', '<?1x?>', '<?',
    '<?pi', '<?pi ', '<?pi x', '<?pi x?', '<?a-', '<?1', '<?a x\0',
    '<?pi>', '<?pi?>', '<?pi x>', '<?pi\tx?>', '<?pi\nx?>', '<?pi\fx?>', '<?pi\rx?>', '<?pi x\r\ny?>',
    '<?PI x?>', '<?a1 x?>', '<?a-b x?>', '<?a_b x?>', '<?ab9Z?>', '<?Z>',
    '<?a.b x?>', '<?a:b x?>', '<?a/b x?>', '<?a"b x?>', '<?a=b?>', '<?a!b?>', '<?a/>', '<?a\vb?>',
    '<?é x?>', '<?aé x?>', '<?p\0i x?>', '<?a\0 x?>',
    '<?xml x?>', '<?XML x?>', '<?xmL x?>', '<?xml>', '<?xml?>', '<?xmlfoo x?>', '<?xmlx>', '<?xm>',
    '<?pi x? y?>', '<?pi a>b?>', '<?pi  x  ?>', '<?pi x\0y?>', '<?pi x??>',
    '<?pi -->?>', '<?a?b?>', '<?a ?>', '<?a ? >', '<?a >', '<?a  ?>', '<?a\n\n?>', '<?a b/>', '<?a b c>',
    '<?pi &amp;?>', '<?a&amp; x?>', '<?pi x?>a', '<!--?pi x?-->',
    '<?import namespace="foo" implementation="#bar">',
    '<svg><?pi x?></svg>', '<math><?pi x?></math>', '<table><?pi x?><tr><?q y?></table>',
    '<select><?pi x?></select>',
  ];
  const describe = node => {
    switch (node.nodeType) {
      case 1: return [node.nodeName, Array.from(node.childNodes, describe)];
      case 3: return ['#text', node.data];
      case 7: return ['#pi', node.target, node.data, node instanceof ProcessingInstruction, node.nodeName];
      case 8: return ['#comment', node.data];
      case 10: return ['#doctype', node.name];
      default: return [node.nodeName];
    }
  };
  const read = parent => [Array.from(parent.childNodes, describe), parent.innerHTML ?? null];
  const host = document.body.appendChild(document.createElement('div'));
  const out = {};
  for (const html of cases) {
    const row = {};
    host.innerHTML = html;
    row.innerHTML = read(host);
    host.textContent = '';
    host.insertAdjacentHTML('beforeend', html);
    row.insertAdjacentHTML = read(host);
    const range = document.createRange();
    range.selectNodeContents(host);
    host.replaceChildren(range.createContextualFragment(html));
    row.createContextualFragment = read(host);
    host.setHTMLUnsafe(html);
    row.setHTMLUnsafe = read(host);
    host.setHTML(html, { sanitizer: { processingInstructions: ['pi', 'a', 'PI'], comments: true } });
    row.setHTMLAllow = read(host);
    host.setHTML(html);
    row.setHTMLDefault = read(host);
    const unsafe = Document.parseHTMLUnsafe(html);
    row.parseHTMLUnsafe = [Array.from(unsafe.childNodes, describe), read(unsafe.body)];
    const safe = Document.parseHTML(html, { sanitizer: { removeProcessingInstructions: ['a'], comments: true } });
    row.parseHTML = [Array.from(safe.childNodes, describe), read(safe.body)];
    out[JSON.stringify(html)] = row;
  }
  // Lone surrogates (read as code units: JSON cannot carry them): a bad target, kept data.
  out.loneSurrogates = ['<?a\ud800 x?>', '<?a x\ud800?>'].map(html => {
    host.innerHTML = html;
    return Array.from(host.childNodes, node => [node.nodeType, node.target ?? null, Array.from(node.nodeValue, c => c.charCodeAt(0))]);
  });
  // Serialization of a parsed instruction: the parent's outerHTML, the node's own reads.
  host.innerHTML = '<?pi data?><?empty?>';
  const [pi, empty] = host.childNodes;
  out.serialization = {
    outerHTML: host.outerHTML,
    piOuter: [pi.outerHTML ?? null, pi.textContent, pi.nodeValue],
    emptyHtml: [empty.target, empty.data, empty.textContent, empty.nodeValue],
    cloned: host.cloneNode(true).innerHTML,
    afterData: (pi.data = 'a?>b', host.innerHTML),
  };
  // The sanitizer's processing-instruction lists over parsed instructions.
  const html = '<?pi a?><?other b?><?PI c?><!--c-->';
  const sanitized = config => { host.setHTML(html, { sanitizer: config }); return host.innerHTML; };
  out.sanitizer = {
    default: sanitized({}),
    allowPi: sanitized({ processingInstructions: ['pi'] }),
    allowObject: sanitized({ processingInstructions: [{ target: 'other' }] }),
    allowEmpty: sanitized({ processingInstructions: [] }),
    remove: sanitized({ removeProcessingInstructions: ['pi'] }),
    removeEmpty: sanitized({ removeProcessingInstructions: [] }),
    unsafeRemove: (host.setHTMLUnsafe(html, { sanitizer: { removeProcessingInstructions: ['other'] } }), host.innerHTML),
    unsafeAll: (host.setHTMLUnsafe(html), host.innerHTML),
  };
  return out;
})()
