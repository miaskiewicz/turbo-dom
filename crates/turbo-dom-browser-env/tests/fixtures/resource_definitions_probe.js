(() => {
  const holder = document.createElement("main");
  document.body.append(holder);
  const style = document.createElement("style");
  document.head.append(style);
  const font = "@font-face{font-family:'Unused Face';src:url('/unused-face.woff2')}";
  const keyframes = "@keyframes Spin{from{width:7px}50%{width:8px}to{width:9px}}";
  const cases = [
    font,
    keyframes,
    font + keyframes,
    font + ".missing{font-family:'Unused Face'}",
    font + "#target{font-family:'Unused Face'}#target{font-family:serif}",
    keyframes + "#target{animation:Spin 1s}#target{animation-name:none}",
    keyframes + "#target{animation-name:spin}",
    font + "#hidden{display:none;font-family:'Unused Face'}",
    "@supports (unknown:1){" + font + "}#target{font-family:'Unused Face'}",
    "@font-face{font-family:'Absent Source'}#target{font-family:'Absent Source'}",
  ];
  const result = cases.map((css) => {
    holder.innerHTML = "<div id='target'></div><div id='hidden'><span>hidden</span></div>";
    style.textContent = "#target{width:43px;height:17px}" + css;
    const node = holder.querySelector("#target"),
      rect = node.getBoundingClientRect();
    return [rect.width, rect.height];
  });
  style.textContent = font;
  const host = document.createElement("div");
  host.innerHTML = "<span style=\"font-family:'Unused Face'\">unassigned</span>";
  document.body.append(host);
  const shadow = host.attachShadow({ mode: "closed" });
  shadow.innerHTML = "<div style='width:43px;height:17px'></div>";
  const rect = shadow.firstChild.getBoundingClientRect();
  result.push([rect.width, rect.height]);
  host.remove();
  style.remove();
  holder.remove();
  return result;
})();
