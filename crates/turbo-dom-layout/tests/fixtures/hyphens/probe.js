// turbo-dom#213: `hyphens: auto` dictionary hyphenation. Per case (an element `lang`, a
// style and markup laid out through innerHTML): the box height, and each line's text and
// width from the per-character client rects (each character's last rect; lines grouped by
// top edge). A drawn hyphen is generated content, so it shows only in the line width.
(() => {
  const cases = [
    ["en-words", "en", "width:90px;hyphens:auto", "responsibility characteristics typography"],
    ["en-narrow", "en", "width:50px;hyphens:auto", "characteristics performance"],
    ["en-us", "en-US", "width:70px;hyphens:auto", "computer typography beautiful"],
    ["en-us-lower", "en-us", "width:60px;hyphens:auto", "information example"],
    ["en-region", "en-CA", "width:80px;hyphens:auto", "characteristics"],
    ["en-fits", "en", "width:400px;hyphens:auto", "hyphenation extraordinary characteristics"],
    ["en-punctuation", "en", "width:80px;hyphens:auto", "(characteristics). typography,"],
    ["en-capitalized", "en", "width:60px;hyphens:auto", "Characteristics Typography"],
    ["en-span-lang", "", "width:60px;hyphens:auto", "<span lang=\"en\">typography</span> typography"],
    ["en-span-auto", "en", "width:60px", "typography <span style=\"hyphens:auto\">typography</span>"],
    ["en-span-edge", "en", "width:60px;hyphens:auto", "<span style=\"color:red\">typog</span>raphy"],
    ["en-break-word", "en", "width:40px;hyphens:auto;overflow-wrap:break-word", "characteristics"],
    ["no-lang", "", "width:60px;hyphens:auto", "characteristics typography"],
    ["unsupported-lang", "xx", "width:60px;hyphens:auto", "characteristics typography"],
    ["manual", "en", "width:60px;hyphens:manual", "characteristics typography"],
    ["manual-default", "en", "width:60px", "characteristics typography"],
    ["none", "en", "width:40px;hyphens:none", "ab­cdefgh typography"],
    ["auto-shy", "en", "width:60px;hyphens:auto", "typo­graphy characteristics"],
    ["limit-word", "en", "width:30px;hyphens:auto;hyphenate-limit-chars:6", "paper water"],
    ["limit-prefix", "en", "width:60px;hyphens:auto;hyphenate-limit-chars:auto 4", "typography"],
    ["limit-suffix", "en", "width:60px;hyphens:auto;hyphenate-limit-chars:auto auto 5", "characteristics"],
    ["limit-short", "en", "width:30px;hyphens:auto;hyphenate-limit-chars:5 1 1", "paper"],
    ["serif", "en", "width:70px;hyphens:auto;font-family:'Liberation Serif'", "characteristics development"],
  ];
  const out = [];
  for (const [id, lang, style, text] of cases) {
    const div = document.createElement("div");
    div.setAttribute("style", `font-family:Inter;font-size:16px;line-height:20px;${style}`);
    if (lang) div.lang = lang;
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
      lang,
      style,
      text,
      height: div.getBoundingClientRect().height,
      // The rect of the character before a hyphenating break includes the hyphen drawn.
      lines: lines.map((l) => ({ text: l.text, width: l.right - l.left })),
    });
    div.remove();
  }
  return JSON.stringify({ cases: out });
})();
