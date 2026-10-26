(() => {
  const create = (text, attributes = {}) => {
    const element = document.createElement("style");
    element.textContent = text;
    for (const [name, value] of Object.entries(attributes))
      element.setAttribute(name, value);
    document.head.append(element);
    return element;
  };
  const style = create(".real {color:red}");
  let reads = 0;
  Object.defineProperty(style, "textContent", {
    configurable: true,
    get() {
      reads++;
      return ".fake {color:blue}";
    },
  });
  const original = style.sheet;
  original.insertRule(".inserted {width:20px}", 1);
  const retained = style.sheet === original;
  const first = original.cssRules[0].selectorText;
  const setter = Object.getOwnPropertyDescriptor(
    Node.prototype,
    "textContent",
  ).set;
  setter.call(style, ".updated {height:30px}");
  const text = {
    reads,
    retained,
    first,
    changed: style.sheet !== original,
    current: style.sheet.cssRules[0].selectorText,
    oldCount: original.cssRules.length,
  };
  style.remove();

  const attributes = create(".media {width:10px}", {
    media: "print",
    title: "real",
  });
  Object.defineProperty(attributes, "getAttribute", {
    value() {
      reads++;
      throw Error("public attribute hook");
    },
  });
  const sheet = attributes.sheet;
  const before = { media: sheet.media.mediaText, title: sheet.title };
  attributes.setAttribute("media", "screen and (min-width: 10px)");
  attributes.setAttribute("title", "updated");
  sheet.media.appendMedium("print");
  sheet.disabled = true;
  const media = {
    before,
    media: sheet.media.mediaText,
    title: sheet.title,
    disabled: sheet.disabled,
    reads,
  };
  attributes.remove();

  const groups = create(
    "@media screen { .inside {color:red} } @font-face {font-family:test;src:url(font.woff2)}",
  );
  const rules = groups.sheet.cssRules;
  rules[0].cssRules[0].style.color = "blue";
  rules[1].style.setProperty("font-family", "updated");
  const declarations = {
    type: rules[0].type,
    selector: rules[0].cssRules[0].selectorText,
    color: rules[0].cssRules[0].style.color,
    fontType: rules[1].type,
    fontFamily: rules[1].style.getPropertyValue("font-family"),
    fontText: rules[1].cssText,
  };
  groups.remove();
  const validation = create(
    "@font-face {font-family:test;src:url(real.woff2);font-display:swap;size-adjust:120%;ascent-override:90%;font-weight:100 900;width:30px}",
  );
  const descriptor = validation.sheet.cssRules[0].style;
  descriptor.setProperty("src", "garbage");
  descriptor.setProperty("size-adjust", "-20%");
  descriptor.setProperty("font-display", "unsupported");
  descriptor.setProperty("color", "blue");
  const descriptors = {
    src: descriptor.getPropertyValue("src"),
    display: descriptor.getPropertyValue("font-display"),
    size: descriptor.getPropertyValue("size-adjust"),
    ascent: descriptor.getPropertyValue("ascent-override"),
    weight: descriptor.getPropertyValue("font-weight"),
    width: descriptor.getPropertyValue("width"),
    color: descriptor.getPropertyValue("color"),
  };
  descriptor.cssText =
    "font-family:secondary;src:url(next.woff2);width:18px;color:red;font-display:swap";
  const cssTextParse = {
    family: descriptor.getPropertyValue("font-family"),
    src: descriptor.getPropertyValue("src"),
    width: descriptor.getPropertyValue("width"),
    color: descriptor.getPropertyValue("color"),
    display: descriptor.getPropertyValue("font-display"),
  };
  validation.remove();
  return { text, media, declarations, descriptors, cssTextParse };
})();
