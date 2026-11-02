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
    // turbo-dom#229: the same break opportunities inside and across inline spans
    // (markup cases: `text` is the markup, laid out through innerHTML).
    ["span-wrap-shy", "width:50px;overflow-wrap:break-word", "<span>wwwwww­wwwwww</span>"],
    ["span-wrap-shy-mid", "width:60px;overflow-wrap:break-word", "x <span>wwww­wwwwwwww</span> y"],
    ["span-nested", "width:40px", "ab<span>cd<b>ef­gh</b>ij</span>"],
    ["span-shy-edge-before", "width:40px", "abcd­<span>efgh</span>"],
    ["span-shy-edge-after", "width:40px", "abcd<span>­efgh</span>"],
    ["span-break-all-edge", "width:50px;word-break:break-all", "aaaa<span>aaaa</span>aaaa"],
    ["span-hyphens-none", "width:40px", "ab<span style=\"hyphens:none\">cd­ef</span>gh"],
  ];
  const out = [];
  for (const [id, style, text] of cases) {
    const div = document.createElement("div");
    div.setAttribute("style", `font-family:Inter;font-size:16px;line-height:20px;${style}`);
    div.lang = "en";
    div.innerHTML = text;
    document.body.append(div);
    const chars = [];
    const walker = document.createTreeWalker(div, NodeFilter.SHOW_TEXT);
    for (let n = walker.nextNode(); n; n = walker.nextNode())
      for (let i = 0; i < n.data.length; i++) chars.push([n, i]);
    const lines = [];
    for (const [node, i] of chars) {
      const r = document.createRange();
      r.setStart(node, i);
      r.setEnd(node, i + 1);
      const rects = r.getClientRects();
      const rect = rects[rects.length - 1];
      if (!rect) continue;
      let line = lines.find((l) => l.top === rect.top);
      if (!line) lines.push((line = { top: rect.top, text: "", left: rect.left, right: rect.right }));
      line.text += node.data[i];
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
