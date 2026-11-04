// Subtree-scoped style invalidation control (turbo-dom#279): every selector class whose
// matching a DOM mutation can change, with class / attribute / structural mutations
// interleaved with reads. After each step every element of the case is read
// (`color` — inherited — and `margin-left`), so a reused computed style that a mutation
// should have dropped shows up as a stale value. Recorded in Chrome 156 by
// scripts/style-invalidation-chrome-control.mjs; asserted by style_invalidation_tests.rs.
(() => {
  const cases = [
    {
      id: "descendant",
      css: ".on .t{margin-left:1px} .on{color:rgb(200, 0, 0)} .x .y .t{margin-left:2px}",
      html: "<div id=a><div id=b><p id=t class=t>t</p></div></div><div id=c><p id=u class=t>u</p></div>",
      steps: [
        ($) => $.a.classList.add("on"),
        ($) => $.c.classList.add("on"),
        ($) => $.a.classList.remove("on"),
        ($) => $.c.append($.t),
        ($) => { $.a.className = "x"; $.b.className = "y"; $.b.append($.u); },
        ($) => $.b.classList.remove("y"),
      ],
    },
    {
      id: "child",
      css: ".on > .t{margin-left:3px} .on > *{color:rgb(0, 100, 0)}",
      html: "<div id=a><p id=t class=t><span id=s class=t>s</span></p></div>",
      steps: [
        ($) => $.a.classList.add("on"),
        ($) => $.t.classList.add("on"),
        ($) => $.a.classList.remove("on"),
        ($) => $.a.append($.s),
      ],
    },
    {
      id: "adjacent",
      css: ".on + .t{margin-left:4px} .on + * .t{color:rgb(0, 0, 150)}",
      html: "<p id=a>a</p><p id=b class=t>b</p><div id=c><i id=d class=t>d</i></div>",
      steps: [
        ($) => $.a.classList.add("on"),
        ($) => $.b.classList.add("on"),
        ($) => $.a.classList.remove("on"),
        ($) => $.b.before($.c),
        ($) => $.a.remove(),
        ($) => { $.c.before($.a); $.a.classList.add("on"); },
      ],
    },
    {
      id: "general-sibling",
      css: ".on ~ .t{margin-left:5px} .on ~ div .t{color:rgb(1, 2, 3)}",
      html: "<p id=a>a</p><p id=b>b</p><p id=c class=t>c</p><div id=d><b id=e class=t>e</b></div>",
      steps: [
        ($) => $.b.classList.add("on"),
        ($) => $.c.after($.b),
        ($) => $.a.classList.add("on"),
        ($) => $.a.classList.remove("on"),
        ($) => $.d.prepend($.b),
      ],
    },
    {
      id: "nth",
      css:
        "p:nth-child(2){margin-left:6px} p:nth-last-child(1){color:rgb(10, 0, 0)} p:nth-child(odd of .k){margin-left:7px}" +
        " i:nth-of-type(2){margin-left:8px} i:nth-last-of-type(1){color:rgb(0, 20, 0)}",
      html: "<div id=r><p id=a>a</p><p id=b class=k>b</p><p id=c>c</p><i id=d>d</i><i id=e>e</i></div>",
      steps: [
        ($) => $.r.prepend(Object.assign(document.createElement("p"), { id: "n" })),
        ($) => $.a.classList.add("k"),
        ($) => $.r.firstChild.remove(),
        ($) => $.r.append($.a),
        ($) => $.d.before(document.createElement("i")),
        ($) => $.e.remove(),
        ($) => $.b.classList.remove("k"),
      ],
    },
    {
      id: "of-type",
      css:
        "span:first-of-type{margin-left:9px} span:last-of-type{color:rgb(30, 0, 0)} span:only-of-type{margin-left:10px}" +
        " b:first-of-type + span{color:rgb(0, 30, 0)}",
      html: "<div id=r><span id=a>a</span><b id=b>b</b><span id=c>c</span></div><div id=q><span id=d>d</span></div>",
      steps: [
        ($) => $.q.append($.c),
        ($) => $.r.prepend($.b),
        ($) => $.a.remove(),
        ($) => $.b.after($.a),
        ($) => $.q.prepend(document.createElement("span")),
      ],
    },
    {
      id: "has",
      css:
        ".box:has(> .k){margin-left:11px} .box:has(.k) .t{color:rgb(40, 0, 0)} p:has(+ .k){margin-left:12px}" +
        " .box:has(.deep){color:rgb(0, 40, 0)}",
      html:
        "<div id=box class=box><p id=p>p</p><p id=q class=t>q</p><div id=w><i id=i>i</i></div></div>",
      steps: [
        ($) => $.q.classList.add("k"),
        ($) => $.i.classList.add("deep"),
        ($) => $.q.classList.remove("k"),
        ($) => $.i.classList.add("k"),
        ($) => $.w.remove(),
        ($) => $.box.append($.w),
        ($) => $.i.classList.remove("deep"),
      ],
    },
    {
      id: "empty-and-child-position",
      css:
        "div:empty{margin-left:13px} p:first-child{color:rgb(50, 0, 0)} p:last-child{margin-left:14px}" +
        " p:only-child{color:rgb(0, 50, 0)} .w:empty + p{margin-left:15px}",
      html: "<div id=e class=w></div><p id=a>a</p><div id=r><p id=b>b</p></div>",
      steps: [
        ($) => $.e.append("text"),
        ($) => ($.e.textContent = ""),
        ($) => $.r.append(Object.assign(document.createElement("p"), { id: "c" })),
        ($) => $.r.lastChild.remove(),
        ($) => $.r.prepend(document.createComment("c")),
        ($) => $.r.prepend(document.createElement("p")),
        ($) => $.e.append(document.createComment("only a comment")),
        ($) => $.e.firstChild.remove(),
      ],
    },
    {
      id: "class-id-attribute",
      css:
        "#x{margin-left:16px} [data-s=\"1\"]{color:rgb(60, 0, 0)} [data-s^=a] .t{margin-left:17px}" +
        " :not(.on) > .t{color:rgb(0, 60, 0)} :is(.a, .b) .t{margin-left:18px} :where(.c) > .t{margin-left:19px}" +
        " [lang|=en] .t{color:rgb(0, 0, 60)}",
      html: "<div id=p><p id=t class=t>t</p></div><div id=q class=on><p id=u class=t>u</p></div>",
      steps: [
        ($) => $.t.setAttribute("id", "x"),
        ($) => $.p.setAttribute("data-s", "1"),
        ($) => $.p.setAttribute("data-s", "ab"),
        ($) => $.q.classList.remove("on"),
        ($) => $.q.classList.add("b"),
        ($) => { $.q.className = "c"; $.t.removeAttribute("id"); },
        ($) => $.p.setAttribute("lang", "en-GB"),
        ($) => $.p.removeAttribute("data-s"),
      ],
    },
    {
      id: "inherited",
      css: ".dark{color:rgb(70, 70, 70)} .dark .lite{color:rgb(80, 0, 0)} .lite{margin-left:20px}",
      html: "<section id=s><div id=d><p id=p><b id=b>b</b></p></div></section><aside id=a><b id=c>c</b></aside>",
      steps: [
        ($) => $.s.classList.add("dark"),
        ($) => $.p.classList.add("lite"),
        ($) => $.a.append($.d),
        ($) => $.a.classList.add("dark"),
        ($) => $.s.append($.d),
        ($) => $.d.style.setProperty("color", "rgb(90, 0, 0)"),
        ($) => $.p.classList.remove("lite"),
      ],
    },
    {
      id: "shadow",
      css: ".k{margin-left:21px}",
      html: "<div id=h><span id=s class=k>s</span><span id=r>r</span></div><p id=p>p</p>",
      setup($) {
        const root = $.h.attachShadow({ mode: "open" });
        root.innerHTML =
          "<style>:host(.on){color:rgb(100, 0, 0)} ::slotted(.k){color:rgb(0, 100, 0)} ::slotted(.on){margin-left:22px}</style><slot></slot>";
      },
      steps: [
        ($) => $.h.classList.add("on"),
        ($) => $.r.classList.add("k"),
        ($) => $.s.classList.add("on"),
        ($) => $.h.classList.remove("on"),
        ($) => $.p.append($.s),
      ],
    },
    {
      id: "container",
      css:
        ".c{container-type:inline-size} @container (min-width: 100000px){ .t{margin-left:23px} }" +
        " .on .t{color:rgb(110, 0, 0)}",
      html: "<div id=c class=c><p id=t class=t>t</p></div><div id=d><p id=u class=t>u</p></div>",
      steps: [
        ($) => $.c.classList.add("on"),
        ($) => $.d.append($.t),
        ($) => $.d.classList.add("on"),
      ],
    },
    {
      id: "form-validity",
      css:
        // Explicit control colours and fieldset margins: the UA values are not what this tests.
        "input{color:rgb(5, 5, 5)} fieldset{margin:0} form:invalid{color:rgb(130, 0, 0)} fieldset:invalid .t{margin-left:26px} input:invalid{margin-left:27px}" +
        " form:valid .t{color:rgb(0, 130, 0)} .sel:user-invalid{color:rgb(0, 0, 130)}",
      html:
        "<form id=f><fieldset id=s><input id=i><span id=t class=t>t</span></fieldset></form>" +
        "<div id=o><input id=j required form=f></div><p id=p class=t>p</p>",
      steps: [
        ($) => $.i.setAttribute("required", ""),
        ($) => $.i.setAttribute("value", "x"),
        ($) => $.j.removeAttribute("form"),
        ($) => $.s.append($.j),
        ($) => $.j.setAttribute("value", "y"),
        ($) => $.o.append(Object.assign(document.createElement("input"), { required: true })),
        ($) => $.f.append($.o),
        ($) => $.o.remove(),
        ($) => $.i.setAttribute("pattern", "z"),
        ($) => $.p.className = "sel",
      ],
    },
    {
      id: "appends-and-last",
      css:
        "li + li{margin-left:28px} li:last-child{color:rgb(140, 0, 0)} li:nth-last-child(2){margin-left:29px}" +
        " ul > :first-child ~ .k{color:rgb(0, 140, 0)}",
      html: "<ul id=u><li id=a>a</li><li id=b>b</li></ul>",
      steps: [
        ($) => $.u.append(document.createElement("li")),
        ($) => $.u.append(Object.assign(document.createElement("li"), { className: "k" })),
        ($) => $.u.lastChild.remove(),
        ($) => $.b.classList.add("k"),
        ($) => $.u.prepend(document.createElement("li")),
        ($) => $.a.remove(),
      ],
    },
    {
      id: "interleaved",
      css:
        ".a .b{color:rgb(120, 0, 0)} .a + .a{margin-left:24px} li:nth-child(3n){color:rgb(0, 120, 0)}" +
        " ul:has(> .sel) li{margin-left:25px} [aria-hidden=true]{color:rgb(0, 0, 120)}",
      html: "<ul id=u><li id=l1>1</li><li id=l2>2</li><li id=l3>3</li></ul><div id=d><span id=s>s</span></div>",
      steps: [
        ($) => { $.d.classList.add("a"); $.s.classList.add("b"); },
        ($) => $.u.insertBefore(document.createElement("li"), $.l1),
        ($) => $.l2.classList.add("sel"),
        ($) => { $.l1.classList.add("a"); $.l2.classList.add("a"); },
        ($) => $.l2.remove(),
        ($) => $.s.setAttribute("aria-hidden", "true"),
        ($) => $.u.append($.l2),
        ($) => $.d.classList.remove("a"),
      ],
    },
  ];

  const read = (host) => {
    const out = [];
    for (const el of host.querySelectorAll("*")) {
      const style = getComputedStyle(el);
      // An unset margin reads "" here (the partial cascade has no initial value for it):
      // compare authored margins only.
      out.push(`${el.id || el.localName}:${style.color}|${style.marginLeft || "0px"}`);
    }
    return out.join(" ");
  };
  const results = {};
  for (const item of cases) {
    const style = document.createElement("style");
    style.textContent = item.css;
    document.head.append(style);
    const host = document.createElement("main");
    host.innerHTML = item.html;
    document.body.append(host);
    const $ = {};
    for (const el of host.querySelectorAll("[id]")) $[el.id] = el;
    if (item.setup) item.setup($);
    const rows = [read(host)];
    for (const step of item.steps) {
      step($);
      rows.push(read(host));
    }
    results[item.id] = rows;
    host.remove();
    style.remove();
  }
  return results;
})();
