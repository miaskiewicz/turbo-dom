(() => {
  const holder = document.createElement("div");
  holder.innerHTML =
    "<div class='x' data-token='{,}'></div><div class='y' data-token='/*,*/'></div><div class='z' id='a,b'></div>";
  document.body.append(holder);
  const style = document.createElement("style");
  document.head.append(style);
  const selectors = [
    "[data-token='{,}']",
    "[data-token='/*,*/']",
    "#a\\,b",
    ":is(.x,.y)",
    "#a\\,b,[data-token='{,}']",
  ];
  const match = (selector) =>
    Array.from(holder.querySelectorAll(selector), (node) => node.className);
  const result = selectors.map((selector) => {
    style.textContent = selector + "{width:43px}";
    const sheet = style.sheet,
      rule = sheet.cssRules[0];
    const initial = match(rule.selectorText);
    sheet.insertRule(rule.cssText, 1);
    const roundtrip = match(sheet.cssRules[1].selectorText);
    rule.selectorText = selector;
    return {
      initial,
      roundtrip,
      assigned: match(rule.selectorText),
      width: rule.style.width,
      length: sheet.cssRules.length,
    };
  });
  style.remove();
  holder.remove();
  return result;
})();
