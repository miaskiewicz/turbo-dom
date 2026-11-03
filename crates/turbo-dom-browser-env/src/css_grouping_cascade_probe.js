(() => {
  // Style rules nested in grouping rules (turbo-dom#273): getComputedStyle applies the
  // children of @supports (with its condition), @layer (cascade layer order: unlayered rules
  // beat layered ones for normal declarations, the reverse for !important), @media and @scope,
  // and never @container (no container here) or @starting-style (steady state). Every row's
  // competing rules share one specificity, so only the grouping rules decide the winner.
  const ids = ["s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s10", "s11",
    "l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9", "l10", "l11",
    "m1", "m2", "m3", "k1", "k2", "t1", "x1", "x2"];
  document.body.innerHTML =
    '<a style="border-color:navy"></a><u id="sib">u</u>' +
    ids.map((id) => `<i id="${id}">${id}</i>`).join("") +
    '<i id="l12" style="color: rgb(12, 12, 12) !important">l12</i>' +
    '<i id="l13" style="color: rgb(13, 13, 13)">l13</i>' +
    '<div class="sa" id="c0"><i id="c1">c1</i><div class="sb"><i id="c2">c2</i></div></div><i id="c3">c3</i>' +
    '<div id="c5"><style>@scope { b { color: rgb(0, 15, 0) } }</style><b id="c6">c6</b></div><b id="c7">c7</b>';
  const style = document.createElement("style");
  style.textContent = [
    "@supports (color: red) { #s1 { color: rgb(0, 9, 0) } }",
    "@supports (colr: red) { #s2 { color: rgb(2, 0, 0) } }",
    "@supports not (colr: red) { #s3 { color: rgb(0, 3, 0) } }",
    "@supports (color: red) and (display: grid) { #s4 { color: rgb(0, 4, 0) } }",
    "@supports (colr: red) or (display: flex) { #s5 { color: rgb(0, 5, 0) } }",
    "@supports (display: nonsense) { #s6 { color: rgb(6, 0, 0) } }",
    "@supports selector(:has(a)) { #s7 { color: rgb(0, 7, 0) } }",
    "@supports selector(:frobnicate) { #s8 { color: rgb(8, 0, 0) } }",
    '@supports (color: red) { @layer x { [style*="navy"] ~ #sib { color: rgb(0, 0, 128) } } }',
    "@supports (color: red) { @supports (colr: x) { #s10 { color: rgb(10, 0, 0) } } }",
    "@supports (color: red) and ((colr: x) or (not (colr: y))) { #s11 { color: rgb(0, 11, 0) } }",
    "@layer a, b;",
    "@layer b { #l1 { color: rgb(0, 0, 1) } }",
    "@layer a { #l1 { color: rgb(0, 0, 2) } }",
    "#l2 { color: rgb(0, 0, 3) }",
    "@layer a { #l2 { color: rgb(0, 0, 4) } }",
    "@layer a { #l3 { color: rgb(0, 0, 5) !important } }",
    "#l3 { color: rgb(0, 0, 6) !important }",
    "@layer a { #l4 { color: rgb(0, 0, 7) !important } }",
    "@layer b { #l4 { color: rgb(0, 0, 8) !important } }",
    "@layer c { @layer d { #l5 { color: rgb(0, 0, 9) } } #l5 { color: rgb(0, 0, 10) } }",
    "@layer c { #l6 { color: rgb(0, 0, 11) } }",
    "@layer c.d { #l6 { color: rgb(0, 0, 12) } }",
    "@layer { #l7 { color: rgb(0, 0, 13) } }",
    "@layer { #l7 { color: rgb(0, 0, 14) } }",
    "@layer f, e;",
    "@layer e { #l8 { color: rgb(0, 0, 15) } }",
    "@layer f { #l8 { color: rgb(0, 0, 16) } }",
    "#l9 { color: rgb(0, 0, 17) !important }",
    "@layer a { #l9 { color: rgb(0, 0, 18) } }",
    "@layer a { #l10 { color: rgb(0, 0, 19) !important } }",
    "#l10 { color: rgb(0, 0, 20) }",
    "@layer c { #l11 { color: rgb(0, 0, 21) !important } }",
    "@layer c.d { #l11 { color: rgb(0, 0, 22) !important } }",
    "@layer a { #l12 { color: rgb(0, 0, 23) !important } }",
    "@layer a { #l13 { color: rgb(0, 0, 24) !important } }",
    "@media all { #m1 { color: rgb(0, 20, 0) } }",
    "@media not all { #m2 { color: rgb(20, 0, 0) } }",
    "@media all { @supports (color: red) { @layer z { #m3 { color: rgb(0, 21, 0) } } } }",
    "@container (min-width: 0px) { #k1 { color: rgb(30, 0, 0) } }",
    "@container card (min-width: 0px) { #k2 { color: rgb(31, 0, 0) } }",
    "@starting-style { #t1 { color: rgb(40, 0, 0) } }",
    "@scope (.sa) to (.sb) { i { color: rgb(0, 14, 0) } }",
    "@scope (.sa) { :scope { color: rgb(0, 16, 0) } }",
    "@supports (colr: x) { #x1 { color: rgb(50, 0, 0) } } #x1 { color: rgb(0, 50, 0) }",
  ].join("\n");
  document.head.append(style);
  const rows = [];
  const read = (id) => rows.push([id, getComputedStyle(document.getElementById(id)).color]);
  for (const id of ["s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "sib", "s10", "s11",
    "l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9", "l10", "l11", "l12", "l13",
    "m1", "m2", "m3", "k1", "k2", "t1", "c0", "c1", "c2", "c3", "c6", "c7", "x1"]) read(id);
  // CSSOM edits inside grouping rules reach the cascade.
  const sheet = style.sheet;
  const supports = [...sheet.cssRules].find((rule) => rule.cssText.startsWith("@supports (color: red) {\n  #s1"));
  supports.insertRule("#x2 { color: rgb(0, 60, 0) }", 1);
  read("x2");
  supports.deleteRule(1);
  read("x2");
  const layerB = [...sheet.cssRules].find((rule) => rule.name === "b");
  layerB.cssRules[0].style.color = "rgb(0, 0, 99)";
  read("l1");
  return rows;
})();
