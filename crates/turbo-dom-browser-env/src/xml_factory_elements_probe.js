(() => {
  const implementation = document.implementation;
  const docs = [new Document(), implementation.createDocument(null, "Root"), implementation.createDocument("http://www.w3.org/1999/xhtml", "html"), implementation.createHTMLDocument("HTML"), implementation.createDocument("http://www.w3.org/2000/svg", "svg")];
  const results = [];
  for (const doc of docs) {
    const nodes = [doc.createElement("MiXeD"), doc.createElement("x:Thing"), Document.prototype.createElement.call(doc, "Camel"), doc.createElementNS("http://www.w3.org/1999/xhtml", "MiXeD"), doc.createElementNS("urn:fixture", "p:Camel")];
    results.push({
      contentType: doc.contentType,
      nodes: nodes.map((node) => [node.tagName, node.nodeName, node.localName, node.prefix, node.namespaceURI, node.ownerDocument === doc]),
    });
  }
  return results;
})();
