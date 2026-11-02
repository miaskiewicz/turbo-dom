(() => {
  // How each interceptor answers in every caller mode: sloppy and strict
  // assignment/deletion, Object.defineProperty, the Reflect forms, Object.assign
  // (always throwing [[Set]]), and a sloppy callee reached from strict code.
  const capture = (body) => {
    try {
      const value = body();
      return { value: value === undefined ? 'undefined' : value };
    } catch (error) {
      return { name: error.name, message: error.message, typeError: error instanceof TypeError };
    }
  };
  // The harness document may carry a title; the collection is compared whole.
  document.head.replaceChildren();
  document.body.innerHTML = '<div id="first"></div><select id="pick"><option>a</option><option>b</option></select>';
  const all = document.all;
  const select = document.getElementById('pick');
  const sloppySet = Function('target', 'key', 'target[key] = 1; return "no-throw";');
  const sloppyDelete = Function('target', 'key', 'return delete target[key];');
  const modes = (target, index, named) => ({
    sloppySetIndex: capture(() => sloppySet(target, index)),
    strictSetIndex: capture(() => { 'use strict'; target[index] = 1; return 'no-throw'; }),
    strictCallsSloppySet: capture(() => { 'use strict'; return sloppySet(target, index); }),
    evalStrictSet: capture(() => { globalThis.__modeTarget = target; return (0, eval)(`'use strict'; __modeTarget[${index}] = 1; 'no-throw'`); }),
    assignIndex: capture(() => { Object.assign(target, { [index]: 1 }); return 'no-throw'; }),
    reflectSetIndex: capture(() => Reflect.set(target, index, 1)),
    defineIndex: capture(() => { Object.defineProperty(target, index, { value: 1 }); return 'no-throw'; }),
    defineAccessorIndex: capture(() => { Object.defineProperty(target, index, { get() { return 1; } }); return 'no-throw'; }),
    reflectDefineIndex: capture(() => Reflect.defineProperty(target, index, { value: 1 })),
    reflectDefineAccessorIndex: capture(() => Reflect.defineProperty(target, index, { get() { return 1; } })),
    sloppyDeleteIndex: capture(() => sloppyDelete(target, index)),
    strictDeleteIndex: capture(() => { 'use strict'; return delete target[index]; }),
    reflectDeleteIndex: capture(() => Reflect.deleteProperty(target, index)),
    ...(named ? {
      sloppySetNamed: capture(() => sloppySet(target, named)),
      strictSetNamed: capture(() => { 'use strict'; target[named] = 1; return 'no-throw'; }),
      assignNamed: capture(() => { Object.assign(target, { [named]: 1 }); return 'no-throw'; }),
      reflectSetNamed: capture(() => Reflect.set(target, named, 1)),
      defineNamed: capture(() => { Object.defineProperty(target, named, { value: 1 }); return 'no-throw'; }),
      defineAccessorNamed: capture(() => { Object.defineProperty(target, named, { get() { return 1; } }); return 'no-throw'; }),
      reflectDefineNamed: capture(() => Reflect.defineProperty(target, named, { value: 1 })),
      reflectDefineAccessorNamed: capture(() => Reflect.defineProperty(target, named, { get() { return 1; } })),
      sloppyDeleteNamed: capture(() => sloppyDelete(target, named)),
      strictDeleteNamed: capture(() => { 'use strict'; return delete target[named]; }),
      reflectDeleteNamed: capture(() => Reflect.deleteProperty(target, named)),
    } : {}),
  });
  const all0 = modes(all, 0, 'first');
  const select0 = modes(select, 0, null);
  delete globalThis.__modeTarget;
  return {
    all: all0,
    allIntact: [all.length, Array.from(all, (node) => node.localName).join(), all.first && all.first.id],
    select: select0,
    selectIntact: Array.from(select.options, (option) => option.text),
    selectStrictWrongType: capture(() => { 'use strict'; select[0] = 'x'; return 'no-throw'; }),
    selectSloppyWrongType: capture(() => Function('s', 's[0] = "x"; return "no-throw";')(select)),
    selectReflectWrongType: capture(() => Reflect.set(select, 0, 'x')),
  };
})()
