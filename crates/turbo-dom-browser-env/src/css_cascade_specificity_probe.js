(() => {
  // Cascade order after layers (turbo-dom#275, #276): specificity (Selectors 4; a rule's
  // specificity for an element is that of its most specific matching selector), @scope
  // proximity (fewer generations between the scoping root and the element wins), then order of
  // appearance; and how :scope / & inside an @scope rule bind to the scoping root. Every
  // selector of row N ends in [data-r=N] (the same (0,1,0) on both sides), so rows never mix.
  const el = (tag, id, attrs = "") => `<${tag} id="${id}" data-r="${id}"${attrs}>${id}</${tag}>`;
  document.body.innerHTML =
    el("i", "sp1") + el("i", "sp2", ' class="c d"') + el("i", "sp3", ' class="c"') +
    el("i", "sp4", ' class="x"') + el("i", "sp5", ' class="x"') + el("i", "sp6") +
    el("i", "sp7") + el("i", "sp8", ' class="c"') + el("i", "sp9") +
    el("i", "sp10", ' style="color: rgb(10, 10, 10)"') + el("i", "sp11") +
    el("i", "sp12", ' class="c"') + el("i", "sp13") + el("b", "sp14", ' class="c"') +
    '<div class="outer"><div class="inner">' + el("i", "p1") + "</div></div>" +
    '<div class="outer2"><div class="inner2">' + el("i", "p2", ' class="k"') + "</div></div>" +
    '<div class="o3">' + el("i", "p3") + "</div>" +
    '<div id="idroot">' + el("i", "p4", ' class="k"') + el("i", "p5", ' class="k"') + "</div>" +
    el("div", "p6", ' class="o4 m"') +
    '<div class="o5"><div class="o5"></div>' + el("i", "p7") + "</div>" +
    '<div class="o6"><span>' + el("i", "p8") + "</span></div>" +
    '<div class="o7">' + el("i", "p9") + "</div>" +
    '<div class="o8">' + el("i", "p10") + "</div>";
  const r = (id) => `[data-r="${id}"]`;
  const style = document.createElement("style");
  style.textContent = [
    `#sp1${r("sp1")} { color: rgb(0, 1, 0) } i${r("sp1")} { color: rgb(1, 0, 0) }`,
    `.c.d${r("sp2")} { color: rgb(0, 2, 0) } .c${r("sp2")} { color: rgb(2, 0, 0) }`,
    `i.c${r("sp3")} { color: rgb(0, 3, 0) } .c${r("sp3")} { color: rgb(3, 0, 0) }`,
    `#sp4${r("sp4")}, i${r("sp4")} { color: rgb(0, 4, 0) } .x${r("sp4")} { color: rgb(4, 0, 0) }`,
    `#nope${r("sp5")}, i${r("sp5")} { color: rgb(5, 0, 0) } .x${r("sp5")} { color: rgb(0, 5, 0) }`,
    `i${r("sp6")} { color: rgb(0, 6, 0) } :where(#sp6)${r("sp6")} { color: rgb(6, 0, 0) }`,
    `:is(#sp7, b)${r("sp7")} { color: rgb(0, 7, 0) } i.zz${r("sp7")}, i${r("sp7")} { color: rgb(7, 0, 0) }`,
    `#sp8${r("sp8")} { color: rgb(0, 8, 0) !important } .c${r("sp8")} { color: rgb(8, 0, 0) !important }`,
    `i:not(.zz)${r("sp9")} { color: rgb(0, 9, 0) } i${r("sp9")} { color: rgb(9, 0, 0) }`,
    `#sp10${r("sp10")} { color: rgb(10, 0, 0) }`,
    `@layer early { #sp11${r("sp11")} { color: rgb(11, 0, 0) } } @layer late { i${r("sp11")} { color: rgb(0, 11, 0) } }`,
    `:nth-child(1n of .c)${r("sp12")} { color: rgb(0, 12, 0) } .c.c${r("sp12")} { color: rgb(12, 0, 0) }`,
    `*#sp13${r("sp13")} { color: rgb(0, 13, 0) } #sp13${r("sp13")} { color: rgb(13, 0, 0) }`,
    `b.c${r("sp14")} { color: rgb(0, 14, 0) } :where(b.c.c.c)${r("sp14")} { color: rgb(14, 0, 0) }`,
    `@scope (.inner) { i${r("p1")} { color: rgb(0, 0, 1) } } @scope (.outer) { i${r("p1")} { color: rgb(0, 0, 2) } }`,
    `@scope (.inner2) { i${r("p2")} { color: rgb(0, 0, 3) } } @scope (.outer2) { i.k${r("p2")} { color: rgb(0, 0, 4) } }`,
    `@scope (.o3) { :scope > i${r("p3")} { color: rgb(0, 0, 5) } } @scope (.o3) { i${r("p3")} { color: rgb(0, 0, 6) } }`,
    `@scope (#idroot) { & > i${r("p4")} { color: rgb(0, 0, 7) } } @scope (#idroot) { :scope > i.k.k.k.k${r("p4")} { color: rgb(0, 0, 8) } }`,
    `.k${r("p5")} { color: rgb(0, 0, 9) } @scope (#idroot) { i${r("p5")} { color: rgb(0, 0, 11) } }`,
    `@scope (.o4) { :scope.m${r("p6")} { color: rgb(0, 0, 12) } }`,
    `@scope (.o5) { :scope + i${r("p7")} { color: rgb(0, 0, 13) } }`,
    `@scope (.o6) { :scope i${r("p8")} { color: rgb(0, 0, 14) } } @scope (.o6) { span > i${r("p8")} { color: rgb(0, 0, 15) } }`,
    `@scope (.o7) { & i${r("p9")} { color: rgb(0, 0, 16) } } @scope (.o7) { :scope i${r("p9")} { color: rgb(0, 0, 17) } }`,
    `@scope (.o8) { :scope i${r("p10")} { color: rgb(0, 0, 18) } } @scope (.o8) { & i${r("p10")} { color: rgb(0, 0, 19) } }`,
  ].join("\n");
  document.head.append(style);
  const ids = ["sp1", "sp2", "sp3", "sp4", "sp5", "sp6", "sp7", "sp8", "sp9", "sp10", "sp11",
    "sp12", "sp13", "sp14", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8", "p9", "p10"];
  return ids.map((id) => [id, getComputedStyle(document.getElementById(id)).color]);
})();
