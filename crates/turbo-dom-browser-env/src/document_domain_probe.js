(async () => {
  // Top, initial about:blank and srcdoc documents of one agent cluster; a
  // document.domain set in any of them is read back in all three.
  const tag = value => Object.prototype.toString.call(value);
  const failure = error => ({ name: error.name, message: error.message, type: tag(error) });
  const frames = globalThis.__domainFrames || await (async () => {
    const blank = document.createElement('iframe');
    const srcdoc = document.createElement('iframe');
    srcdoc.srcdoc = '<!doctype html><html><head></head><body></body></html>';
    const loaded = new Promise(done => srcdoc.addEventListener('load', done, { once: true }));
    document.body.append(blank, srcdoc);
    await loaded;
    return { blank: blank.contentWindow, srcdoc: srcdoc.contentWindow };
  })();
  const documents = { top: document, blank: frames.blank.document, srcdoc: frames.srcdoc.document };
  const read = () => Object.fromEntries(Object.entries(documents).map(([name, doc]) => [name, doc.domain]));
  const set = (name, value) => { try { documents[name].domain = value; return { value: documents[name].domain }; } catch (error) { return failure(error); } };
  const host = document.domain;
  const labels = host.split('.');
  const parent = labels.slice(1).join('.'), site = labels.slice(-2).join('.');
  const steps = [['srcdoc', parent], ['top', site], ['blank', 'other.example'], ['srcdoc', 'com'], ['srcdoc', ''], ['blank', host], ['srcdoc', site]];
  const out = { before: read(), steps: [] };
  for (const [name, value] of steps) out.steps.push({ name, value, result: set(name, value), after: read() });
  // A document.domain read through another realm's accessor sees the same shared value.
  out.crossAccessor = Object.getOwnPropertyDescriptor(frames.srcdoc.Document.prototype, 'domain').get.call(document);
  return out;
})();
