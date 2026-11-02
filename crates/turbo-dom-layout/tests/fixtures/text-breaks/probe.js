// turbo-dom#212 (intra-word breaks), #213 (soft hyphens / hyphens:auto) and #214 (Greek
// polytonic uppercase): per case, the box height, and each line's text and width, from
// the text node's per-character client rects (each character's last rect, so a character
// right after a break counts on its own line; lines grouped by top edge).
(() => {
  const cases = [
    ["break-word", "width:50px;overflow-wrap:break-word", "aaaaaaaaaaaaaaaaaaaa"],
    ["anywhere", "width:50px;overflow-wrap:anywhere", "aaaaaaaaaaaaaaaaaaaa"],
    ["break-all", "width:50px;word-break:break-all", "aaaaaaaaaaaaaaaaaaaa"],
    ["break-word-words", "width:60px;overflow-wrap:break-word", "ab cdefghijklmnop qr"],
    ["break-all-words", "width:60px;word-break:break-all", "ab cdefghijklmnop qr"],
    ["normal-overflows", "width:50px", "aaaaaaaaaaaaaaaaaaaa"],
    ["keep-all", "width:50px;word-break:keep-all;overflow-wrap:normal", "aaaaaaaaaaaaaaaaaaaa"],
    ["shy", "width:40px", "ab­cdefgh"],
    ["shy-fits", "width:200px", "ab­cdefgh"],
    ["shy-two", "width:40px", "ab­cd­ef­gh"],
    ["shy-none", "width:40px;hyphens:none", "ab­cdefgh"],
    ["shy-words", "width:60px", "xy ab­cdef"],
  ];
  const out = [];
  for (const [id, style, text] of cases) {
    const div = document.createElement("div");
    div.setAttribute("style", `font-family:Inter;font-size:16px;line-height:20px;${style}`);
    div.lang = "en";
    div.textContent = text;
    document.body.append(div);
    const node = div.firstChild;
    const lines = [];
    for (let i = 0; i < text.length; i++) {
      const r = document.createRange();
      r.setStart(node, i);
      r.setEnd(node, i + 1);
      const rects = r.getClientRects();
      const rect = rects[rects.length - 1];
      if (!rect) continue;
      let line = lines.find((l) => l.top === rect.top);
      if (!line) lines.push((line = { top: rect.top, text: "", left: rect.left, right: rect.right }));
      line.text += text[i];
      line.right = Math.max(line.right, rect.right);
    }
    out.push({
      id,
      style,
      text,
      height: div.getBoundingClientRect().height,
      lines: lines.map((l) => ({ text: l.text, width: l.right - l.left })),
    });
    div.remove();
  }
  const greek = [];
  for (const source of ["ἀρχή", "ὁδός", "ᾄδω", "Ἀθῆναι", "ῥήτωρ", "ᾳ ῳ ῃ", "ἄι", "ΐ ῒ", "Ἑλλάς", "ᾰᾱᾲᾴᾷᾶᾺᾼ ῂῄῇῃῆῈῊῌ ῐῑῒῖῘ ῠῡῢῤῥῦῨῬ ῲῴῷῳῶῸῺῼ", "ὐ ὑ ἐ ὀ ὰ ἄυ ὤι ῥι ᾿ \u1f16"]) {
    const p = document.createElement("p");
    p.lang = "el";
    p.style.textTransform = "uppercase";
    p.textContent = source;
    document.body.append(p);
    greek.push({ source, upper: p.innerText });
    p.remove();
  }
  return JSON.stringify({ breaks: out, greek });
})();
