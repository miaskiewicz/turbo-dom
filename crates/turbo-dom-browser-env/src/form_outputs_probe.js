(() => {
  const result = {};
  const signed = value => Object.is(value, -0) ? "-0" : value;
  const failure = callback => { try { return ["ok", signed(callback())]; } catch (error) { return [error.name]; } };
  const members = {HTMLMeterElement:["value","min","max","low","high","optimum"],HTMLProgressElement:["value","max","position"],HTMLOutputElement:["htmlFor","name","type","defaultValue","value"]};
  const source = fn => fn ? Function.prototype.toString.call(fn) : null;
  result.descriptors = {};
  result.keys = {};
  for (const [name, list] of Object.entries(members)) {
    const proto = globalThis[name].prototype;
    result.keys[name] = Reflect.ownKeys(proto).map(String).sort();
    for (const member of list) {
      const d = Object.getOwnPropertyDescriptor(proto, member);
      result.descriptors[name+"."+member] = [d.enumerable, d.configurable, "value" in d, d.get.name, d.get.length, source(d.get), "prototype" in d.get, d.set ? d.set.name : null, d.set ? d.set.length : null, source(d.set)];
    }
  }
  // Brand checks: every accessor rejects foreign receivers before converting its argument.
  const strangers = {plain:() => ({}), div:() => document.createElement("div"), meter:() => document.createElement("meter"), progress:() => document.createElement("progress"), output:() => document.createElement("output"), svg:() => document.createElementNS("http://www.w3.org/2000/svg","meter")};
  result.brands = {};
  for (const [name, list] of Object.entries(members)) for (const member of list) {
    const d = Object.getOwnPropertyDescriptor(globalThis[name].prototype, member), row = {};
    for (const [label, make] of Object.entries(strangers)) {
      row[label] = [failure(() => d.get.call(make()))[0]];
      if (d.set) { const log = []; row[label].push(failure(() => d.set.call(make(), {valueOf(){log.push("valueOf");return 1;},toString(){log.push("toString");return "1";}}))[0], log.join()); }
    }
    result.brands[name+"."+member] = row;
  }
  // Attribute grammar: HTML "rules for parsing floating-point number values".
  const garbage = ["", " 1", "1 ", "\t2", "\n3", "\f4", "\r5", "\v6", " 7", "　8", " 9", "+1", "++1", "+-1", "-+1", "+", "-", ".", "-.", "+.5", "-.5", ".5", "1.", "1.e5", ".e5", "-.e1", "1..5", "1.5.5", "1e", "1e+", "1e-", "1E+2x", "1e05", "1e-5", "1.5e", "1.5E-", "1ee5", "1e5.5", "0x10", "0b1", "e5", "1_0", "1,5", "1abc", "Infinity", "-Infinity", "NaN", "1e308", "1.8e308", "-1.8e308", "1e-400", "-1e-400", "5e-324", "-0", "+0", "00.50", "12345678901234567890123", "0.1e1", "\u00001", "1\u0000", "٣", "- 1", " - 1", "1 e5", "0.30000000000000004", "9007199254740993"];
  const meter = document.createElement("meter"), progress = document.createElement("progress");
  result.grammar = garbage.map(text => { meter.setAttribute("min", text); progress.setAttribute("max", text); progress.setAttribute("value", text); return [signed(meter.min), progress.max, signed(progress.value), signed(progress.position)]; });
  meter.removeAttribute("min");
  // Meter clamping: min, then max >= min, value/low into [min,max], high into [low,max], optimum into [min,max].
  const meterState = element => [element.min, element.max, element.value, element.low, element.high, element.optimum].map(signed);
  const meterAttributes = element => ["min","max","value","low","high","optimum"].map(name => element.getAttribute(name));
  result.meterSteps = [meterState(meter)];
  for (const [name, value] of [["min","5"],["max","2"],["value","9"],["low","1"],["high","7"],["optimum","100"],["min","-10"],["low","3"],["high","1"],["max","x"],["min","x"],["optimum","-50"],["value","-5"],["min","-3"],["max","-1"],["low","-2"],["high","-2.5"],["optimum","-2"],["value","-1.5"],["low","9"],["high","-9"],["min","0.5"],["max","0.25"]]) { meter.setAttribute(name, value); result.meterSteps.push([name, value, meterState(meter)]); }
  for (const name of ["min","max","value","low","high","optimum"]) meter.removeAttribute(name);
  result.meterDefaults = [];
  for (const [min, max] of [[null,null],["3",null],["-3",null],["0.5",null],[null,"0"],[null,"-4"],["2","6"],["6","2"]]) {
    const element = document.createElement("meter");
    if (min !== null) element.setAttribute("min", min);
    if (max !== null) element.setAttribute("max", max);
    result.meterDefaults.push(meterState(element));
  }
  // Setters reflect the ECMAScript number serialization without clamping; non-finite throws.
  const inputs = [0.1, 1e21, 1e-7, -0, 5e-324, -100, 123456789012345680000, 1/3, "5", " 7 ", "", null, false, true, [], [3], {valueOf(){return 4;}}, {toString(){return "6";}}, new Date(8)];
  const rejects = [NaN, Infinity, -Infinity, undefined, "abc", {}, [1,2], Symbol("x"), 1n];
  result.meterSetters = {};
  for (const name of ["min","max","value","low","high","optimum"]) {
    const element = document.createElement("meter");
    element.setAttribute(name, "kept");
    const accepted = inputs.map(input => { element[name] = input; return element.getAttribute(name); });
    const rejected = rejects.map(input => { element.setAttribute(name, "kept"); let error; try { element[name] = input; } catch (caught) { error = caught.name === "TypeError" && typeof input === "number" ? [caught.name, caught.message] : [caught.name]; } return [error, element.getAttribute(name)]; });
    result.meterSetters[name] = {accepted, rejected, missing:failure(() => Object.getOwnPropertyDescriptor(HTMLMeterElement.prototype, name).set.call(element)), attributes:meterAttributes(element)};
  }
  // Progress: max ignores <= 0, value clamps negative to 0, position is -1 while indeterminate.
  const progressState = element => [element.getAttribute("max"), element.getAttribute("value"), element.max, signed(element.value), signed(element.position)];
  const bar = document.createElement("progress");
  result.progressSteps = [progressState(bar)];
  for (const [kind, name, value] of [["set","max",0],["attr","max","5"],["set","max",0],["set","max",-3],["set","max",0.5],["set","value",2],["set","value",-0],["set","value",-2],["attr","value","junk"],["attr","value","-1"],["attr","max","-1"],["attr","max","0"],["attr","value","0.25"],["remove","value"],["attr","max","4"],["attr","value","3"],["attr","value","30"],["set","value",0.000001],["set","max",1e-300],["set","value",1e300],["attr","value",""],["remove","max"],["set","max","7"],["set","value",null]]) {
    if (kind === "set") bar[name] = value; else if (kind === "attr") bar.setAttribute(name, value); else bar.removeAttribute(name);
    result.progressSteps.push([kind, name, String(value), progressState(bar)]);
  }
  result.progressSetters = {};
  for (const name of ["max","value"]) {
    const element = document.createElement("progress");
    const accepted = inputs.map(input => { element.removeAttribute(name); element[name] = input; return element.getAttribute(name); });
    const rejected = rejects.map(input => { element.setAttribute(name, "kept"); let error; try { element[name] = input; } catch (caught) { error = caught.name === "TypeError" && typeof input === "number" ? [caught.name, caught.message] : [caught.name]; } return [error, element.getAttribute(name)]; });
    result.progressSetters[name] = {accepted, rejected};
  }
  result.position = [failure(() => { "use strict"; bar.position = 3; }), Reflect.set(bar, "position", 3), bar.position];
  // Parsed markup.
  const holder = document.createElement("div");
  holder.innerHTML = "<meter min=2 max=8 value=5 low=3 high=6 optimum=7></meter><progress value=' 3' max=6></progress><progress></progress><output for=' a  b a ' name=total>1<b>2</b><!--c-->3</output>";
  const [parsedMeter, parsedProgress, emptyProgress, parsedOutput] = holder.children;
  result.parsed = [meterState(parsedMeter), progressState(parsedProgress), progressState(emptyProgress), [parsedOutput.value, parsedOutput.defaultValue, parsedOutput.name, parsedOutput.type, [...parsedOutput.htmlFor], parsedOutput.htmlFor.value, parsedOutput.htmlFor.length]];
  // Output htmlFor: a same-object DOMTokenList over `for`, [PutForwards=value].
  const output = document.createElement("output"), other = document.createElement("output");
  const list = output.htmlFor;
  result.htmlFor = {same:list === output.htmlFor, distinct:list !== other.htmlFor, classDistinct:list !== output.classList, tag:Object.prototype.toString.call(list), proto:Object.getPrototypeOf(list) === DOMTokenList.prototype, initial:[list.length, list.value, output.getAttribute("for")]};
  list.add("x", "y"); result.htmlFor.added = [list.value, output.getAttribute("for"), list.contains("y"), list.item(1), list[0]];
  output.setAttribute("for", "  p\tq  p "); result.htmlFor.attribute = [list.length, [...list], list.value, String(list)];
  output.htmlFor = "m n"; result.htmlFor.forwarded = [list === output.htmlFor, output.getAttribute("for"), [...list]];
  output.htmlFor = null; result.htmlFor.forwardedNull = output.getAttribute("for");
  result.htmlFor.symbol = [failure(() => { output.htmlFor = Symbol(); })[0], output.getAttribute("for")];
  list.remove("null"); result.htmlFor.removed = [output.getAttribute("for"), output.hasAttribute("for")];
  list.toggle("z"); list.replace("z", "w"); result.htmlFor.toggled = output.getAttribute("for");
  result.htmlFor.supports = failure(() => list.supports("x"));
  result.htmlFor.errors = [failure(() => list.add(""))[0], failure(() => list.add("a b"))[0]];
  const forDescriptor = Object.getOwnPropertyDescriptor(HTMLOutputElement.prototype, "htmlFor");
  result.htmlFor.foreignSetter = failure(() => forDescriptor.set.call(document.createElement("label"), "x"))[0];
  // Output name/type.
  result.name = [output.name];
  for (const value of ["n", "", null, undefined, 12, "a\u0000b"]) { output.name = value; result.name.push([output.name, output.getAttribute("name")]); }
  result.name.push(failure(() => { output.name = Symbol(); })[0]);
  output.setAttribute("NAME", "upper"); result.name.push(output.name);
  result.type = [output.type, failure(() => { "use strict"; output.type = "x"; })[0], Reflect.set(output, "type", "x"), output.type, output.hasAttribute("type")];
  output.setAttribute("type", "number"); result.type.push(output.type);
  // Output default value / value mode.
  const snap = element => [element.value, element.defaultValue, element.childNodes.length];
  const steps = [];
  const out = document.createElement("output");
  const run = (label, callback) => { callback(); steps.push([label, snap(out)]); };
  run("text abc", () => { out.textContent = "abc"; });
  run("defaultValue ''", () => { out.defaultValue = ""; });
  run("defaultValue d", () => { out.defaultValue = "d"; });
  run("value v", () => { out.value = "v"; });
  run("defaultValue e", () => { out.defaultValue = "e"; });
  run("value w", () => { out.value = "w"; });
  run("text t", () => { out.textContent = "t"; });
  run("defaultValue e again", () => { out.defaultValue = "e"; });
  run("value null", () => { out.value = null; });
  run("defaultValue undefined", () => { out.defaultValue = undefined; });
  run("value 12", () => { out.value = 12; });
  run("value ''", () => { out.value = ""; });
  run("defaultValue object", () => { out.defaultValue = {toString(){return "obj";}}; });
  result.outputSteps = steps;
  result.outputSymbol = [failure(() => { out.value = Symbol(); })[0], failure(() => { out.defaultValue = Symbol(); })[0], snap(out)];
  const same = document.createElement("output"); same.textContent = "a"; const text = same.firstChild;
  same.value = "a"; const sameValue = same.firstChild === text;
  same.defaultValue = "b"; const valueMode = snap(same);
  const twice = document.createElement("output"); twice.textContent = "q"; const twiceText = twice.firstChild; twice.defaultValue = "q";
  const nested = document.createElement("output"); nested.innerHTML = "<b>x</b>y<!--z-->"; const nestedBefore = snap(nested); nested.value = "z";
  result.identity = [sameValue, valueMode, twice.firstChild === twiceText, snap(twice), nestedBefore, nested.innerHTML];
  // Form reset: restore default value unless it already equals the current value.
  const form = document.createElement("form"); form.id = "owner"; document.body.append(form);
  const resets = [];
  const reset = (label, element) => { form.reset(); resets.push([label, snap(element)]); };
  const a = document.createElement("output"); form.append(a); a.textContent = "start"; a.defaultValue = "dv"; a.value = "changed";
  reset("value mode", a);
  a.defaultValue = "dv2"; resets.push(["default mode after reset", snap(a)]);
  a.value = "dv2"; reset("equal value", a);
  a.defaultValue = "dv3"; resets.push(["stays in value mode", snap(a)]);
  a.textContent = "edited"; reset("after text edit", a);
  const b = document.createElement("output"); b.textContent = "q"; b.value = "q"; form.append(b); reset("captured text", b);
  b.defaultValue = "w"; resets.push(["captured text then default", snap(b)]);
  const cancel = event => event.preventDefault();
  a.value = "pending"; form.addEventListener("reset", cancel); reset("cancelled", a); form.removeEventListener("reset", cancel); reset("uncancelled", a);
  const linked = document.createElement("output"); linked.setAttribute("form", "owner"); document.body.append(linked); linked.defaultValue = "ld"; linked.value = "lv"; reset("form attribute", linked);
  linked.removeAttribute("form"); linked.value = "free"; reset("unassociated", linked);
  const detached = document.createElement("form"), inner = document.createElement("output"); detached.append(inner); inner.defaultValue = "x"; inner.value = "y"; detached.reset(); resets.push(["detached form", snap(inner)]);
  result.resets = resets;
  result.resetEvents = [];
  form.onreset = event => result.resetEvents.push([event.type, event.bubbles, event.cancelable, a.value]); a.value = "beforeEvent"; form.reset(); result.resetEvents.push(a.value); form.onreset = null;
  // Cloning and adoption do not carry the default value override.
  const original = document.createElement("output"); original.textContent = "s"; original.defaultValue = "sd"; original.value = "sv";
  const clone = original.cloneNode(true); const cloneState = snap(clone); clone.defaultValue = "cd";
  const adopted = document.implementation.createHTMLDocument("").adoptNode(original);
  result.clone = [cloneState, snap(clone), snap(adopted)];
  linked.remove(); form.remove(); linked.remove();
  return result;
})();
