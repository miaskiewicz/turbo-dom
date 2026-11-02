(async () => {
  const out = {}, body = document.body, q = selector => body.querySelector(selector);
  const name = node => node === null ? null : node === undefined ? 'undefined' : node.id || node.localName || node.nodeName;
  const caught = fn => { try { return fn(); } catch (error) { return 'E:' + error.name + ':' + error.message; } };
  const describe = (proto, key) => {
    const d = Object.getOwnPropertyDescriptor(proto, key);
    if (!d) return null;
    const fn = f => f ? [f.name, f.length, caught(() => new f())] : null;
    return [d.enumerable, d.configurable, fn(d.get), fn(d.set), 'value' in d];
  };

  // ShadowRoot.referenceTarget and HTMLTemplateElement.shadowRootReferenceTarget surface.
  {
    const r = {};
    r.descriptors = [describe(ShadowRoot.prototype, 'referenceTarget'), describe(HTMLTemplateElement.prototype, 'shadowRootReferenceTarget'), 'referenceTarget' in Element.prototype, 'referenceTarget' in Document.prototype];
    const root = document.createElement('div').attachShadow({ mode: 'open' });
    r.initial = root.referenceTarget;
    r.values = [];
    for (const value of ['a', '', null, undefined, 5, ' x ', 'A B', {}, true]) { root.referenceTarget = value; r.values.push(root.referenceTarget); }
    r.symbol = caught(() => { root.referenceTarget = Symbol('s'); });
    r.afterSymbol = root.referenceTarget;
    const getter = Object.getOwnPropertyDescriptor(ShadowRoot.prototype, 'referenceTarget').get, setter = Object.getOwnPropertyDescriptor(ShadowRoot.prototype, 'referenceTarget').set;
    r.brands = [document.createElement('div'), document, document.createDocumentFragment(), {}, null].map(receiver => [caught(() => getter.call(receiver)), caught(() => setter.call(receiver, 'x'))]);
    r.attach = [null, undefined, '', 'x', 0, false].map(value => caught(() => document.createElement('div').attachShadow({ mode: 'open', referenceTarget: value }).referenceTarget));
    r.attachMissing = document.createElement('div').attachShadow({ mode: 'closed' }).referenceTarget;
    r.attachSymbol = caught(() => document.createElement('div').attachShadow({ mode: 'open', referenceTarget: Symbol('s') }));
    const order = [];
    const init = {};
    for (const key of ['clonable', 'customElementRegistry', 'delegatesFocus', 'mode', 'referenceTarget', 'serializable', 'slotAssignment']) Object.defineProperty(init, key, { get() { order.push(key); return key === 'mode' ? 'open' : key === 'referenceTarget' ? 'z' : key === 'slotAssignment' ? 'named' : undefined; } });
    r.order = [order, caught(() => document.createElement('div').attachShadow(init).referenceTarget)];
    r.orderOnBadMode = (() => { const seen = []; caught(() => document.createElement('div').attachShadow({ get mode() { seen.push('mode'); return 'bogus'; }, get referenceTarget() { seen.push('rt'); return 'z'; } })); return seen; })();
    const template = document.createElement('template');
    r.template = [template.shadowRootReferenceTarget];
    for (const value of ['foo', '', null, undefined, 7]) { template.shadowRootReferenceTarget = value; r.template.push([template.getAttribute('shadowrootreferencetarget'), template.shadowRootReferenceTarget]); }
    template.setAttribute('shadowrootreferencetarget', 'ATTR'); r.template.push(template.shadowRootReferenceTarget);
    r.templateSymbol = caught(() => { template.shadowRootReferenceTarget = Symbol('s'); });
    r.templateBrand = caught(() => Object.getOwnPropertyDescriptor(HTMLTemplateElement.prototype, 'shadowRootReferenceTarget').get.call(document.createElement('div')));
    out.surface = r;
  }

  // Declarative roots, serialization and clonable copies.
  {
    const r = {};
    body.setHTMLUnsafe('<div id=h1><template shadowrootmode=open shadowrootreferencetarget=inner><input id=inner></template></div><div id=h2><template shadowrootmode=closed shadowrootreferencetarget=""></template></div><div id=h3><template shadowrootmode=open></template></div><div id=h4><template shadowrootmode=open shadowrootreferencetarget="a&quot;b&amp;c&lt;"></template></div>');
    r.declarative = [q('#h1').shadowRoot.referenceTarget, q('#h3').shadowRoot.referenceTarget, q('#h4').shadowRoot.referenceTarget];
    r.declarativeTemplate = q('#h1').querySelector('template') === null;
    body.setHTMLUnsafe('<div id=s1><template shadowrootmode=open shadowrootserializable shadowrootreferencetarget=inner><input id=inner></template></div><div id=s2><template shadowrootmode=closed shadowrootserializable shadowrootclonable shadowrootdelegatesfocus shadowrootreferencetarget="a&quot;b&amp;c<"></template></div><div id=s3><template shadowrootmode=open shadowrootserializable shadowrootreferencetarget=""></template></div><div id=s4><template shadowrootmode=open shadowrootserializable></template></div>');
    r.serialized = body.getHTML({ serializableShadowRoots: true });
    const host = document.createElement('div'); host.id = 'clone';
    host.attachShadow({ mode: 'open', clonable: true, referenceTarget: 'in' }).innerHTML = '<input id=in>';
    const copy = host.cloneNode(false);
    r.clone = [copy.shadowRoot.referenceTarget, copy.shadowRoot.firstChild.id];
    host.shadowRoot.referenceTarget = null; r.cloneNull = host.cloneNode(true).shadowRoot.referenceTarget;
    const plain = document.createElement('div'); plain.attachShadow({ mode: 'open', referenceTarget: 'x' });
    r.unclonable = plain.cloneNode(true).shadowRoot;
    const imported = document.importNode(copy, true); r.imported = imported.shadowRoot.referenceTarget;
    const other = document.implementation.createHTMLDocument(''); other.body.append(copy); r.adopted = [copy.shadowRoot.referenceTarget, copy.ownerDocument === other];
    out.declarative = r;
  }

  // Label `for`, input `list`, and the element reflection getters that do not resolve.
  {
    const r = {};
    body.innerHTML = '<label id=lab for=host>L</label><input id=li list=host><div id=host></div><div id=ar aria-labelledby="host x" aria-activedescendant=host></div><span id=x></span><button id=b1 popovertarget=host></button><button id=b2 commandfor=host command=toggle-popover></button><output id=out for=host></output>';
    const root = q('#host').attachShadow({ mode: 'open', referenceTarget: 'in' });
    root.innerHTML = '<input id=in><datalist id=dl></datalist><div id=pop popover></div><span id=plain></span>';
    const inner = root.querySelector('#in'), state = () => [name(q('#lab').control), name(q('#li').list), name(q('#b1').popoverTargetElement), name(q('#b2').commandForElement), q('#ar').ariaLabelledByElements.map(name), name(q('#ar').ariaActiveDescendantElement), [...q('#out').htmlFor]];
    r.input = state();
    root.referenceTarget = 'dl'; r.datalist = state();
    root.referenceTarget = 'plain'; r.plain = state();
    root.referenceTarget = 'missing'; r.missing = state();
    root.referenceTarget = null; r.none = state();
    root.referenceTarget = ''; r.empty = state();
    root.referenceTarget = 'in'; r.labels = [...inner.labels].map(name);
    // Chrome's cached labels list is not invalidated by referenceTarget changes; ID changes do.
    inner.id = 'renamed'; r.renamed = [...state(), [...inner.labels].map(name)];
    inner.id = 'in'; r.restored = [...state(), [...inner.labels].map(name)];
    const nested = document.createElement('div'); nested.id = 'deep'; root.prepend(nested);
    root.referenceTarget = 'deep';
    const deep = nested.attachShadow({ mode: 'closed', referenceTarget: 'leaf' }); deep.innerHTML = '<button id=leaf></button><datalist id=leafList></datalist>';
    r.nested = [name(q('#lab').control), [...deep.querySelector('#leaf').labels].map(name), name(q('#li').list)];
    deep.querySelector('#leaf').id = 'gone'; r.nestedMissing = [name(q('#lab').control), name(q('#li').list)];
    deep.querySelector('#leafList').id = 'leaf'; r.nestedList = [name(q('#lab').control), name(q('#li').list)];
    // Duplicate IDs inside the root: the first in tree order wins.
    root.innerHTML = '<span id=dup></span><input id=dup>'; root.referenceTarget = 'dup';
    r.duplicate = name(q('#lab').control);
    // A label inside the shadow tree, and a label whose own root has a reference target.
    root.innerHTML = '<label id=inside for=in2></label><input id=in2>'; root.referenceTarget = 'in2';
    r.inside = [name(root.querySelector('#inside').control), [...root.querySelector('#in2').labels].map(name)];
    // A wrapping label is not an ID reference.
    body.innerHTML = '<label id=wrap><div id=wh></div></label>';
    q('#wh').attachShadow({ mode: 'open', referenceTarget: 'wi' }).innerHTML = '<input id=wi>';
    r.wrapping = [name(q('#wrap').control), q('#wh').shadowRoot.querySelector('#wi').labels.length];
    body.innerHTML = '<label id=wrap2><div id=wh2></div><input id=after></label>';
    q('#wh2').attachShadow({ mode: 'open' }).innerHTML = '<input id=wi2>';
    r.wrappingPlain = [name(q('#wrap2').control), q('#wh2').shadowRoot.querySelector('#wi2').labels.length];
    q('#wh2').shadowRoot.referenceTarget = 'missing'; r.wrappingMissing = name(q('#wrap2').control);
    // Labels across trees are listed in shadow-including tree order.
    body.innerHTML = '<label id=o1 for=oh></label><div id=oh></div><label id=o2 for=oh></label>';
    const orderRoot = q('#oh').attachShadow({ mode: 'open', referenceTarget: 'oi' });
    orderRoot.innerHTML = '<label id=i1 for=oi></label><input id=oi><label id=i2><input id=other></label>';
    r.order = [...orderRoot.querySelector('#oi').labels].map(name);
    // Labelable custom host: its own reference target wins over its labelability.
    class Field extends HTMLElement { static formAssociated = true; constructor() { super(); this.internals = this.attachInternals(); } }
    customElements.define('rt-field', Field);
    body.innerHTML = '<label id=cl for=fh></label><rt-field id=fh></rt-field>';
    r.customBefore = [name(q('#cl').control), q('#fh').internals.labels.length];
    q('#fh').attachShadow({ mode: 'open', referenceTarget: 'fi' }).innerHTML = '<input id=fi>';
    r.customAfter = [name(q('#cl').control), q('#fh').internals.labels.length, q('#fh').shadowRoot.querySelector('#fi').labels.length];
    q('#fh').shadowRoot.referenceTarget = 'none'; r.customMissing = name(q('#cl').control);
    out.references = r;
  }

  // Invoker activation continues through the host to the reference target.
  {
    const r = {}, log = [];
    body.innerHTML = '<button id=t1 popovertarget=ph></button><button id=t2 commandfor=ph command=show-popover></button><button id=t3 commandfor=ph command=--custom></button><div id=ph></div><button id=t4 popovertarget=missing></button>';
    const root = q('#ph').attachShadow({ mode: 'closed', referenceTarget: 'pp' });
    root.innerHTML = '<div id=pp popover>pop</div><div id=other popover></div>';
    const pp = root.querySelector('#pp');
    for (const type of ['beforetoggle', 'toggle', 'command']) {
      pp.addEventListener(type, event => log.push(['pp', type, event.newState || event.command, name(event.source), name(event.target)]));
      q('#ph').addEventListener(type, event => log.push(['host', type, event.newState || event.command, name(event.source), name(event.target)]));
    }
    const tick = () => new Promise(resolve => setTimeout(resolve, 0));
    q('#t1').click(); await tick(); log.push(['open', pp.matches(':popover-open')]);
    q('#t1').click(); await tick(); log.push(['closed', pp.matches(':popover-open')]);
    q('#t2').click(); await tick(); log.push(['shown', pp.matches(':popover-open')]);
    pp.hidePopover(); await tick();
    q('#t3').click(); await tick();
    root.referenceTarget = null; q('#t1').click(); await tick(); log.push(['no-target', pp.matches(':popover-open'), q('#ph').matches(':popover-open')]);
    root.referenceTarget = 'missing'; q('#ph').popover = 'manual'; q('#t1').click(); await tick(); log.push(['missing-host', pp.matches(':popover-open'), q('#ph').matches(':popover-open')]);
    q('#ph').hidePopover(); q('#ph').removeAttribute('popover'); await tick();
    root.referenceTarget = 'other'; q('#t2').click(); await tick(); log.push(['other', root.querySelector('#other').matches(':popover-open')]);
    // Explicit element references to the host resolve the same way.
    root.querySelector('#other').hidePopover(); await tick(); log.length && log.push('---');
    const explicit = document.createElement('button'); body.append(explicit); explicit.popoverTargetElement = q('#ph'); root.referenceTarget = 'pp';
    explicit.click(); await tick(); log.push(['explicit', name(explicit.popoverTargetElement), pp.matches(':popover-open')]);
    r.log = log;
    out.invokers = r;
  }

  // Form owners through a reference target: Chrome associates the control with the inner form.
  {
    const r = {};
    body.innerHTML = '<div id=fh></div><input id=fi form=fh name=a value=1>';
    const root = q('#fh').attachShadow({ mode: 'open', referenceTarget: 'f' }); root.innerHTML = '<form id=f><input name=b value=2></form>';
    r.owner = [name(q('#fi').form), [...root.querySelector('#f').elements].map(name)];
    out.forms = r;
  }
  return out;
})()
