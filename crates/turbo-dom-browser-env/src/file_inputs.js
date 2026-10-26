// File-state <input> members, FileList and the constructed DataTransfer data store.
// Files are the embedder's genuine File objects, captured before page code runs. A
// browserless engine has no OS file chooser, drag source or dropped directory, so
// webkitEntries and webkitGetAsEntry() are always empty, exactly as Chrome reports for
// files that were not dropped, and drop/effect state stays "none" outside a drag.
(function () {
  'use strict';
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction, named = ns.makeNamedObject;
  const native = g.__fileInputNative; delete g.__fileInputNative;
  const apply = Reflect.apply, construct = Reflect.construct, define = Object.defineProperty, create = Object.create;
  const freeze = Object.freeze, describe = Object.getOwnPropertyDescriptor, hasOwn = Object.prototype.hasOwnProperty;
  const StringValue = String, NumberValue = Number, TypeErrorValue = TypeError, Exception = g.DOMException;
  const PromiseValue = Promise, resolve = Promise.resolve, values = Array.prototype.values;
  const replaceText = String.prototype.replace, charCode = String.prototype.charCodeAt, split = String.prototype.split;
  const startsWith = String.prototype.startsWith, getAttribute = Element.prototype.getAttribute;
  const checkInput = ns.domAccessor('nodeType', false, 'HTML:input'), nodeType = ns.domAccessor('nodeType', false, 'Node');
  const record = (object) => native('record', object), setRecord = (object, value) => native('set-record', object, value);
  const push = (rows, value) => define(rows, rows.length, {__proto__: null, value, writable: true, enumerable: true, configurable: true});
  const lowerAscii = (text) => apply(replaceText, text, [/[A-Z]/g, (letter) => StringValue.fromCharCode(apply(charCode, letter, [0]) + 32)]);
  const stripAscii = (text) => apply(replaceText, text, [/^[\t\n\f\r ]+|[\t\n\f\r ]+$/g, '']);

  // Web IDL conversions with Blink's messages.
  const failure = (kind, owner, member) => kind === 'set' ? "Failed to set the '" + member + "' property on '" + owner + "': " : "Failed to execute '" + member + "' on '" + owner + "': ";
  const string = (value, prefix) => { if (typeof value === 'symbol') throw new TypeErrorValue(prefix + 'Cannot convert a Symbol value to a string'); return StringValue(value); };
  const numeric = (value, prefix) => { if (typeof value === 'symbol') throw new TypeErrorValue(prefix + 'Cannot convert a Symbol value to a number'); return NumberValue(value); };
  const required = (count, wanted, prefix) => {
    if (count < wanted) throw new TypeErrorValue(prefix + wanted + (wanted === 1 ? ' argument' : ' arguments') + ' required, but only ' + count + ' present.');
  };
  const isElement = (value) => { try { return apply(nodeType, value, []) === 1; } catch { return false; } };
  const isInput = (value) => { try { apply(checkInput, value, []); return true; } catch { return false; } };
  const fileState = (element) => lowerAscii(apply(getAttribute, element, ['type']) || '') === 'file';

  // The embedder's File interface, captured before guest code (see capture_file_host).
  let host = null;
  g.__fileInputHost = function (File) {
    if (typeof File !== 'function') return;
    const prototype = File.prototype, blob = Object.getPrototypeOf(prototype);
    const getter = (object, name) => { const descriptor = object && describe(object, name); return descriptor && typeof descriptor.get === 'function' ? descriptor.get : null; };
    const branded = getter(prototype, 'name');
    const read = (accessor, name) => accessor ? (file) => apply(accessor, file, []) : (file) => file[name];
    host = {
      File,
      isFile: branded ? (value) => { if (value === null || typeof value !== 'object') return false; try { apply(branded, value, []); return true; } catch { return false; } }
        : (value) => value instanceof File,
      name: read(branded, 'name'), type: read(getter(blob, 'type'), 'type'), lastModified: read(getter(prototype, 'lastModified'), 'lastModified'),
    };
  };
  const isFile = (value) => host !== null && host.isFile(value);
  const newFile = (parts, name, type, lastModified) => {
    const options = {__proto__: null, type};
    if (lastModified !== undefined) options.lastModified = lastModified;
    return construct(host.File, [parts, name, options]);
  };
  const cloneFile = (file) => newFile([file], host.name(file), host.type(file), host.lastModified(file));

  // A named exotic object over a live row array: indices only, no setter or deleter.
  const indexOf = (key) => { const index = NumberValue(key); return index >= 0 && index < 4294967295 && index % 1 === 0 && StringValue(index) === key ? index : -1; };
  const indexed = (prototype, interfaceName, length, item) => named(prototype, function (operation, key) {
    if (operation === 'keys') return [];
    if (operation === 'indices') { const keys = []; for (let at = 0; at < length(); at++) push(keys, at); return keys; }
    const index = indexOf(key);
    if (index < 0) return undefined;
    const present = index < length();
    if (operation === 'define') throw new TypeErrorValue("Failed to set an indexed property [" + index + "] on '" + interfaceName + "': Index property setter is not supported.");
    if (operation === 'set') return false;
    if (!present) return undefined;
    if (operation === 'get') return item(index);
    if (operation === 'has') return true;
    if (operation === 'descriptor') return {__proto__: null, value: item(index), writable: false, enumerable: true, configurable: true};
    if (operation === 'delete') return false;
    return undefined;
  }, true);

  const interfaceObject = (name, length, body, constructible) => {
    const constructor = make(constructible || function () {
      if (new.target === undefined) throw new TypeErrorValue('Illegal constructor');
      throw new TypeErrorValue("Failed to construct '" + name + "': Illegal constructor");
    }, name, length, true, true, false);
    const prototype = constructor.prototype, own = describe(prototype, 'constructor');
    delete prototype.constructor;
    body(prototype);
    define(prototype, 'constructor', own);
    define(prototype, Symbol.toStringTag, {__proto__: null, value: name, configurable: true});
    define(constructor, 'prototype', {__proto__: null, writable: false});
    define(g, name, {__proto__: null, value: constructor, writable: true, configurable: true});
    return prototype;
  };
  const method = (prototype, name, length, fn) => define(prototype, name, {__proto__: null, value: make(fn, name, length), writable: true, enumerable: true, configurable: true});
  const getter = (prototype, name, fn, setter) => define(prototype, name, {__proto__: null, get: make(fn, 'get ' + name, 0), set: setter ? make(setter, 'set ' + name, 1) : undefined, enumerable: true, configurable: true});
  const branded = (kind) => (receiver) => { const value = record(receiver); if (!value || value.kind !== kind) throw new TypeErrorValue('Illegal invocation'); return value; };

  // FileList.
  const listRecord = branded('FileList');
  const FileListPrototype = interfaceObject('FileList', 0, (prototype) => {
    getter(prototype, 'length', function () { return listRecord(this).files.length; });
    method(prototype, 'item', 1, function (index) {
      const files = listRecord(this).files, prefix = failure('execute', 'FileList', 'item');
      required(arguments.length, 1, prefix);
      index = numeric(index, prefix) >>> 0;
      return index < files.length ? files[index] : null;
    });
  });
  define(FileListPrototype, Symbol.iterator, {__proto__: null, value: values, writable: true, configurable: true});
  const createFileList = (files) => {
    const state = {__proto__: null, kind: 'FileList', files};
    const list = indexed(FileListPrototype, 'FileList', () => state.files.length, (index) => state.files[index]);
    setRecord(list, state);
    return list;
  };
  const replaceFiles = (list, files) => { const rows = listRecord(list).files; rows.length = 0; for (let at = 0; at < files.length; at++) push(rows, files[at]); };

  // Per-input selected files: a FileList the input owns, or one adopted from `files = list`.
  const elementState = (element) => {
    let shared = record(element);
    if (!shared) { shared = create(null); shared.kind = 'Element'; setRecord(element, shared); }
    if (!apply(hasOwn, shared, ['fileList'])) define(shared, 'fileList', {__proto__: null, value: null, writable: true});
    return shared;
  };
  const selected = (element) => { const state = record(element); return state && state.fileList ? listRecord(state.fileList).files : null; };
  const ownList = (element) => { const state = elementState(element); if (!state.fileList) state.fileList = createFileList([]); return state.fileList; };
  const clearSelection = (element) => { const files = selected(element); if (files) files.length = 0; };

  const inputPrototype = g.HTMLInputElement.prototype;
  getter(inputPrototype, 'files', function () {
    apply(checkInput, this, []);
    return fileState(this) ? ownList(this) : null;
  }, function (value) {
    apply(checkInput, this, []);
    if (value === null || value === undefined) return;
    if (!record(value) || record(value).kind !== 'FileList') throw new TypeErrorValue(failure('set', 'HTMLInputElement', 'files') + "Failed to convert value to 'FileList'.");
    if (fileState(this)) elementState(this).fileList = value;
  });
  getter(inputPrototype, 'webkitEntries', function () { apply(checkInput, this, []); return freeze([]); });

  // Value, reset and clone steps for the filename value mode, called by the form-control core.
  ns.registerFileControls({
    value(element) {
      if (!isInput(element) || !fileState(element)) return undefined;
      const files = selected(element);
      return files && files.length && host ? 'C:\\fakepath\\' + host.name(files[0]) : '';
    },
    setValue(element, value) {
      if (!isInput(element) || !fileState(element)) return false;
      value = value === null ? '' : string(value, failure('set', 'HTMLInputElement', 'value'));
      if (value !== '') throw new Exception(failure('set', 'HTMLInputElement', 'value') + 'This input element accepts a filename, which may only be programmatically set to the empty string.', 'InvalidStateError');
      clearSelection(element);
      return true;
    },
    reset(element) { if (isInput(element) && fileState(element)) clearSelection(element); },
    clone(source, clone) {
      const files = isInput(source) && fileState(source) ? selected(source) : null;
      if (!files || !files.length || !host) return;
      const copies = [];
      for (let at = 0; at < files.length; at++) push(copies, cloneFile(files[at]));
      elementState(clone).fileList = createFileList(copies);
    },
  });
  // Entering the file state discards the dirty value; leaving it detaches the FileList.
  native('type-hook', (element) => {
    if (!isInput(element)) return;
    if (fileState(element)) ns.resetControlValue(element);
    else { const state = record(element); if (state && state.fileList) state.fileList = null; }
  });
  // A file control contributes each selected File, or one empty octet-stream File.
  ns.registerFileEntries((element) => {
    if (!host) return null;
    const files = selected(element), entries = [];
    if (files && files.length) for (let at = 0; at < files.length; at++) push(entries, files[at]);
    else push(entries, newFile([], '', 'application/octet-stream'));
    return entries;
  });

  // DataTransfer: a read/write data store that is never in drag-and-drop mode.
  const transferRecord = branded('DataTransfer');
  const normalize = (type) => {
    type = lowerAscii(stripAscii(type));
    if (type === 'text' || apply(startsWith, type, ['text/plain;'])) return 'text/plain';
    return type === 'url' ? 'text/uri-list' : type;
  };
  const changed = (state) => {
    state.types = null;
    if (!state.files) return;
    const files = [];
    for (let at = 0; at < state.items.length; at++) if (state.items[at].kind === 'file') push(files, state.items[at].file);
    replaceFiles(state.files, files);
  };
  const removeAt = (items, index) => { for (let at = index; at < items.length - 1; at++) items[at] = items[at + 1]; items.length--; };
  const removeStrings = (state, type) => {
    let removed = false;
    for (let at = state.items.length - 1; at >= 0; at--) {
      const entry = state.items[at];
      if (entry.kind === 'string' && (type === undefined || entry.type === type)) { removeAt(state.items, at); removed = true; }
    }
    if (removed) changed(state);
  };
  const addString = (state, data, type) => { push(state.items, freeze({__proto__: null, kind: 'string', type, data})); changed(state); };
  const firstURL = (list) => {
    const lines = apply(split, list, ['\n']);
    for (let at = 0; at < lines.length; at++) {
      const line = stripAscii(lines[at]);
      if (line === '' || line[0] === '#') continue;
      const url = native('url', line);
      if (url !== undefined) return url;
    }
    return '';
  };
  const effect = (member) => function (value) { transferRecord(this); string(value, failure('set', 'DataTransfer', member)); };
  interfaceObject('DataTransfer', 0, (prototype) => {
    getter(prototype, 'dropEffect', function () { transferRecord(this); return 'none'; }, effect('dropEffect'));
    getter(prototype, 'effectAllowed', function () { transferRecord(this); return 'none'; }, effect('effectAllowed'));
    getter(prototype, 'items', function () { return createItemList(transferRecord(this)); });
    getter(prototype, 'types', function () {
      const state = transferRecord(this);
      if (!state.types) {
        const types = [];
        let files = false;
        for (let at = 0; at < state.items.length; at++) { const entry = state.items[at]; if (entry.kind === 'file') files = true; else push(types, entry.type); }
        if (files) push(types, 'Files');
        state.types = freeze(types);
      }
      return state.types;
    });
    getter(prototype, 'files', function () {
      const state = transferRecord(this);
      if (!state.files) { state.files = createFileList([]); changed(state); }
      return state.files;
    });
    method(prototype, 'clearData', 0, function (format) {
      const state = transferRecord(this);
      removeStrings(state, format === undefined ? undefined : normalize(string(format, failure('execute', 'DataTransfer', 'clearData'))));
    });
    method(prototype, 'getData', 1, function (format) {
      const state = transferRecord(this), prefix = failure('execute', 'DataTransfer', 'getData');
      required(arguments.length, 1, prefix);
      format = lowerAscii(stripAscii(string(format, prefix)));
      const type = normalize(format);
      for (let at = 0; at < state.items.length; at++) {
        const entry = state.items[at];
        if (entry.kind === 'string' && entry.type === type) return format === 'url' ? firstURL(entry.data) : entry.data;
      }
      return '';
    });
    method(prototype, 'setData', 2, function (format, data) {
      const state = transferRecord(this), prefix = failure('execute', 'DataTransfer', 'setData');
      required(arguments.length, 2, prefix);
      const type = normalize(string(format, prefix));
      data = string(data, prefix);
      removeStrings(state, type);
      addString(state, data, type);
    });
    method(prototype, 'setDragImage', 3, function (image, x, y) {
      transferRecord(this);
      const prefix = failure('execute', 'DataTransfer', 'setDragImage');
      required(arguments.length, 3, prefix);
      if (!isElement(image)) throw new TypeErrorValue(prefix + "parameter 1 is not of type 'Element'.");
      numeric(x, prefix);
      numeric(y, prefix);
    });
  }, function DataTransfer() {
    if (new.target === undefined) throw new TypeErrorValue("Failed to construct 'DataTransfer': Please use the 'new' operator, this DOM object constructor cannot be called as a function.");
    setRecord(this, {__proto__: null, kind: 'DataTransfer', items: [], files: null, types: null});
  });

  // DataTransferItemList and DataTransferItem wrappers are new on every access, like Blink's.
  const itemListRecord = branded('DataTransferItemList'), itemRecord = branded('DataTransferItem');
  let ItemListPrototype, ItemPrototype;
  const createItem = (entry) => { const item = create(ItemPrototype); setRecord(item, {__proto__: null, kind: 'DataTransferItem', entry}); return item; };
  const createItemList = (owner) => {
    const list = indexed(ItemListPrototype, 'DataTransferItemList', () => owner.items.length, (index) => createItem(owner.items[index]));
    setRecord(list, {__proto__: null, kind: 'DataTransferItemList', owner});
    return list;
  };
  ItemListPrototype = interfaceObject('DataTransferItemList', 0, (prototype) => {
    getter(prototype, 'length', function () { return itemListRecord(this).owner.items.length; });
    method(prototype, 'add', 1, function (data, type) {
      const owner = itemListRecord(this).owner, prefix = failure('execute', 'DataTransferItemList', 'add');
      required(arguments.length, 1, prefix);
      if (arguments.length === 1) {
        if (!isFile(data)) throw new TypeErrorValue(prefix + "parameter 1 is not of type 'File'.");
        const entry = freeze({__proto__: null, kind: 'file', type: host.type(data), file: data});
        push(owner.items, entry);
        changed(owner);
        return createItem(entry);
      }
      data = string(data, prefix);
      type = string(type, prefix);
      for (let at = 0; at < owner.items.length; at++) {
        const entry = owner.items[at];
        if (entry.kind === 'string' && entry.type === type) throw new Exception(prefix + "An item already exists for type '" + type + "'.", 'NotSupportedError');
      }
      addString(owner, data, type);
      return createItem(owner.items[owner.items.length - 1]);
    });
    method(prototype, 'clear', 0, function () {
      const owner = itemListRecord(this).owner;
      if (owner.items.length) { owner.items.length = 0; changed(owner); }
    });
    method(prototype, 'remove', 1, function (index) {
      const owner = itemListRecord(this).owner, prefix = failure('execute', 'DataTransferItemList', 'remove');
      required(arguments.length, 1, prefix);
      index = numeric(index, prefix) | 0;
      if (index >= 0 && index < owner.items.length) { removeAt(owner.items, index); changed(owner); }
    });
  });
  define(ItemListPrototype, Symbol.iterator, {__proto__: null, value: values, writable: true, configurable: true});
  ItemPrototype = interfaceObject('DataTransferItem', 0, (prototype) => {
    getter(prototype, 'kind', function () { return itemRecord(this).entry.kind; });
    getter(prototype, 'type', function () { return itemRecord(this).entry.type; });
    method(prototype, 'getAsFile', 0, function () { const entry = itemRecord(this).entry; return entry.kind === 'file' ? entry.file : null; });
    method(prototype, 'getAsString', 1, function (callback) {
      const entry = itemRecord(this).entry, prefix = failure('execute', 'DataTransferItem', 'getAsString');
      required(arguments.length, 1, prefix);
      if (callback === null || callback === undefined) return;
      if (typeof callback !== 'function') throw new TypeErrorValue(prefix + "parameter 1 is not of type 'Function'.");
      if (entry.kind === 'string') native('schedule', () => apply(callback, undefined, [entry.data]));
    });
    method(prototype, 'webkitGetAsEntry', 0, function () { itemRecord(this); return null; });
    method(prototype, 'getAsFileSystemHandle', 0, function () { itemRecord(this); return apply(resolve, PromiseValue, [undefined]); });
  });
})();
