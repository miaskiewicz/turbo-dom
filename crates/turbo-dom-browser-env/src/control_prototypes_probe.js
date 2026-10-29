(() => {
  const inputNames=['accept','autocomplete','checked','defaultChecked','defaultValue','disabled','form','max','min','multiple','name','pattern','placeholder','readOnly','required','selectionDirection','selectionEnd','selectionStart','step','type','value','setRangeText','setSelectionRange'];
  const buttonNames=['disabled','form','name','type','value'];
  const extras=['accept','autoFocus','autocomplete','checked','click','max','min','multiple','pattern','placeholder','readOnly','required','step'];
  const describe=(prototype,name)=>{const d=Object.getOwnPropertyDescriptor(prototype,name);if(!d)return null;const f=value=>typeof value==='function'?[value.name,value.length]:null;return {enumerable:d.enumerable,configurable:d.configurable,writable:d.writable??null,get:f(d.get),set:f(d.set),value:f(d.value)};};
  const snap=element=>[element.value,element.selectionStart,element.selectionEnd,element.selectionDirection];
  const error=action=>{try{return {value:action()};}catch(reason){return {error:reason.name};}};
  const form=document.createElement('form'),input=document.createElement('input'),button=document.createElement('button');form.append(input,button);document.body.append(form);
  input.name='q';input.placeholder='search';input.accept='text/plain';input.multiple=true;input.disabled=true;input.disabled=false;input.defaultValue='default';input.value='abcdef';
  button.name='go';button.value='send';button.type='button';button.disabled=true;
  const reflection=[input.getAttribute('name'),input.placeholder,input.accept,input.multiple,input.disabled,input.defaultValue,input.value,input.form===form,button.name,button.value,button.type,button.disabled,button.form===form];
  const selections=[];
  selections.push(snap(input));input.setSelectionRange(1,4,'backward');selections.push(snap(input));
  for(const mode of ['select','start','end','preserve']){input.value='abcdef';input.setSelectionRange(2,5,'backward');input.setRangeText('XY',1,4,mode);selections.push(snap(input));}
  input.value='abcdef';input.setSelectionRange(2,4);input.setRangeText('Z');selections.push(snap(input));
  input.selectionStart=99;selections.push(snap(input));input.selectionEnd=1;selections.push(snap(input));input.selectionDirection='bad';selections.push(snap(input));
  input.value='1234';input.setSelectionRange(3,1);selections.push(snap(input));input.select();selections.push(snap(input));
  const errors=[()=>input.setSelectionRange(1),()=>input.setRangeText(),()=>input.setRangeText('x',1),()=>input.setRangeText('x',4,1),()=>input.setRangeText('x',0,1,'bad'),()=>input.setRangeText(Symbol()),()=>HTMLInputElement.prototype.setRangeText.call(button,'x'),()=>Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').get.call(button)].map(error);
  input.type='number';const unsupported=[snap(input),error(()=>input.setSelectionRange(0,1)),error(()=>input.setRangeText('x')),error(()=>{input.selectionStart=1;})];
  const textarea=document.createElement('textarea');textarea.value='a\ud800bc';textarea.setSelectionRange(1,2);textarea.setRangeText('X');
  const clicked=document.createElement('input');clicked.type='checkbox';let count=0;clicked.addEventListener('click',()=>count++);clicked.click();
  return {input:Object.fromEntries(inputNames.map(name=>[name,describe(HTMLInputElement.prototype,name)])),button:Object.fromEntries(buttonNames.map(name=>[name,describe(HTMLButtonElement.prototype,name)])),inputExtras:['autoFocus','click'].filter(name=>Object.hasOwn(HTMLInputElement.prototype,name)),buttonExtras:extras.filter(name=>Object.hasOwn(HTMLButtonElement.prototype,name)),reflection,selections,errors,unsupported,textarea:snap(textarea),click:[count,clicked.checked]};
})();
