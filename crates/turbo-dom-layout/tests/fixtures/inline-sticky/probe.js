// turbo-dom#218: inline `position: sticky` boxes in an unscrolled 400x300 document
// (Liberation Serif fixture, line-height 20px, body margin 0): getBoundingClientRect()
// of every [data-cid] element, as {cid: [x, y, width, height]}.
(() => {
  document.body.style.cssText = 'margin:0;font-family:"Liberation Serif";line-height:20px';
  document.body.innerHTML =
    "<div>a<span data-cid=t style='position:sticky;top:10px'>x</span></div>" +
    "<div style='height:100px'>a<span data-cid=t2 style='position:sticky;top:30px'>x</span></div>" +
    "<div style='width:200px'>a<span data-cid=l style='position:sticky;left:100px'>x</span></div>" +
    "<div style='width:200px'>a<span data-cid=l2 style='position:sticky;left:300px'>x</span></div>" +
    "<div style='width:200px'>a<span data-cid=n style='position:sticky;left:100px'>x<b data-cid=nb>y</b></span></div>" +
    "<div style='width:200px;margin-left:50px'>a<span data-cid=r style='position:sticky;right:300px'>x</span></div>" +
    "<div>a<span data-cid=s style='position:sticky'>x</span></div>";
  const out = {};
  for (const el of document.querySelectorAll("[data-cid]")) {
    const r = el.getBoundingClientRect();
    out[el.dataset.cid] = [r.x, r.y, r.width, r.height];
  }
  return out;
})();
