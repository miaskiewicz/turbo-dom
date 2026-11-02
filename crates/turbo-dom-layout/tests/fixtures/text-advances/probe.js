// turbo-dom#191: text run advances (1/64 px LayoutUnit snapping, GPOS kerning and mark
// positioning) and canvas measureText face matching, with the repo's fixture fonts.
// Per case: each span's rect plus Range client rects over every prefix of its text node
// and a middle slice; `range` is one Range across every span of the case (mixed-font
// ranges cross span boundaries). Canvas rows record measureText widths per weight/style.
(() => {
  const rect = (r) => [r.x, r.y, r.width, r.height];
  const families = ["'Liberation Serif'", "Inter", "'Fira Code'"];
  const strings = ["Typography", "AVATAR Wave", "To fi ffi", "éäo", "Hello, World!"];
  const cases = [];
  for (const family of families)
    for (const size of [13, 16, 18.72, 21.5])
      for (const weight of [400, 700])
        for (const text of strings)
          cases.push({
            style: `font-family:${family};font-size:${size}px;font-weight:${weight}`,
            html: [[text, ""]],
          });
  const mixed = [
    [
      ["Hello ", "font-family:Inter"],
      ["World", "font-family:'Liberation Serif'"],
      [" AV", "font-family:'Fira Code'"],
    ],
    [
      ["Typo", "font-family:'Liberation Serif';font-size:17px"],
      ["graphy", "font-family:Inter;font-size:13px;font-weight:700"],
    ],
    [
      ["To", "font-family:Inter"],
      ["Wa", "font-family:Inter;font-weight:700"],
      ["ve", "font-family:'Liberation Serif';font-style:italic"],
    ],
  ];
  for (const parts of mixed) cases.push({ style: "font-family:Inter;font-size:16px", html: parts });
  const out = [];
  for (const { style, html } of cases) {
    const div = document.createElement("div");
    div.setAttribute("style", `width:700px;line-height:24px;${style}`);
    for (const [text, css] of html) {
      const span = document.createElement("span");
      span.setAttribute("style", css);
      span.textContent = text;
      div.append(span);
    }
    document.body.append(div);
    const spans = [...div.children].map((span) => {
      const node = span.firstChild;
      const n = node.length;
      const offsets = [];
      for (let end = 1; end <= n; end++) offsets.push([0, end]);
      if (n > 2) offsets.push([1, n - 1]);
      const ranges = offsets.map(([start, end]) => {
        const r = document.createRange();
        r.setStart(node, start);
        r.setEnd(node, end);
        return { start, end, rects: Array.from(r.getClientRects(), rect) };
      });
      return { rect: rect(span.getBoundingClientRect()), ranges };
    });
    const whole = document.createRange();
    whole.setStart(div.firstChild.firstChild, 0);
    whole.setEnd(div.lastChild.firstChild, div.lastChild.firstChild.length);
    out.push({ style, html, spans, range: rect(whole.getBoundingClientRect()) });
    div.remove();
  }
  const ctx = document.createElement("canvas").getContext("2d");
  const canvas = [];
  for (const family of families)
    for (const weight of [400, 700])
      for (const fstyle of ["normal", "italic"])
        for (const text of ["Typography", "AVATAR Wave", "Hello, World!"]) {
          ctx.font = `${fstyle} ${weight} 16px ${family}`;
          canvas.push({ font: ctx.font, text, width: ctx.measureText(text).width });
        }
  // Fractional sizes: Blink's font cache builds the font at unsigned(size * 100) / 100
  // (turbo-dom#236), so 18.72px measures as 18.71px and 17.3px as 17.29px.
  for (const family of families)
    for (const size of [17.3, 18.72])
      for (const text of ["Typography", "AVATAR Wave"]) {
        ctx.font = `${size}px ${family}`;
        canvas.push({ font: ctx.font, text, width: ctx.measureText(text).width });
      }
  // A kerning pair between a word's last glyph and the following space (Liberation Serif
  // `T`/`Y`/`V` + U+0020) when the line breaks at that space (turbo-dom#236): the word's
  // Range rect, on one line (width 700px) and broken after the word (width 1px).
  const breaks = [];
  for (const word of ["T", "Y", "V", "LT", "AVATAR"])
    for (const width of [700, 1]) {
      const div = document.createElement("div");
      div.setAttribute("style", `width:${width}px;line-height:24px;font-family:'Liberation Serif';font-size:48px`);
      const span = document.createElement("span");
      span.textContent = `${word} Wave`;
      div.append(span);
      document.body.append(div);
      const r = document.createRange();
      r.setStart(span.firstChild, 0);
      r.setEnd(span.firstChild, word.length);
      breaks.push({ word, width, rects: Array.from(r.getClientRects(), rect) });
      div.remove();
    }
  return { cases: out, canvas, breaks };
})();
