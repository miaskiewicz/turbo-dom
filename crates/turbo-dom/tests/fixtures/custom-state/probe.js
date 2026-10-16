(async () => {
  const units = value => Array.from({length: value.length}, (_, index) => value.charCodeAt(index));
  customElements.define('x-native-state-control', class extends HTMLElement {
    constructor() { super(); this.internals = this.attachInternals(); this.internals.states.add('constructed'); }
  });
  const element = document.createElement('x-native-state-control');
  document.body.append(element);
  const states = element.internals.states;
  const values = ['', 'foo', 'Foo', '--ready', '-', '--', '0', '0foo', '-0foo', 'a b', ' ', 'initial', 'inherit', 'unset', 'revert', 'revert-layer', 'default', 'none', '\u0000', '\ud800', '\udc00', '\ufffd', '🔥', 'é', '\\', 'a\\b'];
  const rows = [];
  const match = selector => {try{return {value: element.matches(selector)};}catch(error){return {error: error.name};}};
  for (const value of values) {
    states.clear();
    let error = null;let returned;
    try{returned = states.add(value);}catch(reason){error = reason.name;}
    const escaped = CSS.escape(value);
    rows.push({units:units(value),error,returnsSelf:returned===states,size:states.size,has:states.has(value),stored:Array.from(states,units),selectorUnits:units(`:state(${escaped})`),match:match(`:state(${escaped})`),spaced:match(`:state( ${escaped} )`),upper:match(`:STATE(${escaped})`)});
  }
  states.clear();states.add('persist');
  const observed=[];const observer = new MutationObserver(records=>observed.push(...records.map(record=>record.type)));
  observer.observe(element,{attributes:true,childList:true,characterData:true,subtree:true});
  states.add('temporary');states.delete('temporary');
  await Promise.resolve();observer.disconnect();
  const clone=element.cloneNode(true);
  const other=document.implementation.createHTMLDocument('other');
  const imported=other.importNode(element,true);
  const adopted=other.adoptNode(element);
  const adoptedStates=Array.from(adopted.internals.states,units);
  const detachedMatch=adopted.matches(':state(persist)');
  document.body.append(adopted);
  document.body.append(imported);
  return {rows,lifecycle:{clone:Array.from(clone.internals.states,units),imported:Array.from(imported.internals.states,units),adopted:adoptedStates,adoptIdentity:adopted===element,detachedMatch,reinserted:element.matches(':state(persist)'),attributes:Array.from(element.attributes,attr=>[attr.name,attr.value]),mutationRecords:observed},selectors:[':state(foo bar)',':state()',':state("foo")',':state(foo,bar)',':state(foo/**/)',':state(/*x*/persist)',':state(persist/**/)',':state(per/**/sist)',':state(\\70 ersist)',':state(persist)',':is(:state(persist))',':not(:state(missing))'].map(selector=>({selector,match:match(selector)}))};
})();
