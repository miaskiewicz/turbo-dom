(async () => {
  // Editing control: Document.execCommand / queryCommand* (the DOM command table, argument
  // conversion, brands, non-HTML documents), designMode / contentEditable, and a fixture matrix of
  // commands over contenteditable content recording the return value, the resulting DOM with the
  // selection marked, and the trusted event log (beforeinput / input / clipboard / selectstart).
  // Fixture markers: text positions [ ] |, element positions { } ^ (anchor, focus, caret);
  // adjacent text nodes are split by ‖ and empty text nodes print as ∅.
  const out = {}, body = document.body;
  const run = f => { try { const v = f(); return v === undefined ? 'ok' : ['ok', v]; } catch (e) { return e.name + ': ' + e.message; } };
  const describe = (o, k) => { const x = Object.getOwnPropertyDescriptor(o, k); if (!x) return null; const r = {}; for (const f in x) r[f] = typeof x[f] === 'function' ? [x[f].name, x[f].length] : x[f]; return r; };
  const esc = s => s.replace(/&/g, '&amp;').replace(/ /g, '&nbsp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  function place(root, html) {
    root.innerHTML = html.replace(/[{}^]/g, m => '<!--' + m + '-->');
    let start = null, end = null;
    const walk = node => {
      for (let c = node.firstChild; c;) {
        const next = c.nextSibling;
        if (c.nodeType === 8 && /^[{}^]$/.test(c.data)) { const p = [node, Array.prototype.indexOf.call(node.childNodes, c)]; if (c.data === '{') start = p; else if (c.data === '}') end = p; else start = end = p; c.remove(); }
        else if (c.nodeType === 3) { let t = c.data, i; while ((i = t.search(/[[\]|]/)) >= 0) { const ch = t[i]; t = t.slice(0, i) + t.slice(i + 1); c.data = t; const p = [c, i]; if (ch === '[') start = p; else if (ch === ']') end = p; else start = end = p; } }
        else walk(c);
        c = next;
      }
    };
    walk(root);
    const s = getSelection();
    if (start) s.setBaseAndExtent(start[0], start[1], (end || start)[0], (end || start)[1]); else s.removeAllRanges();
  }
  function mark(root) {
    const s = getSelection(), pts = [];
    if (s.rangeCount) {
      const text = n => n.nodeType === 3;
      if (s.isCollapsed) pts.push([s.anchorNode, s.anchorOffset, text(s.anchorNode) ? '|' : '^']);
      else { pts.push([s.anchorNode, s.anchorOffset, text(s.anchorNode) ? '[' : '{']); pts.push([s.focusNode, s.focusOffset, text(s.focusNode) ? ']' : '}']); }
    }
    const at = (n, o) => pts.filter(p => p[0] === n && p[1] === o).map(p => p[2]).join('');
    const ser = node => {
      if (node.nodeType === 3) { let r = ''; const d = node.data; if (!d.length) r = '∅'; for (let i = 0; i <= d.length; i++) { r += at(node, i); if (i < d.length) r += esc(d[i]); } return r; }
      if (node.nodeType === 8) return '<!--' + node.data + '-->';
      if (node.nodeType !== 1) return '';
      let r = ''; const kids = Array.from(node.childNodes);
      for (let i = 0; i <= kids.length; i++) {
        r += at(node, i);
        if (i < kids.length) {
          const k = kids[i];
          if (i && k.nodeType === 3 && kids[i - 1].nodeType === 3) r += '‖';
          if (k.nodeType === 1) { const tag = k.localName; r += '<' + tag + Array.from(k.attributes).map(a => ' ' + a.name + '="' + a.value.replace(/"/g, '&quot;') + '"').join('') + '>'; if (!/^(br|img|hr|input|wbr)$/.test(tag)) r += ser(k) + '</' + tag + '>'; } else r += ser(k);
        }
      }
      return r;
    };
    const outside = pts.filter(p => !root.contains(p[0])).map(p => p[2] + (p[0] && (p[0].id || p[0].nodeName)) + ':' + p[1]);
    return ser(root) + (outside.length ? ' @' + outside.join(',') : '');
  }
  const log = [];
  const who = t => t === document ? '#document' : t === window ? 'window' : t && (t.id || t.nodeName);
  const fields = e => { const r = [e.type, who(e.target), e.isTrusted, e.bubbles, e.cancelable, e.composed]; if (e instanceof InputEvent) { const ranges = e.getTargetRanges(); r.push(e.inputType, e.data, e.isComposing, e.dataTransfer, ranges.length, ranges.map(x => [x.constructor.name, who(x.startContainer), x.startOffset, who(x.endContainer), x.endOffset]).join('/')); } return r.join(','); };
  for (const t of ['beforeinput', 'input', 'copy', 'cut', 'paste', 'beforecopy', 'beforecut', 'beforepaste', 'selectstart', 'change', 'textInput', 'select']) window.addEventListener(t, e => log.push(fields(e)), true);
  const exec = (cmd, value, ui) => { try { return value === undefined ? document.execCommand(cmd) : document.execCommand(cmd, !!ui, value); } catch (e) { return e.name + ': ' + e.message; } };
  function run1(html, cmd, value, opts = {}) {
    place(body, html);
    const host = body.firstChild;
    if (opts.focus) host.focus();
    if (opts.css) document.execCommand('styleWithCSS', false, 'true');
    log.length = 0;
    const ret = Array.isArray(cmd) ? cmd.map(c => Array.isArray(c) ? exec(...c) : exec(c)) : exec(cmd, value);
    const r = [ret, mark(body), log.slice()];
    document.execCommand('styleWithCSS', false, 'false');
    document.execCommand('defaultParagraphSeparator', false, 'div');
    return r;
  }

  out.interface = {
    document: Reflect.ownKeys(Document.prototype).map(String).filter(k => /ommand|designMode/.test(k)),
    descriptors: Object.fromEntries(['execCommand', 'queryCommandEnabled', 'queryCommandIndeterm', 'queryCommandState', 'queryCommandSupported', 'queryCommandValue', 'designMode'].map(k => [k, describe(Document.prototype, k)])),
    html: Object.fromEntries(['contentEditable', 'isContentEditable'].map(k => [k, describe(HTMLElement.prototype, k)])),
    getTargetRanges: describe(InputEvent.prototype, 'getTargetRanges'),
    own: ['execCommand', 'queryCommandSupported', 'designMode'].filter(k => Object.prototype.hasOwnProperty.call(document, k)),
  };
  // Every DOM-exposed command and a sample of menu-only / unknown names, with no selection.
  const names = 'AlignCenter BackColor BackwardDelete Bold Copy CreateLink Cut DefaultParagraphSeparator Delete DeleteBackward DeleteWordBackward FindString FontName FontSize FontSizeDelta ForeColor FormatBlock ForwardDelete HiliteColor IgnoreSpelling Indent InsertBacktab InsertHorizontalRule InsertHTML InsertImage InsertLineBreak InsertNestedOrderedList InsertNestedUnorderedList InsertNewline InsertOrderedList InsertParagraph InsertTab InsertText InsertUnorderedList Italic JustifyCenter JustifyFull JustifyLeft JustifyNone JustifyRight MakeTextWritingDirectionLeftToRight MoveDown Outdent OverWrite Paste PasteAndMatchStyle PasteGlobalSelection Print Redo RemoveFormat Scroll SelectAll SelectLine Strikethrough StyleWithCSS Subscript Superscript ToggleBold Transpose Underline Undo Unlink Unscript Unselect UseCSS Yank decreaseFontSize insertBrOnReturn foo'.split(' ');
  getSelection().removeAllRanges();
  out.table = Object.fromEntries(names.map(n => [n, ['queryCommandSupported', 'queryCommandEnabled', 'queryCommandState', 'queryCommandIndeterm', 'queryCommandValue'].map(f => run(() => document[f](n))).concat([run(() => document.execCommand(lowerFirst(n)))])]));
  function lowerFirst(n) { return n === 'Print' ? 'nonexistent-print-guard' : n; }
  getSelection().removeAllRanges(); document.execCommand('styleWithCSS', false, 'false'); document.execCommand('defaultParagraphSeparator', false, 'div');
  const xml = document.implementation.createDocument(null, 'r'), other = document.implementation.createHTMLDocument('');
  other.body.innerHTML = '<div contenteditable>hi</div>';
  out.documents = {
    xml: ['execCommand', 'queryCommandSupported', 'queryCommandEnabled', 'queryCommandState', 'queryCommandValue', 'queryCommandIndeterm'].map(f => run(() => Document.prototype[f].call(xml, 'bold'))),
    inert: run(() => [other.execCommand('insertText', false, 'X'), other.queryCommandSupported('bold'), other.queryCommandEnabled('bold'), other.queryCommandEnabled('selectAll'), other.queryCommandValue('defaultParagraphSeparator'), other.queryCommandState('styleWithCSS'), other.designMode]),
  };
  out.args = [
    run(() => document.execCommand()),
    run(() => document.queryCommandState()),
    run(() => document.queryCommandValue()),
    run(() => document.execCommand(Symbol())),
    run(() => document.execCommand('bold', Symbol())),
    run(() => document.execCommand('bold', false, Symbol())),
    run(() => document.queryCommandEnabled(Symbol())),
    run(() => Document.prototype.execCommand.call({}, 'bold')),
    run(() => Document.prototype.queryCommandValue.call(body, 'bold')),
    run(() => Document.prototype.queryCommandSupported.call(undefined, 'bold')),
    (() => { const order = []; try { document.execCommand({ toString() { order.push('cmd'); return 'bold'; } }, { valueOf() { order.push('ui'); return 1; } }, { toString() { order.push('value'); return 'v'; } }); } catch (e) { order.push(e.name); } return order; })(),
    (() => { const order = []; try { Document.prototype.execCommand.call(xml, { toString() { order.push('cmd'); return 'bold'; } }, false, { toString() { order.push('value'); return 'v'; } }); } catch (e) { order.push(e.name); } return order; })(),
    run(() => document.queryCommandSupported('BOLD')),
    run(() => document.queryCommandSupported('bold ')),
    run(() => document.queryCommandSupported('ınserttext')),
  ];
  const dm = [];
  for (const v of ['ON', 'off', 'On ', 'invalid', 'on', 'OFF']) { document.designMode = v; dm.push(document.designMode); }
  document.designMode = 'off';
  out.designMode = dm;
  out.contentEditable = ['true', 'false', 'plaintext-only', 'inherit', 'TRUE', 'Plaintext-Only', 'bogus', '', 'true '].map(v => { const el = document.createElement('div'); el.setAttribute('contenteditable', 'x'); try { el.contentEditable = v; return [el.getAttribute('contenteditable'), el.contentEditable, el.isContentEditable]; } catch (e) { return e.name + ': ' + e.message; } });
  out.contentEditableRead = ['', 'true', 'TRUE', 'false', 'plaintext-only', 'PLAINTEXT-ONLY', 'bogus', null].map(v => { const el = document.createElement('div'); if (v !== null) el.setAttribute('contenteditable', v); const child = document.createElement('span'); el.append(child); return [el.contentEditable, el.isContentEditable, child.isContentEditable]; });
  out.contentEditableBrand = [run(() => Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'contentEditable').get.call({})), run(() => Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'isContentEditable').get.call(document.createElementNS('http://www.w3.org/2000/svg', 'svg'))), run(() => { document.createElement('div').contentEditable = Symbol(); })];

  const C = [
    // insertText
    ['<div id=e contenteditable>he|llo</div>', 'insertText', 'X'],
    ['<div id=e contenteditable>he[ll]o</div>', 'insertText', 'X'],
    ['<div id=e contenteditable>hello|</div>', 'insertText', ' '],
    ['<div id=e contenteditable>|hello</div>', 'insertText', ' '],
    ['<div id=e contenteditable>he|llo</div>', 'insertText', ' '],
    ['<div id=e contenteditable>he|llo</div>', 'insertText', '  a  b'],
    ['<div id=e contenteditable>he |llo</div>', 'insertText', ' '],
    ['<div id=e contenteditable>he&nbsp;|llo</div>', 'insertText', 'x'],
    ['<div id=e contenteditable>he|llo</div>', 'insertText', '   '],
    ['<div id=e contenteditable>hello|</div>', 'insertText', '    '],
    ['<div id=e contenteditable>|hello</div>', 'insertText', '   '],
    ['<div id=e contenteditable>he|llo</div>', 'insertText', 'a  '],
    ['<div id=e contenteditable>hello|</div>', 'insertText', 'a b '],
    ['<div id=e contenteditable>a <b>|b</b></div>', 'insertText', ' '],
    ['<div id=e contenteditable><b>a</b>| b</div>', 'insertText', ' '],
    ['<div id=e contenteditable>a|&nbsp;&nbsp;b</div>', 'insertText', 'x'],
    ['<div id=e contenteditable>{}<br></div>', 'insertText', 'X'],
    ['<div id=e contenteditable><p>^<br></p></div>', 'insertText', 'X'],
    ['<div id=e contenteditable>^hello</div>', 'insertText', 'X'],
    ['<div id=e contenteditable>hello^</div>', 'insertText', 'X'],
    ['<div id=e contenteditable><b>hello</b>^</div>', 'insertText', 'X'],
    ['<div id=e contenteditable>he<b>|llo</b></div>', 'insertText', 'X'],
    ['<div id=e contenteditable>^</div>', 'insertText', 'X'],
    ['<div id=e contenteditable>a<b>[b</b>c]d</div>', 'insertText', 'X'],
    ['<div id=e contenteditable>a<b>[b]</b>c</div>', 'insertText', 'X'],
    ['<div id=e contenteditable>he|llo</div>', 'insertText', 'a\nb'],
    ['<div id=e contenteditable>he|llo</div>', 'insertText', '\t'],
    ['<div id=e contenteditable>he|llo</div>', 'insertText', ''],
    ['<div id=e contenteditable>he[ll]o</div>', 'insertText', ''],
    ['<div id=e contenteditable>he[ll]o</div>', 'insertText', ' '],
    ['<div id=e contenteditable><p>a[b</p><p>c]d</p></div>', 'insertText', 'X'],
    ['<div id=e contenteditable>he|llo</div>', 'INSERTtext', 'X'],
    ['<div id=e contenteditable>he|llo</div>', 'insertText', 5],
    ['<div id=e contenteditable>he|llo</div>', 'insertText', null],
    ['<div id=e contenteditable>he|llo</div>', 'insertText', undefined],
    ['<div id=e contenteditable>he|llo</div>', 'insertText', '😀'],
    // typing style
    ['<div id=e contenteditable>he|llo</div>', [['bold'], ['insertText', 'X'], ['insertText', 'Y']]],
    ['<div id=e contenteditable>he|llo</div>', [['bold'], ['italic'], ['insertText', 'X']]],
    ['<div id=e contenteditable><b>he|llo</b></div>', [['bold'], ['insertText', 'X']]],
    ['<div id=e contenteditable>he|llo</div>', [['bold'], ['bold'], ['insertText', 'X']]],
    ['<div id=e contenteditable>he|llo</div>', [['italic'], ['insertText', 'X']]],
    ['<div id=e contenteditable>he|llo</div>', [['underline'], ['insertText', 'X']]],
    ['<div id=e contenteditable>he|llo</div>', [['foreColor', 'red'], ['insertText', 'X']]],
    ['<div id=e contenteditable>he|llo</div>', [['createLink', 'http://x/'], ['insertText', 'X']]],
    ['<div id=e contenteditable>he|llo</div>', [['bold'], ['insertText', 'X']], undefined, { css: 1 }],
    // insertHTML / insertImage / insertHorizontalRule
    ['<div id=e contenteditable>he|llo</div>', 'insertHTML', '<b>X</b>'],
    ['<div id=e contenteditable>he[ll]o</div>', 'insertHTML', '<i>X</i>y'],
    ['<div id=e contenteditable>he|llo</div>', 'insertHTML', '<p>X</p>'],
    ['<div id=e contenteditable>he|llo</div>', 'insertHTML', 'X<br>Y'],
    ['<div id=e contenteditable>he[ll]o</div>', 'insertHTML', ''],
    ['<div id=e contenteditable>he|llo</div>', 'insertHTML', '<span style="color: red">X</span>'],
    ['<div id=e contenteditable>he|llo</div>', 'insertHTML', '<script>1</script>X'],
    ['<div id=e contenteditable>a|b</div>', 'insertHTML', 'X'],
    ['<div id=e contenteditable>a|b</div>', 'insertHTML', 'X Y'],
    ['<div id=e contenteditable>a|b</div>', 'insertHTML', ' X '],
    ['<div id=e contenteditable>a|b</div>', 'insertHTML', '<div>X</div>'],
    ['<div id=e contenteditable>a|b</div>', 'insertHTML', '<p>X</p><p>Y</p>'],
    ['<div id=e contenteditable><p>a|b</p></div>', 'insertHTML', '<p>X</p>'],
    ['<div id=e contenteditable>a|b</div>', 'insertHTML', '<img src=x>'],
    ['<div id=e contenteditable>^</div>', 'insertHTML', 'X'],
    ['<div id=e contenteditable>a|b</div>', 'insertHTML', '<!--c-->X'],
    ['<div id=e contenteditable>a|b</div>', 'insertHTML', '<b></b>'],
    ['<div id=e contenteditable>he|llo</div>', 'insertHorizontalRule'],
    ['<div id=e contenteditable>he|llo</div>', 'insertHorizontalRule', 'rule'],
    ['<div id=e contenteditable>he|llo</div>', 'insertImage', 'http://x/i.png'],
    ['<div id=e contenteditable>he[ll]o</div>', 'insertImage', 'http://x/i.png'],
    ['<div id=e contenteditable>he|llo</div>', 'insertImage', ''],
    // delete / forwardDelete
    ['<div id=e contenteditable>he|llo</div>', 'delete'],
    ['<div id=e contenteditable>he[ll]o</div>', 'delete'],
    ['<div id=e contenteditable>|hello</div>', 'delete'],
    ['<div id=e contenteditable><p>ab</p><p>|cd</p></div>', 'delete'],
    ['<div id=e contenteditable><p>a[b</p><p>c]d</p></div>', 'delete'],
    ['<div id=e contenteditable><p>a[b</p><p>cd</p><p>e]f</p></div>', 'delete'],
    ['<div id=e contenteditable>ab<br>|cd</div>', 'delete'],
    ['<div id=e contenteditable>a<b>b</b>|c</div>', 'delete'],
    ['<div id=e contenteditable>a<b>b|</b>c</div>', 'delete'],
    ['<div id=e contenteditable>a|</div>', 'delete'],
    ['<div id=e contenteditable>[hello]</div>', 'delete'],
    ['<div id=e contenteditable><p>[hello]</p></div>', 'delete'],
    ['<div id=e contenteditable><p>[a</p><p>b]</p></div>', 'delete'],
    ['<div id=e contenteditable>a<b>[b]</b>c</div>', 'delete'],
    ['<div id=e contenteditable><p>a</p><p>^<br></p></div>', 'delete'],
    ['<div id=e contenteditable>a<img src=x>|b</div>', 'delete'],
    ['<div id=e contenteditable><p>ab</p>|cd</div>', 'delete'],
    ['<div id=e contenteditable>ab<p>|cd</p></div>', 'delete'],
    ['<div id=e contenteditable><ul><li>a</li><li>|b</li></ul></div>', 'delete'],
    ['<div id=e contenteditable>a [b] c</div>', 'delete'],
    ['<div id=e contenteditable>a b| c</div>', 'delete'],
    ['<div id=e contenteditable>a  |b</div>', 'delete'],
    ['<div id=e contenteditable>&#x1F600;|x</div>', 'delete'],
    ['<div id=e contenteditable>é|x</div>', 'delete'],
    ['<div id=e contenteditable>x|é</div>', 'forwardDelete'],
    ['<div id=e contenteditable>he|llo</div>', 'forwardDelete'],
    ['<div id=e contenteditable><p>ab|</p><p>cd</p></div>', 'forwardDelete'],
    ['<div id=e contenteditable>hello|</div>', 'forwardDelete'],
    ['<div id=e contenteditable><p>a|</p><p><br></p></div>', 'forwardDelete'],
    ['<div id=e contenteditable>a|<img src=x>b</div>', 'forwardDelete'],
    ['<div id=e contenteditable>ab|<br>cd</div>', 'forwardDelete'],
    ['<div id=e contenteditable>he[ll]o</div>', 'forwardDelete'],
    // insertParagraph / insertLineBreak
    ['<div id=e contenteditable>he|llo</div>', 'insertParagraph'],
    ['<div id=e contenteditable>hello|</div>', 'insertParagraph'],
    ['<div id=e contenteditable>|hello</div>', 'insertParagraph'],
    ['<div id=e contenteditable><p>he|llo</p></div>', 'insertParagraph'],
    ['<div id=e contenteditable><p>hello|</p></div>', 'insertParagraph'],
    ['<div id=e contenteditable><p>|hello</p></div>', 'insertParagraph'],
    ['<div id=e contenteditable><p><b>he|llo</b></p></div>', 'insertParagraph'],
    ['<div id=e contenteditable><p id=q class=c>he|llo</p></div>', 'insertParagraph'],
    ['<div id=e contenteditable>he[ll]o</div>', 'insertParagraph'],
    ['<div id=e contenteditable><ul><li>he|llo</li></ul></div>', 'insertParagraph'],
    ['<div id=e contenteditable><ul><li>a</li><li>^<br></li></ul></div>', 'insertParagraph'],
    ['<div id=e contenteditable><h1>hello|</h1></div>', 'insertParagraph'],
    ['<div id=e contenteditable><h1>he|llo</h1></div>', 'insertParagraph'],
    ['<div id=e contenteditable><table><tr><td>a|b</td></tr></table></div>', 'insertParagraph'],
    ['<div id=e contenteditable><pre>he|llo</pre></div>', 'insertParagraph'],
    ['<div id=e contenteditable>he|llo</div>', [['defaultParagraphSeparator', 'p'], ['insertParagraph']]],
    ['<div id=e contenteditable>he|llo</div>', 'insertLineBreak'],
    ['<div id=e contenteditable>hello|</div>', 'insertLineBreak'],
    ['<div id=e contenteditable>|hello</div>', 'insertLineBreak'],
    ['<div id=e contenteditable><pre>he|llo</pre></div>', 'insertLineBreak'],
    ['<div id=e contenteditable>he[ll]o</div>', 'insertLineBreak'],
    // inline styles
    ['<div id=e contenteditable>he[ll]o</div>', 'bold'],
    ['<div id=e contenteditable>he[ll]o</div>', 'italic'],
    ['<div id=e contenteditable>he[ll]o</div>', 'underline'],
    ['<div id=e contenteditable>he[ll]o</div>', 'strikethrough'],
    ['<div id=e contenteditable>he[ll]o</div>', 'subscript'],
    ['<div id=e contenteditable>he[ll]o</div>', 'superscript'],
    ['<div id=e contenteditable>he[ll]o</div>', 'bold', undefined, { css: 1 }],
    ['<div id=e contenteditable>he[ll]o</div>', 'italic', undefined, { css: 1 }],
    ['<div id=e contenteditable>he[ll]o</div>', 'underline', undefined, { css: 1 }],
    ['<div id=e contenteditable>he[ll]o</div>', 'strikethrough', undefined, { css: 1 }],
    ['<div id=e contenteditable>he[ll]o</div>', 'subscript', undefined, { css: 1 }],
    ['<div id=e contenteditable>he[ll]o</div>', 'superscript', undefined, { css: 1 }],
    ['<div id=e contenteditable><b>he[ll]o</b></div>', 'bold'],
    ['<div id=e contenteditable><b>[hello]</b></div>', 'bold'],
    ['<div id=e contenteditable>[he<b>ll]o</b></div>', 'bold'],
    ['<div id=e contenteditable><b>[he</b>ll]o</div>', 'bold'],
    ['<div id=e contenteditable>a[b<i>c</i>d]e</div>', 'bold'],
    ['<div id=e contenteditable><strong>he[ll]o</strong></div>', 'bold'],
    ['<div id=e contenteditable><span style="font-weight: bold">he[ll]o</span></div>', 'bold'],
    ['<div id=e contenteditable><span style="font-weight:bold;color:red">he[ll]o</span></div>', 'bold'],
    ['<div id=e contenteditable><span style="color:red">he[ll]o</span></div>', 'bold'],
    ['<div id=e contenteditable><b id=q class=c>a b[c]d e</b></div>', 'bold'],
    ['<div id=e contenteditable><b>a<i>x</i>he[ll]o</b></div>', 'bold'],
    ['<div id=e contenteditable><i><b>he[ll]o</b></i></div>', 'bold'],
    ['<div id=e contenteditable><b>he</b>[ll]<b>o</b></div>', 'bold'],
    ['<div id=e contenteditable>he[ll]<b>o</b></div>', 'bold'],
    ['<div id=e contenteditable><b>h</b>e[ll]o</div>', 'bold'],
    ['<div id=e contenteditable><h1>he[ll]o</h1></div>', 'bold'],
    ['<div id=e contenteditable>a[b<br>c]d</div>', 'bold'],
    ['<div id=e contenteditable>a[b<img src="x">c]d</div>', 'bold'],
    ['<div id=e contenteditable><p>a[b</p><p>c]d</p></div>', 'bold'],
    ['<div id=e contenteditable>{hello}</div>', 'bold'],
    ['<div id=e contenteditable>a[b c]d</div>', 'italic'],
    ['<div id=e contenteditable><em>he[ll]o</em></div>', 'italic'],
    ['<div id=e contenteditable><i>he[ll]o</i></div>', 'bold'],
    ['<div id=e contenteditable><u>he[ll]o</u></div>', 'underline'],
    ['<div id=e contenteditable><a href="/">he[ll]o</a></div>', 'underline'],
    ['<div id=e contenteditable><s>he[ll]o</s></div>', 'strikethrough'],
    ['<div id=e contenteditable><u>he[ll]o</u></div>', 'strikethrough'],
    ['<div id=e contenteditable><sub>he[ll]o</sub></div>', 'superscript'],
    ['<div id=e contenteditable><b>he[ll]o</b></div>', 'bold', undefined, { css: 1 }],
    ['<div id=e contenteditable><span style="font-weight: bold;">he[ll]o</span></div>', 'bold', undefined, { css: 1 }],
    ['<div id=e contenteditable>he[ll]o</div>', [['bold'], ['bold']]],
    ['<div id=e contenteditable>he[ll]o</div>', [['bold'], ['italic']]],
    ['<div id=e contenteditable>he[ll]o</div>', [['bold'], ['italic']], undefined, { css: 1 }],
    ['<div id=e contenteditable>he|llo</div>', 'bold'],
    ['<div id=e contenteditable>he[ll]o</div>', 'foreColor', 'red'],
    ['<div id=e contenteditable>he[ll]o</div>', 'foreColor', '#00ff00'],
    ['<div id=e contenteditable>he[ll]o</div>', 'foreColor', 'rgb(1, 2, 3)'],
    ['<div id=e contenteditable>he[ll]o</div>', 'foreColor', 'bogus'],
    ['<div id=e contenteditable>he[ll]o</div>', 'foreColor', ''],
    ['<div id=e contenteditable>he[ll]o</div>', 'foreColor', 'red', { css: 1 }],
    ['<div id=e contenteditable><font color="#ff0000">he[ll]o</font></div>', 'foreColor', 'blue'],
    ['<div id=e contenteditable>he[ll]o</div>', 'backColor', 'red'],
    ['<div id=e contenteditable>he[ll]o</div>', 'backColor', 'transparent'],
    ['<div id=e contenteditable>he[ll]o</div>', 'hiliteColor', 'red'],
    ['<div id=e contenteditable>he[ll]o</div>', 'fontName', 'Arial'],
    ['<div id=e contenteditable>he[ll]o</div>', 'fontName', ''],
    ['<div id=e contenteditable>he[ll]o</div>', 'fontSize', '5'],
    ['<div id=e contenteditable>he[ll]o</div>', 'fontSize', '9'],
    ['<div id=e contenteditable>he[ll]o</div>', 'fontSize', '+1'],
    ['<div id=e contenteditable>he[ll]o</div>', 'fontSize', 'x'],
    ['<div id=e contenteditable>he[ll]o</div>', 'fontSize', '5', { css: 1 }],
    // links / remove format
    ['<div id=e contenteditable>he[ll]o</div>', 'createLink', 'http://x/'],
    ['<div id=e contenteditable>he|llo</div>', 'createLink', 'http://x/'],
    ['<div id=e contenteditable>he[ll]o</div>', 'createLink', ''],
    ['<div id=e contenteditable><a href="http://y/">he[ll]o</a></div>', 'createLink', 'http://x/'],
    ['<div id=e contenteditable><a href="http://x/">he[ll]o</a></div>', 'unlink'],
    ['<div id=e contenteditable><a href="http://x/">he|llo</a></div>', 'unlink'],
    ['<div id=e contenteditable>a[b<a href="http://x/">cd</a>e]f</div>', 'unlink'],
    ['<div id=e contenteditable><b>h[e<i>l</i>l]o</b></div>', 'removeFormat'],
    ['<div id=e contenteditable>h[e<span style="color: red">l</span><a href="http://x/">l</a>]o</div>', 'removeFormat'],
    ['<div id=e contenteditable>he|llo</div>', 'removeFormat'],
    // block formatting
    ['<div id=e contenteditable>he|llo</div>', 'formatBlock', 'h1'],
    ['<div id=e contenteditable>he|llo</div>', 'formatBlock', '<pre>'],
    ['<div id=e contenteditable>he|llo</div>', 'formatBlock', 'P'],
    ['<div id=e contenteditable><p>he|llo</p></div>', 'formatBlock', 'blockquote'],
    ['<div id=e contenteditable><p>he|llo</p></div>', 'formatBlock', 'p'],
    ['<div id=e contenteditable><p>a[b</p><p>c]d</p></div>', 'formatBlock', 'h2'],
    ['<div id=e contenteditable>he|llo</div>', 'formatBlock', 'span'],
    ['<div id=e contenteditable>he|llo</div>', 'formatBlock', '1x'],
    ['<div id=e contenteditable>he|llo</div>', 'formatBlock', ''],
    ['<div id=e contenteditable>he|llo</div>', 'formatBlock', 'div'],
    ['<div id=e contenteditable>ab<br>c|d<br>ef</div>', 'formatBlock', 'h3'],
    ['<div id=e contenteditable>he|llo</div>', 'insertOrderedList'],
    ['<div id=e contenteditable>he|llo</div>', 'insertUnorderedList'],
    ['<div id=e contenteditable><ul><li>he|llo</li></ul></div>', 'insertUnorderedList'],
    ['<div id=e contenteditable><ul><li>he|llo</li></ul></div>', 'insertOrderedList'],
    ['<div id=e contenteditable><p>a[b</p><p>c]d</p></div>', 'insertOrderedList'],
    ['<div id=e contenteditable><ul><li>a</li><li>b|</li><li>c</li></ul></div>', 'insertUnorderedList'],
    ['<div id=e contenteditable>he|llo</div>', 'justifyCenter'],
    ['<div id=e contenteditable><p>he|llo</p></div>', 'justifyRight'],
    ['<div id=e contenteditable>he|llo</div>', 'indent'],
    ['<div id=e contenteditable><blockquote style="margin: 0 0 0 40px; border: none; padding: 0px;">he|llo</blockquote></div>', 'outdent'],
    // selection commands
    ['<div id=e contenteditable>he|llo</div>', 'selectAll'],
    ['<div id=e>he|llo</div>', 'selectAll'],
    ['<div id=e contenteditable>he[ll]o</div>', 'unselect'],
    ['<div id=e>he[ll]o</div>', 'unselect'],
    ['<div id=e>he|llo</div>', 'unselect'],
    ['<div id=e>he|llo</div>', 'findString', 'l'],
    ['<div id=e>hello |world</div>', 'findString', 'o'],
    ['<div id=e>hello |world</div>', 'findString', 'H'],
    ['<div id=e>hello world|</div>', 'findString', 'zz'],
    ['<div id=e>hello world|</div>', 'findString', ''],
    ['<div id=e contenteditable>he|llo</div>', 'transpose', undefined, { focus: 1 }],
    ['<div id=e contenteditable>hello|</div>', 'transpose', undefined, { focus: 1 }],
    ['<div id=e contenteditable>he[ll]o</div>', 'transpose', undefined, { focus: 1 }],
    // clipboard: no user activation and no clipboard permission
    ['<div id=e contenteditable>he[ll]o</div>', 'copy'],
    ['<div id=e contenteditable>he[ll]o</div>', 'cut'],
    ['<div id=e contenteditable>he|llo</div>', 'paste'],
    ['<div id=e contenteditable>he|llo</div>', 'pasteAndMatchStyle'],
    // editability
    ['<div id=e>he|llo</div>', 'insertText', 'X'],
    ['<div id=e contenteditable=false>he|llo</div>', 'insertText', 'X'],
    ['<div id=e contenteditable><span contenteditable=false>a|b</span></div>', 'insertText', 'X'],
    ['<div id=e contenteditable=TRUE>he|llo</div>', 'insertText', 'X'],
    ['<div id=e contenteditable=plaintext-only>he[ll]o</div>', 'bold'],
    ['<div id=e contenteditable=plaintext-only>he|llo</div>', 'insertText', 'X'],
    ['<div id=e contenteditable=plaintext-only>he|llo</div>', 'insertText', '  '],
    ['<div id=e contenteditable=plaintext-only>hello|</div>', 'insertText', ' '],
    ['<div id=e contenteditable=plaintext-only>he|llo</div>', 'insertText', 'a\nb'],
    ['<div id=e contenteditable=plaintext-only>he|llo</div>', 'insertHTML', '<b>X</b>'],
    ['<div id=e contenteditable=plaintext-only>he|llo</div>', 'insertParagraph'],
    ['<div id=e contenteditable=plaintext-only>he|llo</div>', 'insertLineBreak'],
    ['<div id=e contenteditable=plaintext-only><b>he[ll]o</b></div>', 'delete'],
    ['<div id=e contenteditable>he|llo</div>', 'foo', 'X'],
    ['<div id=e contenteditable>he|llo</div>', 'undo'],
    ['<div id=e contenteditable>he|llo</div>', 'redo'],
    // undo / redo and typing coalescing
    ['<div id=e contenteditable>he|llo</div>', [['insertText', 'A'], ['insertText', 'B'], ['bold'], ['undo'], ['undo'], ['redo'], ['insertText', 'C'], ['redo'], ['undo'], ['undo'], ['undo']]],
    ['<div id=e contenteditable>he|llo</div>', [['insertText', 'A'], ['bold'], ['insertText', 'B'], ['undo']]],
    ['<div id=e contenteditable>he[ll]o</div>', [['bold'], ['undo'], ['redo'], ['undo']]],
    ['<div id=e contenteditable>he|llo</div>', [['insertHTML', '<b>X</b>'], ['insertText', 'Y'], ['undo'], ['undo'], ['redo']]],
    ['<div id=e contenteditable>he|llo</div>', [['delete'], ['delete'], ['undo']]],
    ['<div id=e contenteditable>he|llo</div>', [['insertParagraph'], ['insertText', 'X'], ['undo'], ['redo']]],
    ['<div id=e contenteditable>he|llo</div>', [['formatBlock', 'h1'], ['undo'], ['redo']]],
    ['<div id=e contenteditable>he|llo</div>', [['insertOrderedList'], ['undo']]],
    ['<div id=e contenteditable>he[ll]o</div>', [['bold'], ['italic'], ['undo'], ['undo'], ['redo'], ['redo']]],
    ['<div id=e contenteditable>he|llo</div>', [['forwardDelete'], ['forwardDelete'], ['insertText', 'X'], ['undo']]],
  ];
  out.cases = C.map(c => [c.slice(0, 3).map(x => x === undefined ? 'undefined' : JSON.stringify(x)).join(' ~ '), run1(...c)]);

  // A script selection change closes the open typing command.
  place(body, '<div id=e contenteditable>he|llo</div>');
  document.execCommand('insertText', false, 'A');
  const t0 = body.firstChild.firstChild;
  getSelection().collapse(t0, 3);
  document.execCommand('insertText', false, 'B');
  document.execCommand('undo');
  out.typingClosedBySelection = mark(body);
  // Undo steps of a removed editing host are dropped.
  place(body, '<div id=e contenteditable>he|llo</div>');
  document.execCommand('insertText', false, 'A');
  const removed = body.firstChild;
  removed.remove();
  out.removedHost = [document.queryCommandEnabled('undo'), document.execCommand('undo'), removed.innerHTML];
  // Recursive execCommand from an input listener is refused.
  place(body, '<div id=e contenteditable>he|llo</div>');
  const nested = [];
  const listener = () => { nested.push(document.execCommand('insertText', false, 'Z'), document.queryCommandEnabled('insertText')); };
  body.firstChild.addEventListener('input', listener);
  nested.push(document.execCommand('insertText', false, 'X'), mark(body));
  out.recursion = nested;
  // selectstart is cancelable and cancels selectAll.
  place(body, '<div id=e contenteditable>he|llo</div>');
  body.firstChild.addEventListener('selectstart', e => e.preventDefault());
  out.selectstartCanceled = [document.execCommand('selectAll'), mark(body)];
  // beforeinput for transpose is cancelable.
  place(body, '<div id=e contenteditable>he|llo</div>');
  body.firstChild.focus();
  body.firstChild.addEventListener('beforeinput', e => e.preventDefault());
  out.transposeCanceled = [document.execCommand('transpose'), mark(body)];
  // The input event fires before the selection moves.
  place(body, '<div id=e contenteditable>he|llo</div>');
  const during = [];
  body.firstChild.addEventListener('input', () => { const s = getSelection(); during.push(s.anchorNode.nodeName, s.anchorOffset, body.firstChild.innerHTML); });
  document.execCommand('insertText', false, 'X');
  out.selectionDuringInput = during;
  // Query states with a caret typing style.
  place(body, '<div id=e contenteditable>he|llo</div>');
  document.execCommand('bold'); document.execCommand('italic');
  out.typingStyleState = ['bold', 'italic', 'underline'].map(c => [document.queryCommandState(c), document.queryCommandValue(c)]);
  getSelection().collapse(body.firstChild.firstChild, 1);
  out.typingStyleCleared = ['bold', 'italic'].map(c => document.queryCommandState(c));
  // styleWithCSS / useCSS / defaultParagraphSeparator state.
  const css = [];
  css.push(document.execCommand('styleWithCSS', false, true), document.queryCommandState('styleWithCSS'), document.queryCommandValue('styleWithCSS'), document.queryCommandState('useCSS'));
  css.push(document.execCommand('useCSS', false, true), document.queryCommandState('styleWithCSS'));
  css.push(document.execCommand('useCSS', false, 'false'), document.queryCommandState('styleWithCSS'));
  css.push(document.execCommand('styleWithCSS', false, 'FALSE'), document.queryCommandState('styleWithCSS'));
  for (const v of ['p', 'DIV', 'br', 'span', '', 'P']) css.push([document.execCommand('defaultParagraphSeparator', false, v), document.queryCommandValue('defaultParagraphSeparator')]);
  document.execCommand('defaultParagraphSeparator', false, 'div');
  out.state = css;

  // designMode makes the document editable; the input event targets <body>.
  document.designMode = 'on';
  out.design = [
    run1('<div id=e>he|llo</div>', 'insertText', 'X'),
    run1('<div id=e>he[ll]o</div>', 'bold'),
    run1('<div id=e contenteditable=false>he|llo</div>', 'insertText', 'X'),
    run1('<div id=e>he|llo</div>', 'insertParagraph'),
    run1('<div id=e>he|llo</div>', 'selectAll'),
    (() => { place(body, '<div id=e>he|llo</div>'); return [document.queryCommandEnabled('bold'), document.querySelector('#e').isContentEditable, body.isContentEditable, document.documentElement.isContentEditable]; })(),
  ];
  document.designMode = 'off';

  // queryCommand* over fixtures.
  const Q = ['bold', 'italic', 'underline', 'strikethrough', 'subscript', 'superscript', 'insertOrderedList', 'insertUnorderedList', 'justifyCenter', 'justifyLeft', 'justifyRight', 'justifyFull', 'formatBlock', 'fontName', 'fontSize', 'foreColor', 'backColor', 'hiliteColor', 'createLink', 'styleWithCSS', 'useCSS', 'defaultParagraphSeparator', 'undo', 'redo', 'delete', 'forwardDelete', 'insertText', 'insertHTML', 'insertParagraph', 'insertLineBreak', 'selectAll', 'unselect', 'unlink', 'removeFormat', 'copy', 'cut', 'paste', 'indent', 'outdent', 'insertImage', 'insertHorizontalRule', 'transpose', 'findString', 'print', 'fontSizeDelta', 'pasteAndMatchStyle'];
  const F = ['<div id=e contenteditable>he|llo</div>', '<div id=e contenteditable>he[ll]o</div>', '<div id=e>he|llo</div>', '<div id=e>he[ll]o</div>', '<div id=e contenteditable=plaintext-only>he[ll]o</div>', '<div id=e contenteditable=plaintext-only>he|llo</div>', '<div id=e contenteditable><b>he|llo</b></div>', '<div id=e contenteditable><b>he</b>|llo</div>', '<div id=e contenteditable>he|<b>llo</b></div>', '<div id=e contenteditable>[he<b>ll]o</b></div>', '<div id=e contenteditable><b>[he</b>ll]o</div>', '<div id=e contenteditable><h1>he[ll]o</h1></div>', '<div id=e contenteditable><ol><li>he|llo</li></ol></div>', '<div id=e contenteditable><ul><li>he|llo</li></ul></div>', '<div id=e contenteditable><ul><li>[a</li><li>b]</li></ul></div>', '<div id=e contenteditable><p style="text-align:center">he|llo</p></div>', '<div id=e contenteditable><i><u><a href="/">he|llo</a></u></i></div>', '<div id=e contenteditable><sub>h|i</sub><sup>x</sup></div>', '<div id=e contenteditable><font face="Arial" size="5" color="red">h|i</font></div>', '<div id=e contenteditable><span style="background-color: red; font-family: Georgia; font-size: 20px; color: rgb(0,128,0)">h|i</span></div>', '<div id=e contenteditable><blockquote>a|b</blockquote><pre>c</pre></div>', '<div id=e contenteditable><strike>a|b</strike></div>', '<div id=e contenteditable><span style="text-decoration: underline line-through; font-style: italic; font-weight: 700; vertical-align: super">a|b</span></div>', '<div id=e contenteditable><span style="font-weight: 300">a|b</span></div>', '<div id=e contenteditable><b><span style="font-weight: normal">a|b</span></b></div>', '<div id=e contenteditable>^</div>', '<p id=e>no selection</p>'];
  out.queries = F.map((f, index) => {
    place(body, f);
    if (index === F.length - 1) getSelection().removeAllRanges();
    return [f, Q.map(q => { const r = []; for (const m of ['queryCommandEnabled', 'queryCommandState', 'queryCommandIndeterm', 'queryCommandValue']) r.push(run(() => document[m](q))); return q + ':' + r.map(x => Array.isArray(x) ? (x[1] === true ? 1 : x[1] === false ? 0 : JSON.stringify(x[1])) : x).join(','); })];
  });

  // Focused text controls are edited as plain text.
  const controlStep = (el, f) => { log.length = 0; const r = f(); return [r, el.value, el.selectionStart, el.selectionEnd, log.slice()]; };
  body.innerHTML = '<input id=i value="hello"><textarea id=t>hello</textarea>';
  const input = document.getElementById('i'), area = document.getElementById('t');
  input.focus(); input.setSelectionRange(2, 4); log.length = 0;
  out.textControls = {
    input: [document.queryCommandEnabled('insertText'), document.queryCommandEnabled('bold'), controlStep(input, () => document.execCommand('insertText', false, 'X')), controlStep(input, () => document.execCommand('delete')), controlStep(input, () => document.execCommand('undo')), controlStep(input, () => document.execCommand('redo')), controlStep(input, () => document.execCommand('bold')), controlStep(input, () => document.execCommand('insertHTML', false, '<b>Y</b>')), controlStep(input, () => document.execCommand('insertParagraph')), controlStep(input, () => document.execCommand('insertLineBreak')), controlStep(input, () => document.execCommand('selectAll')), controlStep(input, () => document.execCommand('forwardDelete')), controlStep(input, () => document.execCommand('insertText', false, 'a\nb'))],
  };
  area.focus(); area.setSelectionRange(5, 5); log.length = 0;
  out.textControls.textarea = [controlStep(area, () => document.execCommand('insertText', false, 'a\nb')), controlStep(area, () => document.execCommand('insertParagraph')), controlStep(area, () => document.execCommand('insertLineBreak')), controlStep(area, () => document.execCommand('insertText', false, '  ')), controlStep(area, () => document.execCommand('undo'))];
  input.readOnly = true; input.value = 'ro'; input.focus(); input.setSelectionRange(0, 0); log.length = 0;
  out.textControls.readonly = [document.queryCommandEnabled('insertText'), controlStep(input, () => document.execCommand('insertText', false, 'Z'))];
  input.blur(); area.blur();
  return out;
})()
