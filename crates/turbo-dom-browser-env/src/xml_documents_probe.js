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
    // entity replacement text holding elements, and internal subsets with "]" inside
    // quoted values / comments (the namespace declarations must stay on their elements)
    ["entityElements", '<!DOCTYPE r [<!ENTITY e "<b/>">]><r>&e;<b xmlns:p="urn:p"/></r>'],
    ["entityElementsNs", "<!DOCTYPE r [<!ENTITY e '<b xmlns:q=\"urn:q\"><q:c/></b>'><!ENTITY f \"&e;<i/>\">]><r xmlns:p=\"urn:p\">&f;&e;<c xmlns:p=\"urn:p\" xmlns:z=\"urn:z\"/></r>"],
    ["entityInAttribute", '<!DOCTYPE r [<!ENTITY e "v">]><r a="&e;&lt;&e;"><s xmlns:p="urn:p"/></r>'],
    ["subsetBracket", '<!DOCTYPE r [<!ENTITY a "]"><!ENTITY b "x">]><r xmlns:p="u">&a;&b;</r>'],
    ["subsetComment", "<!DOCTYPE r [<!-- ] > --><?pi ] > ?><!ENTITY b '>]'><!ENTITY d \"<!-- ] -->\">]><r xmlns:p=\"u\"><p:s xmlns:q=\"v\"/>&b;</r>"],
    // internal-subset ATTLIST defaults (turbo-dom#261): applied after the specified
    // attributes in declaration order, the first declaration of a name binding it
    ["attlistCdata", '<!DOCTYPE r [<!ATTLIST r c CDATA "  a  b ">]><r/>'],
    ["attlistFixed", '<!DOCTYPE r [<!ATTLIST r f CDATA #FIXED "x" g CDATA #IMPLIED h CDATA #REQUIRED>]><r/>'],
    ["attlistEnumerated", '<!DOCTYPE r [<!ATTLIST r e (a|b) "b" t NMTOKEN "  q  " n NOTATION (x) "x" i ID #IMPLIED>]><r><r e="a"/></r>'],
    ["attlistOrder", '<!DOCTYPE r [<!ATTLIST r z CDATA "1"><!ATTLIST r a CDATA \'2\' z CDATA "3">]><r m="0"/>'],
    ["attlistNamespaced", '<!DOCTYPE r [<!ATTLIST s p:x CDATA "v" xml:lang CDATA "en">]><r xmlns:p="urn:p"><s/><s p:x="w"/></r>'],
    ["attlistUnbound", '<!DOCTYPE r [<!ATTLIST r p:x CDATA "v">]><r/>'],
    ["attlistReferences", '<!DOCTYPE r [<!ENTITY e "E"><!ATTLIST r c CDATA "&e;&amp;&#65;" d CDATA "a\tb">]><r/>'],
    ["attlistPrefixedElement", '<!DOCTYPE r [<!ATTLIST p:r c CDATA "1">]><p:r xmlns:p="u"/>'],
    ["attlistEntityElement", '<!DOCTYPE r [<!ENTITY e "<s/>"><!ATTLIST s c CDATA "d">]><r>&e;<s/></r>'],
    ["attlistCharacterReferences", '<!DOCTYPE r [<!ATTLIST r c CDATA "&#x42;&#66;&lt;&gt;&apos;&quot;" d CDATA \'&#9;x\'>]><r\n  a="1"\n/>'],
    ["attlistUnboundInEntity", '<!DOCTYPE r [<!ENTITY e "<s/>"><!ATTLIST s p:x CDATA "v">]><r>&e;</r>'],
    // defaulted namespace declarations bind like specified ones
    ["attlistNamespaceDeclarations", '<!DOCTYPE r [<!ATTLIST r xmlns:q CDATA "urn:q" xmlns CDATA "urn:d">]><r><q:c/></r>'],
    ["attlistNamespaceOverride", '<!DOCTYPE r [<!ATTLIST s xmlns:q CDATA "urn:q" xmlns CDATA "urn:d" q:a CDATA "1">]><r><s xmlns:q="urn:v"><q:c/></s><s/><s xmlns=""/></r>'],
    ["attlistNamespaceEscapes", "<!DOCTYPE r [<!ENTITY e 'E'><!ATTLIST r xmlns:q CDATA 'urn:&quot;&lt;&amp;&e;'>]><r q:a=\"1\"/>"],
    ["attlistNamespaceErrorColumn", '<!DOCTYPE r [<!ATTLIST r xmlns:q CDATA "urn:q">]><r><r><q:c/></r><x></r>'],
    ["attlistNamespaceInEntity", "<!DOCTYPE r [<!ATTLIST s xmlns:q CDATA 'urn:q' xmlns CDATA 'urn:d'><!ENTITY e \"<s><q:c/></s>\"><!ENTITY f '<s xmlns:q=\"v\"><q:c/></s>'><!ENTITY g '<x><s/>&e;</x>'>]><r>&e;&f;&g;</r>"],
    ["attlistNamespaceLaughs", '<!DOCTYPE r [<!ATTLIST s xmlns:q CDATA "urn:q"><!ENTITY l0 "<s/>"><!ENTITY l1 "&l0;&l0;&l0;&l0;&l0;&l0;&l0;&l0;&l0;&l0;"><!ENTITY l2 "&l1;&l1;&l1;&l1;&l1;&l1;&l1;&l1;&l1;&l1;"><!ENTITY l3 "&l2;&l2;&l2;&l2;&l2;&l2;&l2;&l2;&l2;&l2;"><!ENTITY l4 "&l3;&l3;&l3;&l3;&l3;&l3;&l3;&l3;&l3;&l3;"><!ENTITY l5 "&l4;&l4;&l4;&l4;&l4;&l4;&l4;&l4;&l4;&l4;"><!ENTITY l6 "&l5;&l5;&l5;&l5;&l5;&l5;&l5;&l5;&l5;&l5;"><!ENTITY l7 "&l6;&l6;&l6;&l6;&l6;&l6;&l6;&l6;&l6;&l6;"><!ENTITY l8 "&l7;&l7;&l7;&l7;&l7;&l7;&l7;&l7;&l7;&l7;"><!ENTITY l9 "&l8;&l8;&l8;&l8;&l8;&l8;&l8;&l8;&l8;&l8;">]><r>&l9;</r>'],
    ["subsetCommentAttlist", "<!DOCTYPE r [<!-- ] > --><?pi ] > ?><!ENTITY b '>]'><!ATTLIST r c CDATA \"]\">]><r xmlns:p=\"u\"><p:s xmlns:q=\"v\"/>&b;</r>"],
    // many / nested entity references: Chrome 156's XML parser counts each reparse that
    // starts before the previous replacement text is consumed (depth 10)
    ["manyEntities", '<!DOCTYPE r [<!ENTITY e "x"><!ENTITY f "<i/>">]><r>' + "&e;&f; ".repeat(60) + "</r>"],
    ["nestedEntities9", '<!DOCTYPE r [<!ENTITY e0 "x"><!ENTITY e1 "&e0;"><!ENTITY e2 "&e1;"><!ENTITY e3 "&e2;"><!ENTITY e4 "&e3;"><!ENTITY e5 "&e4;"><!ENTITY e6 "&e5;"><!ENTITY e7 "&e6;"><!ENTITY e8 "&e7;"><!ENTITY e9 "&e8;">]><r>&e9;</r>'],
    ["nestedEntities10", '<!DOCTYPE r [<!ENTITY e0 "x"><!ENTITY e1 "&e0;"><!ENTITY e2 "&e1;"><!ENTITY e3 "&e2;"><!ENTITY e4 "&e3;"><!ENTITY e5 "&e4;"><!ENTITY e6 "&e5;"><!ENTITY e7 "&e6;"><!ENTITY e8 "&e7;"><!ENTITY e9 "&e8;"><!ENTITY e10 "&e9;">]><r>&e10;</r>'],
    ["nestedEntities30", '<!DOCTYPE r [<!ENTITY e0 "x"><!ENTITY e1 "&e0;"><!ENTITY e2 "&e1;"><!ENTITY e3 "&e2;"><!ENTITY e4 "&e3;"><!ENTITY e5 "&e4;"><!ENTITY e6 "&e5;"><!ENTITY e7 "&e6;"><!ENTITY e8 "&e7;"><!ENTITY e9 "&e8;"><!ENTITY e10 "&e9;"><!ENTITY e11 "&e10;"><!ENTITY e12 "&e11;"><!ENTITY e13 "&e12;"><!ENTITY e14 "&e13;"><!ENTITY e15 "&e14;"><!ENTITY e16 "&e15;"><!ENTITY e17 "&e16;"><!ENTITY e18 "&e17;"><!ENTITY e19 "&e18;"><!ENTITY e20 "&e19;"><!ENTITY e21 "&e20;"><!ENTITY e22 "&e21;"><!ENTITY e23 "&e22;"><!ENTITY e24 "&e23;"><!ENTITY e25 "&e24;"><!ENTITY e26 "&e25;"><!ENTITY e27 "&e26;"><!ENTITY e28 "&e27;"><!ENTITY e29 "&e28;"><!ENTITY e30 "&e29;">]><r>&e30;</r>'],
    ["fanOutEntities", '<!DOCTYPE r [<!ENTITY a "x"><!ENTITY b1 "&a;&a;"><!ENTITY b2 "&b1;&b1;"><!ENTITY b3 "&b2;&b2;">]><r>&b2;|&b3;</r>'],
    ["billionLaughs", '<!DOCTYPE r [<!ENTITY l0 "lol"><!ENTITY l1 "&l0;&l0;&l0;&l0;&l0;&l0;&l0;&l0;&l0;&l0;"><!ENTITY l2 "&l1;&l1;&l1;&l1;&l1;&l1;&l1;&l1;&l1;&l1;"><!ENTITY l3 "&l2;&l2;&l2;&l2;&l2;&l2;&l2;&l2;&l2;&l2;"><!ENTITY l4 "&l3;&l3;&l3;&l3;&l3;&l3;&l3;&l3;&l3;&l3;"><!ENTITY l5 "&l4;&l4;&l4;&l4;&l4;&l4;&l4;&l4;&l4;&l4;"><!ENTITY l6 "&l5;&l5;&l5;&l5;&l5;&l5;&l5;&l5;&l5;&l5;"><!ENTITY l7 "&l6;&l6;&l6;&l6;&l6;&l6;&l6;&l6;&l6;&l6;"><!ENTITY l8 "&l7;&l7;&l7;&l7;&l7;&l7;&l7;&l7;&l7;&l7;"><!ENTITY l9 "&l8;&l8;&l8;&l8;&l8;&l8;&l8;&l8;&l8;&l8;">]><r>&l9;</r>'],
    // an ATTLIST default inserts a declared entity's replacement text once, verbatim
    // (character references decoded, nothing re-expanded): no amplification
    ["attlistDefaultLaughs", '<!DOCTYPE r [<!ENTITY l0 "lol">' + Array.from({ length: 9 }, (_, i) => `<!ENTITY l${i + 1} "${`&l${i};`.repeat(10)}">`).join("") + '<!ATTLIST r a CDATA "&l9;">]><r/>'],
    ["attlistDefaultSelfReference", '<!DOCTYPE r [<!ENTITY e "&e;&e;"><!ATTLIST r a CDATA "&e;">]><r/>'],
    ["attlistNamespaceDefaultLaughs", '<!DOCTYPE r [<!ENTITY l0 "lol">' + Array.from({ length: 9 }, (_, i) => `<!ENTITY l${i + 1} "${`&l${i};`.repeat(10)}">`).join("") + '<!ATTLIST r xmlns:p CDATA "&l9;">]><r/>'],
    ["attlistNamespaceDefaultSelfReference", '<!DOCTYPE r [<!ENTITY e "&e;&e;"><!ATTLIST r xmlns:p CDATA "&e;">]><r><p:c/></r>'],
    ["attlistDefaultReplacementText", '<!DOCTYPE r [<!ENTITY e0 "x"><!ENTITY e1 "&e0;"><!ENTITY c "&#65;&#x42;&lt;x&amp;y&#9;z&zz;"><!ENTITY w "a\tb\nc"><!ATTLIST r a CDATA "&e1;|&c;|&w;|\t">]><r/>'],
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
    cdataInHtml: (() => {
      const div = document.createElement("div");
      div.append("t", document.adoptNode(doc.createCDATASection("a<b&")));
      const clone = div.cloneNode(true);
      return [div.innerHTML, div.textContent, clone.lastChild.nodeType, clone.lastChild.data];
    })(),
    cdataEdits: (() => {
      const c = doc.createCDATASection("abc");
      c.data = "x<y";
      const before = c.textContent;
      c.textContent = "z";
      c.appendData("!");
      const e = doc.createElement("m");
      e.appendChild(doc.createCDATASection(""));
      const empty = e.matches(":empty");
      e.firstChild.data = "q";
      return [before, c.data, c.nodeValue, c.length, empty, e.matches(":empty"), e.textContent];
    })(),
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
