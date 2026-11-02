// turbo-dom#178: a line-height smaller than the font's content area (negative leading).
// Per case: the block's height and each line's top relative to the block (one span per
// word; lines grouped by top edge). Chrome honours the used line-height, so lines overlap.
(() => {
  const cases = [
    ["inter-7", "font-family:Inter;font-size:16px;line-height:7px"],
    ["inter-12", "font-family:Inter;font-size:16px;line-height:12px"],
    ["inter-0.5", "font-family:Inter;font-size:16px;line-height:0.5"],
    ["serif-7", "font-family:'Liberation Serif';font-size:16px;line-height:7px"],
    ["mono-10", "font-family:'Fira Code';font-size:16px;line-height:10px"],
    ["inter-0", "font-family:Inter;font-size:16px;line-height:0"],
  ];
  const out = [];
  for (const [id, style] of cases) {
    const div = document.createElement("div");
    div.setAttribute("style", `width:60px;${style}`);
    div.innerHTML = "<span>aa</span> <span>bb</span> <span>cc</span>";
    document.body.append(div);
    const top = div.getBoundingClientRect().top;
    const tops = [];
    for (const span of div.querySelectorAll("span")) {
      const t = span.getBoundingClientRect().top - top;
      if (!tops.includes(t)) tops.push(t);
    }
    out.push({ id, style, height: div.getBoundingClientRect().height, lineTops: tops });
    div.remove();
  }
  return out;
})();
