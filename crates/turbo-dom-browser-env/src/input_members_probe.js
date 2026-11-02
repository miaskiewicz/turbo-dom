(() => {
  const I = HTMLInputElement.prototype, T = HTMLTextAreaElement.prototype;
  const result = {};
  const shape = (value) => {
    if (typeof value === "number") return Object.is(value, -0) ? "-0" : Number.isFinite(value) ? value : String(value);
    if (value instanceof Date) return "Date:" + value.getTime();
    if (value && typeof value === "object" && value.nodeType === 1) return "element:" + (value.id || value.localName);
    return value === undefined ? "undefined" : value;
  };
  const run = (fn) => { try { return shape(fn()); } catch (error) { return [error.name, error.message]; } };
  const input = (type, attributes) => {
    const element = document.createElement("input");
    if (type !== null) element.setAttribute("type", type);
    for (const name in attributes || {}) element.setAttribute(name, attributes[name]);
    return element;
  };
  const describe = (prototype, name) => {
    const d = Object.getOwnPropertyDescriptor(prototype, name);
    if (!d) return null;
    return [d.enumerable, d.configurable, "writable" in d ? d.writable : null,
      d.get ? [d.get.name, d.get.length] : null, d.set ? [d.set.name, d.set.length] : null,
      typeof d.value === "function" ? [d.value.name, d.value.length] : null];
  };
  const inputMembers = ["align", "alt", "dirName", "maxLength", "minLength", "size", "useMap", "incremental", "webkitdirectory", "indeterminate", "stepUp", "stepDown", "valueAsDate", "valueAsNumber", "list"];
  const textareaMembers = ["cols", "rows", "wrap", "maxLength", "minLength", "dirName", "textLength", "type"];
  result.descriptors = {input: inputMembers.map((name) => [name, describe(I, name)]), textarea: textareaMembers.map((name) => [name, describe(T, name)])};

  // Receivers: wrong interface, non-HTML namespace, plain objects.
  const receivers = [["object", {}], ["div", document.createElement("div")], ["textarea", document.createElement("textarea")], ["input", document.createElement("input")],
    ["svg-input", document.createElementNS("http://www.w3.org/2000/svg", "input")], ["xhtml-input", document.createElementNS("http://www.w3.org/1999/xhtml", "input")],
    ["xhtml-textarea", document.createElementNS("http://www.w3.org/1999/xhtml", "textarea")]];
  result.receivers = [];
  for (const [prototype, names] of [[I, inputMembers], [T, textareaMembers]]) {
    for (const name of names) {
      const d = Object.getOwnPropertyDescriptor(prototype, name);
      for (const [label, receiver] of receivers) {
        const row = [name, label];
        if (d.get) row.push(run(() => d.get.call(receiver)));
        if (d.set) row.push(run(() => d.set.call(receiver, name === "valueAsDate" ? null : name === "valueAsNumber" ? NaN : 3)));
        if (d.value) row.push(run(() => d.value.call(receiver)));
        result.receivers.push(row);
      }
    }
  }

  // DOMString and boolean reflection.
  result.strings = [];
  for (const [factory, property, attribute] of [["input", "align", "align"], ["input", "alt", "alt"], ["input", "dirName", "dirname"], ["input", "useMap", "usemap"], ["textarea", "wrap", "wrap"], ["textarea", "dirName", "dirname"]]) {
    const element = document.createElement(factory);
    const row = [property, element[property]];
    for (const value of ["x", "MiXeD", "", 5, null, undefined, "a\u0000b", {toString() { return "object"; }}]) { element[property] = value; row.push([element[property], element.getAttribute(attribute)]); }
    element.setAttribute(attribute, "attr"); row.push(element[property]);
    row.push(run(() => { element[property] = Symbol("s"); }));
    row.push(run(() => { element[property] = {toString() { throw new RangeError("custom"); }}; }));
    result.strings.push(row);
  }
  result.booleans = [];
  for (const [property, attribute] of [["incremental", "incremental"], ["webkitdirectory", "webkitdirectory"]]) {
    const element = document.createElement("input");
    const row = [property, element[property]];
    for (const value of [true, false, "x", "", 0, 1, null, {}, Symbol.iterator]) { element[property] = value; row.push([element[property], element.getAttribute(attribute)]); }
    element.setAttribute(attribute, "false"); row.push(element[property]);
    result.booleans.push(row);
  }

  // Integer reflection: content-attribute parsing and IDL conversions/exceptions.
  const attributeValues = [null, "", "5", " 7", "+3", "-1", "-0", "0", "05", "5 ", "\t9", "\n9", "\f9", "\r9", " 9", "　9", "1e2", "12abc", "0x10", "+", "-", "2147483647", "2147483648", "4294967296", "99999999999"];
  const idlValues = [0, 5, -1, -0, 1.9, -1.9, NaN, Infinity, 2147483647, 2147483648, 4294967295, 4294967299, -4294967291, "7", " 8 ", "x", null, undefined, true, [], [4], {valueOf() { return 6; }}];
  result.integers = [];
  for (const [factory, property, attribute] of [["input", "maxLength", "maxlength"], ["input", "minLength", "minlength"], ["input", "size", "size"], ["textarea", "maxLength", "maxlength"], ["textarea", "minLength", "minlength"], ["textarea", "cols", "cols"], ["textarea", "rows", "rows"]]) {
    const parsed = attributeValues.map((value) => { const element = document.createElement(factory); if (value !== null) element.setAttribute(attribute, value); return element[property]; });
    const assigned = idlValues.map((value) => { const element = document.createElement(factory); return run(() => { element[property] = value; return [element[property], element.getAttribute(attribute)]; }); });
    const element = document.createElement(factory);
    element.setAttribute(attribute, "11");
    const errors = [run(() => { element[property] = Symbol("s"); }), run(() => { element[property] = 1n; }), run(() => { element[property] = {valueOf() { throw new RangeError("custom"); }}; }), element.getAttribute(attribute)];
    result.integers.push([factory, property, parsed, assigned, errors]);
  }

  // textarea.type and textLength over the normalized API value.
  result.textarea = [];
  {
    const textarea = document.createElement("textarea");
    result.textarea.push(textarea.type, textarea.textLength);
    textarea.setAttribute("type", "text"); result.textarea.push(textarea.type);
    result.textarea.push(run(() => { textarea.type = "x"; return textarea.type; }));
    result.textarea.push(run(() => { "use strict"; textarea.type = "x"; }));
    for (const [mode, text] of [["value", "a\r\nb\rc\nd"], ["value", "\u{1F600}x"], ["value", "\ud800"], ["default", "q\r\nr"], ["content", "p\rq"], ["value", ""]]) {
      const element = document.createElement("textarea");
      if (mode === "value") element.value = text; else if (mode === "default") element.defaultValue = text; else element.textContent = text;
      result.textarea.push([mode, element.textLength, JSON.stringify(element.value), JSON.stringify(element.defaultValue)]);
    }
    const dirty = document.createElement("textarea"); dirty.value = "ab"; dirty.textContent = "xyz\r\n"; result.textarea.push([dirty.textLength, JSON.stringify(dirty.value)]);
  }

  // indeterminate: IDL-only state, cloning, activation.
  result.indeterminate = [];
  {
    const box = input("checkbox");
    result.indeterminate.push(box.indeterminate);
    for (const value of [1, 0, "x", "", {}, null, true]) { box.indeterminate = value; result.indeterminate.push([box.indeterminate, box.hasAttribute("indeterminate")]); }
    box.setAttribute("indeterminate", ""); result.indeterminate.push(box.indeterminate);
    box.removeAttribute("indeterminate"); result.indeterminate.push(box.indeterminate);
    result.indeterminate.push(["clone", box.cloneNode().indeterminate, box.cloneNode(true).indeterminate]);
    const wrapper = document.createElement("div"); wrapper.append(box); const deep = wrapper.cloneNode(true), shallow = wrapper.cloneNode(false);
    result.indeterminate.push(["deep", deep.firstChild.indeterminate, shallow.childNodes.length, document.importNode(wrapper, true).firstChild.indeterminate]);
    const plain = input("checkbox"); result.indeterminate.push(["plain-clone", plain.cloneNode().indeterminate]);
    document.body.append(box);
    box.click(); result.indeterminate.push(["click", box.indeterminate, box.checked]);
    box.indeterminate = true;
    box.addEventListener("click", (event) => event.preventDefault(), {once: true});
    box.click(); result.indeterminate.push(["canceled", box.indeterminate, box.checked]);
    let during;
    box.addEventListener("click", () => { during = box.indeterminate; }, {once: true});
    box.click(); result.indeterminate.push(["during", during, box.indeterminate, box.checked]);
    box.indeterminate = true; box.dispatchEvent(new MouseEvent("click", {bubbles: true, cancelable: true}));
    result.indeterminate.push(["dispatched", box.indeterminate, box.checked]);
    box.indeterminate = true; box.dispatchEvent(new MouseEvent("mousedown")); result.indeterminate.push(["mousedown", box.indeterminate]);
    const radio = input("radio"); radio.indeterminate = true; document.body.append(radio); radio.click(); result.indeterminate.push(["radio", radio.indeterminate, radio.checked]);
    const text = input("text"); text.indeterminate = true; document.body.append(text); text.click(); result.indeterminate.push(["text", text.indeterminate]);
    text.setAttribute("type", "checkbox"); text.click(); result.indeterminate.push(["retyped", text.indeterminate, text.checked]);
    const form = document.createElement("form"), reset = input("checkbox"); reset.indeterminate = true; form.append(reset); form.reset(); result.indeterminate.push(["reset", reset.indeterminate]);
    box.remove(); radio.remove(); text.remove();
  }

  // list: the datalist by ID in the element's tree, honoring type applicability.
  result.list = [];
  {
    document.body.insertAdjacentHTML("beforeend", "<datalist id=d1></datalist><div id=d2></div><div id=dup></div><datalist id=dup></datalist><select><datalist id=nested></datalist></select><svg><g id=svgd></g></svg><input id=a list=d1><input id=b list=d2><input id=c list=nope><input id=e list=dup><input id=f list=''><input id=g list=D1><input id=h list=nested><input id=i list=svgd><input id=j list=' d1'>");
    for (const id of ["a", "b", "c", "e", "f", "g", "h", "i", "j"]) result.list.push([id, shape(document.getElementById(id).list)]);
    const a = document.getElementById("a"); result.list.push(["same", a.list === a.list]);
    for (const type of ["text", "hidden", "password", "checkbox", "radio", "file", "submit", "image", "reset", "button", "color", "range", "date", "month", "week", "time", "datetime-local", "number", "email", "search", "url", "tel", "bogus", "TEXT", "Hidden"]) { a.setAttribute("type", type); result.list.push([type, shape(a.list)]); }
    a.removeAttribute("type");
    const foreign = document.createElementNS("http://example.com/", "datalist"); foreign.id = "foreign"; document.body.append(foreign); a.setAttribute("list", "foreign"); result.list.push(["foreign", shape(a.list)]);
    const later = document.createElement("datalist"); later.id = "later"; a.setAttribute("list", "later"); result.list.push(["missing", shape(a.list)]);
    document.body.append(later); result.list.push(["inserted", shape(a.list)]);
    const earlier = document.createElement("datalist"); earlier.id = "later"; earlier.setAttribute("data-order", "first"); document.body.prepend(earlier); result.list.push(["earlier", a.list === earlier]);
    earlier.remove(); later.removeAttribute("id"); result.list.push(["removed", shape(a.list)]);
    a.removeAttribute("list"); result.list.push(["unset", shape(a.list)]);
    const detached = document.createElement("input"); detached.setAttribute("list", "d1"); result.list.push(["detached", shape(detached.list)]);
    const subtree = document.createElement("div"); subtree.innerHTML = "<datalist id=sub></datalist><input list=sub>"; result.list.push(["subtree", shape(subtree.lastChild.list)]);
    const host = document.createElement("div"); document.body.append(host); const root = host.attachShadow({mode: "open"});
    root.innerHTML = "<datalist id=s1></datalist><input list=s1><input list=d1>";
    for (const element of root.querySelectorAll("input")) result.list.push(["shadow", shape(element.list)]);
    document.body.textContent = "";
  }

  // valueAsNumber / valueAsDate per type.
  const types = ["text", "number", "range", "date", "month", "week", "time", "datetime-local", "checkbox", "hidden", "color", "bogus"];
  const values = ["", "5", "-0", "1e3", ".5", "1.", "+1", "abc", "2024-02-29", "2023-02-29", "0001-01-01", "275760-09-13", "275760-09-14", "12345-01-01", "1969-12-31",
    "2024-02", "0001-01", "275760-09", "275760-10", "2024-13", "2024-W09", "2020-W53", "2021-W53", "275760-W37", "275760-W38", "1970-W01", "2024-w01",
    "13:45", "13:45:30", "13:45:30.5", "13:45:30.25", "13:45:30.125", "13:45:30.1234", "24:00", "23:59:60", "1:00", "00:00",
    "2024-02-29T13:45", "2024-02-29 13:45", "2024-02-29T13:45:00", "2024-02-29T13:45:00.000", "2024-02-29T13:45:01.500", "2024-02-29T13:45:01.010", "2024-02-29t13:45", "275760-09-13T00:00", "275760-09-13T00:01", "0999-01-01T00:00", "2024-02-29T13:45:1"];
  result.readers = [];
  for (const type of types) result.readers.push([type, values.map((value) => { const element = input(type); element.value = value; return [element.value, run(() => element.valueAsNumber), run(() => element.valueAsDate)]; })]);
  const numbers = [0, -0, 0.5, -0.5, 1, -1, 1 / 3, 12.7, 3723004, 86400000.5, 1e21, 1e20, -62135596800000, -62135596800001, 8640000000000000, 8640000000000001, 8639999568000000, 3285488, 3285489, -23628, -23629, NaN, Infinity, -Infinity, "7", null, {valueOf() { return 2; }}];
  result.numberWriters = [];
  for (const type of types) result.numberWriters.push([type, numbers.map((value) => { const element = input(type); return run(() => { element.valueAsNumber = value; return element.value; }); })]);
  const dates = [new Date(Date.UTC(2020, 0, 5, 23, 59)), new Date(Date.UTC(2021, 0, 3)), new Date(Date.UTC(2021, 0, 4)), new Date(Date.UTC(1969, 11, 31, 23, 59, 59, 999)), new Date(-62135596800000), new Date(-62135596800001), new Date(8640000000000000),
    new Date(Date.UTC(2000, 0, 1, 1, 2, 3, 4)), new Date(Date.UTC(2000, 0, 1, 1, 2, 3)), new Date(-0.5), new Date(NaN), null, undefined, 5, "2020-01-01", {}, Object.create(Date.prototype), [new Date(0)]];
  result.dateWriters = [];
  for (const type of types) result.dateWriters.push([type, dates.map((value) => { const element = input(type); return run(() => { element.valueAsDate = value; return element.value; }); })]);
  {
    const element = input("date"); element.value = "2024-01-01"; const first = element.valueAsDate;
    result.dateIdentity = [first === element.valueAsDate, first instanceof Date, Object.getPrototypeOf(first) === Date.prototype];
    const step = input("time", {step: "1"}); result.precision = [run(() => { step.valueAsNumber = 3600000; return step.value; }), run(() => { step.valueAsDate = new Date(3600000); return step.value; })];
    step.setAttribute("step", "0.5"); result.precision.push(run(() => { step.valueAsNumber = 3600000; return step.value; }));
    step.setAttribute("step", "any"); result.precision.push(run(() => { step.valueAsNumber = 3600000; return step.value; }));
    const local = input("datetime-local", {step: "1"}); result.precision.push(run(() => { local.valueAsNumber = 0; return local.value; }));
    const rangeAny = input("range", {step: "any"}); result.precision.push(run(() => { rangeAny.valueAsNumber = 1 / 3; return rangeAny.value; }));
    result.precision.push(run(() => { input("number").valueAsNumber = Symbol("s"); }), run(() => { input("number").valueAsNumber = 1n; }), run(() => { input("text").valueAsNumber = Infinity; }));
  }

  // Range value sanitization.
  result.range = [[{}, ""], [{}, "33.4"], [{}, "33.5"], [{}, "-5"], [{}, "1000"], [{min: "10", max: "5"}, ""], [{min: "0", max: "10", step: "4"}, "6"], [{min: "0", max: "10", step: "4"}, "10"],
    [{step: "any"}, "33.33"], [{min: "1", max: "2", step: "0.3"}, ""], [{max: "-10"}, ""], [{min: "0.1", max: "0.2", step: "0.05"}, "0.17"], [{}, "abc"], [{value: "70"}, null], [{step: "3", value: "70"}, null],
    [{step: "3", value: "71"}, null], [{min: "abc", max: "x"}, "120"], [{step: "0"}, "2.5"], [{step: "-1"}, "2.5"], [{min: "-50"}, ""], [{min: "-50", step: "7"}, ""], [{max: "1e3"}, "999.5"], [{}, "1e1"], [{}, "+5"]]
    .map(([attributes, value]) => { const element = input("range", attributes); if (value !== null) element.value = value; return [element.value, run(() => element.valueAsNumber)]; });

  // stepUp / stepDown: hand-picked rows, then a seeded matrix over every steppable type.
  const step = (type, attributes, value, operation, count) => {
    const element = input(type, attributes);
    if (value !== null) element.value = value;
    return run(() => { if (count === "none") element[operation](); else element[operation](count); return element.value; });
  };
  result.steps = [
    ["number", {}, "", "stepUp", "none"], ["number", {}, "5", "stepUp", 3], ["number", {step: "0.1"}, "0.2", "stepUp", "none"], ["number", {step: "0.1"}, "0.25", "stepUp", "none"], ["number", {step: "0.1"}, "0.25", "stepDown", "none"],
    ["number", {step: "3", min: "-100"}, "3", "stepUp", "none"], ["number", {step: "3", min: "-100"}, "3", "stepDown", "none"], ["number", {step: "any"}, "1", "stepUp", "none"], ["number", {step: "ANY"}, "1", "stepDown", "none"],
    ["number", {max: "3"}, "5", "stepUp", "none"], ["number", {max: "3"}, "5", "stepDown", "none"], ["number", {min: "5", max: "2"}, "1", "stepUp", "none"], ["number", {min: "0", max: "10", step: "4"}, "9", "stepUp", "none"],
    ["number", {}, "1e3", "stepUp", "none"], ["number", {step: "0"}, "1", "stepUp", "none"], ["number", {step: "-2"}, "1", "stepUp", "none"], ["number", {}, "1", "stepUp", 2.9], ["number", {}, "1", "stepUp", -2],
    ["number", {value: "0.5"}, null, "stepUp", "none"], ["number", {}, "1", "stepUp", 4294967297], ["number", {}, "1", "stepUp", "3"], ["number", {}, "1", "stepUp", undefined], ["number", {}, "1", "stepUp", null], ["number", {}, "1", "stepUp", NaN],
    ["number", {}, "1", "stepUp", Symbol("s")], ["number", {}, "1", "stepUp", 1n], ["number", {}, "9007199254740993", "stepUp", "none"], ["number", {}, "0.1", "stepUp", 1e10], ["number", {step: "1e-20"}, "1", "stepUp", "none"],
    ["number", {step: "1e3"}, "0", "stepUp", 1], ["number", {step: "0.000001"}, "0", "stepUp", 1], ["number", {step: "0.0000001"}, "0", "stepUp", 1], ["number", {step: "1e20"}, "0", "stepUp", 3], ["number", {step: "1e21"}, "0", "stepUp", 1],
    ["number", {}, "1e21", "stepUp", 1], ["number", {}, "123456789012345678", "stepUp", 1], ["number", {}, "1234567890123456789", "stepUp", 1], ["number", {step: "0.1"}, "0.123456789012345678", "stepUp", 1],
    ["number", {step: "0.3"}, "1", "stepUp", 1], ["number", {step: "0.3"}, "1", "stepDown", 1], ["number", {step: "1/3"}, "1", "stepUp", 1], ["number", {step: "0.7", min: "0.1"}, "5", "stepDown", 2], ["number", {}, "-0.5", "stepUp", 1],
    ["number", {}, "-0.5", "stepDown", 1], ["number", {step: "2"}, "-3", "stepUp", 1], ["number", {step: "2"}, "-3", "stepDown", 1], ["number", {min: "-1e308"}, "0", "stepDown", 1], ["number", {}, "1.7976931348623157e308", "stepUp", 1],
    ["number", {step: "1e308"}, "1e308", "stepUp", 1], ["number", {step: "0.5"}, "0.0", "stepUp", 0], ["number", {step: "0.25"}, "0.1", "stepUp", 0], ["number", {step: "0.25"}, "0.1", "stepDown", 0], ["number", {max: "10", step: "3"}, "20", "stepDown", 1],
    ["number", {min: "10", step: "3"}, "0", "stepUp", 1], ["number", {min: "10", max: "11", step: "5"}, "0", "stepUp", 1], ["number", {min: "0", max: "4", step: "5"}, "2", "stepUp", 1], ["number", {step: "0.1"}, "0.30000000000000004", "stepUp", 1],
    ["number", {min: "10"}, "", "stepDown", 1], ["number", {max: "-10"}, "", "stepUp", 1], ["number", {min: "1", max: "1.5", step: "1"}, "1.2", "stepUp", 1], ["number", {min: "0.5", max: "0.7", step: "1"}, "", "stepUp", 1],
    ["range", {}, null, "stepUp", "none"], ["range", {}, "100", "stepUp", "none"], ["range", {step: "7"}, null, "stepUp", "none"], ["range", {min: "10", max: "20"}, "12", "stepDown", 5], ["range", {step: "any"}, "5", "stepUp", "none"],
    ["date", {}, "", "stepUp", "none"], ["date", {}, "2024-02-28", "stepUp", 2], ["date", {step: "1.5"}, "2024-02-28", "stepUp", "none"], ["date", {step: "7", min: "2024-01-01"}, "2024-01-03", "stepUp", "none"], ["date", {}, "275760-09-13", "stepUp", 1],
    ["month", {}, "2024-12", "stepUp", "none"], ["month", {}, "", "stepDown", "none"], ["week", {}, "2024-W52", "stepUp", 2], ["week", {}, "", "stepUp", "none"], ["week", {}, "2020-W53", "stepUp", 1],
    ["time", {}, "", "stepUp", "none"], ["time", {}, "23:59", "stepUp", "none"], ["time", {step: "1"}, "10:00", "stepUp", "none"], ["time", {step: "0.001"}, "10:00", "stepUp", "none"], ["time", {step: "0.0001"}, "10:00", "stepUp", "none"],
    ["time", {step: "1"}, "10:00:59", "stepUp", 1], ["time", {step: "0.001"}, "10:00:00.999", "stepUp", 1], ["time", {}, "10:00:00.12", "stepUp", 0], ["time", {step: "120"}, "10:01", "stepUp", 1], ["time", {step: "30"}, "10:00", "stepUp", 1],
    ["time", {min: "22:00", max: "02:00"}, "23:00", "stepUp", 1], ["time", {min: "22:00", max: "02:00"}, "01:00", "stepUp", 120],
    ["datetime-local", {}, "", "stepUp", "none"], ["datetime-local", {}, "2024-02-29T23:59", "stepUp", "none"], ["datetime-local", {step: "1"}, "2024-02-29T23:59", "stepUp", 1], ["datetime-local", {step: "0.5"}, "2024-02-29T23:59", "stepDown", 1],
    ["text", {}, "1", "stepUp", "none"], ["checkbox", {}, "1", "stepUp", "none"], ["color", {}, null, "stepUp", "none"], ["hidden", {}, "1", "stepDown", "none"], ["text", {step: "any"}, "1", "stepUp", Symbol("s")], [null, {}, "1", "stepUp", "none"]
  ].map(([type, attributes, value, operation, count]) => step(type, attributes, value, operation, count));
  let seed = 0x5eed;
  const pick = (list) => { seed = (Math.imul(seed, 1103515245) + 12345) >>> 0; return list[(seed >>> 8) % list.length]; };
  const matrix = {
    number: {values: ["", "0", "1", "-1", "2.5", "9", "10", "11", "-10.5", "0.15", "1e2", "abc", "99999", "0.30000000000000004", "7.77", "-0.25"], limits: [null, "", "0", "-10", "10", "2.5", "0.1", "1e3", "-0.5", "abc", "3", "7", "100", "-3.3"],
      steps: [null, "", "any", "0", "-1", "1", "2", "0.1", "0.3", "3", "0.25", "1e-3", "5", "2.5", "abc", "1.5e1"]},
    range: {values: ["", "0", "1", "50", "2.5", "99", "100", "-1", "33.3", "abc", "0.15"], limits: [null, "", "0", "-10", "10", "2.5", "90", "100", "200", "abc", "33"],
      steps: [null, "", "any", "0", "1", "2", "0.1", "0.3", "3", "7", "2.5", "abc"]},
    date: {values: ["", "2024-02-29", "2024-01-01", "1970-01-01", "2023-12-31", "0001-01-02", "275760-09-12", "bad"], limits: [null, "", "2024-01-01", "2024-03-01", "1970-01-05", "0001-01-01", "275760-09-13", "x"],
      steps: [null, "", "any", "1", "2", "1.5", "7", "0.4", "30", "-1"]},
    month: {values: ["", "2024-02", "1970-01", "2023-12", "0001-02", "275760-08", "x"], limits: [null, "", "2024-01", "2024-06", "1969-01", "0001-01", "275760-09"], steps: [null, "", "1", "2", "12", "1.5", "0.2", "any"]},
    week: {values: ["", "2024-W09", "2020-W53", "1970-W01", "2024-W52", "0001-W02", "275760-W36", "x"], limits: [null, "", "2024-W01", "2024-W10", "1970-W03", "275760-W37"], steps: [null, "", "1", "2", "1.5", "any", "52"]},
    time: {values: ["", "00:00", "10:00", "23:59", "12:30:45", "12:30:45.5", "23:59:59.999", "00:00:00.001", "x"], limits: [null, "", "08:00", "18:00", "23:00", "01:00", "12:30:45.5"],
      steps: [null, "", "1", "60", "0.5", "0.001", "3600", "900", "any", "0", "86400", "90", "0.25"]},
    "datetime-local": {values: ["", "2024-02-29T23:59", "1970-01-01T00:00", "2024-01-01T12:00:30", "0001-01-01T00:01", "2024-02-29 08:00", "x"], limits: [null, "", "2024-02-29T00:00", "2024-03-01T00:00", "1970-01-01T12:00", "275760-09-13T00:00"],
      steps: [null, "", "1", "60", "0.5", "3600", "86400", "any", "0.001"]},
  };
  result.matrix = [];
  for (const type of Object.keys(matrix)) {
    const pools = matrix[type];
    for (let index = 0; index < 90; index++) {
      const attributes = {};
      for (const name of ["min", "max", "step", "value"]) { const choice = pick(name === "step" ? pools.steps : name === "value" ? [null, null].concat(pools.values) : pools.limits); if (choice !== null) attributes[name] = choice; }
      const value = pick([null].concat(pools.values)), operation = pick(["stepUp", "stepDown"]), count = pick(["none", 0, 1, 2, -1, -3, 10, 2.5, -0.5]);
      result.matrix.push([type, JSON.stringify(attributes), value, operation, String(count), step(type, attributes, value, operation, count)]);
    }
  }
  return JSON.parse(JSON.stringify(result, (_key, value) => typeof value === "string" && /[\ud800-\udfff]/.test(value) ? {utf16: Array.from({length: value.length}, (_, index) => value.charCodeAt(index))} : value));
})();
