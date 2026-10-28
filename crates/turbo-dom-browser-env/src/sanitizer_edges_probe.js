(async () => {
  // Sanitizer edges: processing-instruction lists in configurations (they survive get()),
  // declarative shadow roots inside setHTML / parseHTML input (their contents are sanitized),
  // shadow roots as setHTML targets, and comments at document level.
  const out = {};
  const attempt = run => { try { return run(); } catch (e) { return [e.name, String(e.message).length > 0]; } };
  const get = config => attempt(() => { const value = new Sanitizer(config).get(); return [value.processingInstructions, value.removeProcessingInstructions, value.comments]; });
  out.configs = {
    allow: get({ processingInstructions: [{ target: 'x' }, 'y'] }),
    remove: get({ removeProcessingInstructions: ['z', { target: 'w' }] }),
    both: get({ processingInstructions: ['x'], removeProcessingInstructions: ['y'] }),
    badItem: get({ processingInstructions: [{}] }),
    nullItem: get({ processingInstructions: [null] }),
    empty: get({ processingInstructions: [] }),
  };
  const html = '<div id=h><template shadowrootmode=open><b onclick=x>in</b><script>1</script><i>keep</i></template><span onclick=y>light</span></div><!--c--><?pi x?>';
  const render = node => {
    const host = node.querySelector ? node.querySelector('#h') : null;
    return [node.innerHTML === undefined ? null : node.innerHTML, host && host.shadowRoot ? host.shadowRoot.innerHTML : null];
  };
  const target = document.body.appendChild(document.createElement('div'));
  target.setHTML(html, { sanitizer: { processingInstructions: ['pi'] } });
  out.setHTMLPi = render(target);
  target.setHTML(html);
  out.setHTMLDefault = render(target);
  target.setHTML(html, { sanitizer: { comments: true } });
  out.setHTMLComments = render(target);
  target.setHTMLUnsafe(html, { sanitizer: { removeElements: ['i'] } });
  out.setHTMLUnsafe = render(target);
  const host = document.body.appendChild(document.createElement('section'));
  const root = host.attachShadow({ mode: 'open' });
  root.setHTML('<p onclick=x>p</p><script>2</script><template shadowrootmode=closed><b>c</b></template>');
  out.shadowTarget = root.innerHTML;
  const parsed = Document.parseHTML('<!doctype html><!--top--><html><body>' + html + '</body></html>', { sanitizer: { comments: true } });
  out.parseHTML = [parsed.childNodes.length, Array.from(parsed.childNodes, node => node.nodeName), render(parsed.body)];
  const unsafe = Document.parseHTMLUnsafe('<!--top--><p>x</p>', { sanitizer: { comments: false } });
  out.parseHTMLUnsafe = [Array.from(unsafe.childNodes, node => node.nodeName), unsafe.body.innerHTML];
  out.errors = {
    setHTMLOnTemplate: attempt(() => { const t = document.createElement('template'); t.setHTML('<b>x</b>'); return t.content.childNodes.length; }),
    setHTMLOnScript: attempt(() => { const s = document.createElement('script'); s.setHTML('x'); return s.textContent; }),
    receiver: attempt(() => Element.prototype.setHTML.call(document, 'x')),
    badSanitizer: attempt(() => target.setHTML('x', { sanitizer: 1 })),
  };
  return out;
})()
