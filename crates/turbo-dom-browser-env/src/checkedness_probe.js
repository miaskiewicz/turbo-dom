(() => {
  const form=document.createElement('form');document.body.append(form);
  const input=document.createElement('input');input.type='checkbox';form.append(input);
  const snap=element=>[element.checked,element.defaultChecked,element.hasAttribute('checked'),element.matches(':checked'),element.outerHTML];
  const out=[];
  out.push(snap(input));input.defaultChecked=true;out.push(snap(input));
  input.checked=false;out.push(snap(input));input.defaultChecked=false;input.defaultChecked=true;out.push(snap(input));
  input.__checked=true;out.push(snap(input));
  const clone=input.cloneNode();out.push(snap(clone));clone.defaultChecked=false;clone.defaultChecked=true;out.push(snap(clone));
  form.reset();out.push(snap(input));input.defaultChecked=false;out.push(snap(input));
  input.setAttribute('checked','yes');out.push(snap(input));input.checked=false;input.setAttribute('checked','again');out.push(snap(input));
  const text=document.createElement('input');text.setAttribute('checked','');text.checked=true;out.push([text.checked,text.matches(':checked')]);
  const div=document.createElement('div');div.setAttribute('checked','');out.push(div.matches(':checked'));
  const protectedInput=document.createElement('input');protectedInput.type='checkbox';
  for(const name of ['getAttribute','setAttribute','removeAttribute','hasAttribute'])protectedInput[name]=()=>{throw Error('guest hook');};
  protectedInput.defaultChecked=true;protectedInput.checked=false;
  out.push([protectedInput.checked,protectedInput.defaultChecked,protectedInput.matches(':checked')]);
  return out;
})();
