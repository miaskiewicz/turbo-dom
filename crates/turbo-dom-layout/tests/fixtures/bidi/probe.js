// miaskiewicz/turbo-dom#238: UAX #9 bidi inline layout, captured with the repo's fixture
// fonts (Fira Code for Latin and digits, Noto Sans Hebrew / Noto Sans Arabic for RTL
// scripts). Every case lays `html` out inside `#box` and records, at device scale 2:
//   elements  getClientRects() of every [id] element inside the box (document order);
//   texts     per text node (child-index path from #box), the client rect of every UTF-16
//             code unit's Range (empty when it has none);
//   carets    caretPositionFromPoint() along the middle of each case's first line, as
//             [x, y, text path or null, offset].
// Asserted by crates/turbo-dom-layout/tests/bidi_chrome.rs.
(() => {
  const BOX = "font-family:'Fira Code','Noto Sans Hebrew','Noto Sans Arabic';font-size:16px;line-height:24px";
  const HE = "אבג"; // alef bet gimel
  const HE2 = "דהו"; // dalet he vav
  const HE3 = "זחט"; // zayin het tet
  const AR = "مرحبا"; // marhaba
  const AR2 = "بالعالم"; // bil-alam
  const cases = [
    // Paragraph base direction and mixed runs.
    ["ltr-hebrew-word", "ltr", 300, `abc ${HE} def`],
    ["rtl-latin-word", "rtl", 300, `${HE} abc ${HE2}`],
    ["ltr-two-hebrew-words", "ltr", 300, `abc ${HE} ${HE2} def`],
    ["mixed-in-word", "ltr", 300, `abc${HE}def`],
    ["rtl-only", "rtl", 300, `${HE} ${HE2}`],
    // Numbers (EN / AN) and neutrals.
    ["rtl-number", "rtl", 300, `${HE} 123 ${HE2}`],
    ["rtl-decimal-percent", "rtl", 300, `${HE} 12.5% ${HE2}`],
    ["ltr-hebrew-number", "ltr", 300, `abc ${HE} 45 def`],
    ["arabic-indic-digits", "rtl", 300, `${AR} ١٢٣ abc`],
    ["rtl-trailing-punct", "rtl", 300, `${HE}!`],
    ["ltr-hebrew-punct", "ltr", 300, `abc ${HE}! def`],
    ["rtl-parens", "rtl", 300, `${HE} (abc) ${HE2}`],
    ["rtl-hebrew-parens", "rtl", 300, `(${HE})`],
    // Arabic shaping and joining.
    ["arabic", "rtl", 300, `${AR} ${AR2}`],
    ["arabic-in-ltr", "ltr", 300, `abc ${AR} def`],
    // Inline boxes: unicode-bidi on the box.
    ["span-embed-rtl", "ltr", 300, `abc <span id="s" style="direction:rtl;unicode-bidi:embed">def ${HE}</span> ghi`],
    ["span-isolate-rtl", "ltr", 300, `abc <span id="s" style="direction:rtl;unicode-bidi:isolate">def ${HE}</span> ghi`],
    ["span-dir-attr", "ltr", 300, `abc <span id="s" dir="rtl">def ${HE}</span> ghi`],
    ["span-override-rtl", "ltr", 300, `abc <span id="s" style="direction:rtl;unicode-bidi:bidi-override">def</span> ghi`],
    ["span-isolate-override", "ltr", 300, `abc <span id="s" style="direction:rtl;unicode-bidi:isolate-override">def</span> ghi`],
    ["span-override-ltr-in-rtl", "rtl", 300, `${HE} <span id="s" style="direction:ltr;unicode-bidi:bidi-override">${HE2}</span> ${HE3}`],
    ["span-plaintext", "ltr", 300, `abc <span id="s" style="unicode-bidi:plaintext">${HE} def</span> ghi`],
    ["span-embed-ltr-in-rtl", "rtl", 300, `${HE} <span id="s" style="direction:ltr;unicode-bidi:embed">abc ${HE2}</span> ${HE3}`],
    ["span-normal-rtl", "ltr", 300, `abc <span id="s" style="direction:rtl">def</span> ghi`],
    ["span-hebrew-in-ltr", "ltr", 300, `abc <span id="s">${HE}</span> def`],
    // Inline-box borders and padding on the visual sides.
    [
      "span-edges-rtl",
      "ltr",
      300,
      `abc <span id="s" style="direction:rtl;unicode-bidi:embed;padding:0 3px 0 7px;border-left:2px solid;margin-right:5px">def ${HE}</span> ghi`,
    ],
    [
      "span-edges-ltr-in-rtl",
      "rtl",
      300,
      `${HE} <span id="s" style="direction:ltr;unicode-bidi:isolate;padding:0 3px 0 7px;border-right:2px solid">abc</span> ${HE2}`,
    ],
    ["span-edges-hebrew", "ltr", 300, `abc <span id="s" style="padding:0 4px 0 9px">${HE}</span> def`],
    ["span-edges-rtl-para", "rtl", 300, `${HE} <span id="s" style="padding:0 4px 0 9px">${HE2}</span> ${HE3}`],
    // Atomic inlines are neutral.
    ["rtl-atom", "rtl", 300, `${HE} <span id="a" style="display:inline-block;width:12px;height:8px"></span> abc`],
    // Line breaking in logical order, reordering per line.
    ["rtl-wrap", "rtl", 90, `${HE} ${HE2} ${HE3} abc def ${HE}`],
    ["ltr-wrap", "ltr", 90, `abc ${HE} ${HE2} def ${HE3} ghi`],
    ["rtl-wrap-span", "rtl", 90, `${HE} <span id="s" style="border:1px solid">abc ${HE2} def</span> ${HE3}`],
    // Block-level unicode-bidi: plaintext (dir=auto) takes its direction per paragraph.
    ["block-plaintext-hebrew", "auto", 300, `${HE} abc`],
    ["block-plaintext-latin", "auto", 300, `abc ${HE}`],
    ["block-plaintext-style", "ltr", 300, `<div id="p" style="unicode-bidi:plaintext">${HE} abc</div>`],
  ];
  const rect = (r) => [r.x, r.y, r.width, r.height];
  const host = document.createElement("div");
  document.body.style.margin = "0";
  document.body.append(host);
  const out = [];
  for (const [id, dir, width, html] of cases) {
    host.innerHTML = `<div id="box" dir="${dir}" style="${BOX};width:${width}px">${html}</div>`;
    const box = host.firstChild;
    const elements = [...box.querySelectorAll("[id]")].map((el) => ({
      id: el.id,
      rects: [...el.getClientRects()].map(rect),
    }));
    const texts = [];
    const walk = (node, path) => {
      [...node.childNodes].forEach((child, index) => {
        const here = [...path, index];
        if (child.nodeType === Node.TEXT_NODE) {
          const chars = [];
          for (let i = 0; i < child.length; i++) {
            const range = document.createRange();
            range.setStart(child, i);
            range.setEnd(child, i + 1);
            chars.push([...range.getClientRects()].map(rect));
          }
          texts.push({ path: here, text: child.data, chars });
        } else walk(child, here);
      });
    };
    walk(box, []);
    const top = box.getBoundingClientRect();
    const carets = [];
    for (let x = top.x + 1; x < top.x + Math.min(width, 260); x += 6) {
      const y = top.y + 12;
      const caret = document.caretPositionFromPoint(x, y);
      let path = null;
      if (caret && caret.offsetNode.nodeType === Node.TEXT_NODE) {
        path = [];
        for (let n = caret.offsetNode; n !== box; n = n.parentNode) path.unshift([...n.parentNode.childNodes].indexOf(n));
      }
      carets.push([x, y, path, caret ? caret.offset : null]);
    }
    out.push({ id, dir, width, html, box: rect(top), elements, texts, carets });
  }
  host.remove();
  return out;
})();
