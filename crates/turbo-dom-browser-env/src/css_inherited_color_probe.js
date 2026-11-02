(() => {
  // Implicit inheritance must observe CSSOM edits: children have no authored
  // color, so every value below reaches them only through ancestors.
  const style = document.createElement("style");
  style.textContent = ".p { color: red } .q { color: blue }";
  document.head.append(style);
  const holder = document.createElement("div");
  holder.innerHTML =
    "<div class='p' id='p'><span id='c'><b id='g'>x</b></span></div>" +
    "<section class='p' id='h'><i id='slotted'>s</i></section>";
  document.body.append(holder);
  const byId = (id) => document.getElementById(id);
  const host = byId("h");
  const shadow = host.attachShadow({ mode: "open" });
  shadow.innerHTML =
    "<span id='inside'>i</span><slot></slot><em id='styled'>e</em>" +
    "<style>#styled { color: navy }</style>";
  const retained = ["p", "c", "g"].map((id) => getComputedStyle(byId(id)));
  const fresh = () => [
    byId("g"),
    shadow.getElementById("inside"),
    byId("slotted"),
    shadow.getElementById("styled"),
    holder,
  ];
  const rows = [];
  const read = (label) =>
    rows.push([
      label,
      ...retained.map((declaration) => declaration.color),
      ...fresh().map((element) => getComputedStyle(element).color),
    ]);
  const sheet = style.sheet;
  const rule = sheet.cssRules[0];
  read("initial");
  rule.style.color = "green";
  read("mutated");
  rule.style.setProperty("color", "purple", "important");
  read("important");
  rule.style.removeProperty("color");
  read("removed");
  sheet.insertRule(".p { color: orange }", sheet.cssRules.length);
  read("inserted");
  sheet.insertRule("body { color: teal }", 0);
  read("body");
  sheet.deleteRule(sheet.cssRules.length - 1);
  read("deleted");
  rule.style.color = "green";
  read("restored");
  rule.selectorText = ".q";
  read("selector");
  rule.selectorText = ".p";
  sheet.insertRule("#c { color: inherit }", sheet.cssRules.length);
  rule.style.color = "fuchsia";
  read("explicit");
  sheet.disabled = true;
  read("disabled");
  sheet.disabled = false;
  const adopted = new CSSStyleSheet();
  adopted.replaceSync(".p { color: maroon }");
  document.adoptedStyleSheets = [adopted];
  read("adopted");
  adopted.cssRules[0].style.color = "olive";
  read("adoptedMutated");
  document.adoptedStyleSheets = [];
  style.textContent = "#p { color: rgb(1, 2, 3) }";
  read("replacedText");
  holder.remove();
  style.remove();
  return rows;
})();
