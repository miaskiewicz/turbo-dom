// innerHTML / outerHTML on nodes of XML documents (turbo-dom#186 follow-up): getters use
// the XML serialization; the setters parse an XML fragment in the context element's
// namespace scope and reject ill-formed markup. The binding test replaces `parseXML`.
(() => {
  const parseXML = (source, type) => new DOMParser().parseFromString(source, type);
  const run = (fn) => { try { return fn(); } catch (error) { return [error.name, error.message]; } };
  const dump = (node) => node.nodeType === 1
    ? [node.nodeName, node.namespaceURI, node.prefix, Array.from(node.attributes, (a) => [a.name, a.namespaceURI, a.value]), Array.from(node.childNodes, dump)]
    : [node.nodeType, node.nodeValue];
  const fresh = () => parseXML('<r xmlns="urn:d" xmlns:p="urn:p"><p:a x="1"><![CDATA[c<]]><e/>t&amp;</p:a><h xmlns="http://www.w3.org/1999/xhtml"><br/><i></i></h></r>', "application/xml");
  const result = {};
  {
    const doc = fresh(), r = doc.documentElement;
    result.getters = {
      rootInner: r.innerHTML,
      rootOuter: r.outerHTML,
      childOuter: r.firstChild.outerHTML,
      childInner: r.firstChild.innerHTML,
      html: r.lastChild.outerHTML,
    };
  }
  const SETS = [
    ["plain", "<b>t</b>"],
    ["prefixFromScope", "<p:c p:y='2'/>"],
    ["declares", '<q:d xmlns:q="urn:q"/>text'],
    ["multipleRoots", "<a/><b/>"],
    ["textOnly", "a &amp; b &lt;"],
    ["cdataCommentPi", "<![CDATA[x]]><!--c--><?t d?>"],
    ["empty", ""],
    ["unclosed", "<a>"],
    ["mismatch", "<a></b>"],
    ["unboundPrefix", "<z:a/>"],
    ["undefinedEntity", "&nbsp;"],
    ["doctype", "<!DOCTYPE a><a/>"],
  ];
  result.innerSet = {};
  for (const [id, markup] of SETS) {
    const doc = fresh(), a = doc.documentElement.firstChild;
    const error = run(() => { a.innerHTML = markup; return null; });
    result.innerSet[id] = { error, tree: dump(a), inner: a.innerHTML };
  }
  result.outerSet = {};
  for (const [id, markup] of [["plain", "<b/><c/>"], ["mismatch", "<a></b>"]]) {
    const doc = fresh(), r = doc.documentElement;
    const error = run(() => { r.firstChild.outerHTML = markup; return null; });
    result.outerSet[id] = { error, tree: dump(r) };
  }
  return result;
})()
