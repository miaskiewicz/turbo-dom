(() => {
  const output = {};
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(".fixture{width:41px}");
  const host = document.createElement("div");
  document.body.appendChild(host);
  const shadow = host.attachShadow({ mode: "open" });
  const getter = Object.getOwnPropertyDescriptor(ShadowRoot.prototype, "adoptedStyleSheets").get;
  const setter = Object.getOwnPropertyDescriptor(ShadowRoot.prototype, "adoptedStyleSheets").set;
  const before = shadow.adoptedStyleSheets;
  shadow.adoptedStyleSheets = [sheet];
  output.shadow = {
    stable: before === shadow.adoptedStyleSheets,
    sheet: shadow.adoptedStyleSheets[0] === sheet,
    array: Array.isArray(before),
    descriptor: [getter.name, setter.name, getter.length, setter.length],
  };
  const inert = new DOMParser().parseFromString("<!doctype html><html><body></body></html>", "text/html");
  output.errors = [];
  for (const operation of [
    () => { inert.adoptedStyleSheets = [sheet]; },
    () => getter.call(document),
    () => getter.call(document.createDocumentFragment()),
    () => getter.call({}),
    () => setter.call(document, [sheet]),
    () => Object.preventExtensions(shadow.adoptedStyleSheets),
    () => Object.freeze(shadow.adoptedStyleSheets),
  ]) {
    try { operation(); output.errors.push(null); }
    catch (error) { output.errors.push(error.name); }
  }
  let hooks = 0;
  Object.defineProperty(shadow, "ownerDocument", { get() { hooks++; throw Error("shadowed owner"); } });
  try { setter.call(shadow, [sheet]); output.privateOwner = getter.call(shadow)[0] === sheet && hooks === 0; }
  catch (error) { output.privateOwner = error.name; }
  output.extensible = Object.isExtensible(before);
  output.length = before.length;
  host.remove();
  return output;
})();
