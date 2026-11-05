(() => {
  'use strict';
  // turbo-dom#340: Animation.rangeStart / rangeEnd (the animation attachment range): the
  // getters, the setters' string / TimelineRangeOffset conversions and errors,
  // KeyframeAnimationOptions, and a CSSAnimation's animation-range.
  const show = (v) => v === null || typeof v !== 'object' ? v : v instanceof CSSStyleValue ? { css: String(v), ctor: v.constructor.name } : Object.fromEntries(Object.keys(v).map((k) => [k, show(v[k])]).concat([['proto', Object.getPrototypeOf(v) === Object.prototype ? 'Object' : String(Object.getPrototypeOf(v))]]));
  const E = (f) => { try { return show(f()); } catch (e) { return 'throw ' + e.name + ': ' + e.message; } };
  const el = document.createElement('div'); document.body.append(el);
  const out = {};
  const a = el.animate([{ opacity: 0 }, { opacity: 1 }], 1000);
  out.desc = ['rangeStart', 'rangeEnd'].map((n) => { const d = Object.getOwnPropertyDescriptor(Animation.prototype, n); return d && [typeof d.get, typeof d.set, d.enumerable, d.configurable, d.get && d.get.name, d.set && d.set.name, d.get && d.get.length, d.set && d.set.length]; });
  out.keys = Object.getOwnPropertyNames(Animation.prototype);
  out.initial = [E(() => a.rangeStart), E(() => a.rangeEnd)];
  const sets = ['entry 10%', 'entry', 'exit 20px', 'cover', '10%', '20px', 'normal', 'bogus', 'entry 10% 20%', 'ENTRY 10%', 'contain calc(10% + 1px)', 'entry-crossing 0%', 'scroll 50%', '', 'entry 0', '1em', 'entry 1in', 'entry -10%', 'entry 1.5px', '  entry   10%  ', 'none 10%', 'entry 1e1%', 'entry +5px', 'cover 10vw', 'entry calc(5%)', 'normal 10%', 'NORMAL'];
  out.sets = sets.map((v) => { const b = el.animate(null, 1000); const r = E(() => { b.rangeStart = v; return b.rangeStart; }); const r2 = E(() => { b.rangeEnd = v; return b.rangeEnd; }); return [v, r, r2]; });
  const objs = [() => ({ rangeName: 'entry', offset: CSS.percent(10) }), () => ({ rangeName: 'exit' }), () => ({ offset: CSS.px(10) }), () => ({}), () => ({ rangeName: 'bogus', offset: CSS.percent(1) }), () => ({ rangeName: 'none', offset: CSS.percent(1) }), () => CSS.percent(30), () => CSS.px(5), () => CSS.number(1), () => ({ rangeName: 'entry', offset: CSS.number(1) }), () => ({ rangeName: 'cover', offset: CSS.px(3) }), () => null, () => 5, () => ({ rangeName: 'entry', offset: '10%' }), () => ({ rangeName: 'entry', offset: CSS.em(1) }), () => ({ rangeName: 'entry', offset: CSS.in(1) }), () => ({ rangeName: 'entry', offset: CSS.percent(10).add(CSS.px(1)) }), () => ({ rangeName: 'ENTRY', offset: CSS.px(1) }), () => new CSSKeywordValue('normal'), () => ({ get offset() { throw new Error('o'); }, get rangeName() { throw new Error('n'); } }), () => ({ rangeName: 'entry', offset: CSS.percent(-5) })];
  out.objs = objs.map((f) => { const b = el.animate(null, 1000); return [String(f).slice(6), E(() => { b.rangeStart = f(); return b.rangeStart; }), E(() => { b.rangeEnd = f(); return b.rangeEnd; })]; });
  out.options = ['entry 10%', 'normal', 'cover', 'bogus', 'exit 5px'].map((v) => [v, E(() => { const b = el.animate(null, { duration: 1000, rangeStart: v, rangeEnd: v }); return [b.rangeStart, b.rangeEnd]; })]);
  out.optionsMore = [() => ({ rangeEnd: { rangeName: 'exit' } }), () => ({ rangeEnd: 'exit' }), () => ({ rangeStart: 'exit' }), () => ({ rangeEnd: '10%' }), () => ({ rangeEnd: { offset: CSS.px(2) } }), () => ({ rangeStart: { rangeName: 'bogus' } }), () => ({ rangeEnd: 5 }), () => ({ rangeStart: { rangeName: 'entry', offset: CSS.number(1) } })].map((f) => [String(f).slice(6), E(() => { const b = el.animate(null, Object.assign({ duration: 1000 }, f())); return [b.rangeStart, b.rangeEnd]; })]);
  out.optionsObj = E(() => { const b = el.animate(null, { duration: 1000, rangeStart: { rangeName: 'entry', offset: CSS.percent(5) } }); return b.rangeStart; });
  out.newAnim = E(() => { const b = new Animation(new KeyframeEffect(el, null, 1000)); return [b.rangeStart, b.rangeEnd]; });
  out.identity = E(() => { const b = el.animate(null, 1000); b.rangeStart = 'entry 10%'; return [b.rangeStart === b.rangeStart, b.rangeStart.offset === b.rangeStart.offset]; });
  // CSS animation
  const s = document.createElement('style'); s.textContent = '@keyframes k { to { opacity: 0 } } .c { animation: k 1s; animation-range: entry 10% exit 90%; } .d { animation: k 1s; animation-range-start: cover 20px; }'; document.head.append(s);
  const c = document.createElement('div'); c.className = 'c'; document.body.append(c); const d = document.createElement('div'); d.className = 'd'; document.body.append(d);
  out.css = [E(() => { const x = c.getAnimations()[0]; return [x.rangeStart, x.rangeEnd]; }), E(() => { const x = d.getAnimations()[0]; return [x.rangeStart, x.rangeEnd]; })];
  out.cssSet = E(() => { const x = c.getAnimations()[0]; x.rangeStart = 'contain 5%'; c.style.animationRangeStart = 'exit 1%'; getComputedStyle(c).opacity; return [x.rangeStart, getComputedStyle(c).animationRangeStart]; });
  out.thisCheck = E(() => Object.getOwnPropertyDescriptor(Animation.prototype, 'rangeStart').get.call({}));
  out.symbol = E(() => { const b = el.animate(null, 1000); b.rangeStart = Symbol(); });
  return out;
})()
