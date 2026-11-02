// turbo-dom#236: text run advances with the macOS system Times faces (host fonts, not the
// fixture fonts): the faces carry AAT `morx`/`trak`, and Chrome on macOS takes its glyph
// advances from CoreText. Per case: Range widths over every prefix of the span's text,
// the span rect, and canvas measureText of the same string.
(() => {
  const rect = (r) => [r.x, r.y, r.width, r.height];
  const strings = ["Old result", "AVATAR Wave", "Typography", "Hello, World!"];
  const out = [];
  for (const family of ["Times"])
    for (const size of [13, 16, 18.72, 24, 48])
      for (const weight of [400, 700])
        for (const text of strings) {
          const style = `font-family:${family};font-size:${size}px;font-weight:${weight}`;
          const div = document.createElement("div");
          div.setAttribute("style", `width:1200px;line-height:normal;${style}`);
          const span = document.createElement("span");
          span.textContent = text;
          div.append(span);
          document.body.append(div);
          const node = span.firstChild;
          const ranges = [];
          for (let end = 1; end <= node.length; end++) {
            const r = document.createRange();
            r.setStart(node, 0);
            r.setEnd(node, end);
            ranges.push({ end, width: r.getBoundingClientRect().width });
          }
          const ctx = document.createElement("canvas").getContext("2d");
          ctx.font = `${weight} ${size}px ${family}`;
          out.push({ style, text, span: rect(span.getBoundingClientRect()), ranges, canvas: ctx.measureText(text).width });
          div.remove();
        }
  return out;
})();
