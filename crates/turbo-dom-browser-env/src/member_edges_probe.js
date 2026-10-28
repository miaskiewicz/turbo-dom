(async () => {
  // Member edges of the text, animation, range, script, XPath, reference-target, invoker,
  // hyperlink, input, select, ARIA-relation, shorthand and focusgroup bindings. Every row is
  // the member's result, or the thrown exception's name (and message where it is specified).
  let next = 0; const waiters = {};
  globalThis.__cdpInputDone = id => { const resolve = waiters[id]; delete waiters[id]; resolve(); };
  const input = (...steps) => new Promise(resolve => { const id = ++next; waiters[id] = resolve; __cdpInput(JSON.stringify({ id, steps })); });
  const codes = { ArrowRight: 39, ArrowLeft: 37, ArrowUp: 38, ArrowDown: 40, Home: 36, End: 35 };
  const out = {}, body = document.body;
  const caught = run => { try { const value = run(); return value === undefined ? 'undefined' : value; } catch (error) { return 'E:' + error.name; } };
  const message = run => { try { run(); return 'ok'; } catch (error) { return error.name + ': ' + error.message; } };
  const named = run => { try { run(); return 'ok'; } catch (error) { return error.name; } };
  const get = (proto, name, receiver) => caught(() => Object.getOwnPropertyDescriptor(proto, name).get.call(receiver));
  const set = (proto, name, receiver, value) => caught(() => Object.getOwnPropertyDescriptor(proto, name).set.call(receiver, value));
  const box = body.appendChild(document.createElement('div'));
  const html = markup => { box.innerHTML = markup; return box.firstChild; };

  // Normalizing before any Range exists, then with live boundaries in an empty first text.
  {
    const r = {};
    const p = body.appendChild(document.createElement('p'));
    p.append('', 'a', '', 'b');
    p.normalize();
    r.fresh = [p.childNodes.length, p.firstChild.data];
    p.replaceChildren('', 'c', 'd');
    const range = document.createRange();
    range.setStart(p.firstChild, 0); range.setEnd(p.lastChild, 1);
    p.normalize();
    r.live = [p.childNodes.length, p.firstChild.data, range.startContainer === p, range.startOffset, range.endContainer === p.firstChild, range.endOffset];
    p.remove();
    out.normalize = r;
  }

  // innerText / outerText: pre-line breaks, empty preserved text, closed details,
  // text-transform, comments, unpaired surrogates, receivers and the document element.
  {
    const r = {};
    r.preLine = html('<div style="white-space:pre-line">a\nb  c\n</div>').innerText;
    const pre = html('<pre></pre>'); pre.append('', 'x', '', 'y');
    r.preEmpty = pre.innerText;
    html('<details><summary>s</summary><p>hidden <b>p</b></p></details>');
    r.details = [box.innerText, box.querySelector('p').innerText, box.querySelector('b').innerText];
    html('<details><summary>s</summary>t<span>u</span></details>');
    r.detailsText = box.innerText;
    r.lower = html('<div style="text-transform:lowercase">AbC É</div>').innerText;
    r.upper = html('<div style="text-transform:uppercase">stß</div>').innerText;
    const surrogate = html('<div style="text-transform:uppercase"></div>');
    surrogate.textContent = 'a\ud800b';
    r.surrogate = [...surrogate.innerText].map(c => c.charCodeAt(0));
    const lone = html('<div style="text-transform:lowercase"></div>');
    lone.textContent = 'A\udc00B';
    r.loneLower = [...lone.innerText].map(c => c.charCodeAt(0));
    r.comment = html('<div>a<!--c-->b<?pi x?>c</div>').innerText;
    const proto = HTMLElement.prototype;
    r.receivers = ['innerText', 'outerText'].map(name => [get(proto, name, 1), get(proto, name, {}), set(proto, name, 's', 'x'), set(proto, name, {}, 'x')]);
    const root = document.documentElement;
    const before = [root.childNodes.length, root.parentNode === document];
    r.documentOuter = [named(() => { root.outerText = 'x'; }), before, [root.childNodes.length, root.parentNode === document]];
    const detached = document.createElement('span');
    r.detachedOuter = named(() => { detached.outerText = 'x'; });
    out.text = r;
  }

  // Keyframe values: negative zero, parenthesized blocks, empty and invalid values, the
  // zero-length properties.
  {
    const r = {};
    const target = body.appendChild(document.createElement('div'));
    const frames = frame => caught(() => { const animation = target.animate([frame], 1000); const value = animation.effect.getKeyframes(); animation.cancel(); return value; });
    r.negativeZero = frames({ opacity: '-0.0000000000001' });
    r.negativeSmall = frames({ translate: '-0.0000000000001px' });
    r.paren = frames({ width: 'var(--w, (1PX))' });
    r.empty = frames({ width: '' });
    r.invalid = frames({ width: 'red', height: '1px' });
    r.zeros = frames({ top: '0', right: '0', bottom: '0', left: '0', flexBasis: '0', outlineOffset: '0', columnWidth: '0', columnGap: '0', opacity: '0' });
    target.remove();
    out.keyframes = r;
  }

  // HTMLScriptElement async / text.
  {
    const r = {};
    const script = document.createElement('script');
    r.initial = [script.async, script.hasAttribute('async')];
    script.async = true; r.on = [script.async, script.getAttribute('async')];
    script.async = false; r.off = [script.async, script.hasAttribute('async')];
    r.textThrows = message(() => { script.text = { toString() { throw new RangeError('poison'); } }; });
    r.text = [script.text, script.childNodes.length];
    out.script = r;
  }

  // XPath: receivers, comment / processing-instruction / xmlns-attribute snapshot rows.
  {
    const r = {};
    const div = html('<div id=x>a<!--note--><span id=s a=1 b=2></span></div>');
    div.appendChild(document.createProcessingInstruction('target', 'data'));
    div.firstChild.nextSibling.nextSibling.setAttributeNS('http://www.w3.org/2000/xmlns/', 'xmlns:q', 'urn:q');
    const evaluate = (expression, type) => caught(() => { const result = document.evaluate(expression, div, null, type, null); return type === 2 ? result.stringValue : type === 1 ? result.numberValue : result.snapshotLength; });
    r.comments = [evaluate('count(.//comment())', 1), evaluate('string(.//comment())', 2)];
    r.pis = [evaluate('count(.//processing-instruction())', 1), evaluate('name(.//processing-instruction())', 2), evaluate('string(.//processing-instruction("target"))', 2)];
    r.attributes = [evaluate('count(.//@*)', 1), evaluate('name(.//span/@*[2])', 2)];
    const expression = document.createExpression('1');
    const result = document.evaluate('1', document, null, 1, null);
    r.receivers = [
      caught(() => XPathExpression.prototype.evaluate.call(1, document)),
      caught(() => XPathExpression.prototype.evaluate.call({}, document)),
      caught(() => expression.evaluate(document).numberValue),
      get(XPathResult.prototype, 'numberValue', 1),
      get(XPathResult.prototype, 'resultType', 's'),
      get(XPathResult.prototype, 'numberValue', result),
      caught(() => XPathEvaluator.prototype.evaluate.call(1, '1', document)),
      caught(() => new XPathEvaluator().evaluate('1', document).numberValue),
    ];
    out.xpath = r;
  }

  // ShadowRoot.referenceTarget receivers and its declarative serialization escapes.
  {
    const r = {};
    const proto = ShadowRoot.prototype;
    r.receivers = [get(proto, 'referenceTarget', 1), get(proto, 'referenceTarget', {}), get(proto, 'referenceTarget', document.body), set(proto, 'referenceTarget', 1, 'x'), set(proto, 'referenceTarget', document.body, 'x')];
    const host = html('<div></div>');
    const root = host.attachShadow({ mode: 'open', serializable: true });
    root.referenceTarget = 'a b<c>d&e"f';
    r.serialized = box.getHTML({ serializableShadowRoots: true });
    out.referenceTarget = r;
  }

  // Inline event handlers: the form scope, syntax errors.
  {
    const r = {};
    html('<form id=f action=/go><p><button id=b type=button onclick="globalThis.__seen = [typeof elements, typeof action, id]">b</button></p></form>');
    box.querySelector('#b').click();
    r.formScope = globalThis.__seen; delete globalThis.__seen;
    const broken = html('<div onclick="}">x</div>');
    const errors = [];
    const listener = event => { errors.push(event.error && event.error.name); event.preventDefault(); };
    window.addEventListener('error', listener);
    r.syntax = [broken.onclick, broken.getAttribute('onclick')];
    window.removeEventListener('error', listener);
    r.reported = errors;
    out.handlers = r;
  }

  // Interest-invoker selector rewriting: comments and look-alike names.
  {
    const r = {};
    html('<button id=i interestfor=t class=interest-source>i</button><div id=t popover>t</div>');
    r.lookalike = caught(() => [...box.querySelectorAll('.interest-source, [interestfor]')].map(e => e.id));
    r.comment = caught(() => [...box.querySelectorAll('button:not(:interest-source/* c */)')].map(e => e.id));
    r.matches = caught(() => box.firstChild.matches(':interest-source'));
    r.topComment = caught(() => [...box.querySelectorAll('/*a*/button/*b*/:not(:interest-source),:interest-target')].map(e => e.id));
    out.interest = r;
  }

  // Hyperlink utils, input members, select definers, ARIA relations, the style shorthands.
  {
    const r = {};
    const a = html('<a href="http://u:p@example.com:8080/p/q?s=1#h">a</a>');
    r.link = ['origin', 'protocol', 'username', 'password', 'host', 'hostname', 'port', 'pathname', 'search', 'hash'].map(name => a[name]);
    r.linkReceivers = [get(HTMLAnchorElement.prototype, 'hash', 1), get(HTMLAnchorElement.prototype, 'hash', {}), set(HTMLAnchorElement.prototype, 'hash', 1, 'x')];
    const proto = HTMLInputElement.prototype;
    const text = document.createElement('div');
    r.inputReceivers = [get(proto, 'indeterminate', 1), get(proto, 'indeterminate', text), set(proto, 'indeterminate', 1, true), get(proto, 'list', 1), get(proto, 'list', text)];
    const select = html('<select><option>a</option><option>b</option></select>');
    r.selectDefine = [caught(() => Reflect.defineProperty(select, 0, { value: new Option('z') })), caught(() => Reflect.defineProperty(select, 5, {})), select.length, [...select.options].map(o => o.text)];
    r.selectAccessor = message(() => Object.defineProperty(select, 0, { get() { return null; } }));
    const owner = html('<div aria-controls=x></div><span id=x></span>');
    r.ariaFresh = [owner.getAttribute('aria-controls'), owner.ariaControlsElements && owner.ariaControlsElements.map(e => e.id)];
    const plain = document.createElement('div');
    plain.classList.add('c'); plain.dataset.k = '1';
    plain.setAttribute('aria-label', 'l'); plain.setAttribute('aria-controls', 'x');
    r.ariaPlain = [plain.ariaLabel, plain.ariaControlsElements];
    owner.ariaControlsElements = [box.lastChild];
    owner.setAttribute('aria-controls', 'x');
    r.ariaReset = [owner.getAttribute('aria-controls'), owner.ariaControlsElements.map(e => e.id)];
    const style = document.createElement('div').style;
    style.cssText = 'margin: 1px 2px; font: 12px serif';
    r.shorthand = [style.margin, style.getPropertyValue('margin-top'), style.font, style.cssText];
    style.setProperty('inset', '1px'); r.inset = [style.inset, style.top];
    // Indexed definitions on a non-select element and on the select prototype pass through.
    r.defineDiv = caught(() => { const div = document.createElement('div'); Object.defineProperty(div, 0, { value: 'v', configurable: true }); return [div[0], div.hasOwnProperty(0)]; });
    r.defineProto = caught(() => { const done = Reflect.defineProperty(HTMLSelectElement.prototype, 0, { value: 'p', configurable: true }); const own = HTMLSelectElement.prototype.hasOwnProperty(0); delete HTMLSelectElement.prototype[0]; return [done, own]; });
    const cleared = document.createElement('div'); cleared.textContent = 'x'; cleared.textContent = null;
    r.textNull = [cleared.textContent, cleared.childNodes.length];
    r.states = [caught(() => CustomStateSet.prototype.add.call(1, 'x')), caught(() => CustomStateSet.prototype.has.call({}, 'x')), caught(() => CustomStateSet.prototype.delete.call('s', 'x'))];
    r.capture = [caught(() => Element.prototype.setPointerCapture.call(1, 1)), caught(() => Element.prototype.hasPointerCapture.call({}, 1))];
    out.members = r;
  }

  // focusgroup arrows: dir=auto without a strong character, an unknown dir, vertical ArrowLeft.
  {
    const deep = () => { let active = document.activeElement; while (active && active.shadowRoot && active.shadowRoot.activeElement) active = active.shadowRoot.activeElement; return active; };
    const press = async (...keys) => {
      const result = [];
      for (const key of keys) { await input({ type: 'key', key, code: key, keyCode: codes[key], modifiers: 0 }); const active = deep(); result.push(active.id || active.localName); }
      return result;
    };
    const scenario = async (markup, start, keys) => { box.innerHTML = markup; document.getElementById(start).focus(); return press(...keys); };
    const three = '<button id=a>a</button><button id=b>b</button><button id=c>c</button>';
    const r = {};
    r.autoDigits = await scenario('<div dir=auto focusgroup=toolbar><button id=a>1</button><!--x--><span dir=rtl>א</span><style>p{}</style><button id=b>2</button></div>', 'a', ['ArrowRight', 'ArrowLeft', 'ArrowLeft']);
    r.autoComment = await scenario('<div dir=auto focusgroup=toolbar><!--א--><button id=a>1</button><button id=b>2</button></div>', 'a', ['ArrowRight', 'ArrowLeft']);
    r.unknownDir = await scenario('<div dir=rtl><div dir=bogus focusgroup=toolbar>' + three + '</div></div>', 'a', ['ArrowRight', 'ArrowLeft']);
    r.verticalLeft = await scenario('<div style="writing-mode:vertical-rl" focusgroup="toolbar wrap">' + three + '</div>', 'b', ['ArrowLeft', 'ArrowLeft', 'ArrowRight']);
    r.verticalLr = await scenario('<div style="writing-mode:vertical-lr" focusgroup="listbox">' + three + '</div>', 'b', ['ArrowLeft', 'ArrowRight', 'ArrowDown']);
    out.focusgroup = r;
  }
  box.remove();
  return out;
})();
