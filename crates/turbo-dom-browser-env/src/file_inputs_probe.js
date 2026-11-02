(() => {
  const out = {};
  const run = (fn) => { try { const value = fn(); return value === undefined ? "undefined" : value; } catch (error) { return [error.name, error.message]; } };
  const describe = (object, name) => {
    const d = Object.getOwnPropertyDescriptor(object, name);
    if (!d) return null;
    return [d.enumerable, d.configurable, "writable" in d ? d.writable : null, d.get ? [d.get.name, d.get.length] : null,
      d.set ? [d.set.name, d.set.length] : null, typeof d.value === "function" ? [d.value.name, d.value.length] : d.value === undefined ? null : typeof d.value];
  };
  const fileOf = (name, type, bytes) => new File([bytes || name], name, { type: type || "text/plain", lastModified: 5 });
  const transfer = (...names) => { const data = new DataTransfer(); for (const name of names) data.items.add(fileOf(name)); return data; };
  const fileInput = (name) => { const input = document.createElement("input"); input.type = "file"; if (name) input.name = name; return input; };
  const names = (list) => list === null ? null : [list.length, ...Array.prototype.map.call(list, (file) => file.name)];

  // Interface shapes.
  const I = HTMLInputElement.prototype;
  out.inputDescriptors = ["files", "webkitEntries"].map((name) => [name, describe(I, name)]);
  out.interfaces = [];
  for (const name of ["FileList", "DataTransfer", "DataTransferItemList", "DataTransferItem"]) {
    const C = globalThis[name];
    out.interfaces.push([name, describe(globalThis, name), C.length, run(() => { new C(); return "constructed"; }), run(() => { C(); return "called"; }),
      Reflect.ownKeys(C.prototype).map((key) => [String(key), describe(C.prototype, key)]), Object.prototype.toString.call(C.prototype)]);
  }
  out.iterators = [FileList.prototype[Symbol.iterator] === Array.prototype.values, DataTransferItemList.prototype[Symbol.iterator] === Array.prototype.values,
    run(() => Reflect.construct(DataTransfer, [], Object).constructor === Object)];

  // FileList indexed and item() conversion.
  {
    const list = transfer("a", "b").files;
    out.item = [run(() => list.item()), list.item(1).name, list.item(1.9).name, list.item(-4294967295).name, list.item(NaN).name, run(() => list.item(Symbol())),
      list.item({ valueOf() { return 1; } }).name, list.item(2), list.item(-1), list.item(Infinity).name, list.item(4294967296).name, list.item("1").name,
      list[1].name, list["1"].name, list["01"], list[-0].name, list[1.5], list[2], "0" in list, "2" in list, Object.keys(list), Reflect.ownKeys(list),
      run(() => FileList.prototype.item.call({}, 0)), run(() => Object.getOwnPropertyDescriptor(FileList.prototype, "length").get.call({})),
      [...list].map((file) => file.name), list.item(0) === list[0]];
    const one = transfer("a").files;
    out.listMutation = [run(() => { one[0] = 5; return one[0].name; }), run(() => { delete one[0]; return one[0].name; }), run(() => { one[5] = 1; return one[5]; }),
      run(() => { one.foo = 1; return one.foo; }), run(() => { one.length = 9; return one.length; }),
      run(() => Reflect.deleteProperty(one, 0)), run(() => Reflect.deleteProperty(one, 5)),
      run(() => Object.defineProperty(one, "0", { value: 1 }) && "defined"), run(() => Object.defineProperty(one, "5", { value: 1 }) && "defined"),
      Object.getOwnPropertyNames(one), JSON.stringify(Object.getOwnPropertyDescriptor(one, "0"), (key, value) => key === "value" ? typeof value : value),
      Object.getOwnPropertyDescriptor(one, "1") === undefined];
  }

  // HTMLInputElement.files and value in the filename mode.
  {
    const text = document.createElement("input"), file = fileInput();
    out.basics = [text.files, file.files instanceof FileList, file.files === file.files, file.files.length, file.files.item(0), file.files[0], file.value,
      file.webkitEntries === file.webkitEntries, Array.isArray(file.webkitEntries), Object.isFrozen(file.webkitEntries), file.webkitEntries.length,
      text.webkitEntries.length, Object.isFrozen(text.webkitEntries), Object.getPrototypeOf(text.webkitEntries) === Array.prototype];
    const get = Object.getOwnPropertyDescriptor(I, "files").get, set = Object.getOwnPropertyDescriptor(I, "files").set;
    const entries = Object.getOwnPropertyDescriptor(I, "webkitEntries").get;
    out.brands = [run(() => get.call(document.body)), run(() => set.call(document.body, null)), run(() => set.call(document.body, {})),
      run(() => get.call(document.createElementNS("http://www.w3.org/2000/svg", "input"))), run(() => entries.call(document.body)),
      run(() => set.call(fileInput())), run(() => set.call(fileInput(), undefined))];
    const events = [];
    file.addEventListener("input", () => events.push("input"));
    file.addEventListener("change", () => events.push("change"));
    out.assignment = [run(() => { file.files = null; return file.files.length; }), run(() => { file.files = {}; }), run(() => { file.files = [fileOf("x")]; }),
      run(() => { file.files = undefined; return file.files.length; }), run(() => { text.files = transfer("a").files; return text.files; }),
      run(() => { text.files = {}; })];
    const data = transfer("a.txt", "b.txt"), list = data.files;
    file.files = list;
    out.adopted = [file.files === list, names(file.files), file.value, events.slice()];
    data.items.add(fileOf("c.txt"));
    out.live = [names(file.files), data.files === list];
    data.items.remove(0);
    out.live.push(names(file.files));
    out.value = [run(() => { file.value = "x"; }), run(() => { file.value = "C:\\fakepath\\b.txt"; }), names(file.files), run(() => { file.value = null; return file.value; }),
      file.files === list, names(list), data.items.length, data.files.length, data.types.slice(), events.slice()];
    data.setData("text/plain", "rebuilds");
    out.value.push(names(file.files), file.value);
    const other = fileInput();
    other.files = file.files;
    out.value.push(other.files === file.files);
  }

  // Type changes create a new FileList only when the file state is left.
  {
    const data = transfer("a"), input = fileInput();
    input.files = data.files;
    const list = input.files;
    input.type = "FILE";
    out.types = [input.files === list, input.files.length];
    input.setAttribute("type", "file ");
    out.types.push(input.type, input.files, input.value);
    input.type = "file";
    out.types.push(input.files === list, input.files.length, list.length, data.files.length);
    input.removeAttribute("type");
    out.types.push(input.files);
    input.type = "file";
    out.types.push(input.files.length);
    const switched = document.createElement("input");
    switched.value = "dirty";
    switched.setAttribute("value", "attr");
    switched.type = "file";
    out.modes = [switched.value, switched.defaultValue, switched.getAttribute("value")];
    switched.files = data.files;
    out.modes.push(switched.value);
    switched.type = "text";
    out.modes.push(switched.value, switched.files);
    switched.type = "file";
    out.modes.push(switched.value, switched.files.length);
    switched.type = "checkbox";
    out.modes.push(switched.value);
  }

  // Cloning copies Files; reset and validity read the selected files.
  {
    const data = transfer("a", "b"), form = document.createElement("form");
    const input = fileInput("f");
    form.append(input);
    input.files = data.files;
    const clone = input.cloneNode(), deep = form.cloneNode(true);
    out.clone = [names(clone.files), clone.files === input.files, clone.files[0] === input.files[0], clone.files[0].type, clone.files[0].size,
      clone.files[0].lastModified, clone.value, names(deep.firstChild.files)];
    data.items.add(fileOf("c"));
    out.clone.push(clone.files.length);
    input.required = true;
    out.validity = [input.validity.valueMissing, input.checkValidity(), input.validationMessage];
    const list = input.files;
    form.reset();
    out.validity.push(input.files === list, input.files.length, data.items.length, input.validity.valueMissing, input.checkValidity(), input.validationMessage, input.value);
    const empty = fileInput();
    empty.required = true;
    out.validity.push(empty.validity.valueMissing, empty.validationMessage);
  }

  // Form entry lists.
  {
    const data = transfer("a.txt", "b.txt"), form = document.createElement("form");
    const chosen = fileInput("f"), empty = fileInput("empty"), unnamed = fileInput(), disabled = fileInput("disabled");
    chosen.files = data.files;
    unnamed.files = transfer("n").files;
    disabled.disabled = true;
    disabled.files = transfer("d").files;
    form.append(chosen, empty, unnamed, disabled);
    const before = Date.now(), entries = new FormData(form), after = Date.now();
    const shape = (value) => typeof value === "string" ? value : [Object.prototype.toString.call(value), value.name, value.type, value.size,
      value.lastModified >= before && value.lastModified <= after ? "now" : value.lastModified, value === data.files[0], value === data.files[1], value.webkitRelativePath];
    out.formData = [...entries].map(([name, value]) => [name, shape(value)]);
    const again = new FormData(form);
    out.formData.push([again.get("f") === entries.get("f"), again.get("empty") === entries.get("empty")]);
    const seen = [];
    form.addEventListener("formdata", (event) => { for (const [name, value] of event.formData) seen.push([name, typeof value]); });
    new FormData(form);
    out.formData.push(seen);
  }

  // webkitdirectory, multiple and webkitEntries never fabricate entries.
  {
    const directory = fileInput("d");
    directory.webkitdirectory = true;
    directory.files = transfer("a").files;
    out.directory = [names(directory.files), directory.files[0].webkitRelativePath, directory.webkitEntries.length, directory.value, directory.webkitEntries === directory.webkitEntries];
    const single = fileInput();
    single.files = transfer("a", "b").files;
    out.directory.push(single.multiple, single.files.length, single.value);
  }

  // DataTransfer data store.
  {
    const data = new DataTransfer(), file = fileOf("q", "Application/X");
    out.transfer = [data.dropEffect, data.effectAllowed, data.types, data.items.length, data.files.length, data.files === data.files, data.items === data.items];
    const item = data.items.add(file), text = data.items.add("str", "text/x-a");
    out.transfer.push(item.kind, item.type, text.kind, text.type, item === data.items[0], item.getAsFile() === file, text.getAsFile(), item.webkitGetAsEntry(), text.webkitGetAsEntry(),
      data.types, Object.isFrozen(data.types), data.types === data.types, data.files.length, data.files[0] === file, data.getData("TEXT/X-A"), data.getData("Files"));
    const types = data.types;
    data.items.add(file);
    out.transfer.push(types === data.types, data.items.length, data.files[0] === data.files[1]);
    out.add = [run(() => data.items.add()), run(() => data.items.add({})), run(() => data.items.add("a")), run(() => data.items.add("dup", "text/x-a")),
      run(() => data.items.add("upper", "TEXT/X-A").type), run(() => data.items.add(Symbol(), "t")), run(() => data.items.add("v", Symbol())),
      run(() => data.items.add(5, 6).type), run(() => { const added = data.items.add(file, "x"); return [added.kind, added.type, data.getData("x")]; }),
      run(() => DataTransferItemList.prototype.add.call({}, "a", "b")), run(() => Object.getOwnPropertyDescriptor(DataTransfer.prototype, "items").get.call({})),
      data.types, [...data.items].map((entry) => entry.kind + ":" + entry.type), Object.keys(data.items)];
    out.remove = [run(() => data.items.remove()), run(() => data.items.remove(-1)), data.items.length, run(() => data.items.remove(4294967296)), data.items.length,
      data.items[0].kind, data.items[99], "0" in data.items];
    const detached = data.items[0];
    data.items.clear();
    out.remove.push(data.items.length, data.types, detached.kind, detached.type, detached.getAsFile() === file, detached.webkitGetAsEntry());
    const strings = new DataTransfer();
    out.strings = [run(() => strings.getData()), run(() => strings.setData("x")), run(() => strings.getData(Symbol())), run(() => strings.clearData(Symbol())),
      run(() => { strings.setData("Text/Plain", "1"); strings.setData("text", "2"); return [strings.getData("text/plain"), strings.items.length, strings.types]; }),
      run(() => { strings.setData(" text/plain;charset=utf-8 ", "3"); return [strings.getData("TEXT"), strings.types]; }),
      run(() => { strings.setData({ toString() { return "Q"; } }, 1); return strings.types; }),
      run(() => { strings.setData("URL", "http://a b/\nhttp://c/"); return [strings.getData("url"), strings.getData("text/uri-list"), strings.types]; }),
      run(() => { strings.clearData("url"); return strings.types; }), run(() => { strings.clearData(undefined); return strings.types; }),
      run(() => { strings.clearData(); return [strings.types, strings.items.length]; })];
    const uris = new DataTransfer();
    out.uris = [];
    for (const value of ["not a url\r\nhttp://ok/", "#only", "", "  http://sp/  ", "\nhttp://n/", "javascript:alert(1)", "relative/path", "http://a\rhttp://b", "#c\r\nhttp://x/a b\r\nhttp://y/"]) {
      uris.setData("text/uri-list", value);
      out.uris.push([value, uris.getData("url"), uris.getData("URL")]);
    }
    const effects = new DataTransfer();
    out.effects = [];
    for (const value of ["none", "copy", "link", "move", "bogus"]) { effects.dropEffect = value; out.effects.push(effects.dropEffect); }
    for (const value of ["copyMove", "all", "uninitialized", "bogus"]) { effects.effectAllowed = value; out.effects.push(effects.effectAllowed); }
    out.effects.push(run(() => { effects.dropEffect = Symbol(); }), run(() => { effects.effectAllowed = Symbol(); }),
      run(() => effects.setDragImage(document.body)), run(() => effects.setDragImage(document.body, 1, 2)), run(() => effects.setDragImage(document.body, "a", {})),
      run(() => effects.setDragImage(null, 1, 2)), run(() => effects.setDragImage({}, 1, 2)), run(() => effects.setDragImage(document.body, Symbol(), 1)));
    const plain = new DataTransfer().items.add("hello", "text/plain");
    out.callbacks = [run(() => plain.getAsString()), run(() => plain.getAsString(null)), run(() => plain.getAsString(5)), run(() => plain.getAsString({})),
      run(() => DataTransferItem.prototype.getAsFile.call({})), run(() => plain.getAsFileSystemHandle() instanceof Promise)];
    const order = new DataTransfer();
    order.setData("a", "1");
    order.items.add(file);
    order.setData("b", "2");
    out.order = [order.types, [...order.items].map((entry) => entry.kind + ":" + entry.type)];
    order.items.remove(0);
    out.order.push(order.types, [...order.items].map((entry) => entry.kind + ":" + entry.type));
  }

  // The DataTransfer's FileList is rebuilt from its items on every change.
  {
    const data = transfer("q", "r"), input = fileInput();
    input.files = data.files;
    input.value = "";
    const counts = [data.files.length];
    data.setData("x", "1"); counts.push(data.files.length);
    input.value = ""; data.clearData("x"); counts.push(data.files.length);
    input.value = ""; data.clearData("missing"); counts.push(data.files.length);
    data.items.remove(5); counts.push(data.files.length);
    data.items.add("s", "t"); counts.push(data.files.length);
    data.items.clear(); counts.push(data.files.length, input.files.length);
    const separate = new DataTransfer();
    separate.items.add(fileOf("z"));
    input.files = separate.files;
    data.items.add(fileOf("y"));
    counts.push(names(input.files), names(data.files));
    out.rebuild = counts;
  }
  // Strict-mode and Reflect writes on the indexed exotic objects.
  {
    const one = transfer("a").files;
    out.strictMutation = [run(() => (function () { "use strict"; one[0] = 1; })()), run(() => (function () { "use strict"; one[3] = 1; })()),
      run(() => (function () { "use strict"; delete one[0]; })()), run(() => (function () { "use strict"; return delete one[3]; })()),
      run(() => Reflect.set(one, 0, 1)), run(() => Reflect.set(one, 3, 1)), run(() => Reflect.defineProperty(one, 0, { value: 1 })),
      run(() => Reflect.preventExtensions(one)), Object.isExtensible(one)];
    const items = transfer("a").items;
    out.itemListMutation = [run(() => { items[0] = 1; return items[0].kind; }), run(() => (function () { "use strict"; items[0] = 1; })()),
      run(() => Reflect.deleteProperty(items, 0)), run(() => Object.defineProperty(items, "0", { value: 1 }) && "defined"), Object.getOwnPropertyNames(items),
      JSON.stringify(Object.getOwnPropertyDescriptor(items, "0"), (key, value) => key === "value" ? typeof value : value), items[0] === items[0]];
  }

  // Item callbacks run as posted tasks, after microtasks, in call order.
  return (async () => {
    const strings = new DataTransfer(), first = strings.items.add("1", "a"), second = strings.items.add("2", "b");
    const order = [];
    await new Promise((done) => {
      second.getAsString((value) => order.push("b:" + value));
      first.getAsString(function (value) { "use strict"; order.push(["a:" + value, this === undefined, arguments.length]); });
      Promise.resolve().then(() => order.push("micro"));
      setTimeout(() => { order.push("timeout"); done(); }, 0);
    });
    out.callbackOrder = order;
    const detached = strings.items[0];
    strings.items.clear();
    out.detachedString = await new Promise((done) => { detached.getAsString((value) => done(["called", value])); setTimeout(() => done("not called"), 20); });
    const file = new DataTransfer().items.add(fileOf("f"));
    out.fileString = await new Promise((done) => { file.getAsString(() => done("called")); setTimeout(() => done("not called"), 20); });
    out.handles = [String(await second.getAsFileSystemHandle()), String(await file.getAsFileSystemHandle())];
    return out;
  })();
})()
