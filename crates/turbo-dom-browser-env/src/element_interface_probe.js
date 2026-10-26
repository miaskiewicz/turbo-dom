(() => {
  const html = "http://www.w3.org/1999/xhtml";
  const svg = "http://www.w3.org/2000/svg";
  const documents = [
    ["active-html", document],
    ["inert-html", document.implementation.createHTMLDocument("interfaces")],
    ["xml", document.implementation.createDocument(null, "root")],
  ];
  const rows = [];
  for (const [kind, owner] of documents) {
    for (const name of [
      "input",
      "INPUT",
      "Input",
      "p:input",
      "script",
      "SCRIPT",
      "div",
      "unknown",
      "x-custom",
      "a-b",
      "annotation-xml",
      "image",
      "b",
      "acronym",
      "isindex",
      "listing",
      "plaintext",
      "search",
      "svg",
      "SVG",
      "p:svg",
      "x-U",
      "a-é",
      "a-😀",
      "color-profile",
      "font-face",
      "missing-glyph",
      "applet",
      "bgsound",
      "blink",
      "keygen",
      "multicol",
      "nextid",
      "spacer",
      "basefont",
      "big",
      "center",
      "nobr",
      "noembed",
      "noframes",
      "rb",
      "rtc",
      "strike",
      "tt",
      "xmp",
      "address",
      "article",
      "aside",
      "abbr",
      "bdi",
      "bdo",
      "code",
      "dd",
      "dfn",
      "dt",
      "em",
      "figcaption",
      "figure",
      "footer",
      "header",
      "hgroup",
      "i",
      "kbd",
      "main",
      "mark",
      "nav",
      "noscript",
      "rp",
      "rt",
      "ruby",
      "s",
      "samp",
      "section",
      "small",
      "strong",
      "sub",
      "summary",
      "sup",
      "u",
      "var",
      "wbr",
    ]) {
      for (const namespace of [html, null, svg]) {
        if (name.includes(":") && namespace === null) continue;
        const node = owner.createElementNS(namespace, name);
        const asyncGetter = Object.getOwnPropertyDescriptor(
          HTMLScriptElement.prototype,
          "async",
        ).get;
        let scriptAsync;
        try {
          scriptAsync = asyncGetter.call(node);
        } catch (error) {
          scriptAsync = error.name;
        }
        let inputType;
        try {
          inputType = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "type").get.call(
            node,
          );
        } catch (error) {
          inputType = error.name;
        }
        rows.push({
          kind,
          namespace,
          name,
          localName: node.localName,
          prefix: node.prefix,
          tagName: node.tagName,
          constructor: Object.getPrototypeOf(node).constructor.name,
          html: node instanceof HTMLElement,
          input: node instanceof HTMLInputElement,
          unknown: node instanceof HTMLUnknownElement,
          scriptAsync,
          inputType,
        });
      }
    }
  }
  return rows;
})();
