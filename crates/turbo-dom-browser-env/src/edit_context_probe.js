(async () => {
  // EditContext control: interface shape, constructor dictionary conversion, text/selection/bounds updates,
  // HTMLElement.editContext attachment rules, editing commands inside an EditContext host, the
  // TextUpdateEvent / TextFormatUpdateEvent / TextFormat / CharacterBoundsUpdateEvent interfaces, and
  // Element/Document.ariaNotify argument validation.
  const out = {};
  document.body.textContent = '';
  const run = f => { try { const v = f(); return v === undefined ? 'ok' : ['ok', v]; } catch (e) { return e.name + ': ' + e.message; } };
  const d = (o, k) => { const x = Object.getOwnPropertyDescriptor(o, k); if (!x) return null; const r = {}; for (const f in x) r[f] = typeof x[f] === 'function' ? [x[f].name, x[f].length] : x[f]; return r; };
  const P = EditContext.prototype;
  out.proto = Reflect.ownKeys(P).map(String);
  out.chain = [Object.getPrototypeOf(P) === EventTarget.prototype, Object.getPrototypeOf(EditContext) === EventTarget, EditContext.name, EditContext.length];
  out.desc = Object.fromEntries(Reflect.ownKeys(P).filter(k => typeof k === 'string').map(k => [k, d(P, k)]));
  out.call = run(() => EditContext());
  const ec = new EditContext();
  out.defaults = [ec.text, ec.selectionStart, ec.selectionEnd, ec.characterBoundsRangeStart, ec.attachedElements(), ec.characterBounds(), ec.ontextupdate];
  const ec2 = new EditContext({ text: 'hello', selectionStart: 2, selectionEnd: 7 });
  out.init = [ec2.text, ec2.selectionStart, ec2.selectionEnd];
  out.init2 = run(() => { const e = new EditContext({ text: 'abc', selectionStart: 5, selectionEnd: 1 }); return [e.selectionStart, e.selectionEnd]; });
  out.init3 = run(() => { const e = new EditContext({ selectionStart: -1 }); return [e.selectionStart]; });
  out.init4 = run(() => { const e = new EditContext({ selectionStart: 2**32 + 3 }); return [e.selectionStart]; });
  out.init5 = run(() => new EditContext(null).text);
  out.init6 = run(() => new EditContext(5));
  out.init7 = run(() => { const order = []; new EditContext({ get text() { order.push('text'); return 'x'; }, get selectionStart() { order.push('ss'); return 0; }, get selectionEnd() { order.push('se'); return 0; } }); return order; });
  const e = new EditContext({ text: 'hello world' });
  out.updateText = [run(() => e.updateText(0, 5, 'HELLO')), e.text, run(() => e.updateText(6, 100, 'X')), e.text, run(() => e.updateText(4, 2, 'Y')), e.text, run(() => e.updateText()), run(() => e.updateText(1)), run(() => e.updateText(1, 2)), run(() => e.updateText(-1, 2, 'z')), e.text, run(() => e.updateText(100, 200, 'end')), e.text];
  const s = new EditContext({ text: 'hello' });
  out.updateSelection = [run(() => s.updateSelection(1, 3)), s.selectionStart, s.selectionEnd, run(() => s.updateSelection(4, 2)), s.selectionStart, s.selectionEnd, run(() => s.updateSelection(10, 20)), s.selectionStart, s.selectionEnd, run(() => s.updateSelection()), run(() => s.updateSelection(1))];
  const t = new EditContext({ text: 'hello', selectionStart: 5, selectionEnd: 5 });
  t.updateText(0, 5, 'hi'); out.selAfterText = [t.selectionStart, t.selectionEnd];
  const b = new EditContext();
  out.bounds = [run(() => b.updateControlBounds(new DOMRect(1, 2, 3, 4))), run(() => b.updateSelectionBounds(new DOMRect(1, 2, 3, 4))), run(() => b.updateControlBounds({ x: 1 })), run(() => b.updateControlBounds()), run(() => b.updateCharacterBounds(3, [new DOMRect(1, 2, 3, 4), new DOMRect(5, 6, 7, 8)])), b.characterBoundsRangeStart, run(() => b.characterBounds().map(r => [r.constructor.name, r.x, r.y, r.width, r.height])), run(() => b.updateCharacterBounds(1, [{}])), run(() => b.updateCharacterBounds(1)), run(() => b.updateCharacterBounds(1, 'x')), run(() => b.characterBounds() === b.characterBounds()), run(() => { const r = b.characterBounds(); return r[0] === b.characterBounds()[0]; }), run(() => { const r = new DOMRect(1,1,1,1); b.updateCharacterBounds(0, [r]); r.x = 9; return b.characterBounds()[0].x; }), run(() => b.updateControlBounds(new DOMRectReadOnly(1,2,3,4)))];
  out.brands = ['text', 'selectionStart', 'attachedElements', 'updateText', 'ontextupdate'].map(k => { const x = Object.getOwnPropertyDescriptor(P, k); return run(() => (x.get || x.value).call({})); });
  // attachment
  const host = document.body;
  const cases = ['div', 'span', 'p', 'canvas', 'input', 'textarea', 'img', 'article', 'aside', 'blockquote', 'body', 'button', 'footer', 'h1', 'header', 'main', 'nav', 'section', 'a', 'li', 'td', 'video', 'pre', 'dialog', 'x-custom', 'custom-el', 'h6', 'ul'];
  out.attach = cases.map(tag => { const el = document.createElement(tag); const c = new EditContext(); return [tag, run(() => { el.editContext = c; return el.editContext === c; })]; });
  out.attachSvg = run(() => { const el = document.createElementNS('http://www.w3.org/2000/svg', 'svg'); return 'editContext' in el; });
  const a1 = document.createElement('div'), a2 = document.createElement('div'), c1 = new EditContext();
  out.share = [run(() => { a1.editContext = c1; }), run(() => { a2.editContext = c1; }), a2.editContext, c1.attachedElements().length, c1.attachedElements()[0] === a1, run(() => { a1.editContext = c1; }), run(() => { a1.editContext = null; }), c1.attachedElements().length, run(() => { a2.editContext = c1; }), c1.attachedElements()[0] === a2, run(() => { a2.editContext = {}; }), run(() => { a2.editContext = undefined; }), a2.editContext, run(() => { a2.editContext = 5; })];
  const sh = document.createElement('div'); sh.attachShadow({ mode: 'open' }); out.shadowHost = run(() => { sh.editContext = new EditContext(); });
  const ce = document.createElement('div'); document.body.append(ce); const c3 = new EditContext(); ce.editContext = c3;
  out.editable = [ce.isContentEditable, ce.contentEditable, (() => { ce.focus(); return document.activeElement === ce; })(), ce.tabIndex];
  // execCommand inside edit context
  ce.textContent = 'abc'; getSelection().collapse(ce.firstChild, 1);
  const elog = []; for (const ty of ['textupdate', 'textformatupdate', 'characterboundsupdate', 'compositionstart', 'compositionend']) c3.addEventListener(ty, ev => elog.push([ev.type, ev.constructor.name, ev.isTrusted]));
  const ilog = []; ce.addEventListener('beforeinput', ev => ilog.push(['bi', ev.inputType, ev.data])); ce.addEventListener('input', ev => ilog.push(['i', ev.inputType, ev.data]));
  out.exec = [document.queryCommandEnabled('insertText'), document.execCommand('insertText', false, 'X'), ce.innerHTML, c3.text, elog, ilog, document.queryCommandEnabled('bold')];
  ce.editContext = null;
  out.events = {};
  out.events.textupdate = run(() => { const ev = new TextUpdateEvent('textupdate', { text: 'ab', updateRangeStart: 1, updateRangeEnd: 3, selectionStart: 2, selectionEnd: 2 }); return [ev.text, ev.updateRangeStart, ev.updateRangeEnd, ev.selectionStart, ev.selectionEnd, ev.bubbles, ev.cancelable, ev.composed]; });
  out.events.textupdateDefault = run(() => { const ev = new TextUpdateEvent('x'); return [ev.text, ev.updateRangeStart, ev.updateRangeEnd, ev.selectionStart, ev.selectionEnd]; });
  out.events.textupdateNoType = run(() => new TextUpdateEvent());
  out.events.format = run(() => { const ev = new TextFormatUpdateEvent('textformatupdate', { textFormats: [new TextFormat({ rangeStart: 1, rangeEnd: 2, underlineStyle: 'wavy', underlineThickness: 'thin' })] }); const f = ev.getTextFormats(); return [f.length, f[0].rangeStart, f[0].rangeEnd, f[0].underlineStyle, f[0].underlineThickness, f[0] === ev.getTextFormats()[0], ev.getTextFormats() === ev.getTextFormats()]; });
  out.events.formatDefault = run(() => { const f = new TextFormat(); return [f.rangeStart, f.rangeEnd, f.underlineStyle, f.underlineThickness]; });
  out.events.formatBad = run(() => new TextFormat({ underlineStyle: 'bogus' }));
  out.events.formatBad2 = run(() => new TextFormat({ underlineThickness: 'Thin' }));
  out.events.formatDict = run(() => new TextFormatUpdateEvent('x', { textFormats: [{ rangeStart: 1 }] }).getTextFormats().length);
  out.events.cbu = run(() => { const ev = new CharacterBoundsUpdateEvent('characterboundsupdate', { rangeStart: 2, rangeEnd: 5 }); return [ev.rangeStart, ev.rangeEnd, ev.bubbles]; });
  out.events.cbuDefault = run(() => { const ev = new CharacterBoundsUpdateEvent('x'); return [ev.rangeStart, ev.rangeEnd]; });
  out.evDesc = {};
  for (const C of [TextUpdateEvent, TextFormatUpdateEvent, TextFormat, CharacterBoundsUpdateEvent]) { out.evDesc[C.name] = [Object.getPrototypeOf(C.prototype) === (C === TextFormat ? Object.prototype : Event.prototype), C.length, Object.fromEntries(Reflect.ownKeys(C.prototype).filter(k => typeof k === 'string').map(k => [k, d(C.prototype, k)])), run(() => C('x'))]; }
  out.handlers = (() => { const c = new EditContext(); const r = []; const f = () => 1; c.ontextupdate = f; r.push(c.ontextupdate === f); c.ontextupdate = 5; r.push(c.ontextupdate); c.ontextupdate = f; c.dispatchEvent(new Event('textupdate')); return r; })();
  out.htmlDesc = d(HTMLElement.prototype, 'editContext');
  out.globals = ['EditContext', 'TextUpdateEvent', 'TextFormatUpdateEvent', 'TextFormat', 'CharacterBoundsUpdateEvent'].map(n => [n, d(window, n)]);
  const el = document.createElement('div'); document.body.append(el);
  {
  const E = Element.prototype.ariaNotify, D = Document.prototype.ariaNotify;
  out.aria = {}; out.aria.el = [run(() => el.ariaNotify()), run(() => el.ariaNotify('hi')), run(() => el.ariaNotify('hi', {})), run(() => el.ariaNotify('hi', { priority: 'high' })), run(() => el.ariaNotify('hi', { priority: 'normal' })), run(() => el.ariaNotify('hi', { priority: 'bogus' })), run(() => el.ariaNotify('hi', { priority: 'High' })), run(() => el.ariaNotify('hi', 5)), run(() => el.ariaNotify('hi', null)), run(() => el.ariaNotify(Symbol())), run(() => el.ariaNotify('hi', { interrupt: 'all' })), run(() => el.ariaNotify('hi', { interrupt: 'bogus' })), run(() => el.ariaNotify(null)), run(() => el.ariaNotify(undefined)), run(() => el.ariaNotify('')), run(() => el.ariaNotify('x'.repeat(100000)))];
  out.aria.order = (() => { const order = []; try { el.ariaNotify({ toString() { order.push('msg'); return 'm'; } }, { get priority() { order.push('priority'); return 'high'; }, get interrupt() { order.push('interrupt'); return undefined; } }); } catch (e) { order.push(e.name); } return order; })();
  out.aria.doc = [run(() => document.ariaNotify('hi')), run(() => D.call(document.implementation.createHTMLDocument(''), 'x')), run(() => D.call(el, 'x')), run(() => E.call(document, 'x')), run(() => E.call({}, 'x')), run(() => E.call(document.createTextNode(''), 'x'))];
  out.aria.svg = run(() => document.createElementNS('http://www.w3.org/2000/svg', 'svg').ariaNotify('x'));
  out.aria.detached = run(() => document.createElement('span').ariaNotify('x'));
  out.aria.xml = run(() => document.implementation.createDocument(null, 'r').ariaNotify('x'));
  out.aria.desc = [Object.getOwnPropertyDescriptor(Element.prototype, 'ariaNotify'), Object.getOwnPropertyDescriptor(Document.prototype, 'ariaNotify')].map(x => x && [typeof x.value, x.value.name, x.value.length, x.writable, x.enumerable, x.configurable]);
  out.aria.where = ['Element', 'Document', 'HTMLElement', 'Node', 'DocumentFragment', 'ShadowRoot'].map(n => [n, Object.prototype.hasOwnProperty.call(window[n].prototype, 'ariaNotify')]);
  out.aria.ret = el.ariaNotify('x');
  }
  return out;
})()
