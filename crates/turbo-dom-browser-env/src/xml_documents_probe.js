// XML documents (turbo-dom#186): DOMParser's four XML SupportedTypes, the parsed
// XMLDocument's tree (namespaces, prefixes, attributes, text / CDATA / comments / PIs /
// doctype), Chrome's <parsererror> shape for well-formedness errors, and the
// XMLSerializer round trip. The binding test replaces `parseXML` with its native
// parse_xml_document_units host function (standalone V8 has no DOMParser).
(() => {
  const parseXML = (source, type) => new DOMParser().parseFromString(source, type);
  const tag = (value) => Object.prototype.toString.call(value);
  const dump = (node) => {
    const row = [node.nodeType, node.nodeName];
    if (node.nodeType === 1) {
      row.push(node.namespaceURI, node.prefix, node.localName, node.tagName,
        Array.from(node.attributes, (a) => [a.name, a.namespaceURI, a.prefix, a.localName, a.value]));
    } else if (node.nodeType === 10) {
      row.push(node.name, node.publicId, node.systemId);
    } else if (node.nodeType === 7) {
      row.push(node.target, node.data);
    } else if (node.nodeType !== 9) {
      row.push(node.data);
    }
    if (node.nodeType === 1 || node.nodeType === 9) row.push(Array.from(node.childNodes, dump));
    return row;
  };
  const describe = (source, type) => {
    try {
      const doc = parseXML(source, type);
      return {
        tag: tag(doc),
        contentType: doc.contentType,
        compatMode: doc.compatMode,
        characterSet: doc.characterSet,
        root: doc.documentElement && doc.documentElement.nodeName,
        errors: doc.getElementsByTagNameNS("http://www.w3.org/1999/xhtml", "parsererror").length,
        tree: dump(doc),
        xml: new XMLSerializer().serializeToString(doc),
      };
    } catch (error) {
      return { error: error.name, message: error.message };
    }
  };
  const TYPES = ["application/xml", "text/xml", "image/svg+xml", "application/xhtml+xml"];
  const CASES = [
    ["nested", "<a><b/></a>"],
    ["namespaces", '<r xmlns="urn:d" xmlns:p="urn:p"><p:c p:at="1" at2="2"/><d xmlns=""><e/></d><p:f xmlns:p="urn:q"/></r>'],
    ["nodes", '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE r SYSTEM "r.dtd">\n<!--c--><?pi  some data ?><r>t<![CDATA[<x>&]]>&amp;&lt;&gt;&quot;&apos;&#65;&#x42;<?empty?></r><!--after-->\n'],
    ["doctypePublic", '<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd"><html xmlns="http://www.w3.org/1999/xhtml"><head><title>T</title></head><body><p class="x">p<br/></p></body></html>'],
    ["svg", '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 1 1"><use xlink:href="#a"/><foreignObject><div xmlns="http://www.w3.org/1999/xhtml">h</div></foreignObject></svg>'],
    ["whitespace", '<a x="1&#10;2\t3\n4\r5" y=\'q"\' z="&#9;">\r\n  sp \r x\n</a>'],
    ["xmlPrefix", '<a xml:lang="en" xml:space="preserve"><B Mixed="Case">x</B></a>'],
    ["entities", '<!DOCTYPE r [<!ENTITY e "ent"><!ELEMENT r ANY>]><r>&e;&#x1F600;</r>'],
    ["noNamespaceHtml", "<html><body><p>x</p></body></html>"],
    ["serializerEscapes", '<a t="&lt;&amp;&gt;&quot;\'">&lt;&amp;&gt;"\'</a>'],
    ["emptyAttr", '<a b="" c=" "/>'],
    ["nsOrder", '<r b="1" xmlns:z="urn:z" xmlns:a="urn:a" xmlns="urn:d" z:c="2"/>'],
    ["nsRedeclare", '<r xmlns:p="urn:p" xmlns="urn:d"><p:a xmlns:p="urn:p" xmlns="urn:d"/></r>'],
    ["nsEmptyDefault", '<a xmlns=""><b/></a>'],
    ["standalone", '<?xml version="1.0" standalone="yes"?><a/>'],
    ["declarationNoEncoding", "<?xml version='1.1'?>\n<a/>"],
    ["declarationAll", '<?xml version="1.1" encoding="utf-8" standalone="no" ?><a/>'],
    ["declarationLatin", "<?xml version = '1.0' encoding = 'ISO-8859-1'?><a>\u00e9</a>"],
    ["nsInTagWithGt", '<r a=">" xmlns:q="u&amp;q" b="/>"><q:x/></r>'],
    ["doctypeSingleQuotes", "<!DOCTYPE a PUBLIC 'p' 's'><a/>"],
    ["unicodeColumn", "<a>\u00e9\ud83d\ude00</b>"],
    // well-formedness errors
    ["mismatch", "<a><b></a>"],
    ["empty", ""],
    ["whitespaceOnly", "  \n "],
    ["unclosed", "<a>"],
    ["unclosedNested", "<a><b>text"],
    ["extraContent", "<a></a><b/>"],
    ["trailingText", "<a></a>tail"],
    ["unquoted", "<a x=1/>"],
    ["duplicateAttr", '<a x="1" x="2"/>'],
    ["unboundPrefix", "<p:a/>"],
    ["unboundAttrPrefix", '<a p:x="1"/>'],
    ["undefinedEntity", "<a>&foo;</a>"],
    ["bareAmp", "<a>&</a>"],
    ["textOnly", "text"],
    ["lt", "<a>1 < 2</a>"],
    ["badName", "<1a/>"],
    ["commentDashes", "<a><!-- a -- b --></a>"],
    ["multiLine", "<r>\n  <x>1</x>\n  <y>2\n</r>"],
    ["xmlDeclOnly", '<?xml version="1.0"?>'],
    ["lateXmlDecl", '<a/><?xml version="1.0"?>'],
    ["unclosedComment", "<a><!-- x"],
    ["unclosedCdata", "<a><![CDATA[x"],
    ["attrLt", '<a x="<"/>'],
    ["controlChar", "<a>\u0001</a>"],
    ["closeOnly", "</a>"],
  ];
  const result = {};
  for (const [id, source] of CASES) result[id] = describe(source, "application/xml");
  for (const type of TYPES) result["type " + type] = describe("<a><b/></a>", type);
  for (const type of TYPES) result["typeError " + type] = describe("<a><b></a>", type);
  // XMLSerializer over nodes built in an XML document.
  const doc = parseXML('<r xmlns="urn:d"><c/></r>', "application/xml");
  const r = doc.documentElement;
  const added = doc.createElementNS("urn:x", "x:n");
  added.setAttributeNS("urn:y", "y:at", "v");
  added.appendChild(doc.createCDATASection("c]>d"));
  let cdataError;
  try { doc.createCDATASection("c]]>d"); } catch (error) { cdataError = [error.name, error.message]; }
  added.appendChild(doc.createComment("k"));
  added.appendChild(doc.createProcessingInstruction("t", "d"));
  r.appendChild(added);
  r.appendChild(doc.createElementNS("urn:d", "plain"));
  r.appendChild(doc.createElement("nons"));
  result.serializer = {
    doc: new XMLSerializer().serializeToString(doc),
    element: new XMLSerializer().serializeToString(added),
    children: Array.from(added.childNodes, (n) => [n.nodeType, n.nodeName, n.nodeValue]),
    text: r.textContent,
    cdataError,
    htmlNode: new XMLSerializer().serializeToString(document.createElement("br")),
    htmlFragment: (() => {
      const d = document.createElement("div");
      d.innerHTML = '<p a="&quot;">x<br>&amp;</p>';
      return new XMLSerializer().serializeToString(d);
    })(),
    htmlCdata: (() => {
      try { document.createCDATASection("x"); return "created"; } catch (error) { return error.name; }
    })(),
    attributeNoPrefix: (() => {
      const e = doc.createElementNS("urn:e", "e");
      e.setAttributeNS("urn:a", "plain", "1");
      e.setAttributeNS("urn:a", "other", "2");
      e.setAttributeNS("urn:b", "p:x", "3");
      e.setAttributeNS("urn:c", "p:y", "4");
      e.setAttributeNS("http://www.w3.org/XML/1998/namespace", "xml:lang", "en");
      return new XMLSerializer().serializeToString(e);
    })(),
    escapes: (() => {
      const e = doc.createElement("q");
      e.setAttribute("v", "<&>\"'\t\n\r");
      e.appendChild(doc.createTextNode("<&>\"'\t\n\r "));
      return new XMLSerializer().serializeToString(e);
    })(),
    doctype: [
      new XMLSerializer().serializeToString(document.implementation.createDocumentType("a", "", "")),
      new XMLSerializer().serializeToString(document.implementation.createDocumentType("a", "p", "")),
      new XMLSerializer().serializeToString(document.implementation.createDocumentType("a", "", "s")),
    ],
    fragment: (() => {
      const f = doc.createDocumentFragment();
      f.append(doc.createElementNS("urn:d", "x"), "t");
      return new XMLSerializer().serializeToString(f);
    })(),
  };
  const run = (fn) => { try { return fn(); } catch (error) { return [error.name, error.message]; } };
  result.surface = {
    tag: tag(new XMLSerializer()),
    length: [XMLSerializer.length, XMLSerializer.prototype.serializeToString.length],
    call: run(() => XMLSerializer()),
    noArgument: run(() => new XMLSerializer().serializeToString()),
    notNode: run(() => new XMLSerializer().serializeToString({})),
    receiver: run(() => XMLSerializer.prototype.serializeToString.call({}, document)),
  };
  return result;
})()
